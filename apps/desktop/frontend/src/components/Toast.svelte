<script lang="ts">
  import { toasts, notify, type ToastType } from '../lib/toast';
  import { CheckCircle, XCircle, AlertTriangle, Info, X } from 'lucide-svelte';

  function remove(id: number) {
    toasts.update((t) => t.filter((x) => x.id !== id));
  }

  function icon(type: ToastType) {
    if (type === 'success') return CheckCircle;
    if (type === 'error') return XCircle;
    if (type === 'warning') return AlertTriangle;
    return Info;
  }
</script>

{#if $toasts.length > 0}
  <div class="toast-container">
    {#each $toasts as t (t.id)}
      <div class="toast" class:success={t.type === 'success'} class:error={t.type === 'error'} class:warning={t.type === 'warning'}>
        <svelte:component this={icon(t.type)} size="14" />
        <span>{t.message}</span>
        <button class="toast-close" on:click={() => remove(t.id)}><X size="12" /></button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .toast-container {
    position: fixed;
    bottom: 40px;
    right: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 2000;
    pointer-events: none;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    font-size: 12px;
    color: #fff;
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    min-width: 260px;
    max-width: 400px;
    pointer-events: auto;
    animation: slideIn 0.25s ease-out;
  }
  .toast.success { border-left: 3px solid var(--success); color: var(--success); }
  .toast.error { border-left: 3px solid var(--error); color: var(--error); }
  .toast.warning { border-left: 3px solid var(--warning); color: var(--warning); }
  .toast-close {
    border: none;
    background: transparent;
    color: inherit;
    opacity: 0.5;
    padding: 2px;
    margin-left: auto;
    cursor: pointer;
  }
  .toast-close:hover { opacity: 1; }
  @keyframes slideIn {
    from { transform: translateX(100%); opacity: 0; }
    to { transform: translateX(0); opacity: 1; }
  }
</style>
