<script lang="ts">
  export let direction: 'horizontal' | 'vertical' = 'horizontal';
  export let minSize = 160;
  export let maxSize = 600;
  export let size: number;
  export let onResize: (size: number) => void;

  function onMouseDown(e: MouseEvent) {
    e.preventDefault();
    const start = direction === 'horizontal' ? e.clientX : e.clientY;
    const startSize = size;

    function onMove(ev: MouseEvent) {
      const current = direction === 'horizontal' ? ev.clientX : ev.clientY;
      const delta = current - start;
      onResize(Math.max(minSize, Math.min(maxSize, startSize + delta)));
    }
    function onUp() {
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    }
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
    document.body.style.cursor = direction === 'horizontal' ? 'col-resize' : 'row-resize';
    document.body.style.userSelect = 'none';
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="resize-handle"
  class:horizontal={direction === 'horizontal'}
  class:vertical={direction === 'vertical'}
  on:mousedown={onMouseDown}
  role="separator"
  aria-orientation={direction}
></div>

<style>
  .resize-handle {
    flex-shrink: 0;
    background: transparent;
    transition: background 0.15s;
  }
  .resize-handle:hover {
    background: var(--accent-soft);
  }
  .horizontal {
    width: 6px;
    cursor: col-resize;
    align-self: stretch;
  }
  .vertical {
    height: 6px;
    cursor: row-resize;
    width: 100%;
  }
</style>
