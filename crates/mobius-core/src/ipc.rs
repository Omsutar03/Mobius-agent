use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub command: String,
    pub output: String,
}

/// The payload sent from the CLI/GUI Client to the Background Daemon
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IpcRequest {
    pub ppid: u32,
    pub prompt: String,
    pub thinking_level_override: Option<String>,
    pub model_override: Option<String>,
    pub new_session: bool,
    pub query_tokens: bool,
    pub query_model: bool,
    pub query_thinking: bool,
    pub shutdown: bool,
    pub record_history: Option<HistoryEntry>,
}

/// The structured events sent back from the Daemon to the CLI/GUI
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum DaemonEvent {
    ThinkingChunk(String),
    TextChunk(String),
    ToolStart { tool_name: String, details: String },
    ToolFinished { result: String },
    TokenUsage { prompt: usize, completion: usize },
    Error(String),
    Done,
}
