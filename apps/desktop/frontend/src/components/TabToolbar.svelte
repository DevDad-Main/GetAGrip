<script lang="ts">
  import { datasources, activeDatasourceId } from '$lib/stores';
  import { Database } from 'lucide-svelte';

  export let datasourceId: string | null;
  export let schema: string | null;

  function handleDatasourceChange(e: Event) {
    const select = e.target as HTMLSelectElement;
    activeDatasourceId.set(select.value || null);
  }
</script>

<div class="tab-toolbar">
  <span class="tt-label">
    <Database size="12" />
  </span>
  <select
    class="tt-select"
    value={datasourceId ?? ''}
    on:change={handleDatasourceChange}
    title="Select data source"
  >
    <option value="">— no datasource —</option>
    {#each $datasources as ds (ds.id)}
      <option value={ds.id}>{ds.name}</option>
    {/each}
  </select>

  {#if datasourceId}
    <input
      class="tt-schema"
      type="text"
      bind:value={schema}
      placeholder="schema"
      title="Default schema"
    />
  {/if}
</div>

<style>
  .tab-toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
  }
  .tt-label {
    color: var(--text-muted);
    display: flex;
    align-items: center;
  }
  .tt-select {
    font-size: 11px;
    padding: 2px 4px;
    background: var(--bg-input);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    min-width: 120px;
  }
  .tt-schema {
    font-size: 11px;
    padding: 2px 6px;
    background: var(--bg-input);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    width: 80px;
  }
</style>
