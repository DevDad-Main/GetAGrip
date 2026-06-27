<script lang="ts">
  import { saveDatasource, updateDatasource, type ConnectionProfile, type DatasourceInput } from '$lib/tauri';
  import { loadDatasources } from '$lib/stores';
  import { handleConnect } from '$lib/connection';
  import { X } from 'lucide-svelte';

  export let open = false;
  export let onClose: () => void;
  export let editProfile: ConnectionProfile | null = null;

  const DRIVERS = [
    { value: 'mssql', label: 'SQL Server' },
    { value: 'postgres', label: 'PostgreSQL' },
    { value: 'mysql', label: 'MySQL' },
    { value: 'sqlite', label: 'SQLite' },
  ];

  const ENVIRONMENTS = [
    { value: 'red', label: '🔴 Production', color: '#bc3c3c' },
    { value: 'orange', label: '🟠 Staging', color: '#cc7832' },
    { value: 'yellow', label: '🟡 QA', color: '#e5c07b' },
    { value: 'green', label: '🟢 Development', color: '#629755' },
    { value: 'blue', label: '🔵 Testing', color: '#4a9eff' },
    { value: 'purple', label: '🟣 Sandbox', color: '#c678dd' },
    { value: 'none', label: '⚪ None', color: '#7d7d7d' },
  ];

  let name = '';
  let driver = 'mssql';
  let host = 'localhost';
  let port = 1433;
  let database = '';
  let username = '';
  let password = '';
  let useTls = false;
  let environment = 'none';
  let notes = '';
  let submitting = false;
  let error = '';

  $: if (editProfile && open) {
    name = editProfile.name;
    driver = editProfile.driver;
    host = editProfile.host;
    port = editProfile.port;
    database = editProfile.database ?? '';
    username = '';
    password = '';
    useTls = editProfile.use_tls ?? false;
    environment = editProfile.environment ?? 'none';
    notes = editProfile.notes ?? '';
  }

  function resetForm() {
    name = ''; driver = 'mssql'; host = 'localhost'; port = 1433;
    database = ''; username = ''; password = ''; useTls = false;
    environment = 'none'; notes = ''; submitting = false; error = '';
  }

  function handleClose() {
    resetForm();
    onClose();
  }

  async function handleSubmit() {
    if (!name.trim() || !host.trim()) {
      error = 'Name and host are required.';
      return;
    }

    submitting = true;
    error = '';

    const input: DatasourceInput = {
      name: name.trim(),
      driver,
      host: host.trim(),
      port: port || 0,
      database: database.trim() || null,
      username: username.trim() || null,
      password: password || null,
      use_tls: useTls,
      environment: environment || null,
      tags: null,
      notes: notes.trim() || null,
    };

    try {
      let profile: ConnectionProfile;
      if (editProfile) {
        profile = await updateDatasource(editProfile.id, input);
      } else {
        profile = await saveDatasource(input);
      }
      await loadDatasources();
      handleClose();
      // Auto-connect for new datasources
      if (!editProfile) {
        handleConnect(profile);
      }
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') handleClose();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-backdrop" on:click={handleClose} on:keydown={handleKeydown}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="modal" on:click={(e) => e.stopPropagation()} role="dialog" aria-label="Data Source">
      <div class="modal-header">
        <span>{editProfile ? 'Edit Data Source' : 'New Data Source'}</span>
        <button class="modal-close" on:click={handleClose}><X size="14" /></button>
      </div>

      <div class="modal-body">
        <div class="form-grid">
          <label class="field">
            <span>Name *</span>
            <input type="text" bind:value={name} placeholder="My Database" />
          </label>
          <label class="field">
            <span>Driver</span>
            <select bind:value={driver}>
              {#each DRIVERS as d}
                <option value={d.value}>{d.label}</option>
              {/each}
            </select>
          </label>
          <label class="field">
            <span>Host *</span>
            <input type="text" bind:value={host} placeholder="localhost" />
          </label>
          <label class="field">
            <span>Port</span>
            <input type="number" bind:value={port} placeholder="1433" />
          </label>
          <label class="field">
            <span>Database</span>
            <input type="text" bind:value={database} placeholder="master" />
          </label>
          <label class="field">
            <span>Username</span>
            <input type="text" bind:value={username} placeholder="sa" autocomplete="off" />
          </label>
          <label class="field">
            <span>Password</span>
            <input type="password" bind:value={password} placeholder="(stored encrypted)" autocomplete="off" />
          </label>
        </div>

        <div class="form-row">
          <label class="checkbox-field">
            <input type="checkbox" bind:checked={useTls} />
            <span>Use TLS/SSL</span>
          </label>
        </div>

        <label class="field">
          <span>Environment</span>
          <select bind:value={environment}>
            {#each ENVIRONMENTS as e}
              <option value={e.value}>{e.label}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span>Notes</span>
          <textarea bind:value={notes} rows="2" placeholder="Optional notes…" />
        </label>

        {#if error}
          <div class="form-error">{error}</div>
        {/if}
      </div>

      <div class="modal-footer">
        <button on:click={handleClose}>Cancel</button>
        <button class="primary" on:click={handleSubmit} disabled={submitting}>
          {submitting ? 'Saving…' : editProfile ? 'Update' : 'Save'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1001;
  }
  .modal {
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    width: 480px;
    max-width: 90vw;
    display: flex;
    flex-direction: column;
    max-height: 80vh;
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .modal-close {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 2px;
    cursor: pointer;
  }
  .modal-close:hover { color: var(--text); }
  .modal-body {
    padding: 16px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .field span {
    font-size: 11px;
    color: var(--text-muted);
  }
  .field input, .field select, .field textarea {
    padding: 6px 8px;
    font-size: 13px;
  }
  textarea {
    resize: vertical;
    min-height: 40px;
  }
  .form-row {
    display: flex;
    align-items: center;
  }
  .checkbox-field {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
  }
  .form-error {
    font-size: 12px;
    color: var(--error);
    padding: 6px 10px;
    background: rgba(188, 60, 60, 0.1);
    border-radius: var(--radius-sm);
    border: 1px solid rgba(188, 60, 60, 0.2);
  }
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
  }
</style>
