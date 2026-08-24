// TODO: `-l` or `--last` flag
// TODO: `-m` or `--model` flag
// TODO: `--help` flag
// TODO: change `args.daemon_mode` to `args.start_daemon` to match `args.stop_daemon`
// TODO: Load past conversations
mod cli;
mod daemon;
mod engine;
mod ipc;
mod session;
mod tools;

use cli::{CliArgs, FlagAction};
use ipc::IpcRequest;
use std::env;
use std::io::{self, Write};
use std::process::Command;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    // 1. Parse Arguments
    let args = match CliArgs::parse() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("❌ Error: {}", err);
            std::process::exit(1);
        }
    };

    // 2. DAEMON MODE: If the hidden flag is present, start the server and block forever.
    if args.daemon_mode {
        daemon::start_daemon().await;
        return;
    }

    // 3. STOP DAEMON
    if args.stop_daemon {
        let socket_path = daemon::get_socket_path();
        if !socket_path.exists() {
            println!("🛑 Mobius daemon is not currently running.");
            return;
        }

        let mut stream = match UnixStream::connect(&socket_path).await {
            Ok(s) => s,
            Err(_) => {
                let _ = std::fs::remove_file(socket_path);
                println!("🧹 Cleaned up unresponsive daemon socket.");
                return;
            }
        };

        let request = IpcRequest {
            ppid: std::os::unix::process::parent_id(),
            prompt: String::new(),
            thinking_level_override: None,
            shutdown: true,
            record_history: None,
        };

        let json_payload = serde_json::to_string(&request).expect("Failed to serialize request");
        let _ = stream.write_all(json_payload.as_bytes()).await;
        let _ = stream.shutdown().await;

        let mut response = String::new();
        let _ = stream.read_to_string(&mut response).await;
        print!("{}", response);
        return;
    }

    // 4. HIDDEN RECORD HISTORY MODE (Used by shell hook)
    if let Some(ref cmd) = args.record_cmd {
        let ppid = args
            .override_pid
            .unwrap_or_else(|| std::os::unix::process::parent_id());

        // Read command output from stdin asynchronously
        let mut output = String::new();
        let mut stdin = tokio::io::stdin();
        let _ = stdin.read_to_string(&mut output).await;

        // Save directly to disk (bypasses daemon requirement)
        let mut history = session::TerminalHistory::load(ppid);
        history.push_entry(ppid, cmd.clone(), output);
        return;
    }

    // 5. CLIENT MODE (Querying flags without a prompt)
    if args.prompt.is_empty() {
        handle_queries(&args);
        return;
    }

    // 6. CLIENT MODE (Processing a prompt)
    let ppid = args
        .override_pid
        .unwrap_or_else(|| std::os::unix::process::parent_id());
    let socket_path = daemon::get_socket_path();

    // Connect or auto-spawn daemon
    let mut stream = match UnixStream::connect(&socket_path).await {
        Ok(s) => s,
        Err(_) => {
            if socket_path.exists() {
                let _ = std::fs::remove_file(&socket_path);
            }

            let exe = env::current_exe().expect("Failed to get current executable path");
            if let Err(e) = Command::new(exe)
                .arg("--daemon-mode")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                eprintln!("❌ Error: Failed to auto-spawn daemon: {}", e);
                std::process::exit(1);
            }

            let mut connected_stream = None;
            for _ in 0..20 {
                sleep(Duration::from_millis(50)).await;
                if let Ok(s) = UnixStream::connect(&socket_path).await {
                    connected_stream = Some(s);
                    break;
                }
            }

            match connected_stream {
                Some(s) => s,
                None => {
                    eprintln!("❌ Error: Daemon failed to start or bind socket in time.");
                    std::process::exit(1);
                }
            }
        }
    };

    // Inject history if requested
    let mut final_prompt = args.prompt;
    if let Some(n) = args.last_lines {
        let history = session::TerminalHistory::load(ppid);
        let last_n = history.get_last_n(n);

        if !last_n.is_empty() {
            let mut ctx = format!("\n\n[Context: Last {} Terminal Commands]\n", n);
            for entry in last_n {
                ctx.push_str(&format!("$ {}\n{}\n", entry.command, entry.output));
            }
            final_prompt.push_str(&ctx);
        } else {
            eprintln!(
                "⚠️ Warning: No recorded history found for session PID {}.",
                ppid
            );
        }
    }

    let thinking_level_override = match args.thinking_level {
        Some(FlagAction::Set(val)) => Some(val),
        _ => None,
    };

    let request = IpcRequest {
        ppid,
        prompt: final_prompt,
        thinking_level_override,
        shutdown: false,
        record_history: None,
    };

    let json_payload = serde_json::to_string(&request).expect("Failed to serialize request");

    if let Err(e) = stream.write_all(json_payload.as_bytes()).await {
        eprintln!("❌ Error sending request to daemon: {}", e);
        return;
    }

    let _ = stream.shutdown().await;

    let mut buffer = [0; 4096];
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => {
                io::stdout().write_all(&buffer[..n]).unwrap();
                io::stdout().flush().unwrap();
            }
            Err(e) => {
                eprintln!("\n❌ Error reading stream from daemon: {}", e);
                break;
            }
        }
    }
}

fn handle_queries(args: &CliArgs) {
    let ppid = args
        .override_pid
        .unwrap_or_else(|| std::os::unix::process::parent_id());

    if args.new_session {
        session::Session::clear(ppid);
        println!("🧹 Initialized new session.");
    }

    if let Some(n) = args.last_lines {
        let history = session::TerminalHistory::load(ppid);
        let last_n = history.get_last_n(n);

        println!("Terminal History (-l / --last): [PID {}]", ppid);
        if last_n.is_empty() {
            println!("  ⚠️ No history recorded yet.");
        } else {
            for (i, entry) in last_n.iter().enumerate() {
                println!("\n--- [{}] $ {} ---", i + 1, entry.command);
                println!("{}", entry.output.trim());
            }
        }
        return;
    }

    if let Some(ref action) = args.thinking_level {
        let mut session = session::Session::load(ppid);

        match action {
            cli::FlagAction::Query => {
                let current = session.thinking_level.as_deref().unwrap_or("off");
                println!("Thinking Level (-t / --thinking): [QUERY MODE]");
                println!("  💡 Current Level: {}", current);
                println!(
                    "  💡 Available Options: off, min/minimal, low, med/medium, high, xhigh, max"
                );
            }
            cli::FlagAction::Set(val) => {
                if engine::ThinkingLevel::from_str(val).is_some() {
                    session.thinking_level = Some(val.clone());
                    session.save(ppid);
                    println!(
                        "✅ Thinking level set to '{}' for current session (PID {}).",
                        val, ppid
                    );
                } else {
                    eprintln!("❌ Invalid thinking level '{}'.", val);
                    eprintln!(
                        "  💡 Available options: off, min/minimal, low, med/medium, high, xhigh, max"
                    );
                }
            }
        }
        return;
    }

    match &args.model {
        Some(FlagAction::Query) => {
            println!("Model (-m / --model)             : [QUERY MODE]");
            println!("  🤖 Current Model: local (Default)");
            println!("  🤖 Available Options: local, gemini, claude");
        }
        _ => {}
    }
}
