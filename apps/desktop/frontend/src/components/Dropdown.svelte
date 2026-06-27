<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { ChevronDown } from 'lucide-svelte';

  export let value: string | null = null;
  export let options: { id: string; name: string }[] = [];
  export let placeholder = '— select —';

  const dispatch = createEventDispatcher<{ change: string | null }>();
  let open = false;

  function select(id: string | null) {
    value = id;
    open = false;
    dispatch('change', id);
  }

  function toggle() {
    open = !open;
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false;
  }

  const selected = options.find((o) => o.id === value);
</script>

<svelte:window on:click={() => open = false} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="dd" on:keydown={handleKey}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="dd-trigger" on:click|stopPropagation={toggle} role="button" tabindex="0">
    <span class="dd-label" class:placeholder={!selected}>
      {selected?.name ?? placeholder}
    </span>
    <span class="dd-arrow" class:open><ChevronDown size="10" /></span>
  </div>

  {#if open}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dd-menu" on:click|stopPropagation>
      <div
        class="dd-item"
        class:active={value === null}
        on:click={() => select(null)}
      >
        {placeholder}
      </div>
      {#each options as opt (opt.id)}
        <div
          class="dd-item"
          class:active={value === opt.id}
          on:click={() => select(opt.id)}
        >
          {opt.name}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .dd {
    position: relative;
    font-size: 11px;
    user-select: none;
  }
  .dd-trigger {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    background: var(--bg-input);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    cursor: pointer;
    min-width: 140px;
  }
  .dd-trigger:hover {
    border-color: var(--border-strong);
  }
  .dd-label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dd-label.placeholder {
    color: var(--text-muted);
  }
  .dd-arrow {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    color: var(--text-muted);
    transition: transform 0.15s;
  }
  .dd-arrow.open {
    transform: rotate(180deg);
  }
  .dd-menu {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: 2px;
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm, 4px);
    box-shadow: var(--shadow-md, 0 4px 12px rgba(0,0,0,0.3));
    min-width: 100%;
    max-height: 260px;
    overflow-y: auto;
    z-index: 100;
  }
  .dd-item {
    padding: 4px 8px;
    cursor: pointer;
    color: var(--text);
    white-space: nowrap;
  }
  .dd-item:hover {
    background: var(--bg-hover);
  }
  .dd-item.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
</style>
