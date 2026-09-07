use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub command: String,
    pub output: String,
}

/// The payload sent from the CLI Client to the Background Daemon
#[derive(Debug, Serialize, Deserialize)]
pub struct IpcRequest {
    pub ppid: u32,
    pub prompt: String,
    pub thinking_level_override: Option<String>,
    pub shutdown: bool,
    pub record_history: Option<HistoryEntry>,
}
