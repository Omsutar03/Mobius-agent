use futures_util::{SinkExt, StreamExt};
use mobius_core::ipc::{DaemonEvent, IpcRequest};
use std::io::Write;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub async fn send_to_daemon(request: IpcRequest) -> Result<(), Box<dyn std::error::Error>> {
    let ws_url = "ws://127.0.0.1:43812";

    // Connect to the WebSocket daemon
    let (mut ws_stream, _) = connect_async(ws_url).await.map_err(|e| {
        format!(
            "❌ Failed to connect to Mobius Daemon. Is it running? ({})",
            e
        )
    })?;

    // Send the structured request
    let req_json = serde_json::to_string(&request)?;
    ws_stream.send(Message::Text(req_json)).await?;

    let mut stdout = std::io::stdout();

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
                    DaemonEvent::TokenUsage { prompt, completion } => {
                        print!(
                            "\n\n\x1b[90m[Tokens Used: Prompt {}, Completion {}]\x1b[0m\n",
                            prompt, completion
                        );
                        let _ = stdout.flush();
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
