<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { runCommand, detectAvailableShells, type CommandOutput } from '$lib/tauri';
  import { pendingTerminalCommand, terminalShell, availableShells } from '$lib/stores';
  import { notify } from '$lib/toast';
  import { getSettings, setSetting } from '$lib/tauri';
  import { Trash2, Play, X, ChevronDown } from 'lucide-svelte';

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

    // Handle clear natively — discard all output instead of running the binary
    if (trimmed === 'clear' || trimmed === 'cls') {
      entries = [];
      cmdInput = '';
      return;
    }

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

    let output: CommandOutput;
    try {
      output = await runCommand(trimmed, $terminalShell || undefined);
    } catch (e) {
      output = { stdout: '', stderr: `Failed to run: ${e}`, exit_code: -1 };
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
        if (ch === 'Enter') {
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

  // Simple ANSI escape code to HTML converter
  function ansiToHtml(text: string): string {
    // Map of ANSI colors to CSS colors
    const colors: Record<string, string> = {
      '0': '#ffffff',   // Reset (we'll handle separately)
      '1': '#ffffff',   // Bold (we'll make text bold)
      '2': '#ffffff',   // Dim
      '3': '#ffffff',   // Italic
      '4': '#ffffff',   // Underline
      '5': '#ffffff',   // Blink
      '6': '#ffffff',   // Inverse
      '7': '#ffffff',   // Hidden
      '8': '#ffffff',   // Strikethrough
      '9': '#ffffff',   //
      '30': '#000000',  // Black
      '31': '#ff0000',  // Red
      '32': '#00ff00',  // Green
      '33': '#ffff00',  // Yellow
      '34': '#0000ff',  // Blue
      '35': '#ff00ff',  // Magenta
      '36': '#00ffff',  // Cyan
      '37': '#ffffff',  // White
      '90': '#808080',  // Bright Black
      '91': '#ff0000',  // Bright Red
      '92': '#00ff00',  // Bright Green
      '93': '#ffff00',  // Bright Yellow
      '94': '#0000ff',  // Bright Blue
      '95': '#ff00ff',  // Bright Magenta
      '96': '#00ffff',  // Bright Cyan
      '97': '#ffffff',  // Bright White
    };

    // Process ANSI escape codes
    let result = '';
    let pos = 0;
    let match;

    // Regex to find ANSI escape codes: \x1b[...m
    const ansiRegex = /\x1b\[([0-9;]*?)m/g;

    // We'll build the result by processing chunks
    let lastIndex = 0;
    let matchResult;

    while ((matchResult = ansiRegex.exec(text)) !== null) {
      // Add text before the escape code
      result += text.slice(lastIndex, matchResult.index);

      // Process the escape code
      const params = matchResult[1].split(';').filter(p => p !== '');

      // Apply styles (simplified - just handle colors for now)
      let style = '';
      let hasBold = false;
      let hasUnderline = false;

      for (const param of params) {
        if (param === '1') hasBold = true;
        if (param === '4') hasUnderline = true;
        if (colors[param]) {
          // Set color
          style += `color: ${colors[param]}; `;
        }
      }

      if (hasBold) style += 'font-weight: bold; ';
      if (hasUnderline) style += 'text-decoration: underline; ';

      // Add opening span with styles if any
      if (style) {
        result += `<span style="${style.trim()}">`;
      } else {
        // Reset styles
        result += '</span>';
      }

      lastIndex = ansiRegex.lastIndex;
    }

    // Add remaining text
    result += text.slice(lastIndex);

    // Close any open spans (simplified approach)
    // In a real implementation, we'd need to properly manage the span stack
    // For now, we'll just wrap everything in a span and reset at end

    return result;
  }

  // Watch for commands from LSP settings modal

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

  // Watch for commands from LSP settings modal
  let unsub: () => void;

  async function selectShell(shell: string) {
    terminalShell.set(shell);
    try {
      await setSetting('terminalShell', shell);
    } catch { /* best-effort */ }
  }

  async function setCustomShell() {
    try {
      const { open: dialogOpen } = await import('@tauri-apps/plugin-dialog');
      const selected = await dialogOpen({
        multiple: false,
        directory: false,
      });
      if (selected) {
        terminalShell.set(selected);
        await setSetting('terminalShell', selected);
        notify('Terminal shell set to custom path', 'success');
      }
    } catch (e) {
      notify(`Failed to select custom shell: ${e}`, 'error');
    }
  }

  let shellMenuOpen = false;

  function toggleShellMenu() {
    shellMenuOpen = !shellMenuOpen;
  }

  onMount(async () => {
    // Load available shells
    try {
      const shells = await detectAvailableShells();
      availableShells.set(shells);
    } catch { /* ignore */ }

    // Load saved shell preference
    try {
      const settings = await getSettings();
      const saved = (settings as any).terminalShell as string | undefined;
      if (saved) terminalShell.set(saved);
    } catch { /* ignore */ }

    unsub = pendingTerminalCommand.subscribe((cmd) => {
      if (cmd) {
        execute(cmd);
        pendingTerminalCommand.set(null);
      }
    });
  });

  onDestroy(() => {
    if (unsub) unsub();
  });
</script>

<div class="terminal-panel">
  <div class="term-toolbar">
    <span class="term-label">TERMINAL</span>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="term-shell-picker" on:click={toggleShellMenu} on:keydown role="button" tabindex="0" title="Select shell">
      <span class="term-shell-name">{$terminalShell || 'default'}</span>
      <ChevronDown size="10" />
    </div>
    {#if shellMenuOpen}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div class="term-shell-menu" role="menu">
        <button class="term-shell-opt" class:active={!$terminalShell} on:click={() => { selectShell(''); shellMenuOpen = false; }} role="menuitem">
          default ({$terminalShell || 'auto'})
        </button>
        {#each Object.entries($availableShells) as [name, path]}
          <button
            class="term-shell-opt"
            class:active={$terminalShell === path}
            on:click={() => { selectShell(path); shellMenuOpen = false; }}
            role="menuitem"
          >{name} <span class="term-shell-path">{path}</span></button>
        {/each}
        <button class="term-shell-opt" on:click={() => { setCustomShell(); shellMenuOpen = false; }} role="menuitem">
          Set custom path...
        </button>
      </div>
    {/if}
    <div class="term-toolbar-spacer"></div>
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
                {#if entry.output.exit_code !== 0}
                  <div class="term-exit-code">
                    Process exited with code {entry.output.exit_code}
                  </div>
                {/if}
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