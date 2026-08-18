use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Session {
    pub messages: Vec<Message>,
    #[serde(default)]
    pub thinking_level: Option<String>,
}

impl Session {
    fn get_session_dir() -> PathBuf {
        let home = env::var("HOME").expect("Could not find HOME directory");
        let dir = PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("mobius")
            .join("sessions");

        if !dir.exists() {
            fs::create_dir_all(&dir).expect("Failed to create sessions directory");
        }
        dir
    }

    fn get_file_path(ppid: u32) -> PathBuf {
        Self::get_session_dir().join(format!("{}.json", ppid))
    }

    /// Load an existing session from disk, or return a new empty one
    pub fn load(ppid: u32) -> Self {
        let path = Self::get_file_path(ppid);
        if path.exists() {
            let data = fs::read_to_string(path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_else(|_| Session::default())
        } else {
            Session::default()
        }
    }

    /// Save the current session back to disk
    pub fn save(&self, ppid: u32) {
        let path = Self::get_file_path(ppid);
        let data = serde_json::to_string_pretty(self).expect("Failed to serialize session");
        fs::write(path, data).expect("Failed to write session file");
    }

    /// Delete the session file (used when --new-s is passed)
    pub fn clear(ppid: u32) {
        let path = Self::get_file_path(ppid);
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }

    /// Append a new message to the history
    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(Message {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    /// Formats the message history for the LLM API, injecting the System Prompt at the top
    // PERFECTLY WORKING PROMPT
    // let mut api_msgs = vec![serde_json::json!({
    //     "role": "system",
    //     "content": "You are an helpful and concise terminl AI agent called Mobius. You have access to user's terminal via a tool called `bash` tool. It allows you to interact with the system via terminal. So while interacting, whatever command is related to terminal execute it using the 'bash' tool by yourself.

    //     'bash' TOOL USAGE with strict syntax as following (DO NOT USE markdown code blocks):
    //     bash <command>

    //     if joining two commands using '&&' then follow this (DO NOT USE markdown code blocks):
    //     bash <command> && <command>

    //     The keyword 'bash' acts as a tool invoker, the arguments you pass to it will be considered as command to be executed.

    //     'bash' TOOL OUTPUT:
    //     After using 'bash' tool, the command will get executed and whatever is the ouput generated will be fed back to you automatically as a user's prompt

    //     CRITICAL BEHAVIOR RULES:
    //     1. ASSESS THE NEED: If the user asks a general question or greeting, respond naturally in plain text. DO NOT use 'bash' tool. Use tool only if and when necessary.
    //     2. MANDATORY FINAL ANSWER (only if you used tool): After executing a command and receiving the tool's output, you MUST analyze the results and provide a clear, natural language final answer to the user. Never leave a task unfinished or return an empty response.
    //     3. NO LIMITATIONS: You have full terminal access through Mobius. Never claim you cannot access the host system."
    // })];
    pub fn get_api_messages(&self) -> serde_json::Value {
        let mut api_msgs = vec![serde_json::json!({
            "role": "system",
            "content": "You are Mobius, a concise terminal AI assistant with full access to the host file system and terminal using tools available.

            TOOL BLOCK SYNTAX:
            To use a tool, you MUST output a Markdown block using the tool name:

            LIST OF TOOLS:
            1. 'read' - Reads a TEXT BASED FILE's contents:
            ```read
            ~/obsidian_vaults/general/Mobius-Plan.md
            ```

            2. 'bash' - Executes shell commands:
            ```bash
            ls -la ~/obsidian_vaults/general
            ```

            PATH RESOLUTION RULES:
            - If the user mentions a filename that was previously listed in an 'ls' or 'bash' output, COMBINE the directory path from history with the filename (e.g., '~/obsidian_vaults/general/Mobius-Plan.md').
            - Use 'read' instead of 'cat' whenever you need to inspect file contents.

            CRITICAL BEHAVIOR RULES:
            1. NEVER REFUSE FILE ACCESS: Never state 'I do not have access to files' or 'I am an AI'. You have full system access. If a file path is incomplete, locate it or construct the path from chat history.
            2. AUTOMATIC EXECUTION: Any 'bash' or 'read' block you output WILL be executed automatically.
            3. MANDATORY FINAL ANSWER: Always provide a clear summary or answer after receiving tool results."
        })];

        for msg in &self.messages {
            api_msgs.push(serde_json::json!({
                "role": &msg.role,
                "content": &msg.content
            }));
        }

        serde_json::Value::Array(api_msgs)
    }
}
