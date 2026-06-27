<script lang="ts">
  import { datasources, activeDatasourceId } from '$lib/stores';
  import { Database } from 'lucide-svelte';
  import Dropdown from './Dropdown.svelte';

  export let datasourceId: string | null;
  export let schema: string | null;
  export let onChange: (datasourceId: string | null, schema: string | null) => void = () => {};

  $: dsOptions = $datasources.map((ds) => ({ id: ds.id, name: ds.name }));

  // Local mirror that stays in sync with the prop
  let selectedId: string | null = datasourceId;
  $: if (datasourceId !== selectedId) selectedId = datasourceId;

  function handleDsChange(e: CustomEvent<string | null>) {
    const newId = e.detail;
    selectedId = newId;
    activeDatasourceId.set(newId);
    onChange(newId, schema);
  }
</script>

<div class="tab-toolbar">
  <span class="tt-label">
    <Database size="12" />
  </span>
  <Dropdown
    bind:value={selectedId}
    options={dsOptions}
    placeholder="— no datasource —"
    on:change={handleDsChange}
  />

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
