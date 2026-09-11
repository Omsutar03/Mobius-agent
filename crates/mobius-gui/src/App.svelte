<script lang="ts">
  import { onMount, tick } from "svelte";
  import { marked } from "marked";
  import { daemonClient } from "./lib/ws";
  import type { Message, IpcRequest, SessionMetadata, ModelListEntry } from "./lib/types";
  import { stripToolBlocks } from "./lib/utils/format";

  import Sidebar from "./lib/components/Sidebar.svelte";
  import ControlBar from "./lib/components/ControlBar.svelte";
  import ThinkingBlock from "./lib/components/ThinkingBlock.svelte";
  import ToolBadge from "./lib/components/ToolBadge.svelte";
  import Icon from "./lib/components/Icon.svelte";
  import MobiusLogo from "./lib/components/MobiusLogo.svelte";

  let isConnected = false;
  let isStreaming = false;
  let activePid = Math.floor(Math.random() * 89999) + 10000;

  let modelOptions: ModelListEntry[] = [];
  let selectedModelIndex = 0;
  let llmConnected = false;
  let llmModelName: string | null = null;

  let messages: Message[] = [];
  let sessions: SessionMetadata[] = [];
  let promptTokens = 0;
  let contextWindow = 0;

  let currentThinking = "";
  let currentResponse = "";
  let currentTools: Array<{ tool_name: string; details: string; output?: string }> = [];

  let feedContainer: HTMLElement;

  const suggestions = [
    "What is Mobius and what can it do?",
    "Explain this project's architecture",
    "Write a git commit message for my latest changes"
  ];

  async function scrollToBottom() {
    await tick();
    if (feedContainer) {
      feedContainer.scrollTop = feedContainer.scrollHeight;
    }
  }

  function refreshSessions() {
    if (!isConnected) return;
    daemonClient.listSessions(activePid, {
      onSessionList: (list) => {
        sessions = list;
      },
      onError: () => {
        sessions = [];
      }
    });
  }

  let statusTimer: number | null = null;

  function refreshLlmStatus() {
    if (!isConnected) return;
    daemonClient.queryLlmStatus(activePid, {
      onLlmStatus: (status) => {
        llmConnected = status.connected;
        llmModelName = status.model_name;
      },
      onError: () => {
        llmConnected = false;
        llmModelName = null;
      }
    });
  }

  function refreshModelList() {
    if (!isConnected) return;
    daemonClient.queryModelList(activePid, {
      onModelList: (list) => {
        modelOptions = list;
        if (list.length === 0) {
          selectedModelIndex = 0;
          return;
        }
        const selectedIdx = list.findIndex((m) => m.selected);
        if (selectedIdx >= 0) {
          selectedModelIndex = selectedIdx + 1;
        } else if (selectedModelIndex === 0 || selectedModelIndex > list.length) {
          selectedModelIndex = 1;
        }
      },
      onError: () => {
        modelOptions = [];
      }
    });
  }

  function handleModelChange(index: number) {
    if (index < 1 || index > modelOptions.length) return;
    selectedModelIndex = index;
    daemonClient.selectModelByIndex(activePid, index, {
      onError: () => {},
      onDone: () => {
        refreshModelList();
        refreshLlmStatus();
      }
    });
  }

  onMount(() => {
    daemonClient.connect((connected) => {
      isConnected = connected;
      if (connected) {
        refreshSessions();
        refreshLlmStatus();
        refreshModelList();
        if (!statusTimer) {
          statusTimer = window.setInterval(() => {
            refreshLlmStatus();
            refreshModelList();
          }, 5000);
        }
      } else {
        llmConnected = false;
        llmModelName = null;
        modelOptions = [];
        if (statusTimer) {
          window.clearInterval(statusTimer);
          statusTimer = null;
        }
      }
    });
  });

  function handleNewChat() {
    if (isStreaming) return;
    const req: IpcRequest = {
      ppid: activePid,
      prompt: "",
      new_session: true,
      query_model: false,
      query_thinking: false,
      shutdown: false
    };
    daemonClient.sendRequest(req, {
      onDone: () => {
        messages = [];
        promptTokens = 0;
        contextWindow = 0;
        refreshSessions();
      }
    });
  }

  function handleSelectSession(path: string) {
    if (isStreaming) return;
    daemonClient.loadSession(activePid, path, {
      onTextChunk: (chunk) => {
        try {
          const parsed: Array<{ role: string; content: string }> = JSON.parse(chunk);
          messages = parsed.map((m) => ({
            role: m.role === "assistant" ? "assistant" : "user",
            content: m.content
          }));
        } catch {
          messages = [];
        }
        scrollToBottom();
      },
      onError: (err) => {
        messages = [{ role: "assistant", content: `Failed to load that session.`, error: true }];
        scrollToBottom();
      },
      onDone: () => {
        refreshSessions();
      }
    });
  }

  function handleDeleteSession(path: string) {
    if (isStreaming) return;
    daemonClient.deleteSession(activePid, path, {
      onError: (err) => {
        messages = [...messages, { role: "assistant", content: `Failed to delete session.`, error: true }];
        scrollToBottom();
      },
      onDone: () => {
        refreshSessions();
      }
    });
  }

  function handlePromptSubmit(prompt: string, thinking: string) {
    messages = [...messages, { role: "user", content: prompt }];
    scrollToBottom();

    isStreaming = true;
    currentThinking = "";
    currentResponse = "";
    currentTools = [];

    const request: IpcRequest = {
      ppid: activePid,
      prompt,
      model_number:
        selectedModelIndex > 0 && selectedModelIndex <= modelOptions.length ? selectedModelIndex : null,
      thinking_level_override: thinking,
      new_session: false,
      query_model: false,
      query_thinking: false,
      shutdown: false
    };

    daemonClient.sendRequest(request, {
      onThinkingChunk: (chunk) => {
        currentThinking += chunk;
        scrollToBottom();
      },
      onTextChunk: (chunk) => {
        currentResponse += chunk;
        scrollToBottom();
      },
      onToolStart: (tool) => {
        currentTools = [...currentTools, tool];
        scrollToBottom();
      },
      onToolFinished: (tool) => {
        currentTools = currentTools.map((t) =>
          t.tool_name === tool.tool_name ? { ...t, output: tool.output } : t
        );
        scrollToBottom();
      },
      onTokenUsage: (usage) => {
        promptTokens = usage.prompt;
        contextWindow = usage.context_window;
      },
      onDone: () => {
        messages = [
          ...messages,
          {
            role: "assistant",
            content: stripToolBlocks(currentResponse),
            thinking: currentThinking,
            tools: [...currentTools]
          }
        ];
        currentThinking = "";
        currentResponse = "";
        currentTools = [];
        isStreaming = false;
        refreshSessions();
        scrollToBottom();
      },
      onError: (err) => {
        messages = [...messages, { role: "assistant", content: `Something went wrong: ${err}`, error: true }];
        isStreaming = false;
        scrollToBottom();
      }
    });
  }
