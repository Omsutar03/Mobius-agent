# Mobius

Terminal-first AI agent harness backed by local LLMs (llama.cpp, Ollama, LM Studio). A background daemon serves chat, tool execution, and session persistence over WebSocket; the CLI and a Tauri desktop GUI both connect to it.

## Architecture

```
┌──────────────────────┐   WebSocket (43812)   ┌─────────────────────────┐
│  mobius-cli          │ ─────────────────────▶ │  mobius-daemon          │
│  (terminal client)   │  IpcRequest/DaemonEvent│  (Tokio WS server)     │
└──────────────────────┘                        │                         │
┌──────────────────────┐                        │  mobius-core            │
│  mobius-gui          │ ─────────────────────▶ │   ├ ipc.rs  (protocol)  │
│  (Tauri + Svelte 5)  │                        │   ├ engine.rs (LLM)    │
└──────────────────────┘                        │   ├ session.rs         │
                                                │   ├ inferences.rs      │
                                                │   └ tools.rs           │
                                                └───────────┬────────────┘
                                                            │ OpenAI-compatible
                                                  ┌─────────┼─────────┐
                                                  ▼         ▼         ▼
                                             llama.cpp  Ollama   LM Studio
                                             (:8080)  (:11434)  (:1234)
```

**mobius-core** — shared library. Protocol types (`IpcRequest` / `DaemonEvent`), `Session` persistence, LLM inference (`ask_mobius`), local model discovery, and tool execution (bash, file read/write/edit, web search, read webpage).

**mobius-daemon** — background WebSocket service on `ws://127.0.0.1:43812`. Accepts IPC requests, runs inference with streaming, manages sessions on disk.

**mobius-cli** — terminal client. Auto-starts the daemon if not running. Number-based model and thinking-level selection.

**mobius-gui** — desktop app (Tauri 2, Svelte 5, TypeScript). Connection status, live model dropdown (numbered), past session browser, context-window usage display.

## Prerequisites

- Rust (stable) + Cargo
- Node.js (18+) + npm
- One of: llama.cpp, Ollama, or LM Studio running locally

## Build

### 1. Rust crates (CLI + daemon + core)

```bash
cargo build --release
```

Binaries land in `target/release/`:
- `mobius` — CLI
- `mobius-daemon` — WebSocket daemon

### 2. Desktop GUI

```bash
cd crates/mobius-gui
npm install
npm run tauri build
```

The Tauri binary is built alongside the Svelte frontend; output appears in `crates/mobius-gui/target/release/bundle/`.

## Quick Start

1. Start a local LLM server (e.g. `ollama serve` or launch llama.cpp / LM Studio).
2. Run the CLI:
   ```bash
   ./target/release/mobius "Explain Tokio async channels"
   ```
   The daemon starts automatically on first use.
3. Or launch the GUI:
   ```bash
   ./target/release/mobius --gui
   ```

## Usage

```bash
mobius [FLAGS] [PROMPT]

FLAGS:
  -h, --help              Show help
  --gui                   Launch the desktop GUI
  -n, --new-s             Archive current session & start fresh
  -s, --session [PID]     Interactive session browser (TUI) or switch to PID
  -m, --model [NUM]       Query available models or select one by number
  -t, --thinking [LVL]    Query or set thinking level (off/min/low/med/high/xhigh/max)
  --stop-daemon           Stop the background daemon
```

### Examples

```bash
mobius "What is the capital of France?"       # chat
mobius -m                                      # list available local models
mobius -m 2                                    # select model #2
mobius -t                                      # show thinking level + options
mobius -t 3                                    # set thinking level (0=off … 6=max)
mobius -s                                      # open session browser (TUI)
mobius -s 12345                                # switch to session for PID 12345
mobius -n                                      # start a fresh session
mobius --stop-daemon                           # shut down the daemon
```

## Supported Backends

| Backend       | Port   | API base                    |
|---------------|--------|-----------------------------|
| llama.cpp     | 8080   | `/v1/chat/completions`      |
| Ollama        | 11434  | `/v1/chat/completions`      |
| LM Studio     | 1234   | `/v1/chat/completions`      |

Models are discovered automatically via `/v1/models` on each port. Only running servers appear in the list.

## Session Storage

Sessions are stored as JSON files in `~/.local/state/mobius/sessions/<ppid>.json`, one per terminal PID. Each file tracks message history, selected model provider/name, and thinking level.
