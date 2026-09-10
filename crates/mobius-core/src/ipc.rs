use serde::{Deserialize, Serialize};

/// The payload sent from the CLI/GUI Client to the Background Daemon
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IpcRequest {
    pub ppid: u32,
    pub prompt: String,
    pub thinking_level_override: Option<String>,
    pub model_override: Option<String>,
    pub new_session: bool,
    pub query_model: bool,
    pub query_thinking: bool,
    pub shutdown: bool,
    #[serde(default)]
    pub list_sessions: bool,
    #[serde(default)]
    pub load_session: Option<String>,
    #[serde(default)]
    pub delete_session: Option<String>,
    #[serde(default)]
    pub query_llm_status: bool,
    #[serde(default)]
    pub llm_provider: Option<String>,
}

/// The structured events sent back from the Daemon to the CLI/GUI
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum DaemonEvent {
    ThinkingChunk(String),
    TextChunk(String),
    ToolStart { tool_name: String, details: String },
    ToolFinished { result: String },
    TokenUsage { prompt: usize, completion: usize, context_window: usize },
    LlmStatus {
        connected: bool,
        provider: String,
        model_name: Option<String>,
    },
    Error(String),
    Done,
    SessionList(Vec<SessionMetadata>),
}

/// Serializable session metadata for IPC transport
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionMetadata {
    pub filename: String,
    pub path: String,
    pub preview: String,
    pub is_active: bool,
    pub message_count: usize,
}
