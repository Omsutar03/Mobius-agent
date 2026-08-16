use reqwest::Client;
use serde_json::json;
//use std::io::{self, Write};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl ThinkingLevel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "off" => Some(Self::Off),
            "minimal" | "min" => Some(Self::Minimal),
            "low" => Some(Self::Low),
            "medium" | "med" => Some(Self::Medium),
            "high" => Some(Self::High),
            "xhigh" => Some(Self::XHigh),
            "max" => Some(Self::Max),
            _ => None,
        }
    }

    pub fn to_params(&self) -> (bool, &'static str) {
        match self {
            ThinkingLevel::Off => (false, "none"),
            ThinkingLevel::Minimal => (true, "minimal"),
            ThinkingLevel::Low => (true, "low"),
            ThinkingLevel::Medium => (true, "medium"),
            ThinkingLevel::High => (true, "high"),
            ThinkingLevel::XHigh => (true, "xhigh"),
            ThinkingLevel::Max => (true, "max"),
        }
    }
}

pub async fn ask_mobius(
    client: &Client,
    server_url: &str,
    messages_payload: serde_json::Value,
    thinking_level: ThinkingLevel,
    stream: &mut UnixStream,
) -> Result<String, Box<dyn std::error::Error>> {
    let (enable_thinking, effort_str) = thinking_level.to_params();

    let payload = json!({
        "messages": messages_payload,
        "chat_template_kwargs": {
            "enable_thinking": enable_thinking,
            "reasoning_effort": effort_str
        },
        "reasoning_effort": effort_str,
        "stream": true
    });

    let mut response = client.post(server_url).json(&payload).send().await?;

    if !response.status().is_success() {
        return Err(format!("Server returned HTTP status {}", response.status()).into());
    }

    let mut full_text = String::new();
    let mut buffer = String::new();

    let mut printed_mobius_prefix = false;
    let mut has_thought = false;
    let mut in_thinking_block = false;

    while let Some(chunk) = response.chunk().await? {
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(newline_idx) = buffer.find('\n') {
            let line = buffer[..newline_idx].trim().to_string();
            buffer.drain(..=newline_idx);

            if line.starts_with("data: ") {
                let json_data = &line[6..];

                if json_data == "[DONE]" {
                    break;
                }

                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_data) {
                    let delta = &parsed["choices"][0]["delta"];

                    if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str())
                    {
                        if !reasoning.is_empty() {
                            has_thought = true;
                            // Write raw bytes to the Unix stream instead of stdout
                            let formatted = format!("\x1B[90m{}\x1B[0m", reasoning);
                            stream.write_all(formatted.as_bytes()).await?;
                            stream.flush().await?;
                            // full_text.push_str(reasoning);
                        }
                    }

                    if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                        if content.contains("<think>") || content.contains("<thinking>") {
                            in_thinking_block = true;
                        }

                        if in_thinking_block {
                            has_thought = true;
                            let formatted = format!("\x1B[90m{}\x1B[0m", content);
                            stream.write_all(formatted.as_bytes()).await?;
                            stream.flush().await?;
                            // full_text.push_str(content);
                        } else {
                            if !printed_mobius_prefix {
                                if has_thought {
                                    stream.write_all(b"\n").await?;
                                }
                                stream.write_all(b"\x1B[32mMobius:\x1B[0m ").await?;
                                stream.flush().await?;
                                printed_mobius_prefix = true;
                            }

                            stream.write_all(content.as_bytes()).await?;
                            stream.flush().await?;
                            full_text.push_str(content);
                        }

                        if content.contains("</think>") || content.contains("</thinking>") {
                            in_thinking_block = false;
                        }
                    }
                }
            }
        }
    }

    stream.write_all(b"\n").await?;
    stream.flush().await?;

    Ok(full_text)
}
