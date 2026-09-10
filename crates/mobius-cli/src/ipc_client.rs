use futures_util::{SinkExt, StreamExt};
use mobius_core::ipc::{DaemonEvent, IpcRequest};
use std::io::Write;
use std::process::Stdio;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const WS_URL: &str = "ws://127.0.0.1:43812";
const MAX_START_ATTEMPTS: u32 = 40;
const START_RETRY_DELAY: Duration = Duration::from_millis(100);
const BASH_START_DELAY: Duration = Duration::from_millis(500);

/// Formats a token count as K/M with one decimal, e.g. 52340 -> "52.3K".
fn fmt_tokens(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Locates the `mobius-daemon` binary: first next to the current executable
/// (so `cargo run`, debug, and release builds all work), then on PATH.
fn find_daemon_binary() -> Option<std::path::PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("mobius-daemon");
            if sibling.exists() {
                return Some(sibling);
            }
        }
    }
    // Fall back to a PATH lookup
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("mobius-daemon");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Returns true if a Mobius daemon is already listening on the default port.
async fn daemon_is_up() -> bool {
    match tokio::net::TcpStream::connect("127.0.0.1:43812").await {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Returns true if a Mobius daemon is already listening on the default port.
pub async fn daemon_running() -> bool {
    daemon_is_up().await
}

/// Spawns the daemon as a detached background service (stdout/stderr/stdin nulled)
/// and returns once it is accepting connections on the default port.
pub async fn spawn_daemon_detached() -> Result<(), String> {
    let daemon = find_daemon_binary().ok_or_else(|| {
        "Could not find the 'mobius-daemon' binary. Is the daemon crate built? Try: cargo build -p mobius-daemon"
            .to_string()
    })?;

    match std::process::Command::new(&daemon)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => {
            // Wait (bounded) for the daemon's listening socket to appear
            let mut attempts = 0;
            while !daemon_is_up().await && attempts < MAX_START_ATTEMPTS {
                sleep(START_RETRY_DELAY).await;
                attempts += 1;
            }
            // Extra grace for the WebSocket accept loop to spin up
            sleep(BASH_START_DELAY).await;

            if daemon_is_up().await {
                Ok(())
            } else {
                Err("Daemon failed to start or bind port in time.".to_string())
            }
        }
        Err(e) => Err(format!(
            "Failed to auto-spawn daemon '{}': {}",
            daemon.display(),
            e
        )),
    }
}

pub async fn send_to_daemon(request: IpcRequest) -> Result<(), Box<dyn std::error::Error>> {
    // Auto-start the daemon first if it isn't already running (unless we are
    // explicitly trying to stop it - never spawn a daemon just to kill it).
    if !request.shutdown && !daemon_is_up().await {
        spawn_daemon_detached().await.map_err(|e| {
            format!(
                "❌ Mobius daemon is not running. Failed to auto-start it: {}. \
                 You can also start it manually in another terminal with: cargo run -p mobius-daemon",
                e
            )
        })?;
    }

    // Connect to the WebSocket daemon
    let (mut ws_stream, _) = connect_async(WS_URL).await.map_err(|e| {
        format!(
            "❌ Failed to connect to Mobius Daemon. Is it running? ({})",
            e
        )
    })?;

    // Send the structured request
    let req_json = serde_json::to_string(&request)?;
    ws_stream.send(Message::Text(req_json)).await?;

    let mut stdout = std::io::stdout();
    let mut mobius_prefix_printed = false;

    // Stream and render events
    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            if let Ok(event) = serde_json::from_str::<DaemonEvent>(&text) {
                match event {
                    DaemonEvent::ThinkingChunk(chunk) => {
                        // Grey/Dim ANSI text for thinking blocks
                        print!("\x1b[90m{}\x1b[0m", chunk);
                        let _ = stdout.flush();
                    }
                    DaemonEvent::TextChunk(chunk) => {
                        // Prepend "Mobius: " (bold green) before the first response chunk
                        if !mobius_prefix_printed {
                            print!("\x1b[1;32mMobius: \x1b[0m");
                            mobius_prefix_printed = true;
                        }
                        // Standard terminal text
                        print!("{}", chunk);
                        let _ = stdout.flush();
                    }
                    DaemonEvent::ToolStart { tool_name, details } => {
                        print!(
                            "\n\x1b[33m[🛠️  Executing Tool: {} | {}]\x1b[0m\n",
                            tool_name, details
                        );
                        let _ = stdout.flush();
                    }
                    DaemonEvent::ToolFinished { .. } => {
                        // Tool finished (silently wait for the model's follow-up)
                    }
                    DaemonEvent::TokenUsage {
                        prompt,
                        completion: _,
                        context_window,
                    } => {
                        let pct = if context_window > 0 {
                            prompt as f64 * 100.0 / context_window as f64
                        } else {
                            0.0
                        };
                        print!(
                            "\n\n\x1b[90mContext: {:.1}% ({}/{})\x1b[0m\n",
                            pct,
                            fmt_tokens(prompt),
                            fmt_tokens(context_window),
                        );
                        let _ = stdout.flush();
                    }
                    DaemonEvent::LlmStatus { .. } => {
                        // GUI-only status event - CLI ignores it
                    }
                    DaemonEvent::Error(err) => {
                        eprintln!("\n\x1b[31m[Daemon Error]: {}\x1b[0m", err);
                    }
                    DaemonEvent::Done => {
                        println!(); // Print final newline when done
                        break;
                    }
                    DaemonEvent::SessionList(sessions) => {
                        for s in sessions {
                            let marker = if s.is_active { "●" } else { "○" };
                            println!("{} {}", marker, s.filename);
                            println!("   {}", s.preview);
                            println!("   {} messages", s.message_count);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
