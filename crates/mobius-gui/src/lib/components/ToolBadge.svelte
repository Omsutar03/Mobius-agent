<script lang="ts">
  import Icon from "./Icon.svelte";

  export let toolName: string;
  export let details: string;
  export let output: string | undefined = undefined;

  let showDetails = false;

  function formatToolName(name: string) {
    return name.replace(/_/g, " ").toLowerCase();
  }
</script>

<div class="tool-badge-container">
  <button
    class="badge-pill"
    class:active={showDetails}
    aria-expanded={showDetails}
    on:click={() => (showDetails = !showDetails)}
  >
    <span class="status-icon">
      <Icon name="zap" size={13} />
    </span>
    <span class="tool-name">{formatToolName(toolName)}</span>
    <span class="tool-details">{details}</span>
    {#if output}
      <span class="toggle-arrow">
        <Icon name={showDetails ? "chevron-up" : "chevron-down"} size={13} />
      </span>
    {/if}
  </button>

  {#if showDetails && output}
    <div class="tool-output-wrapper">
      <div class="output-header">Execution Output</div>
      <pre class="tool-output">{output}</pre>
    </div>
  {/if}
</div>

<style>
  .tool-badge-container {
    margin: 4px 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .badge-pill {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 5px 9px;
    border-radius: 999px;
    font-family: var(--mono);
    font-size: 0.75rem;
    cursor: pointer;
    max-width: 100%;
    transition: border-color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }
  .badge-pill:hover,
  .badge-pill.active {
    background: #fef08a11;
    border-color: rgba(251, 191, 36, 0.35);
    color: var(--text-secondary);
  }
  .status-icon {
    display: flex;
    align-items: center;
    color: var(--warning);
    flex-shrink: 0;
  }
  .tool-name {
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: capitalize;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .tool-details {
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 380px;
  }
  .toggle-arrow {
    display: flex;
    align-items: center;
    color: var(--text-faint);
    margin-left: 2px;
  }
  .tool-output-wrapper {
    margin-top: 6px;
    width: 100%;
    max-width: 650px;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  .output-header {
    background: var(--bg-elevated);
    padding: 5px 10px;
    font-size: 0.66rem;
    font-weight: 600;
    font-family: var(--mono);
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .tool-output {
    margin: 0;
    padding: 10px 12px;
    color: var(--info);
    font-family: var(--mono);
    font-size: 0.76rem;
    max-height: 220px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-all;
  }
</style>