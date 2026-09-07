use reqwest::Client;
use tokio::sync::mpsc::Sender;
use crate::ipc::DaemonEvent;

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

#[derive(Debug, Default, Clone, Copy)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

pub async fn ask_mobius(
    client: &Client,
    engine: &crate::inferences::InferenceEngine,
    model_name: &str,
    messages_payload: serde_json::Value,
    thinking_level: ThinkingLevel,
    event_tx: &Sender<DaemonEvent>,
    config: &crate::config::MobiusConfig,
) -> Result<(String, TokenUsage), Box<dyn std::error::Error + Send + Sync>> {
    let server_url = engine.get_url();
    let payload = engine.build_payload(messages_payload, thinking_level, model_name, config);

    let mut response = client.post(server_url).json(&payload).send().await?;
    let status = response.status();

    if !status.is_success() {
        let err_body = response.text().await.unwrap_or_default();
        return Err(format!("Server HTTP Status Error ({}): {}", status, err_body).into());
    }

    let mut full_text = String::new(); // Accumulates ALL delta.content
    let mut buffer = String::new();
    let mut usage = TokenUsage::default();
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
                    // 1. Extract usage metrics when the engine sends them
                    if let Some(usage_obj) = parsed.get("usage") {
                        if let Some(p) = usage_obj.get("prompt_tokens").and_then(|v| v.as_u64()) {
                            usage.prompt_tokens = p as usize;
                        }
                        if let Some(c) = usage_obj.get("completion_tokens").and_then(|v| v.as_u64()) {
                            usage.completion_tokens = c as usize;
                        }
                        if let Some(t) = usage_obj.get("total_tokens").and_then(|v| v.as_u64()) {
                            usage.total_tokens = t as usize;
                        }
                    }
                    // 2. Safe indexing using `.get(0)` to prevent panics on empty 'choices' arrays
                    if let Some(first_choice) = parsed
                        .get("choices")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first())
                    {
                        let delta = &first_choice["delta"];

                        // API Native reasoning content
                        if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                            if !reasoning.is_empty() {
                                let _ = event_tx
                                    .send(DaemonEvent::ThinkingChunk(reasoning.to_string()))
                                    .await;
                            }
                        }

                        // Standard content
                        if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                            full_text.push_str(content);

                            if content.contains("<think>") || content.contains("<thinking>") {
                                in_thinking_block = true;
                            }

                            if in_thinking_block {
                                let _ = event_tx
                                    .send(DaemonEvent::ThinkingChunk(content.to_string()))
                                    .await;

                                if content.contains("</think>") || content.contains("</thinking>") {
                                    in_thinking_block = false;
                                }
                            } else {
                                let _ = event_tx
                                    .send(DaemonEvent::TextChunk(content.to_string()))
                                    .await;
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Clean up any inline thinking blocks before saving text to history / passing to tool parser
    let final_cleaned_text = clean_thinking_blocks(&full_text);
    Ok((final_cleaned_text, usage))
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
