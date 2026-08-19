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
use engine::{ThinkingLevel, ask_mobius};
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
                // If it exists but can't connect, it might be a dead ghost file
                let _ = std::fs::remove_file(socket_path);
                println!("🧹 Cleaned up unresponsive daemon socket.");
                return;
            }
        };

        let request = IpcRequest {
            ppid: std::os::unix::process::parent_id(),
            prompt: String::new(),
            thinking_level_override: None,
            shutdown: true, // Triggers the daemon shutdown!
        };

        let json_payload = serde_json::to_string(&request).expect("Failed to serialize request");
        let _ = stream.write_all(json_payload.as_bytes()).await;
        let _ = stream.shutdown().await;

        let mut response = String::new();
        let _ = stream.read_to_string(&mut response).await;
        print!("{}", response);
        return;
    }

    // 4. CLIENT MODE (Querying flags without a prompt)
    if args.prompt.is_empty() {
        handle_queries(&args);
        return;
    }

    // 5. CLIENT MODE (Processing a prompt)
    let socket_path = daemon::get_socket_path();

    // Attempt to connect immediately to check if an active daemon is running
    let mut stream = match UnixStream::connect(&socket_path).await {
        Ok(s) => s, // Connection succeeded: Active daemon found
        Err(_) => {
            // Connection failed: Clean up dead socket file if it exists
            if socket_path.exists() {
                let _ = std::fs::remove_file(&socket_path);
            }

            // Spawn background daemon process
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

            // Poll-retry connection while the newly spawned daemon binds to socket[cite: 5]
            let mut connected_stream = None;
            for _ in 0..20 {
                sleep(Duration::from_millis(50)).await;
                if let Ok(s) = UnixStream::connect(&socket_path).await {
                    connected_stream = Some(s);
                    break;
                }
            }

            // Unwrap stream or fail if daemon couldn't bind in 1 second
            match connected_stream {
                Some(s) => s,
                None => {
                    eprintln!("❌ Error: Daemon failed to start or bind socket in time.");
                    std::process::exit(1);
                }
            }
        }
    };

    // Prepare the JSON payload
    let thinking_level_override = match args.thinking_level {
        Some(FlagAction::Set(val)) => Some(val), // If the flag exists (Some), AND its action is Set, extract 'val'
        _ => None, // If it is anything else (None, or a different FlagAction variant), return None
    };

    let request = IpcRequest {
        ppid: std::os::unix::process::parent_id(), // Grab the terminal tab's Process ID
        prompt: args.prompt,
        thinking_level_override,
        shutdown: false,
    };

    let json_payload = serde_json::to_string(&request).expect("Failed to serialize request");

    // Send the payload to the daemon over the socket
    if let Err(e) = stream.write_all(json_payload.as_bytes()).await {
        eprintln!("❌ Error sending request to daemon: {}", e);
        return;
    }

    // Shut down the *write* half of the client socket.
    // This tells the daemon "I'm done sending the request", but keeps the read half open.
    let _ = stream.shutdown().await;

    // Listen for the daemon streaming the response back to us
    let mut buffer = [0; 4096];
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => break, // EOF: Daemon finished streaming and closed the connection
            Ok(n) => {
                // Print the raw ANSI-colored bytes directly to the terminal stdout
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

// Helper function to handle metadata queries (-t, -m)
fn handle_queries(args: &CliArgs) {
    let ppid = std::os::unix::process::parent_id();

    // `-n` or `--new-s` flag
    if args.new_session {
        session::Session::clear(ppid);
        println!("🧹 Initialized new session.");
    }

    // `-t` or `--thinking` flag
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
                // Validate if the input string is a recognized level
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

    // '-m' or '--model' flag
    match &args.model {
        Some(FlagAction::Query) => {
            println!("Model (-m / --model)             : [QUERY MODE]");
            println!("  🤖 Current Model: local (Default)");
            println!("  🤖 Available Options: local, gemini, claude");
        }
        _ => {}
    }
}
