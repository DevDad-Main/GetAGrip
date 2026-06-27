<script lang="ts">
  export let direction: 'horizontal' | 'vertical' = 'horizontal';
  export let minSize = 160;
  export let maxSize = 600;
  export let size: number;
  export let onResize: (size: number) => void;
  export let onCollapse: (() => void) | null = null;
  export let collapseThreshold = 60;

  function onMouseDown(e: MouseEvent) {
    e.preventDefault();
    const start = direction === 'horizontal' ? e.clientX : e.clientY;
    const startSize = size;

    function onMove(ev: MouseEvent) {
      const current = direction === 'horizontal' ? ev.clientX : ev.clientY;
      let delta = current - start;
      if (direction === 'vertical') delta = -delta;
      const newSize = startSize + delta;

      if (newSize < collapseThreshold && onCollapse) {
        onCollapse();
        onUp();
        return;
      }

      onResize(Math.max(minSize, Math.min(maxSize, newSize)));
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
>
  <div class="grip-bar"></div>
</div>

<style>
  .resize-handle {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    transition: background 0.15s;
    position: relative;
  }
  .resize-handle:hover {
    background: var(--bg-hover);
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
  .grip-bar {
    width: 3px;
    height: 30px;
    border-radius: 2px;
    background: var(--text-faint);
    opacity: 0.3;
    transition: opacity 0.15s, background 0.15s;
  }
  .resize-handle:hover .grip-bar {
    opacity: 0.7;
    background: var(--accent);
  }
  .vertical .grip-bar {
    width: 30px;
    height: 3px;
  }
</style>
