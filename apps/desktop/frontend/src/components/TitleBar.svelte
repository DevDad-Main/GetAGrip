<script lang="ts">
  import MenuBar from './MenuBar.svelte';

  export let title = 'GetAGrip';
  export let openSettings: () => void;
</script>

<header class="titlebar" data-tauri-drag-region>
  <span class="brand">{title}</span>
  <MenuBar {openSettings} />
  <span class="spacer"></span>
</header>

<style>
  .titlebar {
    display: flex;
    align-items: center;
    padding: 0 12px;
    background: var(--bg-elev);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    height: var(--titlebar-h);
    flex-shrink: 0;
    -webkit-user-select: none;
    user-select: none;
  }
  .brand {
    color: var(--text);
    letter-spacing: 0.5px;
    margin-right: 16px;
  }
  .spacer { flex: 1; }
</style>
