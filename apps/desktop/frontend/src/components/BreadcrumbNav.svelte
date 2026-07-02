<script lang="ts">
  import { breadcrumbs, activePaneId, activeTabId } from '$lib/stores';
  import { ChevronRight } from 'lucide-svelte';
</script>

{#if $breadcrumbs.length > 0}
  <div class="breadcrumb-bar">
    {#each $breadcrumbs as crumb, i (i)}
      {#if i > 0}
        <span class="crumb-sep"><ChevronRight size="10" /></span>
      {/if}
      <span
        class="crumb"
        class:active={i === $breadcrumbs.length - 1}
        title={crumb.filePath ?? crumb.label}
      >
        {crumb.label}
      </span>
    {/each}
    {#if $activeTabId}
      <span class="crumb-actions">
        <span class="crumb-action" title="Copy path" on:click={() => {
          navigator.clipboard.writeText($breadcrumbs.map(c => c.label).join(' > '));
        }}>📋</span>
      </span>
    {/if}
  </div>
{/if}

<style>
  .breadcrumb-bar {
    display: flex;
    align-items: center;
    padding: 2px 8px;
    gap: 2px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    min-height: 20px;
    overflow-x: auto;
    scrollbar-width: none;
    flex-shrink: 0;
  }
  .breadcrumb-bar::-webkit-scrollbar { height: 0; }
  .crumb {
    color: var(--text-faint);
    white-space: nowrap;
    padding: 1px 4px;
    border-radius: 2px;
    cursor: default;
  }
  .crumb.active {
    color: var(--text);
  }
  .crumb-sep {
    display: flex;
    align-items: center;
    color: var(--text-faint);
    opacity: 0.5;
  }
  .crumb-actions {
    margin-left: auto;
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.1s;
  }
  .breadcrumb-bar:hover .crumb-actions {
    opacity: 1;
  }
  .crumb-action {
    cursor: pointer;
    padding: 0 4px;
    font-size: 10px;
    opacity: 0.6;
  }
  .crumb-action:hover { opacity: 1; }
</style>
