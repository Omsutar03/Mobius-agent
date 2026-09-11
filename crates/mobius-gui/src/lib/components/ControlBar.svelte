<script lang="ts">
  import type { ModelListEntry } from "../types";
  import Icon from "./Icon.svelte";

  export let onSubmit: (prompt: string, thinking: string) => void;
  export let promptTokens: number = 0;
  export let contextWindow: number = 0;
  export let isStreaming: boolean = false;
  export let modelOptions: ModelListEntry[] = [];
  export let modelIndex: number = 0;
  export let onModelChange: (index: number) => void = () => {};

  let prompt = "";
  let selectedThinking = "med";
  let textareaEl: HTMLTextAreaElement;

  $: contextPct =
    contextWindow > 0 ? Math.min((promptTokens / contextWindow) * 100, 100) : 0;
  $: contextLevel = contextPct >= 90 ? "critical" : contextPct >= 70 ? "warning" : "ok";

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
    onSubmit(prompt, selectedThinking);
    prompt = "";
    if (textareaEl) textareaEl.style.height = "auto";
  }

  function handleModelSelect(e: Event) {
    const val = parseInt((e.currentTarget as HTMLSelectElement).value, 10);
    onModelChange(Number.isNaN(val) ? 0 : val);
  }

  function formatK(num: number) {
    return num >= 1000 ? (num / 1000).toFixed(1).replace(/\.0$/, "") + "K" : num.toString();
  }
</script>

