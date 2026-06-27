<script lang="ts">
  import { connectionState, connectionUrl, connectionName, connectionProduct, connectionVersion, explorerNodes } from '$lib/stores';
  import { testConnection, connect, type ConnectResult } from '$lib/tauri';

  let { open = false, onClose }: { open: boolean; onClose: () => void } = $props();

  // Form fields
  let host = $state('localhost');
  let port = $state('1433');
  let database = $state('');
  let username = $state('');
  let password = $state('');
  let trustCert = $state(true);

  // Mode: URL (advanced) or form
  let useUrl = $state(false);
  let url = $state('');

  // Status
  let testing = $state(false);
  let connecting = $state(false);
  let error = $state<string | null>(null);

  function buildUrl(): string {
    if (useUrl) return url;
    const enc = (s: string) => encodeURIComponent(s);
    let u = 'sqlserver://';
    if (username) u += enc(username);
    if (password) u += ':' + enc(password);
    if (username || password) u += '@';
    u += host;
    if (port && port !== '1433') u += ':' + port;
    if (database) u += '/' + enc(database);
    const params: string[] = [];
    if (trustCert) params.push('trustServerCertificate=true');
    if (params.length) u += '?' + params.join('&');
    return u;
  }

  async function handleTest() {
    error = null;
    testing = true;
    try {
      await testConnection(buildUrl());
    } catch (e: unknown) {
      error = String(e);
    } finally {
      testing = false;
    }
  }

  async function handleConnect() {
    error = null;
    connecting = true;
    connectionState.set('connecting');
    try {
      const finalUrl = buildUrl();
      const name = host + (database ? ` / ${database}` : '');
      const result: ConnectResult = await connect(finalUrl, name);
      // Update stores
      connectionUrl.set(finalUrl);
      connectionName.set(result.name);
      connectionProduct.set(result.product_name);
      connectionVersion.set(result.version);
      connectionState.set('connected');
      // Set explorer roots — skip the synthetic "Server" root, just show databases
      explorerNodes.set(result.nodes.filter(n => n.kind === 'Database'));
      onClose();
    } catch (e: unknown) {
      error = String(e);
      connectionState.set('disconnected');
    } finally {
      connecting = false;
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-backdrop" onclick={onClose}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Connect to database">
      <div class="modal-header">
        Connect to Database
        <button class="modal-close" onclick={onClose} title="Close">×</button>
      </div>

      <div class="modal-body">
        <div class="mode-toggle">
          <button class:active={!useUrl} onclick={() => useUrl = false}>Form</button>
          <button class:active={useUrl} onclick={() => useUrl = true}>URL</button>
        </div>

        {#if useUrl}
          <label class="field">
            <span class="label">Connection URL</span>
            <input type="text" bind:value={url} placeholder="sqlserver://user:pass@host/db?trustServerCertificate=true" />
          </label>
        {:else}
          <div class="form-grid">
            <label class="field">
              <span class="label">Host</span>
              <input type="text" bind:value={host} placeholder="localhost" />
            </label>
            <label class="field">
              <span class="label">Port</span>
              <input type="text" bind:value={port} placeholder="1433" />
            </label>
          </div>
          <label class="field">
            <span class="label">Database</span>
            <input type="text" bind:value={database} placeholder="master" />
          </label>
          <label class="field">
            <span class="label">Username</span>
            <input type="text" bind:value={username} placeholder="sa" />
          </label>
          <label class="field">
            <span class="label">Password</span>
            <input type="password" bind:value={password} placeholder="••••••••" />
          </label>
          <label class="field checkbox">
            <input type="checkbox" bind:checked={trustCert} />
            <span>Trust server certificate</span>
          </label>
        {/if}

        {#if error}
          <div class="error-banner">{error}</div>
        {/if}
      </div>

      <div class="modal-footer">
        <button onclick={handleTest} disabled={testing || connecting}>
          {testing ? 'Testing…' : 'Test Connection'}
        </button>
        <button class="primary" onclick={handleConnect} disabled={connecting || testing}>
          {connecting ? 'Connecting…' : 'Connect'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .modal {
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    width: 420px;
    max-width: 90vw;
    display: flex;
    flex-direction: column;
  }
  .modal-header {
    display: flex;
    align-items: center;
    padding: 10px 16px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    border-bottom: 1px solid var(--border);
  }
  .modal-close {
    margin-left: auto;
    border: none;
    background: transparent;
    font-size: 18px;
    padding: 0 4px;
    color: var(--text-muted);
  }
  .modal-close:hover {
    color: var(--text);
  }
  .modal-body {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .mode-toggle {
    display: flex;
    gap: 0;
    margin-bottom: 4px;
  }
  .mode-toggle button {
    border-radius: 0;
    padding: 4px 16px;
    font-size: 11px;
    background: var(--bg-input);
    border-color: var(--border);
  }
  .mode-toggle button:first-child {
    border-radius: var(--radius-sm) 0 0 var(--radius-sm);
  }
  .mode-toggle button:last-child {
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    border-left: none;
  }
  .mode-toggle button.active {
    background: var(--accent-emphasis);
    border-color: var(--accent);
    color: #fff;
  }
  .form-grid {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .field .label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }
  .field.checkbox {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
  }
  .field.checkbox .label {
    display: none;
  }
  .field.checkbox span {
    font-size: 12px;
    color: var(--text-muted);
  }
  .error-banner {
    background: rgba(188, 60, 60, 0.15);
    border: 1px solid var(--error);
    color: var(--error);
    border-radius: var(--radius-sm);
    padding: 6px 10px;
    font-size: 12px;
  }
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }
</style>
