use crate::session::Session;
use reqwest::Client;
use std::env;
use std::fs;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

use crate::engine::{ThinkingLevel, ask_mobius};
use crate::ipc::IpcRequest;

/// Resolves the path to `~/.local/state/mobius/mobius.sock`
pub fn get_socket_path() -> PathBuf {
    let home = env::var("HOME").expect("Could not find HOME directory");
    let state_dir = PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("mobius");

    // Ensure the directory exists
    if !state_dir.exists() {
        fs::create_dir_all(&state_dir).expect("Failed to create mobius state directory");
    }

    state_dir.join("mobius.sock")
}

/// Starts the background listener loop
pub async fn start_daemon() {
    let socket_path = get_socket_path();

    // Unix sockets leave a ghost file behind if the process crashes.
    // We must delete the old file before we can bind to the path again.
    if socket_path.exists() {
        fs::remove_file(&socket_path).expect("Failed to clean up old Unix socket");
    }

    let listener = UnixListener::bind(&socket_path).expect("Failed to bind Unix socket");

    // We create a single HTTP client and share it across all connections
    // to benefit from connection pooling to llama-server.
    let http_client = Client::new();

    println!("🚀 Mobius Daemon started. Listening on {:?}", socket_path);

    // Infinite loop waiting for clients to connect
    loop {
        match listener.accept().await {
            Ok((mut stream, _addr)) => {
                let client_clone = http_client.clone();

                // Spawn a new asynchronous task for each incoming connection.
                // This allows the daemon to handle multiple terminal tabs querying at once!
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(client_clone, &mut stream).await {
                        eprintln!("❌ Connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("⚠️ Failed to accept client connection: {}", e);
            }
        }
    }
}

async fn handle_connection(
    client: Client,
    stream: &mut UnixStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![0; 8192];
    let bytes_read = stream.read(&mut buffer).await?;

    if bytes_read == 0 {
        return Ok(());
    }

    let json_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request: IpcRequest = serde_json::from_str(&json_str)?;

    // --- SHUTDOWN LOGIC ---
    if request.shutdown {
        let socket_path = get_socket_path();
        if socket_path.exists() {
            let _ = fs::remove_file(socket_path); // Clean up the ghost file!
        }

        // Send a polite confirmation back to the client terminal
        stream
            .write_all(b"[\x1B[32mOK\x1B[0m] Mobius daemon shut down successfully.\n")
            .await?;
        stream.flush().await?;

        // Immediately terminate the background process
        std::process::exit(0);
    }

    let thinking_level = request
        .thinking_level_override
        .as_deref()
        .and_then(ThinkingLevel::from_str)
        .unwrap_or(ThinkingLevel::Off);

    // --- NEW MEMORY LOGIC ---

    // 1. If user passed `-n`, wipe the slate clean
    if request.new_session {
        Session::clear(request.ppid);
    }

    // 2. Load past history from disk (or start fresh)
    let mut session = Session::load(request.ppid);

    // 3. Append the user's new prompt
    session.add_message("user", &request.prompt);

    // 4. Generate the payload to send to the engine
    let messages_payload = session.get_api_messages();
    let server_url = "http://localhost:8080/v1/chat/completions";

    // 5. Trigger the engine and capture the final response
    let final_response = ask_mobius(
        &client,
        server_url,
        messages_payload,
        thinking_level,
        stream,
    )
    .await?;

    // 6. Append the AI's response and save back to disk
    session.add_message("assistant", &final_response);
    session.save(request.ppid);

    Ok(())
}
