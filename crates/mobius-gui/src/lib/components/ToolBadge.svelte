<script lang="ts">
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
    on:click={() => (showDetails = !showDetails)}
  >
    <span class="status-icon">⚡</span>
    <span class="tool-name">{formatToolName(toolName)}</span>
    <span class="tool-details">{details}</span>
    {#if output}
      <span class="toggle-arrow">{showDetails ? "▲" : "▼"}</span>
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
    margin: 6px 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .badge-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: rgba(39, 39, 42, 0.6);
    border: 1px solid rgba(63, 63, 70, 0.7);
    color: #a1a1aa;
    padding: 4px 10px;
    border-radius: 6px;
    font-family: ui-monospace, Consolas, monospace;
    font-size: 0.78rem;
    cursor: pointer;
    max-width: 100%;
    transition: all 0.15s ease;
  }
  .badge-pill:hover, .badge-pill.active {
    background: rgba(45, 45, 52, 0.9);
    border-color: #52525b;
    color: #e4e4e7;
  }
  .status-icon {
    font-size: 0.75rem;
    color: #eab308;
  }
  .tool-name {
    font-weight: 600;
    color: #f4f4f5;
    text-transform: capitalize;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .tool-details {
    color: #71717a;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 380px;
  }
  .toggle-arrow {
    font-size: 0.65rem;
    color: #71717a;
    margin-left: 2px;
  }
  .tool-output-wrapper {
    margin-top: 6px;
    width: 100%;
    max-width: 650px;
    background: #09090b;
    border: 1px solid #27272a;
    border-radius: 6px;
    overflow: hidden;
  }
  .output-header {
    background: #18181b;
    padding: 4px 10px;
    font-size: 0.7rem;
    font-family: ui-monospace, Consolas, monospace;
    color: #71717a;
    border-bottom: 1px solid #27272a;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .tool-output {
    margin: 0;
    padding: 10px;
    color: #4ade80;
    font-family: ui-monospace, Consolas, monospace;
    font-size: 0.78rem;
    max-height: 220px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-all;
  }
</style>
