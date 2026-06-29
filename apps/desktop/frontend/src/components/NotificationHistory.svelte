<script lang="ts">
  import { notificationHistory, clearNotificationHistory, type ToastType } from '../lib/toast';
  import { Bell, Trash2, CheckCircle, XCircle, AlertTriangle, Info, Clock } from 'lucide-svelte';

  export let visible = false;

  function icon(type: ToastType) {
    if (type === 'success') return CheckCircle;
    if (type === 'error') return XCircle;
    if (type === 'warning') return AlertTriangle;
    return Info;
  }

  function fmtTime(ts: number): string {
    const d = new Date(ts);
    const now = new Date();
    const sameDay = d.toDateString() === now.toDateString();
    const time = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    if (sameDay) return time;
    return `${d.toLocaleDateString([], { month: 'short', day: 'numeric' })} ${time}`;
  }
</script>

{#if visible}
  <aside class="notif-panel">
    <div class="notif-header">
      <Bell size="12" />
      <span>Notifications</span>
      {#if $notificationHistory.length > 0}
        <button class="clear-btn" on:click={clearNotificationHistory} title="Clear all">
          <Trash2 size="10" />
        </button>
      {/if}
    </div>
    <div class="notif-list">
      {#if $notificationHistory.length === 0}
        <div class="notif-empty">No notifications</div>
      {:else}
        {#each $notificationHistory as n (n.id)}
          <div class="notif-item" class:notif-error={n.type === 'error'} class:notif-warning={n.type === 'warning'} class:notif-success={n.type === 'success'}>
            <svelte:component this={icon(n.type)} size="12" />
            <div class="notif-body">
              <span class="notif-msg">{n.message}</span>
              <span class="notif-time">{fmtTime(n.timestamp)}</span>
            </div>
          </div>
        {/each}
      {/if}
    </div>
  </aside>
{/if}

<style>
  .notif-panel {
    position: fixed;
    bottom: 32px;
    right: 12px;
    width: 340px;
    max-height: 320px;
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    z-index: 1500;
    display: flex;
    flex-direction: column;
    font-size: 11px;
    overflow: hidden;
  }
  .notif-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    color: var(--text);
    font-weight: 600;
  }
  .clear-btn {
    margin-left: auto;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px;
    display: flex;
    border-radius: 2px;
  }
  .clear-btn:hover { color: var(--text); background: var(--bg-hover); }
  .notif-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  .notif-empty {
    padding: 20px;
    text-align: center;
    color: var(--text-muted);
  }
  .notif-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 6px 10px;
    color: var(--text);
  }
  .notif-item:hover { background: var(--bg-hover); }
  .notif-item.notif-error { border-left: 2px solid var(--error); }
  .notif-item.notif-warning { border-left: 2px solid var(--warning); }
  .notif-item.notif-success { border-left: 2px solid var(--success); }
  .notif-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .notif-msg {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .notif-time {
    font-size: 10px;
    color: var(--text-muted);
  }
</style>
