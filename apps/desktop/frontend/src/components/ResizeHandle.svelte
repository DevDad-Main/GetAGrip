<script lang="ts">
  export let direction: 'horizontal' | 'vertical' = 'horizontal';
  export let minSize = 160;
  export let maxSize = 600;
  export let size: number;
  export let onResize: (size: number) => void;

  let dragging = false;

  function onMouseDown(e: MouseEvent) {
    e.preventDefault();
    dragging = true;
    const start = direction === 'horizontal' ? e.clientX : e.clientY;
    const startSize = size;

    function onMouseMove(e: MouseEvent) {
      if (!dragging) return;
      const current = direction === 'horizontal' ? e.clientX : e.clientY;
      const delta = current - start;
      const newSize = Math.max(minSize, Math.min(maxSize, startSize + delta));
      onResize(newSize);
    }

    function onMouseUp() {
      dragging = false;
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    }

    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
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
  tabindex="0"
></div>

<style>
  .resize-handle {
    flex-shrink: 0;
    background: var(--border);
    transition: background 0.15s;
  }
  .resize-handle:hover {
    background: var(--accent);
  }
  .horizontal {
    width: 3px;
    cursor: col-resize;
  }
  .vertical {
    height: 3px;
    cursor: row-resize;
  }
</style>