<div class="control-bar">
  <div class="settings-strip">
    <label class="setting-pill">
      <Icon name="cpu" size={13} />
      <span class="pill-label">Model</span>
      <select
        value={modelIndex}
        on:change={handleModelSelect}
        disabled={modelOptions.length === 0}
      >
        {#if modelOptions.length === 0}
          <option value={0}>No models detected</option>
        {/if}
        {#each modelOptions as opt, i}
          <option value={i + 1}>{opt.display_name}</option>
        {/each}
      </select>
    </label>

    <label class="setting-pill">
      <Icon name="sparkles" size={13} />
      <span class="pill-label">Thinking</span>
      <select bind:value={selectedThinking}>
        <option value="off">Off</option>
        <option value="min">Minimal</option>
        <option value="low">Low</option>
        <option value="med">Medium</option>
        <option value="high">High</option>
        <option value="max">Max</option>
      </select>
    </label>

    <div
      class="token-pill"
      class:warning={contextLevel === "warning"}
      class:critical={contextLevel === "critical"}
      title="Context window usage"
    >
      <Icon name="activity" size={13} />
      <span>
        Context: {contextPct.toFixed(1)}%
        <span class="token-frac">({formatK(promptTokens)}/{formatK(contextWindow)})</span>
      </span>
    </div>
  </div>

  <div class="prompt-card">
    <div class="context-track" aria-hidden="true">
      <div class="context-fill {contextLevel}" style="width:{contextPct}%"></div>
    </div>

    <textarea
      bind:this={textareaEl}
      bind:value={prompt}
      on:input={handleInput}
      on:keydown={handleKeydown}
      placeholder="Ask Mobius anything..."
      aria-label="Message"
      rows="1"
      disabled={isStreaming}
    ></textarea>

    <button
      type="button"
      class="send-btn"
      class:streaming={isStreaming}
      on:click={send}
      disabled={isStreaming || !prompt.trim()}
      aria-label={isStreaming ? "Waiting for response" : "Send message"}
      title={isStreaming ? "Waiting for response" : "Send message"}
    >
      {#if isStreaming}
        <span class="spinner" aria-hidden="true"></span>
      {:else}
        <Icon name="send" size={17} />
      {/if}
    </button>
  </div>

  <div class="hint-row" aria-hidden="true">
    <span><kbd>Enter</kbd> to send</span>
    <span class="hint-sep">·</span>
    <span><kbd>Shift</kbd> + <kbd>Enter</kbd> new line</span>
  </div>
</div>

<style>
  .control-bar {
    padding: 14px 24px 16px;
    background: var(--bg-base);
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 860px;
    margin: 0 auto;
    width: 100%;
    border-top: 1px solid var(--border);
  }

  .settings-strip {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 0.78rem;
    flex-wrap: wrap;
  }

  .setting-pill {
    display: flex;
    align-items: center;
    gap: 7px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    padding: 6px 10px;
    border-radius: var(--radius-md);
    color: var(--text-muted);
    cursor: pointer;
    transition: border-color var(--dur-fast) var(--ease);
  }
  .setting-pill:hover {
    border-color: var(--border-strong);
  }
  .setting-pill:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .pill-label {
    font-size: 0.66rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-faint);
  }

  select {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 0.78rem;
    outline: none;
    cursor: pointer;
    font-family: inherit;
    max-width: 200px;
  }
  select:disabled {
    color: var(--text-faint);
    cursor: not-allowed;
  }
  select option {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }

  .token-pill {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 6px;
    font-family: var(--mono);
    font-size: 0.72rem;
    padding: 6px 10px;
    border-radius: var(--radius-md);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--info);
    white-space: nowrap;
  }
  .token-pill.warning {
    color: var(--warning);
  }
  .token-pill.critical {
    color: var(--error);
  }
  .token-frac {
    color: var(--text-muted);
  }

  /* ── Prompt card ────────────────────────────────────────────────────────── */
  .prompt-card {
    position: relative;
    display: flex;
    align-items: flex-end;
    background: var(--bg-surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    padding: 10px 12px 10px 16px;
    overflow: hidden;
    transition: border-color var(--dur-med) var(--ease), box-shadow var(--dur-med) var(--ease);
  }
  .prompt-card:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.18), 0 4px 24px rgba(124, 58, 237, 0.12);
  }

  .context-track {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--bg-active);
  }
  .context-fill {
    height: 100%;
    background: var(--info);
    transition: width var(--dur-slow) var(--ease);
  }
  .context-fill.warning {
    background: var(--warning);
  }
  .context-fill.critical {
    background: var(--error);
  }

  textarea {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    resize: none;
    font-family: inherit;
    font-size: 0.94rem;
    line-height: 1.5;
    max-height: 180px;
    padding: 6px 44px 6px 0;
  }
  textarea::placeholder {
    color: var(--text-faint);
  }
  textarea:disabled {
    color: var(--text-muted);
    cursor: wait;
  }

  .send-btn {
    position: absolute;
    right: 10px;
    bottom: 10px;
    width: 34px;
    height: 34px;
    background: var(--brand-gradient);
    border: none;
    border-radius: var(--radius-md);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: filter var(--dur-fast) var(--ease), box-shadow var(--dur-fast) var(--ease),
      transform var(--dur-fast) var(--ease);
    box-shadow: 0 2px 10px rgba(124, 58, 237, 0.35);
  }
  .send-btn:hover:not(:disabled) {
    filter: brightness(1.1);
    transform: translateY(-1px);
    box-shadow: 0 4px 16px rgba(124, 58, 237, 0.45);
  }
  .send-btn:active:not(:disabled) {
    transform: translateY(0);
  }
  .send-btn:disabled {
    background: var(--bg-active);
    color: var(--text-faint);
    box-shadow: none;
    cursor: not-allowed;
  }
  .send-btn.streaming {
    background: var(--bg-active);
    color: var(--text-secondary);
  }

  .spinner {
    width: 15px;
    height: 15px;
    border: 2px solid rgba(255, 255, 255, 0.2);
    border-top-color: var(--text-secondary);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .hint-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.68rem;
    color: var(--text-faint);
    padding: 0 4px;
  }
  .hint-row kbd {
    font-family: var(--mono);
    font-size: 0.64rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: 4px;
    padding: 1px 4px;
    color: var(--text-muted);
  }
  .hint-sep {
    opacity: 0.5;
  }
</style>