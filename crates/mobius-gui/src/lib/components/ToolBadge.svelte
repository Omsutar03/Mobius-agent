<script lang="ts">
  export let toolName: string;
  export let details: string;
  export let output: string | undefined = undefined;

  let showDetails = false;
</script>

<div class="tool-badge">
  <button class="badge-button" on:click={() => (showDetails = !showDetails)}>
    <span class="tool-icon">🛠️</span>
    <span class="tool-name">{toolName}</span>
    <span class="tool-details">{details}</span>
  </button>

  {#if showDetails && output}
    <pre class="tool-output">{output}</pre>
  {/if}
</div>

<style>
  .tool-badge {
    margin: 6px 0;
  }
  .badge-button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background-color: #272010;
    border: 1px solid #5c4510;
    color: #e5a93c;
    padding: 4px 10px;
    border-radius: 4px;
    font-family: monospace;
    font-size: 0.82rem;
    cursor: pointer;
  }
  .badge-button:hover {
    background-color: #362c16;
  }
  .tool-name {
    font-weight: bold;
    text-transform: uppercase;
  }
  .tool-details {
    color: #b08f46;
  }
  .tool-output {
    margin-top: 6px;
    padding: 10px;
    background: #121212;
    border: 1px solid #333;
    border-radius: 4px;
    color: #00ff66;
    font-family: monospace;
    font-size: 0.8rem;
    max-height: 200px;
    overflow-y: auto;
    white-space: pre-wrap;
  }
</style>
