<script lang="ts">
  import { diagnostics, jumpToPosition } from '$lib/stores';
  import { AlertCircle, AlertTriangle, Info, X, ArrowUp } from 'lucide-svelte';

  export let visible = false;
  export let onClose: () => void = () => {};

  function jumpTo(line: number, column: number) {
    jumpToPosition.set({ line, column });
  }

  $: sorted = [...$diagnostics].sort((a, b) => a.line - b.line || a.column - b.column);
  $: errs = sorted.filter((d) => d.severity === 'error');
  $: warns = sorted.filter((d) => d.severity === 'warning');

  function icon(severity: string) {
    if (severity === 'error') return AlertCircle;
    if (severity === 'warning') return AlertTriangle;
    return Info;
  }
</script>

{#if visible}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="dp-overlay" on:click={onClose}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dp-panel" on:click|stopPropagation>
      <div class="dp-header">
        <span class="dp-title">
          Problems
          {#if sorted.length > 0}
            <span class="dp-count">({errs.length} err, {warns.length} warn)</span>
          {/if}
        </span>
        <button class="dp-close" on:click={onClose}><X size="14" /></button>
      </div>
      <div class="dp-list">
        {#if sorted.length === 0}
          <div class="dp-empty">No problems detected</div>
        {:else}
          {#each sorted as d, i (`${d.line}-${d.column}-${i}`)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="dp-item"
              class:dp-err={d.severity === 'error'}
              class:dp-warn={d.severity === 'warning'}
              on:click={() => jumpTo(d.line, d.column)}
            >
              <span class="dp-icon"><svelte:component this={icon(d.severity)} size="13" /></span>
              <span class="dp-msg">{d.message}</span>
              <span class="dp-pos">Ln {d.line}, Col {d.column}</span>
              <span class="dp-go"><ArrowUp size="12" /></span>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .dp-overlay {
    position: fixed; inset: 0; z-index: 9000;
    pointer-events: all;
  }
  .dp-panel {
    position: absolute;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-bottom: none;
    border-radius: 8px 8px 0 0;
    box-shadow: 0 -4px 20px rgba(0,0,0,0.4);
    width: min(700px, calc(100vw - 60px));
    max-height: 400px;
    display: flex; flex-direction: column;
    font-size: 12px;
  }
  .dp-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    color: var(--text);
    font-weight: 600;
  }
  .dp-count { font-weight: 400; color: var(--text-muted); font-size: 11px; }
  .dp-close {
    border: none; background: transparent; color: var(--text-muted);
    cursor: pointer; padding: 2px; display: flex;
  }
  .dp-close:hover { color: var(--text); }
  .dp-list { flex: 1; overflow-y: auto; padding: 4px 0; }
  .dp-empty {
    padding: 24px; text-align: center; color: var(--text-muted);
    font-size: 12px;
  }
  .dp-item {
    display: flex; align-items: flex-start; gap: 8px;
    padding: 5px 14px;
    cursor: pointer;
    color: var(--text);
    line-height: 1.4;
  }
  .dp-item:hover { background: var(--bg-hover); }
  .dp-err { border-left: 3px solid var(--error, #f44747); }
  .dp-warn { border-left: 3px solid var(--warning, #cca700); }
  .dp-icon { flex-shrink: 0; margin-top: 1px; }
  .dp-err .dp-icon { color: var(--error, #f44747); }
  .dp-warn .dp-icon { color: var(--warning, #cca700); }
  .dp-msg { flex: 1; min-width: 0; }
  .dp-pos {
    flex-shrink: 0; color: var(--text-muted); font-size: 10px;
    white-space: nowrap;
  }
  .dp-go {
    flex-shrink: 0; color: var(--text-faint);
    opacity: 0; transition: opacity 0.1s;
  }
  .dp-item:hover .dp-go { opacity: 1; }
</style>
