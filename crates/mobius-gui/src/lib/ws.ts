import type { IpcRequest, DaemonEvent, SessionMetadata, ModelListEntry } from "./types";

export type EventCallbacks = {
  onThinkingChunk?: (chunk: string) => void;
  onTextChunk?: (chunk: string) => void;
  onToolStart?: (tool: { tool_name: string; details: string }) => void;
  onToolFinished?: (tool: { tool_name: string; output: string }) => void;
  onTokenUsage?: (usage: {
    prompt: number;
    completion: number;
    context_window: number;
  }) => void;
  onLlmStatus?: (status: {
    connected: boolean;
    provider: string;
    model_name: string | null;
  }) => void;
  onError?: (error: string) => void;
  onDone?: () => void;
  onSessionList?: (sessions: SessionMetadata[]) => void;
  onModelList?: (models: ModelListEntry[]) => void;
};

export class DaemonClient {
  private url: string;
  private ws: WebSocket | null = null;
  private isConnected: boolean = false;
  private reconnectTimer: number | null = null;
  private reqQueue: Array<{ request: IpcRequest; callbacks: EventCallbacks }> = [];
  private activeCallbacks: EventCallbacks | null = null;

  constructor(url: string = "ws://127.0.0.1:43812") {
    this.url = url;
  }

  public connect(onStatusChange?: (connected: boolean) => void): Promise<void> {
    return new Promise((resolve) => {
      try {
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
          this.isConnected = true;
          if (onStatusChange) onStatusChange(true);
          resolve();
        };

        this.ws.onmessage = (event: MessageEvent) => {
          let data: DaemonEvent;
          try {
            data = JSON.parse(event.data);
          } catch (e) {
            if (this.activeCallbacks?.onError) {
              this.activeCallbacks.onError("Failed to parse daemon payload.");
            }
            return;
          }

          const cb = this.activeCallbacks;
          switch (data.type) {
            case "ThinkingChunk":
              if (cb?.onThinkingChunk) cb.onThinkingChunk(data.payload);
              break;
            case "TextChunk":
              if (cb?.onTextChunk) cb.onTextChunk(data.payload);
              break;
            case "ToolStart":
              if (cb?.onToolStart) cb.onToolStart(data.payload);
              break;
            case "ToolFinished":
              if (cb?.onToolFinished) {
                cb.onToolFinished({
                  tool_name: "",
                  output: data.payload.result,
                });
              }
              break;
            case "TokenUsage":
              if (cb?.onTokenUsage) cb.onTokenUsage(data.payload);
              break;
            case "LlmStatus":
              if (cb?.onLlmStatus) cb.onLlmStatus(data.payload);
              break;
            case "Error":
              if (cb?.onError) cb.onError(data.payload);
              break;
            case "Done":
              // Terminates the active request. Advance BEFORE the callback so a
              // callback that issues a new request doesn't deadlock the queue.
              this.activeCallbacks = null;
              if (cb?.onDone) cb.onDone();
              this.processQueue();
              break;
            case "SessionList":
              if (cb?.onSessionList) cb.onSessionList(data.payload);
              break;
            case "ModelList":
              if (cb?.onModelList) cb.onModelList(data.payload);
              break;
          }
        };

        this.ws.onclose = () => {
          this.isConnected = false;
          this.reqQueue = [];
          this.activeCallbacks = null;
          if (onStatusChange) onStatusChange(false);
          // Try auto-reconnect after 3 seconds
          if (!this.reconnectTimer) {
            this.reconnectTimer = window.setTimeout(() => {
              this.reconnectTimer = null;
              this.connect(onStatusChange);
            }, 3000);
          }
        };

        this.ws.onerror = () => {
          this.isConnected = false;
          if (onStatusChange) onStatusChange(false);
        };
      } catch (err) {
        this.isConnected = false;
        if (onStatusChange) onStatusChange(false);
        resolve();
      }
    });
  }

  private processQueue() {
    if (this.activeCallbacks || this.reqQueue.length === 0) return;
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;
    const next = this.reqQueue.shift()!;
    this.activeCallbacks = next.callbacks;
    this.ws.send(JSON.stringify(next.request));
  }

  public sendRequest(request: IpcRequest, callbacks: EventCallbacks): boolean {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      if (callbacks.onError) {
        callbacks.onError("Daemon is disconnected. Make sure mobius-daemon is running.");
      }
      return false;
    }

    this.reqQueue.push({ request, callbacks });
    this.processQueue();
    return true;
  }

  public getConnected(): boolean {
    return this.isConnected;
  }

  public listSessions(ppid: number, callbacks: EventCallbacks): boolean {
    return this.sendRequest(
      {
        ppid,
        prompt: "",
        new_session: false,
        query_model: false,
        query_thinking: false,
        shutdown: false,
        list_sessions: true
      },
      callbacks
    );
  }

  public loadSession(ppid: number, path: string, callbacks: EventCallbacks): boolean {
    return this.sendRequest(
      {
        ppid,
        prompt: "",
        new_session: false,
        query_model: false,
        query_thinking: false,
        shutdown: false,
        load_session: path
      },
      callbacks
    );
  }

  public deleteSession(ppid: number, path: string, callbacks: EventCallbacks): boolean {
    return this.sendRequest(
      {
        ppid,
        prompt: "",
        new_session: false,
        query_model: false,
        query_thinking: false,
        shutdown: false,
        delete_session: path
      },
      callbacks
    );
  }

  public queryLlmStatus(ppid: number, callbacks: EventCallbacks): boolean {
    return this.sendRequest(
      {
        ppid,
        prompt: "",
        new_session: false,
        query_model: false,
        query_thinking: false,
        shutdown: false,
        query_llm_status: true
      },
      callbacks
    );
  }

  public queryModelList(ppid: number, callbacks: EventCallbacks): boolean {
    return this.sendRequest(
      {
        ppid,
        prompt: "",
        new_session: false,
        query_model: false,
        query_thinking: false,
        shutdown: false,
        query_model_list: true
      },
      callbacks
    );
  }

  public selectModelByIndex(ppid: number, index: number, callbacks: EventCallbacks): boolean {
    return this.sendRequest(
      {
        ppid,
        prompt: "",
        new_session: false,
        query_model: false,
        query_thinking: false,
        shutdown: false,
        model_number: index
      },
      callbacks
    );
  }
}

export const daemonClient = new DaemonClient();
