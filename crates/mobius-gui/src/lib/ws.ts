import type { IpcRequest, DaemonEvent, SessionMetadata } from "./types";

export type EventCallbacks = {
  onThinkingChunk?: (chunk: string) => void;
  onTextChunk?: (chunk: string) => void;
  onToolStart?: (tool: { tool_name: string; details: string }) => void;
  onToolFinished?: (tool: { tool_name: string; output: string }) => void;
  onTokenUsage?: (usage: { prompt: number; completion: number }) => void;
  onError?: (error: string) => void;
  onDone?: () => void;
  onSessionList?: (sessions: SessionMetadata[]) => void;
};

export class DaemonClient {
  private url: string;
  private ws: WebSocket | null = null;
  private isConnected: boolean = false;
  private reconnectTimer: number | null = null;

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

        this.ws.onclose = () => {
          this.isConnected = false;
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

  public sendRequest(request: IpcRequest, callbacks: EventCallbacks): boolean {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      if (callbacks.onError) {
        callbacks.onError("Daemon is disconnected. Make sure mobius-daemon is running.");
      }
      return false;
    }

    // Set message listener for this single query response stream
    this.ws.onmessage = (event: MessageEvent) => {
      try {
        const data: DaemonEvent = JSON.parse(event.data);

        switch (data.type) {
          case "ThinkingChunk":
            if (callbacks.onThinkingChunk) callbacks.onThinkingChunk(data.payload);
            break;
          case "TextChunk":
            if (callbacks.onTextChunk) callbacks.onTextChunk(data.payload);
            break;
          case "ToolStart":
            if (callbacks.onToolStart) callbacks.onToolStart(data.payload);
            break;
          case "ToolFinished":
            if (callbacks.onToolFinished) {
              callbacks.onToolFinished({
                tool_name: "",
                output: data.payload.result,
              });
            }
            break;
          case "TokenUsage":
            if (callbacks.onTokenUsage) callbacks.onTokenUsage(data.payload);
            break;
          case "Error":
            if (callbacks.onError) callbacks.onError(data.payload);
            break;
          case "Done":
            if (callbacks.onDone) callbacks.onDone();
            break;
          case "SessionList":
            if (callbacks.onSessionList) callbacks.onSessionList(data.payload);
            break;
        }
      } catch (e) {
        if (callbacks.onError) callbacks.onError("Failed to parse daemon payload.");
      }
    };

    this.ws.send(JSON.stringify(request));
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
        query_tokens: false,
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
        query_tokens: false,
        query_model: false,
        query_thinking: false,
        shutdown: false,
        load_session: path
      },
      callbacks
    );
  }
}

export const daemonClient = new DaemonClient();
