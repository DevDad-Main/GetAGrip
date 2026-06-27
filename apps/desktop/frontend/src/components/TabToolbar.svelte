<script lang="ts">
  import { datasources, activeDatasourceId } from '$lib/stores';
  import { Database, ChevronDown } from 'lucide-svelte';

  export let datasourceId: string | null;
  export let schema: string | null;
  export let onChange: (datasourceId: string | null, schema: string | null) => void = () => {};

  $: dsOptions = $datasources.map((ds) => ({ id: ds.id, name: ds.name }));
  $: selected = dsOptions.find((o) => o.id === datasourceId);

  let open = false;

  function toggle() { open = !open; }
  function pick(id: string | null) {
    open = false;
    activeDatasourceId.set(id);
    onChange(id, schema);
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false;
  }
</script>

<svelte:window on:click={() => open = false} />

<div class="tab-toolbar" on:keydown={handleKey}>
  <span class="tt-label"><Database size="12" /></span>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="dd">
    <div class="dd-trigger" on:click|stopPropagation={toggle} role="button" tabindex="0">
      <span class="dd-label" class:placeholder={!selected}>
        {selected?.name ?? '— no datasource —'}
      </span>
      <span class="dd-arrow"><ChevronDown size="11" /></span>
    </div>
    {#if open}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="dd-menu" on:click|stopPropagation>
        <div class="dd-item" class:active={!datasourceId} on:click={() => pick(null)}>
          — no datasource —
        </div>
        {#each dsOptions as opt (opt.id)}
          <div class="dd-item" class:active={datasourceId === opt.id} on:click={() => pick(opt.id)}>
            {opt.name}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  {#if datasourceId}
    <input class="tt-schema" type="text" bind:value={schema} placeholder="schema" title="Default schema" />
  {/if}
</div>

<style>
  .tab-toolbar { display: flex; align-items: center; gap: 6px; padding: 2px 8px; }
  .tt-label { color: var(--text-muted); display: flex; align-items: center; }

  .dd { position: relative; font-size: 11px; user-select: none; }
  .dd-trigger {
    display: flex; align-items: center; gap: 4px;
    padding: 3px 8px;
    background: var(--bg-input); color: var(--text);
    border: 1px solid var(--border); border-radius: var(--radius-sm, 4px);
    cursor: pointer; min-width: 170px; max-width: 220px;
    line-height: 1.4;
  }
  .dd-trigger:hover { border-color: var(--border-strong); background: var(--bg-input-focus); }
  .dd-label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dd-label.placeholder { color: var(--text-muted); }
  .dd-arrow { color: var(--text-muted); display: flex; align-items: center; flex-shrink: 0; }

  .dd-menu {
    position: absolute; top: 100%; left: 0; margin-top: 2px;
    background: var(--bg-elev); border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm, 4px);
    box-shadow: var(--shadow-md, 0 4px 12px rgba(0,0,0,0.3));
    min-width: 100%; max-height: 260px; overflow-y: auto; z-index: 100;
  }
  .dd-item {
    padding: 5px 10px; cursor: pointer; color: var(--text); white-space: nowrap;
  }
  .dd-item:hover { background: var(--bg-hover); }
  .dd-item.active { background: var(--accent-soft); color: var(--accent); font-weight: 500; }

  .tt-schema {
    font-size: 11px; padding: 3px 6px;
    background: var(--bg-input); color: var(--text);
    border: 1px solid var(--border); border-radius: var(--radius-sm);
    width: 80px; line-height: 1.4;
  }
  .tt-schema:focus { outline: none; border-color: var(--accent); }
</style>
