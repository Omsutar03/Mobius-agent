<script lang="ts">
  export let onSubmit: (prompt: string, model: string, thinking: string) => void;
  export let promptTokens: number = 0;
  export let completionTokens: number = 0;
  export let isStreaming: boolean = false;

  let prompt = "";
  let selectedModel = "llama";
  let selectedThinking = "off";

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  function send() {
    if (!prompt.trim() || isStreaming) return;
    onSubmit(prompt, selectedModel, selectedThinking);
    prompt = "";
  }
</script>

<div class="control-bar">
  <div class="settings-strip">
    <div class="setting-group">
      <label for="model-select">Model:</label>
      <select id="model-select" bind:value={selectedModel}>
        <option value="llama">llama.cpp (8080)</option>
        <option value="ollama">Ollama (11434)</option>
        <option value="lmstudio">LM Studio (1234)</option>
      </select>
    </div>

    <div class="setting-group">
      <label for="thinking-select">Thinking:</label>
      <select id="thinking-select" bind:value={selectedThinking}>
        <option value="off">Off</option>
        <option value="min">Minimal</option>
        <option value="low">Low</option>
        <option value="med">Medium</option>
        <option value="high">High</option>
        <option value="max">Max</option>
      </select>
    </div>

    <div class="token-usage">
      Tokens: <span class="tok-in">{promptTokens} in</span> / <span class="tok-out">{completionTokens} out</span>
    </div>
  </div>

  <div class="input-container">
    <textarea
      bind:value={prompt}
      on:keydown={handleKeydown}
      placeholder="Ask Mobius anything... (Shift+Enter for newline)"
      rows="2"
      disabled={isStreaming}
    ></textarea>
    <button type="button" on:click={send} disabled={isStreaming || !prompt.trim()}>
      {isStreaming ? "Generating..." : "Send"}
    </button>
  </div>
</div>

<style>
  .control-bar {
    padding: 12px 16px;
    background-color: #161618;
    border-top: 1px solid #222;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .settings-strip {
    display: flex;
    gap: 16px;
    align-items: center;
    font-size: 0.8rem;
    color: #aaa;
  }
  .setting-group {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  select {
    background: #222;
    border: 1px solid #333;
    color: #eee;
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 0.8rem;
  }
  .token-usage {
    margin-left: auto;
    font-family: monospace;
  }
  .tok-in { color: #38bdf8; }
  .tok-out { color: #4ade80; }
  .input-container {
    display: flex;
    gap: 10px;
  }
  textarea {
    flex: 1;
    background: #09090b;
    border: 1px solid #27272a;
    border-radius: 6px;
    color: #f4f4f5;
    padding: 10px;
    resize: none;
    font-family: inherit;
    font-size: 0.9rem;
  }
  textarea:focus {
    outline: 1px solid #3b82f6;
  }
  button {
    padding: 0 20px;
    background-color: #2563eb;
    border: none;
    border-radius: 6px;
    color: white;
    font-weight: 600;
    cursor: pointer;
  }
  button:disabled {
    background-color: #27272a;
    color: #71717a;
    cursor: not-allowed;
  }
</style>
