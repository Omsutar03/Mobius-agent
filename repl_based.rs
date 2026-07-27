use reedline::{
    EditCommand, EditMode, Emacs, KeyCode, KeyModifiers, Prompt, PromptEditMode,
    PromptHistorySearch, Reedline, ReedlineEvent, Signal, default_emacs_keybindings,
};
use reqwest::Client;
use serde_json::json;
use std::io::{self, Write};
use std::time::Duration;

// --- Splashscreen ---
fn splash_screen() -> &'static str {
    r#"
        ███╗   ███╗ ██████╗ ██████╗ ██╗██╗   ██╗███████╗
        ████╗ ████║██╔═══██╗██╔══██╗██║██║   ██║██╔════╝
        ██╔████╔██║██║   ██║██████╔╝██║██║   ██║███████╗
        ██║╚██╔╝██║██║   ██║██╔══██╗██║██║   ██║╚════██║
        ██║ ╚═╝ ██║╚██████╔╝██████╔╝██║╚██████╔╝███████║
        ╚═╝     ╚═╝ ╚═════╝ ╚═════╝ ╚═╝ ╚═════╝ ╚══════╝
        "#
}

// --- Thinking Level Enum ---
#[derive(Debug, Clone, Copy, PartialEq)]
enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl ThinkingLevel {
    fn next(&self) -> Self {
        match self {
            ThinkingLevel::Off => ThinkingLevel::Minimal,
            ThinkingLevel::Minimal => ThinkingLevel::Low,
            ThinkingLevel::Low => ThinkingLevel::Medium,
            ThinkingLevel::Medium => ThinkingLevel::High,
            ThinkingLevel::High => ThinkingLevel::XHigh,
            ThinkingLevel::XHigh => ThinkingLevel::Max,
            ThinkingLevel::Max => ThinkingLevel::Off,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            ThinkingLevel::Off => "Off",
            ThinkingLevel::Minimal => "Minimal",
            ThinkingLevel::Low => "Low",
            ThinkingLevel::Medium => "Medium",
            ThinkingLevel::High => "High",
            ThinkingLevel::XHigh => "xHigh",
            ThinkingLevel::Max => "Max",
        }
    }

    fn to_params(&self) -> (bool, &'static str) {
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

// --- Custom Reedline Prompt ---
struct MobiusPrompt {
    thinking_level: ThinkingLevel,
}

impl Prompt for MobiusPrompt {
    fn render_prompt_left(&self) -> std::borrow::Cow<str> {
        format!(
            "\x1B[36mYou\x1B[0m [\x1B[33mThinking: {}\x1B[0m]: ",
            self.thinking_level.label()
        )
        .into()
    }

    fn render_prompt_right(&self) -> std::borrow::Cow<str> {
        "".into()
    }

    fn render_prompt_indicator(&self, _edit_mode: PromptEditMode) -> std::borrow::Cow<str> {
        "".into()
    }

    fn render_prompt_multiline_indicator(&self) -> std::borrow::Cow<str> {
        ": ".into()
    }

    fn render_prompt_history_search_indicator(
        &self,
        _history_search: PromptHistorySearch,
    ) -> std::borrow::Cow<'_, str> {
        "".into()
    }
}

// --- Custom Keybinding Setup ---
fn create_custom_edit_mode() -> Box<dyn EditMode> {
    let mut keybindings = default_emacs_keybindings();

    // Bind Ctrl + Backspace to delete whole word behind cursor
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Backspace,
        ReedlineEvent::Edit(vec![EditCommand::BackspaceWord]),
    );

    // Bind Shift + Tab to trigger toggle_thinking action
    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::BackTab,
        ReedlineEvent::ExecuteHostCommand("__mobius_toggle_thinking__".to_string()),
    );

    // Fallback for terminal emulators that send BackTab without explicit SHIFT modifier
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::BackTab,
        ReedlineEvent::ExecuteHostCommand("__mobius_toggle_thinking__".to_string()),
    );

    Box::new(Emacs::new(keybindings))
}

// --- Query Mobius Engine ---
// async fn ask_mobius(
//     client: &Client,
//     server_url: &str,
//     prompt: &str,
//     thinking_level: ThinkingLevel,
// ) -> Result<String, Box<dyn std::error::Error>> {
//     let (enable_thinking, effort_str) = thinking_level.to_params();

//     let payload = json!({
//         "messages": [
//             {
//                 "role": "system",
//                 "content": "You are Mobius agent, a concise terminal AI assistant. Provide direct, helpful answers."
//             },
//             {
//                 "role": "user",
//                 "content": prompt
//             }
//         ],
//         "chat_template_kwargs": {
//             "enable_thinking": enable_thinking,
//             "reasoning_effort": effort_str
//         },
//         "reasoning_effort": effort_str,
//         "stream": true
//     });

//     // let sprite = tokio::spawn(async move {
//     //     let frames = [
//     //         "(•‿•)ゝ",
//     //         "(•‿•)ゞ",
//     //         "(•‿•)ゝ",
//     //         "(•_•)ゞ",
//     //         "(•_•)ゝ",
//     //         "(•_•)ゞ",
//     //         "(-_-)ゝ",
//     //         "(-_-)ゞ",
//     //         "(-_-)ゝ",
//     //         "(⇀‸↼)ゞ",
//     //         "(⇀‸↼)ゝ",
//     //         "(⇀‸↼)ゞ",
//     //     ];

