<script lang="ts">
  export let onSubmit: (prompt: string, model: string, thinking: string) => void;
  export let promptTokens: number = 0;
  export let completionTokens: number = 0;
  export let isStreaming: boolean = false;

  let prompt = "";
  let selectedModel = "llama";
  let selectedThinking = "med";
  let textareaEl: HTMLTextAreaElement;

  function handleInput() {
    if (!textareaEl) return;
    textareaEl.style.height = "auto";
    textareaEl.style.height = Math.min(textareaEl.scrollHeight, 180) + "px";
  }

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
    if (textareaEl) textareaEl.style.height = "auto";
  }

  function formatK(num: number) {
    return num >= 1000 ? (num / 1000).toFixed(1) + "k" : num.toString();
  }
</script>

<div class="control-bar">
  <div class="settings-strip">
    <div class="setting-pill">
      <span class="label">Model</span>
      <select bind:value={selectedModel}>
        <option value="llama">llama.cpp (8080)</option>
        <option value="ollama">Ollama (11434)</option>
        <option value="lmstudio">LM Studio (1234)</option>
      </select>
    </div>

    <div class="setting-pill">
      <span class="label">Thinking</span>
      <select bind:value={selectedThinking}>
        <option value="off">Off</option>
        <option value="min">Minimal</option>
        <option value="low">Low</option>
        <option value="med">Medium</option>
        <option value="high">High</option>
        <option value="max">Max</option>
      </select>
    </div>

    <div class="token-badges">
      <span class="tok-pill in">▲ {formatK(promptTokens)}</span>
      <span class="tok-pill out">▼ {formatK(completionTokens)}</span>
    </div>
  </div>

  <div class="prompt-card">
    <textarea
      bind:this={textareaEl}
      bind:value={prompt}
      on:input={handleInput}
      on:keydown={handleKeydown}
      placeholder="Ask Mobius anything... (Shift+Enter for new line)"
      rows="1"
      disabled={isStreaming}
    ></textarea>

    <button
      type="button"
      class="send-btn"
      on:click={send}
      disabled={isStreaming || !prompt.trim()}
      aria-label="Send message"
    >
      {#if isStreaming}
        <span class="spinner"></span>
      {:else}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <line x1="12" y1="19" x2="12" y2="5"></line>
          <polyline points="5 12 12 5 19 12"></polyline>
        </svg>
      {/if}
    </button>
  </div>
</div>

<style>
  .control-bar {
    padding: 12px 20px 18px;
    background-color: #0d0d0e;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 860px;
    margin: 0 auto;
    width: 100%;
  }
  .settings-strip {
    display: flex;
    gap: 10px;
    align-items: center;
    font-size: 0.78rem;
  }
  .setting-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #18181b;
    border: 1px solid #27272a;
    padding: 3px 8px;
    border-radius: 6px;
    color: #71717a;
  }
  .setting-pill .label {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  select {
    background: transparent;
    border: none;
    color: #f4f4f5;
    font-size: 0.78rem;
    outline: none;
    cursor: pointer;
  }
  select option {
    background: #18181b;
    color: #f4f4f5;
  }
  .token-badges {
    margin-left: auto;
    display: flex;
    gap: 6px;
  }
  .tok-pill {
    font-family: ui-monospace, Consolas, monospace;
    font-size: 0.72rem;
    padding: 2px 8px;
    border-radius: 4px;
    background: #18181b;
    border: 1px solid #27272a;
  }
  .tok-pill.in { color: #38bdf8; }
  .tok-pill.out { color: #4ade80; }

  /* Unified Prompt Box */
  .prompt-card {
    position: relative;
    display: flex;
    align-items: flex-end;
    background: #18181b;
    border: 1px solid #27272a;
    border-radius: 10px;
    padding: 8px 12px;
    transition: border-color 0.2s ease, box-shadow 0.2s ease;
  }
  .prompt-card:focus-within {
    border-color: #3b82f6;
    box-shadow: 0 0 0 1px rgba(59, 130, 246, 0.3);
  }
  textarea {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #f4f4f5;
    resize: none;
    font-family: inherit;
    font-size: 0.92rem;
    line-height: 1.45;
    max-height: 180px;
    padding: 4px 36px 4px 0;
  }
  textarea::placeholder {
    color: #52525b;
  }
  .send-btn {
    position: absolute;
    right: 8px;
    bottom: 8px;
    width: 32px;
    height: 32px;
    background-color: #3b82f6;
    border: none;
    border-radius: 6px;
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 0.15s ease, opacity 0.15s ease;
  }
  .send-btn:hover:not(:disabled) {
    background-color: #2563eb;
  }
  .send-btn:disabled {
    background-color: #27272a;
    color: #52525b;
    cursor: not-allowed;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.2);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
