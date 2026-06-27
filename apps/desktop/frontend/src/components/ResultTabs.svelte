<script lang="ts">
  import { resultSets, activeResultSetId } from '$lib/stores';
  import type { ResultSet } from '$lib/stores';
  import { X, PinOff, Pin } from 'lucide-svelte';

  function selectTab(id: string) {
    activeResultSetId.set(id);
  }

  function closeTab(e: MouseEvent, id: string) {
    e.stopPropagation();
    resultSets.update((rs) => {
      const filtered = rs.filter((r) => r.id !== id);
      const current = rs.find((r) => r.id === id);
      if (current && id === getActiveId()) {
        const idx = rs.indexOf(current);
        const next = filtered[Math.min(idx, filtered.length - 1)];
        if (next) activeResultSetId.set(next.id);
        else activeResultSetId.set(null);
      }
      return filtered;
    });
  }

  function togglePin(e: MouseEvent, id: string) {
    e.stopPropagation();
    resultSets.update((rs) =>
      rs.map((r) => (r.id === id ? { ...r, pinned: !r.pinned } : r)),
    );
  }

  function getActiveId(): string | null {
    let id: string | null = null;
    activeResultSetId.subscribe((v) => (id = v))();
    return id;
  }

  // Auto-select first result set if none active and sets exist
  $: if ($resultSets.length > 0 && !$activeResultSetId) {
    activeResultSetId.set($resultSets[0].id);
  }
</script>

{#if $resultSets.length > 0}
  <div class="rs-tabs">
    {#each $resultSets as rs (rs.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div class="rs-tab" class:active={rs.id === $activeResultSetId} class:pinned={rs.pinned} on:click={() => selectTab(rs.id)}>
        {#if rs.pinned}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <button class="rs-pin" on:click={(e) => togglePin(e, rs.id)} title="Unpin">
            <Pin size="10" />
          </button>
        {:else}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <button class="rs-pin" on:click={(e) => togglePin(e, rs.id)} title="Pin">
            <PinOff size="10" />
          </button>
        {/if}
        <span class="rs-title">Result {rs.id.replace('rs', '')}</span>
        <span class="rs-meta">{rs.rows.length}r {rs.elapsedMs}ms</span>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <button class="rs-close" on:click={(e) => closeTab(e, rs.id)} title="Close"><X size="11" /></button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .rs-tabs {
    display: flex;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
    flex-shrink: 0;
  }
  .rs-tab {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    font-size: 11px;
    color: var(--text-muted);
    cursor: pointer;
    border-right: 1px solid var(--border);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .rs-tab:hover { background: var(--bg-hover); }
  .rs-tab.active {
    color: var(--text);
    background: var(--bg-elev);
    border-bottom: 2px solid var(--accent);
  }
  .rs-tab.pinned { border-left: 2px solid var(--warning); }
  .rs-title { max-width: 100px; overflow: hidden; text-overflow: ellipsis; }
  .rs-meta { font-size: 10px; color: var(--text-faint); }
  .rs-pin, .rs-close {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 1px;
    cursor: pointer;
    display: flex;
  }
  .rs-pin:hover, .rs-close:hover { color: var(--text); }
  .rs-tabs::-webkit-scrollbar { height: 0; }
</style>
