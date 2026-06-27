<script lang="ts">
  import { connectionState, statusText } from '$lib/stores';
</script>

<footer class="statusbar">
  <span
    class="status-dot"
    class:connected={$connectionState === 'connected'}
  ></span>
  <span>{$statusText}</span>
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
  }
  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
    background: #999;
  }
  .status-dot.connected {
    background: var(--success);
  }
</style>
