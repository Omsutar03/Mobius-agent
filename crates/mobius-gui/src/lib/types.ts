// Matches mobius_core::ipc::IpcRequest
export interface IpcRequest {
  ppid: number;
  prompt: string;
  thinking_level_override?: string | null;
  new_session: boolean;
  query_model: boolean;
  query_thinking: boolean;
  shutdown: boolean;
  list_sessions?: boolean;
  load_session?: string | null;
  delete_session?: string | null;
  query_llm_status?: boolean;
  llm_provider?: string | null;
  model_number?: number | null;
  query_model_list?: boolean;
}

// Matches #[serde(tag = "type", content = "payload")] DaemonEvent JSON serialization
export type DaemonEvent =
  | { type: "ThinkingChunk"; payload: string }
  | { type: "TextChunk"; payload: string }
  | { type: "ToolStart"; payload: { tool_name: string; details: string } }
  | { type: "ToolFinished"; payload: { result: string } }
  | { type: "TokenUsage"; payload: { prompt: number; completion: number; context_window: number } }
  | {
      type: "LlmStatus";
      payload: { connected: boolean; provider: string; model_name: string | null };
    }
  | { type: "Error"; payload: string }
  | { type: "Done" }
  | { type: "SessionList"; payload: SessionMetadata[] }
  | { type: "ModelList"; payload: ModelListEntry[] };

// Matches mobius_core::ipc::ModelListEntry
export interface ModelListEntry {
  provider: string;
  model_name: string;
  display_name: string;
  selected: boolean;
}

export interface Message {
  role: "user" | "assistant";
  content: string;
  thinking?: string;
  tools?: Array<{ tool_name: string; details: string; output?: string }>;
  error?: boolean;
}

// Matches mobius_core::ipc::SessionMetadata
export interface SessionMetadata {
  filename: string;
  path: string;
  preview: string;
  is_active: boolean;
  message_count: number;
}
