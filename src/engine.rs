use reqwest::Client;
use serde_json::json;
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
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let (enable_thinking, effort_str) = thinking_level.to_params();

    // ALWAYS pass chat_template_kwargs so llama-server explicitly receives enable_thinking: false
    let payload = json!({
        "messages": messages_payload,
        "stream": true,
        "chat_template_kwargs": {
            "enable_thinking": enable_thinking,
            "reasoning_effort": effort_str
        },
        "reasoning_effort": effort_str,
        "temperature": 0.1,
        "min_p": 0.05,
        "top_p": 0.9,
        "presence_penalty": 0.0,
        "frequency_penalty": 0.0
    });

    let mut response = client.post(server_url).json(&payload).send().await?;
    let status = response.status();

    if !status.is_success() {
        let err_body = response.text().await.unwrap_or_default();
        return Err(format!("Server HTTP Status Error ({}): {}", status, err_body).into());
    }

    let mut full_text = String::new(); // Accumulates ALL delta.content
    let mut buffer = String::new();

    let mut printed_mobius_prefix = false;
    let mut has_thought = false;
    let mut in_thinking_block = false;

    // Labeled outer loop to break out of TCP streaming immediately on [DONE]
    'stream_loop: while let Some(chunk) = response.chunk().await? {
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(newline_idx) = buffer.find('\n') {
            let line = buffer[..newline_idx].trim().to_string();
            buffer.drain(..=newline_idx);

            if let Some(json_data) = line.strip_prefix("data: ") {
                if json_data == "[DONE]" {
                    break 'stream_loop;
                }

                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_data) {
                    let delta = &parsed["choices"][0]["delta"];

                    // 1. Handle API's native reasoning_content (DeepSeek / standard API)
                    if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str())
                    {
                        if !reasoning.is_empty() {
                            has_thought = true;
                            let formatted = format!("\x1B[90m{}\x1B[0m", reasoning);
                            stream.write_all(formatted.as_bytes()).await?;
                            stream.flush().await?;
                        }
                    }

                    // 2. Handle standard content (which may contain inline <think> tags)
                    if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                        full_text.push_str(content); // Save ALL content to be cleaned later

                        if content.contains("<think>") || content.contains("<thinking>") {
                            in_thinking_block = true;
                        }

                        if in_thinking_block {
                            has_thought = true;
                            let formatted = format!("\x1B[90m{}\x1B[0m", content);
                            stream.write_all(formatted.as_bytes()).await?;
                            stream.flush().await?;

                            if content.contains("</think>") || content.contains("</thinking>") {
                                in_thinking_block = false;
                            }
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
                        }
                    }
                }
            }
        }
    }

    stream.write_all(b"\n").await?;
    stream.flush().await?;

    // 3. Clean up any inline thinking blocks before saving text to history / passing to tool parser
    let final_cleaned_text = clean_thinking_blocks(&full_text);

    Ok(final_cleaned_text)
}

// Helper function to safely strip thinking blocks so they don't pollute chat history
fn clean_thinking_blocks(text: &str) -> String {
    let mut result = text.to_string();

    // Strip <think>...</think>
    while let Some(start) = result.find("<think>") {
        if let Some(end) = result[start..].find("</think>") {
            let abs_end = start + end + 8;
            result = format!("{}{}", &result[..start], &result[abs_end..]);
        } else {
            // Unclosed tag (dangling thought), strip everything after it
            result = result[..start].to_string();
            break;
        }
    }

    // Strip <thinking>...</thinking>
    while let Some(start) = result.find("<thinking>") {
        if let Some(end) = result[start..].find("</thinking>") {
            let abs_end = start + end + 11;
            result = format!("{}{}", &result[..start], &result[abs_end..]);
        } else {
            result = result[..start].to_string();
            break;
        }
    }

    result.trim().to_string()
}
