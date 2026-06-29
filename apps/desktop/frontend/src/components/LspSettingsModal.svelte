<script lang="ts">
  import { getLspServers, setLspPath, installLsp, type LspServerInfo } from '$lib/tauri';
  import { onMount } from 'svelte';
  import { X, FolderOpen, Package, RefreshCw, CheckCircle, XCircle, AlertTriangle, Loader, Terminal } from 'lucide-svelte';
  import { notify } from '$lib/toast';
  import { resultsPanelHeight, activeBottomTab } from '$lib/stores';

  export let open = false;
  export let onClose: () => void;
  export let onOpenTerminal: ((cmd: string) => void) | null = null;

  let servers: LspServerInfo[] = [];
  let loading = true;
  let installing = '';
  let installCommands: string[] = [];

  onMount(load);

  async function load() {
    loading = true;
    try {
      servers = await getLspServers();
    } catch (e) {
      notify(`Failed to load LSP info: ${e}`, 'error');
    }
    loading = false;
  }

  async function browseBinary(driver: string) {
    try {
      const { open: dialogOpen } = await import('@tauri-apps/plugin-dialog');
      const selected = await dialogOpen({
        multiple: false,
        filters: [{
          name: 'Binary',
          extensions: ['*'],
        }],
      });
      if (selected) {
        await setLspPath(driver, selected);
        notify(`${driver} LSP path set`, 'success');
        await load();
      }
    } catch (e) {
      notify(`Browse failed: ${e}`, 'error');
    }
  }

  async function clearPath(driver: string) {
    await setLspPath(driver, null);
    notify(`${driver} LSP path cleared`, 'info');
    await load();
  }

  async function doInstall(driver: string) {
    installing = driver;
    installCommands = [];
    try {
      const cmds = await installLsp(driver);
      if (cmds.length > 0) {
        installCommands = cmds;
      }
    } catch (e) {
      notify(`Install info failed: ${e}`, 'error');
    }
    installing = '';
  }

  function runInTerminal(cmd: string) {
    resultsPanelHeight.set(200);
    activeBottomTab.set('terminal');
    onClose();
    // Dispatch custom event for the terminal panel to pick up
    window.dispatchEvent(new CustomEvent('terminal-run', { detail: cmd }));
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="lsp-backdrop" on:click={onClose}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="lsp-modal" on:click|stopPropagation role="dialog" aria-label="LSP Language Servers">
      <div class="lsp-header">
        <span>Language Servers (LSP)</span>
        <div class="lsp-header-actions">
          <button class="lsp-refresh" on:click={load} title="Refresh"><RefreshCw size="12" /></button>
          <button class="lsp-close" on:click={onClose}><X size="14" /></button>
        </div>
      </div>
      <div class="lsp-body">
        {#if loading}
          <div class="lsp-loading"><Loader size="16" class="spin" /> Loading…</div>
        {:else}
          <div class="lsp-info">
            Language servers provide enhanced autocompletion, diagnostics, and
            syntax analysis. Configure a binary path or install one below.
          </div>
          {#each servers as server}
            <div class="lsp-server" class:installed={server.installed}>
              <div class="lsp-server-header">
                <span class="lsp-status-dot" class:online={server.installed}>
                  {#if server.installed}
                    <CheckCircle size="12" />
                  {:else}
                    <XCircle size="12" />
                  {/if}
                </span>
                <span class="lsp-driver-name">{server.display_name}</span>
                <span class="lsp-driver-id">{server.driver}</span>
                <span class="lsp-status-badge" class:active={server.installed}>
                  {server.installed ? 'Active' : 'Not found'}
                </span>
              </div>
              <div class="lsp-server-path">
                {#if server.path}
                  {#if server.auto_detected}
                    <span class="lsp-path-label">Auto-detected:</span>
                  {:else}
                    <span class="lsp-path-label">Path:</span>
                  {/if}
                  <code>{server.path}</code>
                {:else}
                  <span class="lsp-path-label">Path:</span>
                  <span class="lsp-path-empty">Not configured</span>
                {/if}
              </div>
              <div class="lsp-server-actions">
                <button class="lsp-btn lsp-btn-browse" on:click={() => browseBinary(server.driver)} title="Select LSP binary">
                  <FolderOpen size="12" /> Browse
                </button>
                {#if server.path && !server.auto_detected}
                  <button class="lsp-btn lsp-btn-clear" on:click={() => clearPath(server.driver)} title="Clear custom path">
                    Clear
                  </button>
                {/if}
                <button class="lsp-btn lsp-btn-install" on:click={() => doInstall(server.driver)} disabled={installing === server.driver} title="Show install instructions">
                  {#if installing === server.driver}
                    <Loader size="12" class="spin" />
                  {:else}
                    <Package size="12" />
                  {/if}
                  Install
                </button>
              </div>
              {#if installCommands.length > 0 && installing === ''}
                <div class="lsp-install-cmds">
                  <span class="lsp-install-label">Install commands:</span>
                  {#each installCommands as cmd}
                    <div class="lsp-cmd-row">
                      <code class="lsp-cmd">{cmd}</code>
                      <button class="lsp-btn lsp-btn-run" on:click={() => runInTerminal(cmd)} title="Run in terminal">
                        <Terminal size="10" /> Run
                      </button>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .lsp-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.4);
    display: flex; align-items: center; justify-content: center; z-index: 1002;
  }
  .lsp-modal {
    background: var(--bg-elev); border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg); box-shadow: var(--shadow-lg);
    width: 560px; max-width: 92vw; max-height: 80vh;
    display: flex; flex-direction: column;
  }
  .lsp-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 12px 16px; border-bottom: 1px solid var(--border);
    font-size: 13px; font-weight: 600; color: var(--text); flex-shrink: 0;
  }
  .lsp-header-actions {
    display: flex; gap: 4px;
  }
  .lsp-close, .lsp-refresh {
    border: none; background: transparent; color: var(--text-muted); padding: 2px;
    cursor: pointer; display: flex; align-items: center; border-radius: 2px;
  }
  .lsp-close:hover, .lsp-refresh:hover { color: var(--text); background: var(--bg-hover); }
  .lsp-body {
    flex: 1; overflow-y: auto; padding: 16px 20px;
  }
  .lsp-loading {
    display: flex; align-items: center; gap: 8px; justify-content: center;
    padding: 40px; color: var(--text-faint); font-size: 12px;
  }
  .lsp-info {
    font-size: 11px; color: var(--text-muted); margin-bottom: 16px;
    line-height: 1.5; padding: 8px 10px; background: var(--bg);
    border-radius: var(--radius-md);
  }
  .lsp-server {
    padding: 12px; margin-bottom: 10px; border-radius: var(--radius-md);
    border: 1px solid var(--border); background: var(--bg);
  }
  .lsp-server.installed {
    border-color: var(--border-strong);
  }
  .lsp-server-header {
    display: flex; align-items: center; gap: 8px; margin-bottom: 8px;
  }
  .lsp-status-dot {
    display: flex; align-items: center; color: var(--error);
  }
  .lsp-status-dot.online {
    color: var(--success);
  }
  .lsp-driver-name {
    font-size: 13px; font-weight: 600; color: var(--text);
  }
  .lsp-driver-id {
    font-size: 10px; color: var(--text-faint);
    background: var(--bg-elev); padding: 1px 6px; border-radius: 3px;
  }
  .lsp-status-badge {
    margin-left: auto; font-size: 10px; font-weight: 600;
    padding: 2px 8px; border-radius: 4px;
    background: var(--bg-elev); color: var(--text-muted);
  }
  .lsp-status-badge.active {
    background: var(--success); color: #fff;
  }
  .lsp-server-path {
    display: flex; align-items: center; gap: 6px; font-size: 11px;
    color: var(--text-muted); margin-bottom: 10px;
  }
  .lsp-path-label {
    flex-shrink: 0;
  }
  .lsp-server-path code {
    font-family: var(--font-mono); font-size: 10px;
    background: var(--bg-elev); padding: 2px 6px; border-radius: 3px;
    color: var(--text); word-break: break-all;
  }
  .lsp-path-empty {
    color: var(--text-faint); font-style: italic;
  }
  .lsp-server-actions {
    display: flex; gap: 6px;
  }
  .lsp-btn {
    font-size: 11px; padding: 4px 10px; border: 1px solid var(--border);
    border-radius: var(--radius-sm); cursor: pointer;
    display: flex; align-items: center; gap: 4px;
    background: var(--bg-elev); color: var(--text);
  }
  .lsp-btn:hover { background: var(--bg-hover); border-color: var(--border-strong); }
  .lsp-btn:disabled { opacity: 0.5; cursor: default; }
  .lsp-btn-browse { color: var(--accent); }
  .lsp-btn-install { }
  .lsp-btn-clear { color: var(--warning); }
  .lsp-install-cmds {
    margin-top: 8px; padding: 8px; background: var(--bg-elev);
    border-radius: var(--radius-sm); display: flex; flex-direction: column; gap: 4px;
  }
  .lsp-install-label {
    font-size: 10px; color: var(--text-faint); text-transform: uppercase; letter-spacing: 0.5px;
  }
  .lsp-cmd-row {
    display: flex; align-items: center; gap: 6px;
  }
  .lsp-cmd-row .lsp-cmd {
    flex: 1;
  }
  .lsp-cmd {
    font-family: var(--font-mono); font-size: 11px;
    padding: 4px 8px; background: var(--bg-input); border-radius: 3px;
    color: var(--accent); user-select: all;
  }
  .lsp-btn-run {
    color: var(--success); flex-shrink: 0;
  }
  :global(.spin) {
    animation: lsp-spin 1s linear infinite;
  }
  @keyframes lsp-spin {
    to { transform: rotate(360deg); }
  }
</style>