</script>

<main class="app-layout">
  <Sidebar
    {activePid}
    {isConnected}
    llmConnected={llmConnected}
    llmModelName={llmModelName}
    {sessions}
    onNewChat={handleNewChat}
    onSelectSession={handleSelectSession}
    onDeleteSession={handleDeleteSession}
  />

  <section class="chat-viewport">
    <div class="message-feed" bind:this={feedContainer}>
      {#if messages.length === 0 && !isStreaming}
        <div class="empty-state">
<div class="empty-logo">
          <MobiusLogo size={34} strokeWidth={30} />
        </div>
          <h1 class="empty-title">Mobius</h1>
          <p class="empty-sub">
            {#if !llmConnected}
              The LLM isn't connected yet. Make sure the daemon is running, then pick a model below.
            {:else}
              Your local AI agent, ready when you are.
            {/if}
          </p>
          <div class="empty-suggestions">
            {#each suggestions as s}
              <button class="suggestion-chip" on:click={() => handlePromptSubmit(s, "med")}>
                <span class="chip-icon">
                  <Icon name="sparkles" size={14} />
                </span>
                <span>{s}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      {#each messages as msg}
        <div class="message-row {msg.role}" class:error={msg.error}>
          <div class="avatar">
            {#if msg.role === "user"}
              <Icon name="user" size={16} />
            {:else}
              <MobiusLogo size={20} strokeWidth={30} />
            {/if}
          </div>
          <div class="message-body">
            {#if msg.error}
              <div class="error-banner" role="alert">
                <span class="banner-icon">
                  <Icon name="alert" size={16} />
                </span>
                <span>{msg.content}</span>
              </div>
            {:else}
              {#if msg.thinking}
                <ThinkingBlock content={msg.thinking} />
              {/if}

              {#if msg.tools}
                {#each msg.tools as tool}
                  <ToolBadge toolName={tool.tool_name} details={tool.details} output={tool.output} />
                {/each}
              {/if}

              <div class="markdown-content">
                {@html marked.parse(msg.content)}
              </div>
            {/if}
          </div>
        </div>
      {/each}

      {#if isStreaming}
        <div class="message-row assistant streaming">
          <div class="avatar">
            <MobiusLogo size={20} strokeWidth={30} />
          </div>
          <div class="message-body">
            {#if currentThinking}
              <ThinkingBlock content={currentThinking} />
            {/if}

            {#each currentTools as tool}
              <ToolBadge toolName={tool.tool_name} details={tool.details} output={tool.output} />
            {/each}

            {#if currentResponse}
              <div class="markdown-content">
                {@html marked.parse(stripToolBlocks(currentResponse))}
                <span class="typing-caret" aria-hidden="true"></span>
              </div>
            {:else}
              <div class="typing-indicator" aria-label="Assistant is thinking">
                <span></span><span></span><span></span>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <ControlBar
      {isStreaming}
      {promptTokens}
      {contextWindow}
      modelOptions={modelOptions}
      modelIndex={selectedModelIndex}
      onModelChange={handleModelChange}
      onSubmit={handlePromptSubmit}
    />
  </section>
</main>

<style>
  .app-layout {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background-color: var(--bg-base);
  }
  .chat-viewport {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
  }

  /* ── Message feed ───────────────────────────────────────────────────────── */
  .message-feed {
    flex: 1;
    overflow-y: auto;
    padding: 32px 28px 24px;
    display: flex;
    flex-direction: column;
    gap: 22px;
    background:
      radial-gradient(1200px 400px at 50% -10%, rgba(124, 58, 237, 0.08), transparent 70%),
      var(--bg-base);
  }

  .message-row {
    display: flex;
    gap: 12px;
    max-width: 860px;
    width: 100%;
    margin: 0 auto;
    text-align: left;
  }

  .message-row.user {
    align-self: flex-end;
    flex-direction: row-reverse;
  }

  .avatar {
    width: 30px;
    height: 30px;
    border-radius: 9px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-top: 2px;
    box-shadow: var(--shadow-elevated);
  }
  .message-row.user .avatar {
    background: var(--bg-active);
    border: 1px solid var(--border-strong);
    color: var(--text-secondary);
  }
  .message-row.assistant .avatar {
    background: var(--brand-gradient);
    border: 1px solid var(--accent-border);
    color: #fff;
  }

  .message-body {
    max-width: 88%;
    min-width: 0;
    word-break: break-word;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  /* User bubble */
  .message-row.user .message-body {
    background: linear-gradient(180deg, rgba(139, 92, 246, 0.22), rgba(139, 92, 246, 0.14));
    border: 1px solid var(--accent-border);
    border-radius: var(--radius-lg) var(--radius-lg) 4px var(--radius-lg);
    padding: 10px 16px;
    color: var(--text-contrast);
  }

  /* Assistant bubble */
  .message-row.assistant .message-body {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg) var(--radius-lg) var(--radius-lg) 4px;
    padding: 14px 18px;
    color: var(--text-primary);
  }

  /* Error message */
  .message-row.error .message-body {
    background: var(--error-soft);
    border-color: rgba(248, 113, 113, 0.35);
  }
  .message-row.error .avatar {
    background: var(--error-strong);
    border-color: transparent;
    color: #fff;
  }
  .error-banner {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    color: var(--error);
    font-size: 0.85rem;
    line-height: 1.5;
  }
  .banner-icon {
    display: flex;
    align-items: flex-start;
    margin-top: 2px;
    flex-shrink: 0;
  }

  /* ── Empty state ────────────────────────────────────────────────────────── */
  .empty-state {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    max-width: 460px;
    text-align: center;
    padding: 12px 20px 40px;
  }
  .empty-logo {
    width: 64px;
    height: 64px;
    border-radius: 18px;
    background: var(--brand-gradient);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: var(--brand-glow);
    margin-bottom: 4px;
  }
  .empty-title {
    margin: 0;
    font-size: 1.6rem;
    font-weight: 650;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }
  .empty-sub {
    margin: 0;
    font-size: 0.92rem;
    line-height: 1.6;
    color: var(--text-muted);
  }
  .empty-suggestions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    margin-top: 12px;
  }
  .suggestion-chip {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 0.88rem;
    cursor: pointer;
    text-align: left;
    transition: border-color var(--dur-med) var(--ease), background var(--dur-med) var(--ease),
      color var(--dur-med) var(--ease), transform var(--dur-med) var(--ease);
  }
  .chip-icon {
    display: flex;
    align-items: center;
    color: var(--accent);
    flex-shrink: 0;
  }
  .suggestion-chip:hover {
    background: var(--bg-elevated);
    border-color: var(--accent-border);
    color: var(--text-primary);
    transform: translateY(-1px);
  }

  /* ── Streaming indicators ───────────────────────────────────────────────── */
  .typing-caret {
    display: inline-block;
    width: 8px;
    height: 15px;
    margin-left: 2px;
    border-radius: 2px;
    background: var(--accent);
    vertical-align: text-bottom;
    animation: caret-blink 0.9s steps(2, start) infinite;
  }
  @keyframes caret-blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0;
    }
  }
  .typing-indicator {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--text-muted);
    font-size: 0.85rem;
    padding: 4px 0;
  }
  .typing-indicator span {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    opacity: 0.35;
    animation: typing-bounce 1.2s var(--ease) infinite;
  }
  .typing-indicator span:nth-child(2) {
    animation-delay: 0.15s;
  }
  .typing-indicator span:nth-child(3) {
    animation-delay: 0.3s;
  }
  @keyframes typing-bounce {
    0%,
    60%,
    100% {
      transform: translateY(0);
      opacity: 0.35;
    }
    30% {
      transform: translateY(-4px);
      opacity: 1;
    }
  }

  /* ── Markdown ───────────────────────────────────────────────────────────── */
  .markdown-content {
    line-height: 1.65;
    font-size: 0.95rem;
    color: var(--text-primary);
  }
  :global(.markdown-content p) {
    margin: 0 0 10px 0;
  }
  :global(.markdown-content p:last-child) {
    margin-bottom: 0;
  }
  :global(.markdown-content h1, .markdown-content h2, .markdown-content h3) {
    margin: 16px 0 8px;
    line-height: 1.3;
    letter-spacing: -0.01em;
  }
  :global(.markdown-content h1) {
    font-size: 1.35rem;
  }
  :global(.markdown-content h2) {
    font-size: 1.15rem;
  }
  :global(.markdown-content h3) {
    font-size: 1rem;
  }
  :global(.markdown-content ul, .markdown-content ol) {
    padding-left: 22px;
    margin: 8px 0;
  }
  :global(.markdown-content li) {
    margin: 4px 0;
  }
  :global(.markdown-content a) {
    color: var(--chain);
    text-decoration: none;
  }
  :global(.markdown-content a:hover) {
    text-decoration: underline;
  }
  :global(.markdown-content code) {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 5px;
    font-family: var(--mono);
    font-size: 0.85em;
    color: var(--info);
  }
  :global(.markdown-content pre) {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    padding: 14px 16px;
    border-radius: var(--radius-md);
    overflow-x: auto;
    margin: 10px 0;
  }
  :global(.markdown-content pre code) {
    background: transparent;
    border: none;
    padding: 0;
    color: var(--text-secondary);
    font-size: 0.85rem;
    line-height: 1.6;
  }
  :global(.markdown-content blockquote) {
    margin: 10px 0;
    padding: 2px 14px;
    border-left: 3px solid var(--accent);
    color: var(--text-secondary);
  }
  :global(.markdown-content table) {
    border-collapse: collapse;
    margin: 10px 0;
    font-size: 0.88rem;
  }
  :global(.markdown-content th, .markdown-content td) {
    border: 1px solid var(--border);
    padding: 6px 12px;
    text-align: left;
  }
  :global(.markdown-content th) {
    background: var(--bg-elevated);
    color: var(--text-secondary);
  }
  :global(.markdown-content hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 16px 0;
  }
  :global(.markdown-content img) {
    max-width: 100%;
    border-radius: var(--radius-md);
  }
</style>