<script lang="ts">
  import { connectionState, statusText } from '$lib/stores';
  import { Circle, CircleDot } from 'lucide-svelte';
</script>

<footer class="statusbar">
  {#if $connectionState === 'connected'}
    <CircleDot size="10" class="status-dot connected" />
  {:else}
    <Circle size="10" class="status-dot" />
  {/if}
  <span class="status-text">{$statusText}</span>
</footer>

<style>
  .statusbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    background: var(--accent-emphasis);
    color: #fff;
    font-size: 11px;
    height: var(--statusbar-h);
    flex-shrink: 0;
  }
  .status-dot {
    flex-shrink: 0;
    color: #999;
  }
  .status-dot.connected {
    color: var(--success);
  }
  .status-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
