<script lang="ts">
  import { onMount, tick } from "svelte";
  import { marked } from "marked";
  import { daemonClient } from "./lib/ws";
  import type { Message, IpcRequest, SessionMetadata } from "./lib/types";
  import { stripToolBlocks } from "./lib/utils/format";

  import Sidebar from "./lib/components/Sidebar.svelte";
  import ControlBar from "./lib/components/ControlBar.svelte";
  import ThinkingBlock from "./lib/components/ThinkingBlock.svelte";
  import ToolBadge from "./lib/components/ToolBadge.svelte";

  let isConnected = false;
  let isStreaming = false;
  let activePid = Math.floor(Math.random() * 89999) + 10000;

  let messages: Message[] = [];
  let sessions: SessionMetadata[] = [];
  let promptTokens = 0;
  let completionTokens = 0;

  let currentThinking = "";
  let currentResponse = "";
  let currentTools: Array<{ tool_name: string; details: string; output?: string }> = [];

  let feedContainer: HTMLElement;

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

  onMount(() => {
    daemonClient.connect((connected) => {
      isConnected = connected;
      if (connected) {
        refreshSessions();
      }
    });
  });

  function handleNewChat() {
    if (isStreaming) return;
    const req: IpcRequest = {
      ppid: activePid,
      prompt: "",
      new_session: true,
      query_tokens: false,
      query_model: false,
      query_thinking: false,
      shutdown: false
    };
    daemonClient.sendRequest(req, {
      onDone: () => {
        messages = [];
        promptTokens = 0;
        completionTokens = 0;
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
          // Not JSON - treat as a simple confirmation and start empty
          messages = [];
        }
        scrollToBottom();
      },
      onError: (err) => {
        messages = [...messages, { role: "assistant", content: `❌ Error loading session: ${err}` }];
        scrollToBottom();
      },
      onDone: () => {
        refreshSessions();
      }
    });
  }

  function handlePromptSubmit(prompt: string, model: string, thinking: string) {
    messages = [...messages, { role: "user", content: prompt }];
    scrollToBottom();

    isStreaming = true;
    currentThinking = "";
    currentResponse = "";
    currentTools = [];

    const request: IpcRequest = {
      ppid: activePid,
      prompt,
      model_override: model,
      thinking_level_override: thinking === "off" ? null : thinking,
      new_session: false,
      query_tokens: false,
      query_model: false,
      query_thinking: false,
      shutdown: false,
      record_history: null
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
        completionTokens = usage.completion;
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
        messages = [...messages, { role: "assistant", content: `❌ Error: ${err}` }];
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
    {sessions}
    onNewChat={handleNewChat}
    onSelectSession={handleSelectSession}
  />

  <section class="chat-viewport">
    <div class="message-feed" bind:this={feedContainer}>
      {#each messages as msg}
        <div class="message-row {msg.role}">
          <div class="avatar">
            {#if msg.role === "user"}
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path><circle cx="12" cy="7" r="4"></circle></svg>
            {:else}
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8z"/><path d="M12 6v6l4 2"/></svg>
            {/if}
          </div>
          <div class="message-body">
            {#if msg.thinking}
              <ThinkingBlock content={msg.thinking} />
            {/if}

            {#if msg.tools}
              {#each msg.tools as tool}
                <ToolBadge toolName={tool.tool_name} details={tool.details} output={tool.output} />
              {/each}
            {/if}

            <div class="markdown-content">
              {@html marked.parse(stripToolBlocks(msg.content))}
            </div>
          </div>
        </div>
      {/each}

      {#if isStreaming}
        <div class="message-row assistant streaming">
          <div class="avatar">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8z"/><path d="M12 6v6l4 2"/></svg>
          </div>
          <div class="message-body">
            {#if currentThinking}
              <ThinkingBlock content={currentThinking} />
            {/if}

            {#each currentTools as tool}
              <ToolBadge toolName={tool.tool_name} details={tool.details} output={tool.output} />
            {/each}

            <div class="markdown-content">
              {@html marked.parse(stripToolBlocks(currentResponse) || "...")}
            </div>
          </div>
        </div>
      {/if}
    </div>

    <ControlBar
      {isStreaming}
      {promptTokens}
      {completionTokens}
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
    background-color: #0d0d0e;
  }
  .chat-viewport {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
  }
  .message-feed {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
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
  .message-row.user .message-body {
    background-color: #1e293b;
    border: 1px solid #334155;
    border-radius: 12px 12px 2px 12px;
    padding: 12px 16px;
    color: #f8fafc;
  }

  .message-row.assistant {
    align-self: flex-start;
  }
  .message-row.assistant .message-body {
    background-color: #161618;
    border: 1px solid #27272a;
    border-radius: 12px 12px 12px 2px;
    padding: 14px 18px;
    color: #e4e4e7;
  }

  .avatar {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .message-row.user .avatar {
    background-color: #0284c7;
    color: white;
  }
  .message-row.assistant .avatar {
    background-color: #16a34a;
    color: white;
  }

  .message-body {
    max-width: 88%;
    word-break: break-word;
  }

  .markdown-content {
    line-height: 1.6;
    font-size: 0.95rem;
  }
  :global(.markdown-content p) {
    margin: 0 0 8px 0;
  }
  :global(.markdown-content p:last-child) {
    margin-bottom: 0;
  }
  :global(.markdown-content ul, .markdown-content ol) {
    padding-left: 20px;
    margin: 8px 0;
  }
  :global(.markdown-content pre) {
    background-color: #09090b;
    border: 1px solid #27272a;
    padding: 12px;
    border-radius: 6px;
    overflow-x: auto;
  }
</style>
