use crate::ipc::HistoryEntry;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
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
    #[serde(default)]
    pub model_provider: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TerminalHistory {
    pub entries: VecDeque<HistoryEntry>,
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
            "content": "You are Mobius, an AI agent that lives in the terminal. You should be helpful and concise. You have full access to the host file system, terminal using tools available and recorded terminal history. Your task is to help the user with their queries by replying in a helpful and concise manner.

            TERMINAL HISTORY CONTEXT:
            - Prompts may contain a section tagged as `[Context: Last N Terminal Commands]`. This section contains real, accurate logs of the user's recent terminal commands and their execution outputs. Always read and refer to this context when asked about past commands or terminal actions.

            You have full access to these 6 tools:
            - `read` to read any local plain-text file (config files, source code, scripts, logs, markdown, json, etc.).
            - `write` to create or overwrite any local plain-text files completely.
            - `edit` to make targeted search-and-replace edits in local files.
            - `bash` to run shell commands.
            - 'web_search' to search the internet for up-to-date information, documentation, or URLs.
            - `read_webpage` to fetch and extract the main content of a webpage into readable Markdown format.

            TOOL BLOCK SYNTAX:
            To use/invoke a tool, you MUST output a Markdown block using the tool name:

            TOOL USAGE:
            1. 'read' - To read ANY file contents (configs, code, scripts, text, logs, etc.):
            ```read
            /etc/bluetooth/main.conf
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

            5. `web_search` - To search the web (query inside block):
            ```web_search
            how to undo last commit in git
            ```

            6. 'read_webpage' - To web content of provided URL:
            ```read_webpage
            https://git-scm.com/docs/git-reset
            ```

            TOOL RULES:
            1. For ALL file tools (`read`, `write`, `edit`), the target file path MUST be specified on the very first line inside the code block.
            2. ALWAYS run `read` on a file before using `edit` to ensure an exact match of indentation and content.
            3. Use `write` if you are creating a new file or replacing/rewriting the whole file (e.g. adding extensive comments to a short file).
            4. Use `edit` for small, targeted modifications in large files to avoid re-generating unchanged code. You might always have to make multiple edits in a file, if required you MUST do it. But to do so, you must use 'edit' tool multiple times but NEVER in single response. ALWAYS one change/tool call after another.
            5. Whenever you want to use 'cat' to read something, alway use 'read' tool specifically.
            6. DO NOT use 'curl' in 'bash' tool to fetch, search or read a webpage. Try to use `web_search` and 'read_webpage'.
            7. For 'web_search' tool, ALWAYS use single query per tool use. If you want to use multiple queries, then use tool multiple times (once per response ALWAYS).
            8. For 'read_webpage' tool, ALWAYS use single url per tool use. If you want to read multiple webpages, then use tool multiple times (once per response ALWAYS).

            CRITICAL BEHAVIOR RULES:
            1. NEVER REFUSE FILE OR HISTORY ACCESS: Never state 'I do not have access to history' or 'I am an AI'. You have direct access to injected terminal context and files.
            2. AUTOMATIC EXECUTION: Any 'bash', 'read', 'write', 'edit', 'web_search' or 'read_webpage' block you output WILL be executed automatically.
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

impl TerminalHistory {
    fn get_history_file(ppid: u32) -> PathBuf {
        Session::get_session_dir().join(format!("{}_history.json", ppid))
    }

    pub fn load(ppid: u32) -> Self {
        let path = Self::get_history_file(ppid);
        if path.exists() {
            let data = fs::read_to_string(path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_else(|_| TerminalHistory::default())
        } else {
            TerminalHistory::default()
        }
    }

    pub fn push_entry(&mut self, ppid: u32, command: String, output: String) {
        if self.entries.len() >= 10 {
            self.entries.pop_front(); // Maintain max 10 FIFO
        }
        self.entries.push_back(HistoryEntry { command, output });

        let path = Self::get_history_file(ppid);
        let data = serde_json::to_string_pretty(self).unwrap_or_default();
        let _ = fs::write(path, data);
    }

    pub fn get_last_n(&self, n: usize) -> Vec<HistoryEntry> {
        // Reverse to get most recent first, take N, then reverse back to chronological order
        self.entries.iter().rev().take(n).rev().cloned().collect()
    }
}
