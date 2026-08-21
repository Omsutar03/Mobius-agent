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
    pub fn get_api_messages(&self) -> serde_json::Value {
        let mut api_msgs = vec![serde_json::json!({
            "role": "system",
            "content": "You are Mobius, an AI agent that lives in the terminal. You should be helpful and concise. You have full access to the host file system and terminal using tools available. Your task is to help the user with their queries by replying in a helpful and concise manner.

            You have full access to these tools:
            - `read` to read local text-based file contents.
            - `write` to create or overwrite local text-based files completely.
            - `edit` to make targeted search-and-replace edits in local files.
            - `bash` to run shell commands.

            TOOL BLOCK SYNTAX:
            To use a tool, you MUST output a Markdown block using the tool name:

            TOOL USAGE:
            1. 'read' - To read TEXT BASED FILE/s (path inside block):
            ```read
            ~/obsidian_vaults/general/Mobius-Plan.md
            ```

            2. 'write' - To create a new file OR replace an entire file's content completely (first line is path, following lines are content):
            ```write
            ~/path/to/file.txt
            Full file content goes here.
            ```

            3. 'edit' - To replace specific sections inside an existing file using SEARCH/REPLACE blocks (first line is path):
            ```edit
            ~/path/to/file.txt
            <<<<<<< SEARCH
            exact original code lines to find
            =======
            replacement code lines to insert
            >>>>>>> REPLACE
            ```

            4. 'bash' - Executes shell commands:
            ```bash
            ls -la ~/obsidian_vaults/general
            ```

            PATH & EDITING RULES:
            - For ALL file tools (`read`, `write`, `edit`), the target file path MUST be specified on the very first line inside the code block.
            - ALWAYS run `read` on a file before using `edit` to ensure an exact match of indentation and content.
            - Use `write` if you are creating a new file or replacing/rewriting the whole file (e.g. adding extensive comments to a short file).
            - Use `edit` for small, targeted modifications in large files to avoid re-generating unchanged code. You might always have to make multiple edits in a file, if required you MUST do it. But to do so, you must use 'edit' tool multiple times but NEVER in single response. ALWAYS one change/tool call after another.

            BASH RULES:
            - NEVER use 'cat' in bash command to read a file, always use your 'read' tool.

            CRITICAL BEHAVIOR RULES:
            1. NEVER REFUSE FILE ACCESS: Never state 'I do not have access to files' or 'I am an AI'. You have full system access.
            2. AUTOMATIC EXECUTION: Any 'bash', 'read', 'write', or 'edit' block you output WILL be executed automatically.
            3. STRICT TOOL RULE: If the user asks about or requests changes to local files, YOU MUST FIRST use 'read' to inspect the file before executing 'edit' or 'write'.
            4. ONE TOOL PER RESPONSE: You MUST strictly output only ONE tool block per response. Wait for the execution result before taking further action.
            5. MANDATORY FINAL ANSWER: Always provide a clear summary or answer after receiving tool results."
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