//     //     let mut f = 0;
//     //     loop {
//     //         print!(
//     //             "\r\x1B[2KMobius is thinking... {}",
//     //             frames[f % frames.len()]
//     //         );
//     //         let _ = io::stdout().flush();
//     //         tokio::time::sleep(Duration::from_millis(300)).await;
//     //         f += 1;
//     //     }
//     // });

//     // Send the request and wait ONLY for the initial connection (Time To First Token)
//     let mut response = client.post(server_url).json(&payload).send().await?;

//     // The moment we get HTTP headers back, stop the sprite!
//     // sprite.abort();
//     print!("\r\x1B[2K"); // Clear the sprite line
//     print!("\x1B[32mMobius:\x1B[0m "); // Print the Mobius prefix in green
//     let _ = io::stdout().flush();

//     if !response.status().is_success() {
//         return Err(format!("Server returned HTTP status {}", response.status()).into());
//     }

//     let mut full_text = String::new();
//     let mut buffer = String::new();

//     // Stream the body chunk by chunk
//     while let Some(chunk) = response.chunk().await? {
//         // Convert the raw bytes to a string and add to our buffer
//         buffer.push_str(&String::from_utf8_lossy(&chunk));

//         // Process complete lines (Server-Sent Events are separated by newlines)
//         while let Some(newline_idx) = buffer.find('\n') {
//             let line = buffer[..newline_idx].trim().to_string();
//             buffer.drain(..=newline_idx); // Remove the processed line from buffer

//             if line.starts_with("data: ") {
//                 let json_data = &line[6..];

//                 // llama.cpp sends [DONE] when the stream is finished
//                 if json_data == "[DONE]" {
//                     break;
//                 }

//                 if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_data) {
//                     // Extract the token delta
//                     if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
//                         print!("{}", content);
//                         let _ = io::stdout().flush();
//                         full_text.push_str(content);
//                     }
//                 }
//             }
//         }
//     }

//     Ok(full_text)
// }

async fn ask_mobius(
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

    // Print the Mobius prefix in Green before the stream starts
    print!("\x1B[32mMobius:\x1B[0m ");
    let _ = io::stdout().flush();

    let mut full_text = String::new();
    let mut buffer = String::new();

    // State tracker for raw <think> tags
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

                    // 1. Handle API-native reasoning (DeepSeek R1 via modern llama.cpp)
                    if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str())
                    {
                        // \x1B[90m turns text dark grey, \x1B[0m resets it
                        print!("\x1B[90m{}\x1B[0m", reasoning);
                        let _ = io::stdout().flush();
                        full_text.push_str(reasoning);
                    }

                    // 2. Handle standard content (with fallback for literal <think> tags)
                    if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                        // Check if the token contains the opening tag
                        if content.contains("<think>") || content.contains("<thinking>") {
                            in_thinking_block = true;
                        }

                        // Print in grey if we are in a thinking block, otherwise default terminal color
                        if in_thinking_block {
                            print!("\x1B[90m{}\x1B[0m", content);
                        } else {
                            print!("{}", content);
                        }

                        // Check if the token contains the closing tag
                        if content.contains("</think>") || content.contains("</thinking>") {
                            in_thinking_block = false;
                        }

                        let _ = io::stdout().flush();
                        full_text.push_str(content);
                    }
                }
            }
        }
    }

    Ok(full_text)
}

// --- Main Function ---
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", splash_screen());
    println!("Tip: Press 'Shift+Tab' anytime while typing to toggle thinking mode.\n");

    let client = Client::new();
    let server_url = "http://localhost:8080/v1/chat/completions";

    let mut line_editor = Reedline::create().with_edit_mode(create_custom_edit_mode());
    let mut current_thinking = ThinkingLevel::Off;

    loop {
        let prompt_style = MobiusPrompt {
            thinking_level: current_thinking,
        };

        let sig = line_editor.read_line(&prompt_style);

        match sig {
            Ok(Signal::Success(buffer)) => {
                let prompt = buffer.trim();

                // 1. Intercept our unique keybinding signal FIRST
                if prompt == "__mobius_toggle_thinking__" {
                    current_thinking = current_thinking.next();
                    continue; // Immediately loop back to re-render the new prompt
                }

                // 2. Ignore empty inputs (like hitting Enter on a blank line)
                if prompt.is_empty() {
                    continue;
                }

                // 3. Check for exit commands
                if prompt.eq_ignore_ascii_case("/q") || prompt.eq_ignore_ascii_case("/quit") {
                    println!("Shutting down Mobius....");
                    break;
                }

                // 4. Finally, if it wasn't intercepted, send it to the AI
                match ask_mobius(&client, server_url, prompt, current_thinking).await {
                    Ok(_full_response_text) => println!("\n"),
                    Err(err) => println!("Error: {}\n", err),
                }
            }

            // Gracefully handle Ctrl+C or Ctrl+D interrupts
            Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => {
                println!("\nShutting down Mobius....");
                break;
            }

            Err(err) => {
                println!("Error reading input: {}", err);
                break;
            }
        }
    }

    Ok(())
}
