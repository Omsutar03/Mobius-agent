<script lang="ts">
  import Icon from "./Icon.svelte";

  export let content: string = "";
  let isOpen: boolean = true;

  function toggle() {
    isOpen = !isOpen;
  }
</script>

{#if content}
  <div class="thinking-block">
    <button
      class="header"
      on:click={toggle}
      aria-expanded={isOpen}
      aria-controls="thinking-body"
    >
      <span class="header-icon">
        <Icon name="cpu" size={14} />
      </span>
      <span class="title">Thinking</span>
      <span class="length">{content.trim().length} chars</span>
      <span class="chevron">
        <Icon name={isOpen ? "chevron-up" : "chevron-down"} size={14} />
      </span>
    </button>

    {#if isOpen}
      <div id="thinking-body" class="content">
        <pre>{content}</pre>
      </div>
    {/if}
  </div>
{/if}

<style>
  .thinking-block {
    margin: 8px 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-inset);
    overflow: hidden;
  }
  .header {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-elevated);
    border: none;
    border-bottom: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.8rem;
    font-family: var(--sans);
    text-align: left;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .header:hover {
    background: var(--bg-hover);
    color: var(--text-secondary);
  }
  .header-icon {
    display: flex;
    align-items: center;
    color: var(--chain);
  }
  .title {
    font-weight: 600;
    flex: 1;
  }
  .length {
    font-size: 0.7rem;
    font-family: var(--mono);
    color: var(--text-faint);
  }
  .chevron {
    display: flex;
    align-items: center;
    color: var(--text-faint);
    transition: transform var(--dur-med) var(--ease);
  }
  .content {
    padding: 0;
  }
  .content pre {
    padding: 12px 14px;
    margin: 0;
    color: var(--text-muted);
    font-family: var(--mono);
    font-size: 0.82rem;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 320px;
    overflow-y: auto;
    background: transparent;
  }
</style>