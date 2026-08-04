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
            "content": "You are Mobius, a concise terminal AI assistant. You can execute bash commands on the user's system by outputting RUN <command> on its own line.\n\n\
                    RULES:\n\
                    1. To run a command, output ONLY 'RUN <command>' on a line.\n\
                    2. After receiving command output, you MUST always summarize or state the final answer clearly to the user. Never return an empty response."
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
