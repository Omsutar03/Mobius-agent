// TODO: `-l` or `--last` flag
mod cli;
mod daemon;
mod engine;
mod ipc;
mod session;

use cli::{CliArgs, FlagAction};
use engine::{ThinkingLevel, ask_mobius};
//use reqwest::Client;
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
                // If it exists but we can't connect, it might be a dead ghost file
                let _ = std::fs::remove_file(socket_path);
                println!("🧹 Cleaned up unresponsive daemon socket.");
                return;
            }
        };

        let request = IpcRequest {
            ppid: std::os::unix::process::parent_id(),
            prompt: String::new(),
            new_session: false,
            thinking_level_override: None,
            shutdown: true, // Trigger the shutdown!
        };

        let json_payload = serde_json::to_string(&request).unwrap();
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
    // Check if the daemon socket exists. If not, auto-spawn the daemon.
    let socket_path = daemon::get_socket_path();
    if !socket_path.exists() {
        let exe = env::current_exe().expect("Failed to get current executable path");

        match Command::new(exe)
            .arg("--daemon-mode")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(_) => {
                // Wait briefly for the daemon to boot up and create the socket file
                let mut attempts = 0;
                while !socket_path.exists() && attempts < 20 {
                    sleep(Duration::from_millis(50)).await;
                    attempts += 1;
                }

                if !socket_path.exists() {
                    eprintln!("❌ Error: Daemon failed to start or bind socket in time.");
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("❌ Error: Failed to auto-spawn daemon: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Connect to the background daemon
    let mut stream = match UnixStream::connect(&socket_path).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Error: Could not connect to Mobius daemon: {}", e);
            std::process::exit(1);
        }
    };

    // Prepare the JSON payload
    let thinking_level_override = match args.thinking_level {
        Some(FlagAction::Set(val)) => Some(val),
        _ => None,
    };

    let request = IpcRequest {
        ppid: std::os::unix::process::parent_id(), // Grab the terminal tab's Process ID
        prompt: args.prompt,
        new_session: args.new_session,
        thinking_level_override,
        shutdown: false,
    };

    let json_payload = serde_json::to_string(&request).unwrap();

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

    // match CliArgs::parse() {
    //     Ok(args) => {
    //         // For now, if the user provides a prompt, we just send it to the engine directly.
    //         if !args.prompt.is_empty() {
    //             // Determine the thinking level for this request
    //             let thinking_level =
    //                 ThinkingLevel::from_str("medium").unwrap_or(ThinkingLevel::Off);

    //             // Hardcoded for testing, will move this to config.rs later
    //             let server_url = "http://localhost:8080/v1/chat/completions";
    //             let client = Client::new();

    //             match ask_mobius(&client, server_url, &args.prompt, thinking_level).await {
    //                 Ok(_) => {} // Stream is already printed to stdout
    //                 Err(e) => eprintln!("❌ Mobius Engine Error: {}", e),
    //             }

    //             return;
    //         }

    //         // `-n` or `--new-s` flag
    //         if args.new_session {
    //             println!("🧹 Initializing new session..."); // Placeholder
    //         }

    //         // `-t` or `--thinking` flag
    //         match &args.thinking_level {
    //             Some(FlagAction::Query) => {
    //                 println!("Thinking Level (-t / --thinking): [QUERY MODE]");
    //                 println!("  💡 Current Level: Off (Default)");
    //                 println!("  💡 Available Options: off, minimal, low, medium, high, xhigh, max");
    //             }
    //             _ => {}
    //         }

    //         // '-m' or '--model' flag
    //         match &args.model {
    //             Some(FlagAction::Query) => {
    //                 println!("Model (-m / --model)             : [QUERY MODE]");
    //                 println!("  🤖 Current Model: local (Default)");
    //                 println!("  🤖 Available Options: local, gemini, claude");
    //             }
    //             _ => {}
    //         }
    //     }
    //     Err(err) => {
    //         eprintln!("❌ Error: {}", err);
    //     }
    // }
}

// Helper function to handle metadata queries (-t, -m)
fn handle_queries(args: &CliArgs) {
    // `-n` or `--new-s` flag
    if args.new_session {
        println!("🧹 Initializing new session...");
    }

    // `-t` or `--thinking` flag
    match &args.thinking_level {
        Some(FlagAction::Query) => {
            println!("Thinking Level (-t / --thinking): [QUERY MODE]");
            println!("  💡 Current Level: Off (Default)");
            println!("  💡 Available Options: off, minimal, low, medium, high, xhigh, max");
        }
        _ => {}
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
