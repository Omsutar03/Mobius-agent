<script lang="ts">
  export let activePid: number;
  export let onNewChat: () => void;
  export let isConnected: boolean;
</script>

<aside class="sidebar">
  <div class="sidebar-header">
    <h2>Mobius Agent</h2>
    <div class="status-indicator">
      <span class="dot" class:connected={isConnected}></span>
      <span class="status-text">{isConnected ? "Connected" : "Disconnected"}</span>
    </div>
  </div>

  <button class="new-chat-btn" on:click={onNewChat}>
    <span>+</span> New Chat
  </button>

  <div class="session-info">
    <span class="label">Current Active PID:</span>
    <span class="pid-badge">{activePid}</span>
  </div>
</aside>

<style>
  .sidebar {
    width: 240px;
    background-color: #111113;
    border-right: 1px solid #222;
    display: flex;
    flex-direction: column;
    padding: 16px;
    gap: 16px;
  }
  .sidebar-header h2 {
    margin: 0 0 6px 0;
    font-size: 1.1rem;
    color: #fff;
  }
  .status-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
    color: #888;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: #e53935;
  }
  .dot.connected {
    background-color: #4caf50;
  }
  .new-chat-btn {
    width: 100%;
    padding: 10px;
    background-color: #238636;
    border: none;
    border-radius: 6px;
    color: #fff;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }
  .new-chat-btn:hover {
    background-color: #2ea043;
  }
  .session-info {
    margin-top: auto;
    padding-top: 12px;
    border-top: 1px solid #222;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.8rem;
    color: #888;
  }
  .pid-badge {
    background: #222;
    padding: 2px 6px;
    border-radius: 4px;
    color: #38bdf8;
    font-family: monospace;
  }
</style>
