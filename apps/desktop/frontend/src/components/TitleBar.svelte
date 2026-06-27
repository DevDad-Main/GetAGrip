<script lang="ts">
  let { title = 'GetAGrip' }: { title?: string } = $props();
</script>

<header class="titlebar" data-tauri-drag-region>
  <span class="brand">{title}</span>
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
    -webkit-user-select: none;
    user-select: none;
  }
  .brand {
    color: var(--text);
    letter-spacing: 0.5px;
  }
  .spacer {
    flex: 1;
  }
</style>
