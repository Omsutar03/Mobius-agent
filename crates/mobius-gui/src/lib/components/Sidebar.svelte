<script lang="ts">
  import type { SessionMetadata } from "../types";

  export let activePid: number;
  export let onNewChat: () => void;
  export let isConnected: boolean;
  export let sessions: SessionMetadata[] = [];
  export let onSelectSession: (path: string) => void;
  export let onDeleteSession: (path: string) => void;

  let collapsed = false;
  let deletingSession: SessionMetadata | null = null;
</script>

<svelte:window on:keydown={(e) => { if (e.key === "Escape") deletingSession = null; }} />

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
            <div
              class="session-item"
              class:active={s.is_active}
              role="button"
              tabindex="0"
              title={s.path}
              on:click={() => onSelectSession(s.path)}
              on:keydown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  onSelectSession(s.path);
                }
              }}
            >
              <span class="session-marker" class:active={s.is_active}></span>
              <span class="session-preview">{s.preview}</span>
              {#if !s.is_active}
                <button
                  class="delete-btn"
                  title="Delete session"
                  on:click|stopPropagation={() => (deletingSession = s)}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg>
                </button>
              {/if}
            </div>
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

{#if deletingSession}
  {@const target = deletingSession}
  <div class="modal-backdrop" role="presentation">
    <div class="modal">
      <h3 class="modal-title">Delete session?</h3>
      <p class="modal-text">Are you sure you want to delete this session?</p>
      <p class="modal-preview">{target.preview}</p>
      <div class="modal-actions">
        <button class="modal-cancel" on:click={() => (deletingSession = null)}>Cancel</button>
        <button
          class="modal-confirm"
          on:click={() => {
            onDeleteSession(target.path);
            deletingSession = null;
          }}
        >Delete</button>
      </div>
    </div>
  </div>
{/if}

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
    flex: 1;
    min-width: 0;
  }
  .delete-btn {
    background: transparent;
    border: none;
    color: #888;
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 0.1s, color 0.1s;
  }
  .session-item:hover .delete-btn {
    opacity: 1;
  }
  .session-item:hover .delete-btn:hover {
    color: #ef4444;
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
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal {
    background: #1c1c1f;
    border: 1px solid #333;
    border-radius: 10px;
    padding: 20px;
    width: 320px;
    max-width: 90vw;
    color: #eee;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.6);
    text-align: left;
  }
  .modal-title {
    margin: 0 0 8px 0;
    font-size: 1rem;
    color: #fff;
  }
  .modal-text {
    margin: 0 0 6px 0;
    font-size: 0.85rem;
    color: #aaa;
  }
  .modal-preview {
    margin: 0 0 16px 0;
    font-size: 0.85rem;
    color: #ddd;
    background: #111113;
    border: 1px solid #2a2a2e;
    border-radius: 6px;
    padding: 8px 10px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .modal-cancel,
  .modal-confirm {
    padding: 8px 14px;
    border-radius: 6px;
    border: none;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .modal-cancel {
    background: #2a2a2e;
    color: #ccc;
  }
  .modal-cancel:hover {
    background: #3a3a3e;
  }
  .modal-confirm {
    background: #dc2626;
    color: #fff;
    font-weight: 600;
  }
  .modal-confirm:hover {
    background: #ef4444;
  }
</style>