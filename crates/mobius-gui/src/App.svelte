<script lang="ts">
  import { onMount } from "svelte";
  import { marked } from "marked";
  import { daemonClient } from "./lib/ws";
  import type { Message, IpcRequest } from "./lib/types";
  import { stripToolBlocks } from "./lib/utils/format";

  import Sidebar from "./lib/components/Sidebar.svelte";
  import ControlBar from "./lib/components/ControlBar.svelte";
  import ThinkingBlock from "./lib/components/ThinkingBlock.svelte";
  import ToolBadge from "./lib/components/ToolBadge.svelte";

  let isConnected = false;
  let isStreaming = false;
  let activePid = Math.floor(Math.random() * 89999) + 10000; // Simulated PID for GUI instance

  let messages: Message[] = [];
  let promptTokens = 0;
  let completionTokens = 0;

  let currentThinking = "";
  let currentResponse = "";
  let currentTools: Array<{ tool_name: string; details: string; output?: string }> = [];

  onMount(() => {
    daemonClient.connect((connected) => {
      isConnected = connected;
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
    daemonClient.sendRequest(req, {});
    messages = [];
    promptTokens = 0;
    completionTokens = 0;
  }

  function handlePromptSubmit(prompt: string, model: string, thinking: string) {
    messages = [...messages, { role: "user", content: prompt }];

    isStreaming = true;
    currentThinking = "";
    currentResponse = "";
    currentTools = [];

    const request: IpcRequest = {
      ppid: activePid,
      prompt,
      model_override: model, // The updated server.rs will now process this alongside the prompt
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
      },
      onTextChunk: (chunk) => {
        currentResponse += chunk;
      },
      onToolStart: (tool) => {
        currentTools = [...currentTools, tool];
      },
      onToolFinished: (tool) => {
        currentTools = currentTools.map((t) =>
          t.tool_name === tool.tool_name ? { ...t, output: tool.output } : t
        );
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
      },
      onError: (err) => {
        messages = [...messages, { role: "assistant", content: `❌ Error: ${err}` }];
        isStreaming = false;
      }
    });
  }
</script>

<main class="app-layout">
  <Sidebar {activePid} {isConnected} onNewChat={handleNewChat} />

  <section class="chat-viewport">
    <div class="message-feed">
      {#each messages as msg}
        <div class="message-row {msg.role}">
          <div class="avatar">{msg.role === "user" ? "U" : "M"}</div>
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
          <div class="avatar">M</div>
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
  :global(body) {
    margin: 0;
    padding: 0;
    background-color: #0d0d0e;
    color: #e4e4e7;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }
  .app-layout {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }
  .chat-viewport {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .message-feed {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .message-row {
    display: flex;
    gap: 12px;
    max-width: 850px;
    margin: 0 auto;
    width: 100%;
  }
  .avatar {
    width: 32px;
    height: 32px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    font-size: 0.85rem;
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
    flex: 1;
    overflow-x: auto;
  }
  .markdown-content {
    line-height: 1.6;
    font-size: 0.95rem;
  }
  :global(.markdown-content p) {
    margin: 0 0 8px 0;
  }
  :global(.markdown-content pre) {
    background-color: #18181b;
    padding: 12px;
    border-radius: 6px;
    overflow-x: auto;
  }
</style>
