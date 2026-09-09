use futures_util::{SinkExt, StreamExt};
use mobius_core::config::MobiusConfig;
use mobius_core::engine::{ThinkingLevel, ask_mobius};
use mobius_core::inferences::{InferenceEngine, discover_local_models, get_context_window};
use mobius_core::ipc::{DaemonEvent, IpcRequest, SessionMetadata};
use mobius_core::session::Session;
use mobius_core::tools;
use reqwest::Client;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

pub const DEFAULT_PORT: u16 = 43812;

pub async fn start_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("127.0.0.1:{}", DEFAULT_PORT);
    let listener = TcpListener::bind(&addr).await?;

    let http_client = Client::builder()
        .pool_max_idle_per_host(0)
        .timeout(Duration::from_secs(300))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| Client::new());

    println!("🚀 Mobius WebSocket Daemon listening on ws://{}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let client_clone = http_client.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(client_clone, stream).await {
                        eprintln!("❌ Connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("⚠️ Failed to accept TCP connection: {}", e);
            }
        }
    }
}

async fn handle_connection(
    client: Client,
    stream: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut ws_stream = accept_async(stream).await?;

    while let Some(msg) = ws_stream.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(_) => break, // Graceful disconnect when client closes connection
        };

        if msg.is_close() {
            break;
        }
        if !msg.is_text() {
            continue;
        }

        let request: IpcRequest = match serde_json::from_str(msg.to_text()?) {
            Ok(req) => req,
            Err(e) => {
                let err_event = DaemonEvent::Error(format!("Invalid request format: {}", e));
                let _ = ws_stream
                    .send(Message::Text(serde_json::to_string(&err_event)?))
                    .await;
                continue;
            }
        };

        // --- HISTORY RECORDING LOGIC (removed with --last feature) ---

        // --- SHUTDOWN LOGIC ---
        if request.shutdown {
            let msg_event = DaemonEvent::TextChunk("Mobius daemon shutting down...\n".into());
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&msg_event)?))
                .await;
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            std::process::exit(0);
        }

        // --- LIST SESSIONS ---
        if request.list_sessions {
            let session_files = Session::list_all_sessions(request.ppid);
            let metadata: Vec<SessionMetadata> = session_files
                .iter()
                .map(|sf| {
                    let message_count = if let Ok(data) = std::fs::read_to_string(&sf.path) {
                        serde_json::from_str::<Session>(&data)
                            .map(|s| s.messages.len())
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    SessionMetadata {
                        filename: sf.filename.clone(),
                        path: sf.path.to_string_lossy().to_string(),
                        preview: sf.preview.clone(),
                        is_active: sf.is_active,
                        message_count,
                    }
                })
                .collect();
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(
                    &DaemonEvent::SessionList(metadata),
                )?))
                .await;
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            continue;
        }

        let mut session = Session::load(request.ppid);

        // --- LOAD SESSION (switch to a past session) ---
        if let Some(ref target_path) = request.load_session {
            let path = std::path::PathBuf::from(target_path);
            if path.exists() {
                if let Ok(data) = std::fs::read_to_string(&path) {
                    if let Ok(target_session) = serde_json::from_str::<Session>(&data) {
                        target_session.save(request.ppid);
                        let messages_json =
                            serde_json::to_string(&target_session.messages).unwrap_or_default();
                        let _ = ws_stream
                            .send(Message::Text(serde_json::to_string(
                                &DaemonEvent::TextChunk(messages_json),
                            )?))
                            .await;
                        let _ = ws_stream
                            .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                            .await;
                        continue;
                    }
                }
            }
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(
                    &DaemonEvent::Error("Failed to load session".into()),
                )?))
                .await;
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            continue;
        }

        // --- DELETE SESSION ---
        if let Some(ref target_path) = request.delete_session {
            let path = std::path::PathBuf::from(target_path);
            let is_active = path
                .file_name()
                .map(|f| f.to_string_lossy() == format!("{}.json", request.ppid))
                .unwrap_or(false);
            if is_active {
                let _ = ws_stream
                    .send(Message::Text(serde_json::to_string(
                        &DaemonEvent::Error("Cannot delete the currently active session".into()),
                    )?))
                    .await;
            } else if let Err(e) = std::fs::remove_file(&path) {
                let _ = ws_stream
                    .send(Message::Text(serde_json::to_string(
                        &DaemonEvent::Error(format!("Failed to delete session: {}", e)),
                    )?))
                    .await;
            }
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            continue;
        }

        // --- NEW SESSION (-n / --new-s) ---
        if request.new_session {
            Session::archive_and_reset(request.ppid);
            let msg =
                DaemonEvent::TextChunk("✨ Started fresh session for this terminal.\n".into());
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&msg)?))
                .await;
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            continue;
        }

        // --- QUERY TOKENS (--tokens) ---
        if request.query_tokens {
            let info = format!(
                "📊 Session Context Tokens:\n  Last Prompt: {}\n  Last Completion: {}\n  Context Limit: {}\n",
                session.last_prompt_tokens, session.last_completion_tokens, session.context_window
            );
            let msg = DaemonEvent::TextChunk(info);
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&msg)?))
                .await;
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            continue;
        }

        // --- MODEL SELECTION / QUERY (-m / --model) ---
        if let Some(ref new_model) = request.model_override {
            session.model_provider = Some(new_model.clone());
            session.save(request.ppid);

            // Only return early if there is no prompt provided with this request
            if request.prompt.trim().is_empty() && !request.query_model {
                let msg = DaemonEvent::TextChunk(format!(
                    "🔄 Switched model provider to: {}\n",
                    new_model
                ));
                let _ = ws_stream
                    .send(Message::Text(serde_json::to_string(&msg)?))
                    .await;
                let _ = ws_stream
                    .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                    .await;
                return Ok(());
            }
        }

        if request.query_model {
            let active_models = discover_local_models().await;
            let current = session.model_provider.as_deref().unwrap_or("llama");
            let mut list = format!("🤖 Active Provider: {}\n\nAvailable Engines:\n", current);
            for m in active_models {
                list.push_str(&format!("  • {:?}: {}\n", m.engine, m.model_name));
            }
            let msg = DaemonEvent::TextChunk(list);
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&msg)?))
                .await;
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            continue;
        }

        // --- THINKING LEVEL QUERY (-t / --thinking) ---
        if request.query_thinking {
            let current = session.thinking_level.as_deref().unwrap_or("off");
            let msg = DaemonEvent::TextChunk(format!("🧠 Current Thinking Level: {}\n", current));
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&msg)?))
                .await;
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            continue;
        }

        // Ignore empty prompts if no command action was specified
        if request.prompt.trim().is_empty() {
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            continue;
        }

        // --- SESSION & INFERENCE PIPELINE ---
        let (tx, mut rx) = mpsc::channel::<DaemonEvent>(100);

        let mut ws_sink = ws_stream;
        let forward_handle = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Ok(json) = serde_json::to_string(&event) {
                    if ws_sink.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
            ws_sink
        });

        if let Some(ref override_level) = request.thinking_level_override {
            session.thinking_level = Some(override_level.clone());
        }

        let thinking_str = session.thinking_level.as_deref().unwrap_or("off");
        let thinking_level = ThinkingLevel::from_str(thinking_str).unwrap_or(ThinkingLevel::Off);

        session.add_message("user", &request.prompt);

        let provider_str = session.model_provider.as_deref().unwrap_or("llama");
        let engine = match provider_str {
            "ollama" => InferenceEngine::Ollama,
            "lmstudio" => InferenceEngine::GenericOpenAI,
            _ => InferenceEngine::LlamaCpp,
        };

        let active_models = discover_local_models().await;
        let model_name = active_models
            .iter()
            .find(|m| m.engine == engine)
            .map(|m| m.model_name.clone())
            .unwrap_or_else(|| "default".to_string());

        let mut max_tool_turns = 5;
        let config = MobiusConfig::load();

        while max_tool_turns > 0 {
            let messages_payload = session.get_api_messages();

            let (response, usage) = match ask_mobius(
                &client,
                &engine,
                &model_name,
                messages_payload,
                thinking_level,
                &tx,
                &config,
            )
            .await
            {
                Ok(res) => res,
                Err(e) => {
                    let _ = tx.send(DaemonEvent::Error(e.to_string())).await;
                    break;
                }
            };

            session.last_prompt_tokens = usage.prompt_tokens;
            session.last_completion_tokens = usage.completion_tokens;
            session.context_window = get_context_window(&client, &engine, &model_name).await;

            let _ = tx
                .send(DaemonEvent::TokenUsage {
                    prompt: usage.prompt_tokens,
                    completion: usage.completion_tokens,
                })
                .await;

            session.add_message("assistant", &response);

            if let Some(tool_call) = tools::parse_tool_call(&response) {
                match tool_call {
                    tools::ToolCall::Shell(cmd) => {
                        let _ = tx
                            .send(DaemonEvent::ToolStart {
                                tool_name: "bash".into(),
                                details: cmd.clone(),
                            })
                            .await;

                        let output_str = match tools::execute_shell_command(&cmd, 10).await {
                            Ok(out) => out.to_llm_string(),
                            Err(err_msg) => format!("[Execution Error]: {}", err_msg),
                        };

                        let _ = tx
                            .send(DaemonEvent::ToolFinished {
                                result: output_str.clone(),
                            })
                            .await;

                        let tool_feedback = format!(
                            "[Tool Output for `{}`]:\n{}\n\nPlease provide the final response to the user based on this output.",
                            cmd, output_str
                        );
                        session.add_message("user", &tool_feedback);
                    }
                    tools::ToolCall::Read { path } => {
                        let _ = tx
                            .send(DaemonEvent::ToolStart {
                                tool_name: "read".into(),
                                details: path.clone(),
                            })
                            .await;

                        let output_str = tools::execute_read_command(&path).await;

                        let _ = tx
                            .send(DaemonEvent::ToolFinished {
                                result: output_str.clone(),
                            })
                            .await;

                        let tool_feedback = format!(
                            "{}\n\nPlease analyze this file content and answer the user.",
                            output_str
                        );
                        session.add_message("user", &tool_feedback);
                    }
                    tools::ToolCall::Write { path, content } => {
                        let _ = tx
                            .send(DaemonEvent::ToolStart {
                                tool_name: "write".into(),
                                details: path.clone(),
                            })
                            .await;

                        let output_str = tools::execute_write_command(&path, &content).await;

                        let _ = tx
                            .send(DaemonEvent::ToolFinished {
                                result: output_str.clone(),
                            })
                            .await;

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
                        let _ = tx
                            .send(DaemonEvent::ToolStart {
                                tool_name: "edit".into(),
                                details: path.clone(),
                            })
                            .await;

                        let output_str =
                            tools::execute_edit_command(&path, &old_text, &new_text).await;

                        let _ = tx
                            .send(DaemonEvent::ToolFinished {
                                result: output_str.clone(),
                            })
                            .await;

                        let tool_feedback = format!(
                            "{}\n\nPlease verify the result and inform the user.",
                            output_str
                        );
                        session.add_message("user", &tool_feedback);
                    }
                    tools::ToolCall::WebSearch { query } => {
                        let _ = tx
                            .send(DaemonEvent::ToolStart {
                                tool_name: "web_search".into(),
                                details: query.clone(),
                            })
                            .await;

                        let output_str = tools::execute_web_search_command(&query).await;

                        let _ = tx
                            .send(DaemonEvent::ToolFinished {
                                result: output_str.clone(),
                            })
                            .await;

                        let tool_feedback = format!(
                            "{}\n\nPlease review these results. If you need more details from a specific result, use the 'read_webpage' tool on its URL.",
                            output_str
                        );
                        session.add_message("user", &tool_feedback);
                    }
                    tools::ToolCall::ReadWebPage { url } => {
                        let _ = tx
                            .send(DaemonEvent::ToolStart {
                                tool_name: "read_webpage".into(),
                                details: url.clone(),
                            })
                            .await;

                        let output_str = tools::execute_read_webpage_command(&url).await;

                        let _ = tx
                            .send(DaemonEvent::ToolFinished {
                                result: output_str.clone(),
                            })
                            .await;

                        let tool_feedback = format!(
                            "{}\n\nPlease analyze this page content to answer the user's question.",
                            output_str
                        );
                        session.add_message("user", &tool_feedback);
                    }
                }

                max_tool_turns -= 1;
            } else {
                break;
            }
        }

        session.save(request.ppid);

        let _ = tx.send(DaemonEvent::Done).await;
        drop(tx);

        let _ws_stream = forward_handle.await?;
        break;
    }

    Ok(())
}
