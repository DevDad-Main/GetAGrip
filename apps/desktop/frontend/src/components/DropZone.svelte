<script lang="ts">
  import { dragState, activeDropZone } from '$lib/stores';
</script>

{#if $dragState}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="dropzone-overlay" role="region" aria-label="Drop zone">
    <div class="drop-target drop-left" class:hot={$activeDropZone === 'left'}>
      <div class="drop-target-inner">
        <span class="drop-label">← Split Left</span>
      </div>
    </div>
    <div class="drop-target drop-right" class:hot={$activeDropZone === 'right'}>
      <div class="drop-target-inner">
        <span class="drop-label">Split Right →</span>
      </div>
    </div>
  
    <div class="drop-ghost">
      <span class="ghost-icon">⬡</span>
      <span class="ghost-label">{$dragState?.title ?? 'item'}</span>
    </div>
  </div>
{/if}

<style>
  .dropzone-overlay {
    position: fixed;
    inset: 0;
    z-index: 9000;
    background: rgba(0,0,0,0.12);
    pointer-events: none;
  }
  .drop-target {
    position: absolute;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
  }
  .drop-target-inner {
    display: flex;
    align-items: center;
    justify-content: center;
    width: calc(100% - 8px);
    height: calc(100% - 8px);
    border: 2px dashed rgba(255,255,255,0.15);
    border-radius: 6px;
    transition: all 0.1s;
  }
  .drop-target.hot .drop-target-inner {
    border-color: var(--accent, #4a9eff);
    background: rgba(74, 158, 255, 0.08);
  }
  .drop-target.hot .drop-label {
    color: var(--accent, #4a9eff);
  }
  .drop-label {
    font-size: 11px;
    font-weight: 600;
    color: rgba(255,255,255,0.3);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: color 0.1s;
  }
  .drop-left { left: 0; top: 0; bottom: 0; width: 25%; }
  .drop-right { right: 0; top: 0; bottom: 0; width: 25%; }
  .drop-ghost {
    position: fixed;
    pointer-events: none;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    background: var(--bg-elev);
    border: 1px solid var(--accent);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
    font-size: 11px;
    color: var(--text);
    opacity: 0.9;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
  }
  .ghost-icon { color: var(--accent); font-size: 14px; }
  .ghost-label { max-width: 160px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
