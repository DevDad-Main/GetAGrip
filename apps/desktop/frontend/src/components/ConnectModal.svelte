<script lang="ts">
  import { connectionState, connectionUrl, connectionName, connectionProduct, connectionVersion } from '$lib/stores';
  import { treeNodes } from '$lib/treeState';
  import { testConnection, connect, type ConnectResult } from '$lib/tauri';

  export let open = false;
  export let onClose: () => void;

  // Form fields
  let host = 'localhost';
  let port = '1433';
  let database = '';
  let username = 'sa';
  let password = '';
  let trustCert = true;
  let useUrl = false;
  let url = '';

  let testing = false;
  let connecting = false;
  let error: string | null = null;

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
      // Set tree nodes — skip the synthetic "Server" root, just show databases
      treeNodes.set(result.nodes.filter((n) => n.kind === 'Database'));
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
  <div class="modal-backdrop" on:click|self={onClose}>
    <div class="modal" role="dialog" aria-modal="true" aria-label="Connect to database">
      <div class="modal-header">
        <span class="modal-title">Connect to SQL Server</span>
        <button class="modal-close" on:click={onClose} title="Close">×</button>
      </div>
      <div class="modal-body">
        <div class="field mode-toggle">
          <button class:active={!useUrl} on:click={() => (useUrl = false)}>Form</button>
          <button class:active={useUrl} on:click={() => (useUrl = true)}>URL</button>
        </div>

        {#if useUrl}
          <div class="field">
            <input type="text" bind:value={url} placeholder="sqlserver://user:pass@host/db?trustServerCertificate=true" />
          </div>
        {:else}
          <div class="field-row">
            <div class="field">
              <label>Host</label>
              <input type="text" bind:value={host} placeholder="localhost" />
            </div>
            <div class="field field-sm">
              <label>Port</label>
              <input type="text" bind:value={port} placeholder="1433" />
            </div>
          </div>
          <div class="field">
            <label>Database <span class="optional">(optional)</span></label>
            <input type="text" bind:value={database} placeholder="master" />
          </div>
          <div class="field-row">
            <div class="field">
              <label>Username</label>
              <input type="text" bind:value={username} placeholder="sa" />
            </div>
            <div class="field">
              <label>Password</label>
              <input type="password" bind:value={password} placeholder="••••••••" />
            </div>
          </div>
          <div class="field checkbox">
            <label>
              <input type="checkbox" bind:checked={trustCert} />
              Trust server certificate
            </label>
          </div>
        {/if}

        {#if error}
          <div class="modal-error">{error}</div>
        {/if}
      </div>
      <div class="modal-footer">
        <button class="btn" on:click={handleTest} disabled={testing || connecting}>
          {#if testing}...{:else}Test{/if}
        </button>
        <button class="btn btn-primary" on:click={handleConnect} disabled={testing || connecting}>
          {#if connecting}Connecting...{:else}Connect{/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal {
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    width: 420px;
    max-width: 90vw;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-lg);
  }
  .modal-header {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }
  .modal-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .modal-close {
    margin-left: auto;
    border: none;
    background: transparent;
    font-size: 18px;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0 4px;
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
    gap: 4px;
    margin-bottom: 4px;
  }
  .mode-toggle button {
    flex: 1;
    padding: 4px 12px;
    font-size: 11px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-sm);
  }
  .mode-toggle button.active {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .field-sm {
    width: 90px;
    flex-shrink: 0;
  }
  .field-row {
    display: flex;
    gap: 10px;
  }
  .field label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .optional {
    font-weight: 400;
    text-transform: none;
    color: var(--text-faint);
  }
  .field input[type="text"],
  .field input[type="password"] {
    background: var(--bg-input);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 6px 10px;
    font-size: 12px;
    border-radius: var(--radius-sm);
    outline: none;
  }
  .field input:focus {
    border-color: var(--accent);
  }
  .field.checkbox label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    text-transform: none;
    letter-spacing: 0;
    color: var(--text);
    cursor: pointer;
  }
  .modal-error {
    background: rgba(188, 60, 60, 0.15);
    border: 1px solid var(--error);
    color: var(--error);
    padding: 8px 12px;
    font-size: 11px;
    border-radius: var(--radius-sm);
  }
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }
  .btn {
    padding: 6px 16px;
    font-size: 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-input);
    color: var(--text);
    cursor: pointer;
  }
  .btn:hover:not(:disabled) {
    background: var(--bg-input-focus);
  }
  .btn-primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .btn-primary:hover:not(:disabled) {
    background: var(--accent-emphasis);
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
