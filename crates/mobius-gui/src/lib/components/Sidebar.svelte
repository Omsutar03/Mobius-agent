<script lang="ts">
  import type { SessionMetadata } from "../types";
  import Icon from "./Icon.svelte";
  import MobiusLogo from "./MobiusLogo.svelte";

  export let activePid: number;
  export let onNewChat: () => void;
  export let isConnected: boolean;
  export let llmConnected: boolean = false;
  export let llmModelName: string | null = null;
  export let sessions: SessionMetadata[] = [];
  export let onSelectSession: (path: string) => void;
  export let onDeleteSession: (path: string) => void;

  let collapsed = true;
  let deletingSession: SessionMetadata | null = null;

  $: isReady = isConnected && llmConnected;
  $: statusLabel = isReady ? "Connected" : "Disconnected";

  function focusDialog(node: HTMLElement) {
    node.focus();
  }
</script>

<svelte:window
  on:keydown={(e) => {
    if (e.key === "Escape") deletingSession = null;
  }}
/>

<aside class="sidebar" class:collapsed>
  <div class="sidebar-header">
    <div class="brand-mark" title="Mobius">
      <MobiusLogo size={20} strokeWidth={34} />
    </div>
    <div class="header-text">
      <h2>Mobius</h2>
      <div class="status-indicator">
        <span class="dot" class:connected={isReady}></span>
        <span class="status-label">{statusLabel}</span>
      </div>
    </div>
  </div>

  {#if isReady && llmModelName && collapsed}
    <div class="collapsed-model" title={llmModelName}>
      <span class="model-dot"></span>
    </div>
  {/if}

  {#if !collapsed}
    <button class="new-chat-btn" on:click={onNewChat}>
      <Icon name="plus" size={16} />
      <span>New Chat</span>
    </button>

    <div class="sessions-section">
      <div class="sessions-header">
        <span class="label">
          <Icon name="message" size={12} />
          Past Sessions
        </span>
        <span class="count">{sessions.length}</span>
      </div>

      {#if sessions.length === 0}
        <div class="sessions-empty">
          <Icon name="history" size={14} />
          <span>No past sessions yet.</span>
        </div>
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
                  e.preventDefault();
                  onSelectSession(s.path);
                }
              }}
            >
              <span class="session-marker" class:active={s.is_active}></span>
              <span class="session-preview">{s.preview}</span>
              {#if s.is_active}
                <span class="live-chip">live</span>
              {:else}
                <button
                  class="delete-btn"
                  title="Delete session"
                  aria-label="Delete session"
                  on:click|stopPropagation={() => (deletingSession = s)}
                >
                  <Icon name="trash" size={14} />
                </button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <div class="session-info">
      <span class="info-label">Agent PID</span>
      <code class="pid-badge">{activePid}</code>
    </div>
  {/if}

  {#if !collapsed && llmConnected && llmModelName}
    <div class="model-strip">
      <span class="model-dot"></span>
      <span class="model-name" title={llmModelName}>{llmModelName}</span>
    </div>
  {/if}

  <button
    class="collapse-btn"
    title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
    aria-expanded={!collapsed}
    aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
    on:click={() => (collapsed = !collapsed)}
  >
    {#if collapsed}
      <Icon name="chevron-right" size={16} />
    {:else}
      <Icon name="chevron-left" size={16} />
    {/if}
  </button>
</aside>

{#if deletingSession}
  {@const target = deletingSession}
  <div
    class="modal-backdrop"
    role="presentation"
    tabindex="-1"
    on:click={() => (deletingSession = null)}
    on:keydown={(e) => {
      if (e.key === "Escape") deletingSession = null;
    }}
  >
    <div
      class="modal"
      role="alertdialog"
      aria-modal="true"
      tabindex="-1"
      aria-labelledby="delete-modal-title"
      aria-describedby="delete-modal-desc"
      use:focusDialog
      on:click|stopPropagation
      on:keydown={(e) => {
        if (e.key === "Escape") deletingSession = null;
      }}
    >
      <div class="modal-icon">
        <Icon name="trash" size={18} />
      </div>
      <h3 class="modal-title" id="delete-modal-title">Delete this session?</h3>
      <p class="modal-text" id="delete-modal-desc">
        This permanently removes the conversation and cannot be undone.
      </p>
      <div class="modal-preview">
        <span class="preview-icon">
          <Icon name="message" size={13} />
        </span>
        <span>{target.preview}</span>
      </div>
      <div class="modal-actions">
        <button class="modal-cancel" on:click={() => (deletingSession = null)}>Cancel</button>
        <button
          class="modal-confirm"
          on:click={() => {
            onDeleteSession(target.path);
            deletingSession = null;
          }}
        >
          Delete
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .sidebar {
    position: relative;
    width: 256px;
    background: var(--bg-surface);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 16px 14px;
    gap: 14px;
    transition: width var(--dur-med) var(--ease);
    flex-shrink: 0;
    z-index: 1;
  }
  .sidebar.collapsed {
    width: 56px;
    padding: 16px 10px;
    align-items: center;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
  }
  .sidebar.collapsed .sidebar-header {
    justify-content: center;
  }
  .sidebar.collapsed .header-text {
    display: none;
  }
  .brand-mark {
    width: 32px;
    height: 32px;
    border-radius: 9px;
    background: var(--brand-gradient);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow: var(--brand-glow);
  }
  .header-text {
    min-width: 0;
  }
  .sidebar-header h2 {
    margin: 0 0 3px 0;
    font-size: 1.02rem;
    font-weight: 650;
    letter-spacing: -0.01em;
    color: var(--text-primary);
  }
  .status-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.72rem;
    color: var(--text-muted);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--error);
    box-shadow: 0 0 0 3px var(--error-soft);
  }
  .dot.connected {
    background: var(--success);
    box-shadow: 0 0 0 3px var(--success-soft);
  }

  .collapse-btn {
    position: absolute;
    right: -14px;
    top: 50%;
    transform: translateY(-50%);
    width: 22px;
    height: 56px;
    border: 1px solid var(--border-strong);
    border-left: none;
    border-radius: 0 var(--radius-md) var(--radius-md) 0;
    background: var(--bg-elevated);
    color: var(--text-muted);
    cursor: pointer;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    z-index: 5;
    box-shadow: 3px 0 12px rgba(0, 0, 0, 0.35);
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .collapse-btn:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .new-chat-btn {
    width: 100%;
    padding: 11px 12px;
    background: var(--brand-gradient);
    border: none;
    border-radius: var(--radius-md);
    color: #fff;
    font-weight: 600;
    font-size: 0.88rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    transition: filter var(--dur-fast) var(--ease), box-shadow var(--dur-fast) var(--ease);
    box-shadow: 0 2px 12px rgba(124, 58, 237, 0.35);
  }
  .new-chat-btn:hover {
    filter: brightness(1.1);
    box-shadow: 0 4px 20px rgba(124, 58, 237, 0.45);
  }

  .collapsed-model {
    display: flex;
    justify-content: center;
  }

  .sessions-section {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 100%;
  }
  .sessions-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.68rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .sessions-header .label {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .sessions-header .count {
    background: var(--bg-active);
    padding: 1px 7px;
    border-radius: 999px;
    color: var(--text-muted);
    font-size: 0.68rem;
  }
  .sessions-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .sessions-empty {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 0.82rem;
    color: var(--text-faint);
    padding: 8px 4px;
  }
  .session-item {
    width: 100%;
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 10px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    cursor: pointer;
    text-align: left;
    transition: background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
  }
  .session-item:hover {
    background: var(--bg-elevated);
    border-color: var(--border);
  }
  .session-item.active {
    background: var(--accent-soft);
    border-color: var(--accent-border);
    color: var(--text-primary);
  }
  .session-marker {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-faint);
    flex-shrink: 0;
  }
  .session-marker.active {
    background: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .session-preview {
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }
  .live-chip {
    flex-shrink: 0;
    font-size: 0.6rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--success);
    background: var(--success-soft);
    border: 1px solid rgba(52, 211, 153, 0.25);
    border-radius: 999px;
    padding: 1px 6px;
  }
  .delete-btn {
    background: transparent;
    border: none;
    color: var(--text-faint);
    cursor: pointer;
    padding: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
    opacity: 0;
    transition: opacity var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease),
      background var(--dur-fast) var(--ease);
  }
  .session-item:hover .delete-btn,
  .delete-btn:focus-visible {
    opacity: 1;
  }
  .delete-btn:hover {
    color: var(--error);
    background: var(--error-soft);
  }

  .session-info {
    margin-top: auto;
    padding-top: 12px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.74rem;
    color: var(--text-muted);
  }
  .info-label {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .pid-badge {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    color: var(--info);
    font-family: var(--mono);
    font-size: 0.74rem;
  }

  .model-strip {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 0.74rem;
    color: var(--text-secondary);
  }
  .model-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
    flex-shrink: 0;
  }
  .collapsed-model .model-dot {
    width: 10px;
    height: 10px;
  }
  .model-name {
    font-family: var(--mono);
    font-size: 0.72rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  /* ── Delete confirmation ────────────────────────────────────────────────── */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(3px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal {
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    padding: 24px;
    width: 340px;
    max-width: 90vw;
    color: var(--text-primary);
    box-shadow: var(--shadow-modal);
    text-align: left;
  }
  .modal-icon {
    width: 40px;
    height: 40px;
    border-radius: 12px;
    background: var(--error-soft);
    color: var(--error);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 14px;
  }
  .modal-title {
    margin: 0 0 6px 0;
    font-size: 1.05rem;
    font-weight: 650;
    letter-spacing: -0.01em;
    color: var(--text-primary);
  }
  .modal-text {
    margin: 0 0 14px 0;
    font-size: 0.85rem;
    line-height: 1.55;
    color: var(--text-muted);
  }
  .modal-preview {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 20px 0;
    font-size: 0.84rem;
    color: var(--text-secondary);
    background: var(--bg-inset);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 9px 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .preview-icon {
    display: flex;
    align-items: center;
    color: var(--accent);
    flex-shrink: 0;
  }
  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .modal-cancel,
  .modal-confirm {
    padding: 9px 16px;
    border-radius: var(--radius-md);
    border: none;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease), filter var(--dur-fast) var(--ease);
  }
  .modal-cancel {
    background: var(--bg-hover);
    color: var(--text-secondary);
    border: 1px solid var(--border-strong);
  }
  .modal-cancel:hover {
    background: var(--bg-active);
    color: var(--text-primary);
  }
  .modal-confirm {
    background: var(--error-strong);
    color: #fff;
  }
  .modal-confirm:hover {
    filter: brightness(1.12);
  }
</style>