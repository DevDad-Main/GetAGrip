<script lang="ts">
  import { datasources, activeDatasourceId } from '$lib/stores';
  import { Database, ChevronDown } from 'lucide-svelte';

  export let datasourceId: string | null;
  export let schema: string | null;
  export let onChange: (datasourceId: string | null, schema: string | null) => void = () => {};

  function handleDsChange(e: Event) {
    const select = e.target as HTMLSelectElement;
    const newId = select.value || null;
    activeDatasourceId.set(newId);
    onChange(newId, schema);
  }
</script>

<div class="tab-toolbar">
  <span class="tt-label">
    <Database size="12" />
  </span>
  <div class="tt-select-wrap">
    <select
      class="tt-select"
      value={datasourceId ?? ''}
      on:change={handleDsChange}
      title="Select data source"
    >
      <option value="">— no datasource —</option>
      {#each $datasources as ds (ds.id)}
        <option value={ds.id}>{ds.name}</option>
      {/each}
    </select>
    <span class="tt-chevron"><ChevronDown size="11" /></span>
  </div>

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
  .tt-select-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }
  .tt-select {
    font-size: 11px;
    padding: 3px 24px 3px 8px;
    background: var(--bg-input);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    min-width: 170px;
    max-width: 220px;
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    line-height: 1.4;
  }
  .tt-select:hover {
    border-color: var(--border-strong);
    background: var(--bg-input-focus);
  }
  .tt-select:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent-soft);
  }
  .tt-select option {
    background: var(--bg-elev);
    color: var(--text);
    padding: 4px 8px;
  }
  .tt-chevron {
    position: absolute;
    right: 6px;
    pointer-events: none;
    color: var(--text-muted);
    display: flex;
    align-items: center;
  }
  .tt-schema {
    font-size: 11px;
    padding: 3px 6px;
    background: var(--bg-input);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    width: 80px;
    line-height: 1.4;
  }
  .tt-schema:focus {
    outline: none;
    border-color: var(--accent);
  }
</style>
