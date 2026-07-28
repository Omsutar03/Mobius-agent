use reqwest::Client;
use serde_json::json;
use std::io::{self, Write};

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
            "minimal" => Some(Self::Minimal),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
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
    prompt: &str,
    thinking_level: ThinkingLevel,
) -> Result<String, Box<dyn std::error::Error>> {
    let (enable_thinking, effort_str) = thinking_level.to_params();

    let payload = json!({
        "messages": [
            {
                "role": "system",
                "content": "You are Mobius agent, a concise terminal AI assistant. Provide direct, helpful answers."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
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

    // Stream state flags
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

                    // 1. Handle native reasoning API field (e.g. reasoning_content)
                    if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str())
                    {
                        if !reasoning.is_empty() {
                            has_thought = true;
                            // Print thinking tokens in dark grey (\x1B[90m)
                            print!("\x1B[90m{}\x1B[0m", reasoning);
                            let _ = io::stdout().flush();
                            full_text.push_str(reasoning);
                        }
                    }

                    // 2. Handle standard content tokens
                    if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                        if content.contains("<think>") || content.contains("<thinking>") {
                            in_thinking_block = true;
                        }

                        if in_thinking_block {
                            has_thought = true;
                            print!("\x1B[90m{}\x1B[0m", content);
                            let _ = io::stdout().flush();
                            full_text.push_str(content);
                        } else {
                            // This is actual response content!
                            // If this is the first content token, print the Mobius prefix
                            if !printed_mobius_prefix {
                                if has_thought {
                                    println!(); // Line break after the thinking block
                                }
                                print!("\x1B[32mMobius:\x1B[0m ");
                                let _ = io::stdout().flush();
                                printed_mobius_prefix = true;
                            }

                            print!("{}", content);
                            let _ = io::stdout().flush();
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

    println!(); // Final newline after stream completes
    Ok(full_text)
}
