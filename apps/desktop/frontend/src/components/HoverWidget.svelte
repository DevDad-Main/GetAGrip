<script lang="ts">
  export let visible = false;
  export let content = '';
  export let x = 0;
  export let y = 0;
</script>

{#if visible && content}
  <div class="hw" style="top: {y}px; left: {x}px;">
    <div class="hw-inner">
      {@html content}
    </div>
  </div>
{/if}

<style>
  .hw {
    position: fixed;
    z-index: 10001;
    pointer-events: none;
    max-width: 420px;
  }
  .hw-inner {
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md, 6px);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5);
    padding: 10px 14px;
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    color: var(--text);
    line-height: 1.6;
    white-space: pre-wrap;
  }
  .hw-inner :global(strong) {
    color: var(--accent);
    font-weight: 600;
  }
  .hw-inner :global(.hw-error) {
    color: var(--error, #f44747);
    font-weight: 700;
  }
  .hw-inner :global(.hw-warning) {
    color: var(--warning, #cca700);
    font-weight: 700;
  }
  .hw-inner :global(.hw-hint) {
    color: var(--text-muted);
    font-weight: 600;
  }
  .hw-inner :global(em) {
    color: var(--text-muted);
    font-style: italic;
  }
</style>
