use futures_util::{SinkExt, StreamExt};
use mobius_core::config::MobiusConfig;
use mobius_core::engine::{ThinkingLevel, ask_mobius};
use mobius_core::inferences::{InferenceEngine, discover_local_models, get_context_window};
use mobius_core::ipc::{DaemonEvent, IpcRequest, ModelListEntry, SessionMetadata};
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
            let msg_event = DaemonEvent::Info("Mobius daemon shutting down...\n".into());
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
                DaemonEvent::Info("✨ Started fresh session for this terminal.\n".into());
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&msg)?))
                .await;
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            continue;
        }

        // --- MODEL SELECTION / QUERY (-m / --model) ---
        let mut switch_msg: Option<String> = None;
        if let Some(n) = request.model_number {
            let active_models = discover_local_models().await;
            if n > 0 && n <= active_models.len() {
                let choice = &active_models[n - 1];
                session.model_provider = Some(choice.engine.as_provider_str().to_string());
                session.model_name = Some(choice.model_name.clone());
                session.save(request.ppid);
                switch_msg = Some(format!("🔄 Switched to [{}] {}\n", n, choice.display_name));
            } else {
                let _ = ws_stream
                    .send(Message::Text(serde_json::to_string(&DaemonEvent::Error(format!(
                        "Invalid model selection: [{}]. Use `mobius -m` to list available models.\n",
                        n
                    )))?))
                    .await;
                let _ = ws_stream
                    .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                    .await;
                continue;
            }
        }

        if request.prompt.trim().is_empty() && !request.query_model && !request.query_model_list {
            if let Some(msg) = switch_msg {
                let _ = ws_stream
                    .send(Message::Text(serde_json::to_string(&DaemonEvent::Info(msg))?))
                    .await;
                let _ = ws_stream
                    .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                    .await;
                continue;
            }
        }

        if request.query_model_list {
            let active_models = discover_local_models().await;
            let list: Vec<ModelListEntry> = active_models
                .iter()
                .enumerate()
                .map(|(i, m)| ModelListEntry {
                    provider: m.engine.as_provider_str().to_string(),
                    model_name: m.model_name.clone(),
                    display_name: format!("[{}] {}", i + 1, m.display_name),
                    selected: session.model_name.as_deref() == Some(m.model_name.as_str()),
                })
                .collect();
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::ModelList(
                    list,
                ))?))
                .await;
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            continue;
        }

        if request.query_model {
            let active_models = discover_local_models().await;
            let current_provider = session.model_provider.as_deref().unwrap_or("llama");
            let mut list = format!("🤖 Active Provider: {}\n", current_provider);
            if let Some(ref current_model) = session.model_name {
                list.push_str(&format!("   Current Model:   {}\n", current_model));
            }
            if active_models.is_empty() {
                list.push_str(
                    "\n⚠️ No local models detected. Start llama.cpp (8080), Ollama (11434), or LM Studio (1234).\n",
                );
            } else {
                list.push_str("\nAvailable Models:\n");
                for (i, m) in active_models.iter().enumerate() {
                    let marker = if session.model_name.as_deref() == Some(m.model_name.as_str()) {
                        "➡"
                    } else {
                        "  "
                    };
                    list.push_str(&format!(
                        "  {} [{}] {}: {}\n",
                        marker,
                        i + 1,
                        m.engine.as_provider_str(),
                        m.model_name
                    ));
                }
                list.push_str("\nSelect by number, e.g. `mobius -m 2`.\n");
            }
            let msg = DaemonEvent::Info(list);
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
            let mut list = format!("🧠 Current Thinking Level: {}\n\n", current);
            list.push_str("Available Levels:\n");
            for (i, level) in ThinkingLevel::ALL.iter().enumerate() {
                let marker = if level.as_str() == current { "➡" } else { "  " };
                list.push_str(&format!("  {} [{}] {}\n", marker, i, level.as_str()));
            }
            let msg = DaemonEvent::Info(list);
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&msg)?))
                .await;
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            continue;
        }

        // --- THINKING LEVEL SET (standalone -t <value>) ---
        if request.thinking_level_override.is_some() && request.prompt.trim().is_empty() {
            let level = request.thinking_level_override.as_deref().unwrap_or("off");
            if ThinkingLevel::from_str(level).is_none() {
                let _ = ws_stream
                    .send(Message::Text(serde_json::to_string(&DaemonEvent::Error(
                        format!("Invalid thinking level: '{}'. Use `mobius -t` to list available levels.\n", level),
                    ))?))
                    .await;
                let _ = ws_stream
                    .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                    .await;
                continue;
            }
            session.thinking_level = Some(level.to_string());
            session.save(request.ppid);
            let msg = DaemonEvent::Info(format!("🧠 Thinking level set to: {}\n", level));
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&msg)?))
                .await;
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&DaemonEvent::Done)?))
                .await;
            continue;
        }

        // --- LLM PROVIDER STATUS (GUI sidebar) ---
        if request.query_llm_status {
            let provider = request
                .llm_provider
                .as_deref()
                .or(session.model_provider.as_deref())
                .unwrap_or("llama");
            let engine = InferenceEngine::from_provider(provider);
            let active = discover_local_models().await;
            let model_info = active.iter().find(|m| m.engine == engine);
            let status_event = DaemonEvent::LlmStatus {
                connected: model_info.is_some(),
                provider: provider.to_string(),
                model_name: session
                    .model_name
                    .clone()
                    .or_else(|| model_info.map(|m| m.model_name.clone())),
            };
            let _ = ws_stream
                .send(Message::Text(serde_json::to_string(&status_event)?))
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
        let engine = InferenceEngine::from_provider(provider_str);

        let active_models = discover_local_models().await;
        let model_name = session
            .model_name
            .clone()
            .or_else(|| {
                active_models
                    .iter()
                    .find(|m| m.engine == engine)
                    .map(|m| m.model_name.clone())
            })
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
                    prompt: session.last_prompt_tokens,
                    completion: session.last_completion_tokens,
                    context_window: session.context_window,
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

        // Hand the sink back and keep the connection open for the next request
        ws_stream = forward_handle.await?;
        continue;
    }

    Ok(())
}
