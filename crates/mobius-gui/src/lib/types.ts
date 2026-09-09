// Matches mobius_core::ipc::IpcRequest
export interface IpcRequest {
  ppid: number;
  prompt: string;
  thinking_level_override?: string | null;
  model_override?: string | null;
  new_session: boolean;
  query_tokens: boolean;
  query_model: boolean;
  query_thinking: boolean;
  shutdown: boolean;
  record_history?: any | null;
  list_sessions?: boolean;
  load_session?: string | null;
  delete_session?: string | null;
}

// Matches #[serde(tag = "type", content = "payload")] DaemonEvent JSON serialization
export type DaemonEvent =
  | { type: "ThinkingChunk"; payload: string }
  | { type: "TextChunk"; payload: string }
  | { type: "ToolStart"; payload: { tool_name: string; details: string } }
  | { type: "ToolFinished"; payload: { result: string } }
  | { type: "TokenUsage"; payload: { prompt: number; completion: number } }
  | { type: "Error"; payload: string }
  | { type: "Done" }
  | { type: "SessionList"; payload: SessionMetadata[] };

export interface Message {
  role: "user" | "assistant";
  content: string;
  thinking?: string;
  tools?: Array<{ tool_name: string; details: string; output?: string }>;
}

// Matches mobius_core::ipc::SessionMetadata
export interface SessionMetadata {
  filename: string;
  path: string;
  preview: string;
  is_active: boolean;
  message_count: number;
}
