<script lang="ts">
  import { onMount } from 'svelte';
  import { runCommand, type CommandOutput } from '$lib/tauri';
  import { Trash2, Play, X } from 'lucide-svelte';

  interface TermEntry {
    id: number;
    command: string;
    output: CommandOutput | null;
    running: boolean;
    timestamp: number;
  }

  let entries: TermEntry[] = [];
  let cmdInput = '';
  let entryId = 0;
  let termEl: HTMLDivElement;

  function scrollBottom() {
    if (termEl) requestAnimationFrame(() => { termEl.scrollTop = termEl.scrollHeight; });
  }

  async function execute(cmd: string) {
    const trimmed = cmd.trim();
    if (!trimmed) return;

    const id = ++entryId;
    const entry: TermEntry = {
      id,
      command: trimmed,
      output: null,
      running: true,
      timestamp: Date.now(),
    };
    entries = [...entries, entry];
    cmdInput = '';
    scrollBottom();

    // Parse the command into program + args
    const parts = parseCommand(trimmed);
    let output: CommandOutput;
    if (parts) {
      output = await runCommand(parts.program, parts.args);
    } else {
      output = { stdout: '', stderr: `Unable to parse command: ${trimmed}`, exit_code: -1 };
    }

    entries = entries.map((e) =>
      e.id === id ? { ...e, output, running: false } : e,
    );
    scrollBottom();
  }

  function parseCommand(cmd: string): { program: string; args: string[] } | null {
    const trimmed = cmd.trim();
    if (!trimmed) return null;

    // Handle quoted strings
    const parts: string[] = [];
    let current = '';
    let inQuote = false;
    let quoteChar = '';

    for (const ch of trimmed) {
      if (inQuote) {
        if (ch === quoteChar) {
          inQuote = false;
        } else {
          current += ch;
        }
      } else if (ch === '"' || ch === "'") {
        inQuote = true;
        quoteChar = ch;
      } else if (ch === ' ') {
        if (current) { parts.push(current); current = ''; }
      } else {
        current += ch;
      }
    }
    if (current) parts.push(current);

    if (parts.length === 0) return null;
    return { program: parts[0], args: parts.slice(1) };
  }

  function clearOutput() {
    entries = [];
  }

  function removeEntry(id: number) {
    entries = entries.filter((e) => e.id !== id);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      execute(cmdInput);
    }
  }

  function runAgain(cmd: string) {
    cmdInput = cmd;
    execute(cmd);
  }

  function fmtTime(ts: number): string {
    return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  }

  // Listen for commands dispatched from LSP settings modal
  function handleTerminalRun(e: CustomEvent<string>) {
    execute(e.detail);
  }

  onMount(() => {
    window.addEventListener('terminal-run', handleTerminalRun as EventListener);
    return () => window.removeEventListener('terminal-run', handleTerminalRun as EventListener);
  });
</script>

<div class="terminal-panel">
  <div class="term-toolbar">
    <span class="term-label">TERMINAL</span>
    <button class="term-btn" on:click={clearOutput} title="Clear all output"><Trash2 size="10" /></button>
  </div>
  <div class="term-output" bind:this={termEl}>
    {#if entries.length === 0}
      <div class="term-empty">Type a command and press Enter to run it.</div>
    {:else}
      {#each entries as entry (entry.id)}
        <div class="term-entry" class:term-error={entry.output && entry.output.exit_code !== 0}>
          <div class="term-cmd-line">
            <span class="term-prompt">$</span>
            <code class="term-cmd">{entry.command}</code>
            <span class="term-time">{fmtTime(entry.timestamp)}</span>
            <button class="term-re-run" on:click={() => runAgain(entry.command)} title="Run again"><Play size="10" /></button>
            <button class="term-close-entry" on:click={() => removeEntry(entry.id)} title="Remove"><X size="10" /></button>
          </div>
          {#if entry.running}
            <div class="term-running">
              <span class="term-spinner"></span> Running…
            </div>
          {:else if entry.output}
            <div class="term-output-block">
              {#if entry.output.stdout}
                <pre class="term-stdout">{entry.output.stdout}</pre>
              {/if}
              {#if entry.output.stderr}
                <pre class="term-stderr">{entry.output.stderr}</pre>
              {/if}
              <div class="term-exit-code">
                Process exited with code {entry.output.exit_code}
              </div>
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
  <div class="term-input-bar">
    <span class="term-prompt">$</span>
    <input
      type="text"
      class="term-input"
      bind:value={cmdInput}
      on:keydown={handleKeydown}
      placeholder="Type a command…"
      autofocus
    />
    <button class="term-run-btn" on:click={() => execute(cmdInput)} disabled={!cmdInput.trim()}><Play size="10" /></button>
  </div>
</div>

<style>
  .terminal-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg);
    font-family: var(--font-mono);
    font-size: 12px;
    overflow: hidden;
  }
  .term-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .term-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 1px;
    color: var(--text-muted);
  }
  .term-btn {
    margin-left: auto;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px;
    display: flex;
  }
  .term-btn:hover { color: var(--text); }
  .term-output {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
    min-height: 0;
  }
  .term-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-faint);
    font-size: 11px;
  }
  .term-entry {
    padding: 4px 10px;
    border-bottom: 1px solid var(--border);
  }
  .term-entry.term-error {
    background: rgba(188, 60, 60, 0.05);
  }
  .term-cmd-line {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text);
    font-size: 11px;
  }
  .term-prompt {
    color: var(--success);
    font-weight: 700;
    flex-shrink: 0;
  }
  .term-cmd {
    color: var(--accent);
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .term-time {
    color: var(--text-faint);
    font-size: 9px;
    font-family: var(--font-sans);
    flex-shrink: 0;
  }
  .term-re-run, .term-close-entry {
    border: none;
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
    padding: 1px;
    display: flex;
    opacity: 0;
    transition: opacity 0.1s;
  }
  .term-entry:hover .term-re-run,
  .term-entry:hover .term-close-entry {
    opacity: 1;
  }
  .term-re-run:hover { color: var(--accent); }
  .term-close-entry:hover { color: var(--text); }
  .term-running {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 0 4px 14px;
    color: var(--text-muted);
    font-size: 10px;
  }
  .term-spinner {
    width: 8px;
    height: 8px;
    border: 2px solid var(--accent);
    border-top-color: transparent;
    border-radius: 50%;
    animation: term-spin 0.8s linear infinite;
  }
  @keyframes term-spin {
    to { transform: rotate(360deg); }
  }
  .term-output-block {
    padding: 2px 0 2px 14px;
  }
  .term-stdout {
    margin: 0;
    color: var(--text);
    font-size: 10px;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .term-stderr {
    margin: 0;
    color: var(--error);
    font-size: 10px;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .term-exit-code {
    font-size: 9px;
    color: var(--text-faint);
    margin-top: 2px;
  }
  .term-input-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg-elev);
  }
  .term-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 12px;
    outline: none;
    padding: 2px 0;
  }
  .term-input::placeholder {
    color: var(--text-faint);
    font-family: var(--font-sans);
    font-size: 11px;
  }
  .term-run-btn {
    border: none;
    background: transparent;
    color: var(--accent);
    cursor: pointer;
    padding: 2px;
    display: flex;
  }
  .term-run-btn:disabled { color: var(--text-faint); cursor: default; }
  .term-run-btn:hover:not(:disabled) { color: var(--accent-emphasis); }
</style>
