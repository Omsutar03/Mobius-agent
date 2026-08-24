use crate::Duration;
use crate::session::Session;
use reqwest::Client;
use std::env;
use std::fs;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

use crate::engine::{ThinkingLevel, ask_mobius};
use crate::ipc::IpcRequest;
use crate::tools;

// Resolves the path to `~/.local/state/mobius/mobius.sock`
pub fn get_socket_path() -> PathBuf {
    let home = env::var("HOME").expect("Could not find HOME directory");
    let state_dir = PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("mobius");

    if !state_dir.exists() {
        fs::create_dir_all(&state_dir).expect("Failed to create mobius state directory");
    }

    state_dir.join("mobius.sock")
}

// Starts the background listener loop
pub async fn start_daemon() {
    let socket_path = get_socket_path();

    // Unix sockets leave a ghost file behind if the process crashes.
    // Must delete the old file before we can bind to the path again.
    if socket_path.exists() {
        fs::remove_file(&socket_path).expect("Failed to clean up old Unix socket");
    }

    let listener = UnixListener::bind(&socket_path).expect("Failed to bind Unix socket");

    // Disable idle socket reuse on the shared reqwest::Client so every tool turn opens a fresh HTTP connection.
    let http_client = Client::builder()
        .pool_max_idle_per_host(0) // Disables keep-alive socket reuse for SSE streams
        .timeout(Duration::from_secs(300))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| Client::new());

    println!("🚀 Mobius Daemon started. Listening on {:?}", socket_path);

    // Infinite loop waiting for clients to connect
    loop {
        match listener.accept().await {
            Ok((mut stream, _addr)) => {
                let client_clone = http_client.clone();

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
    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer).await?;

    if buffer.is_empty() {
        return Ok(());
    }

    let request: IpcRequest = serde_json::from_slice(&buffer)?;

    // --- HISTORY RECORDING LOGIC ---
    if let Some(entry) = request.record_history {
        let mut history = crate::session::TerminalHistory::load(request.ppid);
        history.push_entry(request.ppid, entry.command, entry.output);
        let _ = stream.write_all(b"OK").await;
        return Ok(());
    }

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

    // --- SESSION & INFERENCE PIPELINE ---

    // 1. Load past history from disk (or start fresh)
    let mut session = Session::load(request.ppid);

    // 2. Persist thinking level override to the session if explicitly passed via CLI
    if let Some(ref override_level) = request.thinking_level_override {
        session.thinking_level = Some(override_level.clone());
    }

    // Resolve active Thinking Level
    let thinking_str = session.thinking_level.as_deref().unwrap_or("off");
    let thinking_level = ThinkingLevel::from_str(thinking_str).unwrap_or(ThinkingLevel::Off); // Extra fallback for edited/corrupt session file

    // 3. Append the user's new prompt
    session.add_message("user", &request.prompt);

    let server_url = "http://localhost:8080/v1/chat/completions";
    let mut max_tool_turns = 5;

    // 4. Multi-turn agent execution loop
    while max_tool_turns > 0 {
        let messages_payload = session.get_api_messages();

        // Query LLM and stream chunk response
        let response = match ask_mobius(
            &client,
            server_url,
            messages_payload,
            thinking_level,
            stream,
        )
        .await
        {
            Ok(res) => res,
            Err(e) => {
                // Convert to String immediately to drop non-Send `e` before .await
                let err_str = e.to_string();
                let err_msg = format!("\n\x1B[31m❌ [Mobius Engine Error]: {}\x1B[0m\n", err_str);
                let _ = stream.write_all(err_msg.as_bytes()).await;
                let _ = stream.flush().await;
                return Err(err_str.into());
            }
        };

        // Save model's reponse to history
        session.add_message("assistant", &response);

        // Check if the response contains local tool command
        if let Some(tool_call) = tools::parse_tool_call(&response) {
            match tool_call {
                tools::ToolCall::Shell(cmd) => {
                    // Print visual status mesasge to the user terinal
                    let status_msg = format!("\x1B[33m⚡ [Executing]:\x1B[0m {}\n", cmd);
                    stream.write_all(status_msg.as_bytes()).await?;

                    // Execute shell command with timeout
                    let output_str = match tools::execute_shell_command(&cmd, 10).await {
                        Ok(out) => out.to_llm_string(),
                        Err(err_msg) => format!("[Execution Error]: {}", err_msg),
                    };

                    // Feed execution result back into chat history for next iteration
                    let tool_feedback = format!(
                        "[Tool Output for `{}`]:\n{}\n\nPlease provide the final response to the user based on this output.",
                        cmd, output_str
                    );
                    session.add_message("user", &tool_feedback);
                }
                tools::ToolCall::Read { path } => {
                    let status_msg = format!("\x1B[36m📄 [Reading File]:\x1B[0m {}\n", path);
                    stream.write_all(status_msg.as_bytes()).await?;

                    let output_str = tools::execute_read_command(&path).await;

                    let tool_feedback = format!(
                        "{}\n\nPlease analyze this file content and answer the user.",
                        output_str
                    );
                    session.add_message("user", &tool_feedback);
                }
                tools::ToolCall::Write { path, content } => {
                    let status_msg = format!("\x1B[33m✍️  [Writing File]:\x1B[0m {}\n", path);
                    stream.write_all(status_msg.as_bytes()).await?;

                    let output_str = tools::execute_write_command(&path, &content).await;

                    let tool_feedback = format!(
                        "{}\n\nPlease confirm to the user that the file was created or updated successfully.",
                        output_str
                    );
                    session.add_message("user", &tool_feedback);
                }
                tools::ToolCall::Edit {
                    path,
                    old_text,
                    new_text,
                } => {
                    let status_msg = format!("\x1B[35m✏️  [Editing File]:\x1B[0m {}\n", path);
                    stream.write_all(status_msg.as_bytes()).await?;

                    let output_str = tools::execute_edit_command(&path, &old_text, &new_text).await;

                    let tool_feedback = format!(
                        "{}\n\nPlease verify the result and inform the user.",
                        output_str
                    );
                    session.add_message("user", &tool_feedback);
                }
                tools::ToolCall::WebSearch { query } => {
                    let status_msg = format!("\x1B[34m🔍 [Searching Web]:\x1B[0m {}\n", query);
                    stream.write_all(status_msg.as_bytes()).await?;

                    let output_str = tools::execute_web_search_command(&query).await;

                    let tool_feedback = format!(
                        "{}\n\nPlease review these results. If you need more details from a specific result, use the 'read_webpage' tool on its URL.",
                        output_str
                    );
                    session.add_message("user", &tool_feedback);
                }
                tools::ToolCall::ReadWebPage { url } => {
                    let status_msg = format!("\x1B[36m🌐 [Reading Webpage]:\x1B[0m {}\n", url);
                    stream.write_all(status_msg.as_bytes()).await?;

                    let output_str = tools::execute_read_webpage_command(&url).await;

                    let tool_feedback = format!(
                        "{}\n\nPlease analyze this page content to answer the user's question.",
                        output_str
                    );
                    session.add_message("user", &tool_feedback);
                }
            }

            max_tool_turns -= 1;
        } else {
            break; // No tool call requested
        }
    }

    // Save final updated session back to disk
    session.save(request.ppid);

    Ok(())
}
