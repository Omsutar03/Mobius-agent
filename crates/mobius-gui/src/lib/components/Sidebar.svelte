<script lang="ts">
  import type { SessionMetadata } from "../types";

  export let activePid: number;
  export let onNewChat: () => void;
  export let isConnected: boolean;
  export let sessions: SessionMetadata[] = [];
  export let onSelectSession: (path: string) => void;

  let collapsed = false;
</script>

<aside class="sidebar" class:collapsed>
  <div class="sidebar-header">
    <div class="header-text">
      <h2>Mobius Agent</h2>
      <div class="status-indicator">
        <span class="dot" class:connected={isConnected}></span>
        <span class="status-text">{isConnected ? "Connected" : "Disconnected"}</span>
      </div>
    </div>
    <button
      class="collapse-btn"
      title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
      on:click={() => (collapsed = !collapsed)}
    >
      {#if collapsed}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 18l6-6-6-6"/></svg>
      {:else}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M15 18l-6-6 6-6"/></svg>
      {/if}
    </button>
  </div>

  {#if !collapsed}
    <button class="new-chat-btn" on:click={onNewChat}>
      <span>+</span> New Chat
    </button>

    <div class="sessions-section">
      <div class="sessions-header">
        <span class="label">Past Sessions</span>
        <span class="count">{sessions.length}</span>
      </div>

      {#if sessions.length === 0}
        <div class="sessions-empty">No past sessions yet.</div>
      {:else}
        <div class="sessions-list">
          {#each sessions as s}
            <button
              class="session-item"
              class:active={s.is_active}
              title={s.path}
              on:click={() => onSelectSession(s.path)}
            >
              <span class="session-marker" class:active={s.is_active}></span>
              <span class="session-preview">{s.preview}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <div class="session-info">
      <span class="label">Current Active PID:</span>
      <span class="pid-badge">{activePid}</span>
    </div>
  {/if}
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
    transition: width 0.2s ease;
    overflow: hidden;
    flex-shrink: 0;
  }
  .sidebar.collapsed {
    width: 48px;
    padding: 12px 8px;
  }
  .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 8px;
  }
  .sidebar.collapsed .header-text {
    display: none;
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
  .collapse-btn {
    background: transparent;
    border: 1px solid #2a2a2e;
    border-radius: 6px;
    color: #888;
    cursor: pointer;
    padding: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .collapse-btn:hover {
    color: #fff;
    border-color: #3a3a3e;
    background: #1c1c1f;
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
  .sessions-section {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .sessions-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .sessions-header .count {
    background: #222;
    padding: 1px 6px;
    border-radius: 8px;
    color: #999;
    font-size: 0.7rem;
  }
  .sessions-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .sessions-list::-webkit-scrollbar {
    width: 4px;
  }
  .sessions-list::-webkit-scrollbar-thumb {
    background: #333;
    border-radius: 2px;
  }
  .sessions-empty {
    font-size: 0.8rem;
    color: #555;
    padding: 8px 2px;
  }
  .session-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: #ccc;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
  }
  .session-item:hover {
    background: #1c1c1f;
    border-color: #2a2a2e;
  }
  .session-item.active {
    background: #1a2733;
    border-color: #0e7490;
  }
  .session-marker {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #3a3a3e;
    flex-shrink: 0;
  }
  .session-marker.active {
    background: #22d3ee;
  }
  .session-preview {
    font-size: 0.8rem;
    color: #ddd;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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