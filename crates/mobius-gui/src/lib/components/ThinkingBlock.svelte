<script lang="ts">
  export let content: string = "";
  let isOpen: boolean = true;

  function toggle() {
    isOpen = !isOpen;
  }
</script>

{#if content}
  <div class="thinking-block">
    <button class="header" on:click={toggle}>
      <span class="icon">{isOpen ? "▼" : "▶"}</span>
      <span class="title">Thinking Process</span>
      <span class="length">({content.length} chars)</span>
    </button>

    {#if isOpen}
      <pre class="content">{content}</pre>
    {/if}
  </div>
{/if}

<style>
  .thinking-block {
    margin: 8px 0;
    border: 1px solid #333;
    border-radius: 6px;
    background-color: #181818;
    overflow: hidden;
  }
  .header {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: #222;
    border: none;
    color: #888;
    cursor: pointer;
    font-size: 0.85rem;
    text-align: left;
  }
  .header:hover {
    background: #2a2a2a;
    color: #ccc;
  }
  .title {
    font-weight: 600;
  }
  .length {
    font-size: 0.75rem;
    opacity: 0.6;
  }
  .content {
    padding: 12px;
    margin: 0;
    color: #aaa;
    font-family: monospace;
    font-size: 0.85rem;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 300px;
    overflow-y: auto;
    border-top: 1px solid #282828;
  }
</style>
