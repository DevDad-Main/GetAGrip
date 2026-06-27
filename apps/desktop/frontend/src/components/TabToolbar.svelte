<script lang="ts">
  import { datasources, activeDatasourceId } from '$lib/stores';
  import { Database } from 'lucide-svelte';

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
  }
  .tt-select-wrap::after {
    content: '';
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    pointer-events: none;
    border-left: 4px solid transparent;
    border-right: 4px solid transparent;
    border-top: 5px solid var(--text-muted);
  }
  .tt-select {
    font-size: 11px;
    padding: 2px 20px 2px 6px;
    background: var(--bg-input);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    min-width: 160px;
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
  }
  .tt-select:hover {
    border-color: var(--border-strong);
  }
  .tt-select:focus {
    outline: none;
    border-color: var(--accent);
  }
  .tt-select option {
    background: var(--bg-elev);
    color: var(--text);
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
