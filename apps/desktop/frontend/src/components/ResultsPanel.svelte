<script lang="ts">
  import { resultSets, activeResultSetId } from '$lib/stores';
  import ResultTabs from './ResultTabs.svelte';
  import ResultGrid from './ResultGrid.svelte';

  $: active = $resultSets.find((r) => r.id === $activeResultSetId) ?? null;
</script>

{#if $resultSets.length > 0}
  <section class="results-panel">
    <div class="rp-header">
      <span class="rp-title">RESULTS</span>
    </div>
    <ResultTabs />
    <div class="rp-body">
      {#if active}
        <ResultGrid result={active} />
      {:else}
        <div class="rp-empty">Select a result set to view.</div>
      {/if}
    </div>
  </section>
{/if}

<style>
  .results-panel {
    border-top: 1px solid var(--border);
    background: var(--bg);
    height: 220px;
    min-height: 100px;
    max-height: 60vh;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .rp-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 12px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 1px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .rp-body {
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }
  .rp-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-faint);
    font-size: 12px;
  }
</style>
