<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { runCommand, detectAvailableShells, type CommandOutput } from '$lib/tauri';
  import { pendingTerminalCommand, terminalShell, availableShells } from '$lib/stores';
  import { notify } from '$lib/toast';
  import { getSettings, setSetting } from '$lib/tauri';
  import { Trash2, Play, X, ChevronDown } from 'lucide-svelte';

  // XTerm imports
  import { Terminal } from 'xterm';
  import { FitAddon } from 'xterm-addon-fit';

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
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let isTerminalReady = false;
  let shellProcess: any = null;

  // Simple ANSI to HTML converter (fallback for non-xterm rendering)
  function ansiToHtml(text: string): string {
    // Map of ANSI colors to CSS colors
    const colors: Record<string, string> = {
      '0': '#ffffff',   // Reset (we'll handle separately)
      '1': '#ffffff',   // Bold
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
      '94': '#0000ff',  // Blue
      '95': '#ff00ff',  // Magenta
      '96': '#00ffff',  // Cyan
      '97': '#ffffff',  // White
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

  // Terminal functions
  async function initializeTerminal() {
    if (!termEl || isTerminalReady) return;

    try {
      // Create terminal instance
      terminal = new Terminal({
        cursorBlink: true,
        fontSize: 14,
        fontFamily: "'Fira Code', 'JetBrains Mono', 'Menlo', 'Consolas', 'Monaco', monospace",
        theme: {
          background: '#1e1e1e',
          foreground: '#cccccc',
          cursor: '#ffffff',
          black: '#000000',
          red: '#ff0000',
          green: '#00ff00',
          yellow: '#ffff00',
          blue: '#0000ff',
          magenta: '#ff00ff',
          cyan: '#00ffff',
          white: '#ffffff',
          brightBlack: '#808080',
          brightRed: '#ff0000',
          brightGreen: '#00ff00',
          brightYellow: '#ffff00',
          brightBlue: '#0000ff',
          brightMagenta: '#ff00ff',
          brightCyan: '#00ffff',
          brightWhite: '#ffffff'
        }
      });

      // Add fit addon to resize with container
      fitAddon = new FitAddon();
      terminal.loadAddon(fitAddon);

      // Mount terminal
      terminal.open(termEl);
      fitAddon.fit();

      // Handle resize
      window.addEventListener('resize', () => {
        fitAddon?.fit();
      });

      // Start shell
      await startShell();

      isTerminalReady = true;
    } catch (error) {
      console.error('Failed to initialize terminal:', error);
      notify(`Failed to initialize terminal: ${error}`, 'error');
    }
  }

  async function startShell() {
    if (!terminal) return;

    try {
      // Get shell to use
      const shell = $terminalShell || (await getDefaultShell());

      // Clear current output
      terminal.writeln(`\x1b[32m$ ${shell}\x1b[0m`); // Green prompt

      // TODO: In a real implementation, we would use Tauri's process API
      // to create a pseudo-terminal and hook it up to xterm
      // For now, we'll simulate with our existing command system
      terminal.writeln('Note: Full PTY integration coming soon. Using command mode for now.\r\n');

      // Listen for data from terminal (this would be replaced with real PTY)
      terminal.onData(handleTerminalData);
    } catch (error) {
      console.error('Failed to start shell:', error);
      notify(`Failed to start shell: ${error}`, 'error');
    }
  }

  function handleTerminalData(data: string) {
    // This would normally be handled by the PTY
    // For now, we'll just echo it back
    if (terminal) {
      terminal.write(data);
    }
  }

  async function getDefaultShell(): Promise<string> {
    try {
      const settings = await getSettings();
      const saved = (settings as any).terminalShell as string | undefined;
      if (saved) return saved;

      const shells = await detectAvailableShells();
      // Prefer user's shell, then bash/zsh/fish, then sh
      const envShell = process.env.SHELL || '';
      if (shells[envShell.split('/').pop() || '']) {
        return envShell;
      }

      // Return first available shell in preference order
      const prefs = ['fish', 'zsh', 'bash', 'sh'];
      for (const shell of prefs) {
        if (shells[shell]) {
          return shells[shell];
        }
      }

      // Fallback to first available
      const first = Object.values(shells)[0];
      return first || '/bin/sh';
    } catch (error) {
      console.error('Failed to get default shell:', error);
      return '/bin/sh';
    }
  }

  function executeCommand(command: string) {
    if (!command.trim()) return;

    if (terminal) {
      // Write command to terminal
      terminal.write(`\r\n\x1b[32m$ ${command}\x1b[0m\r\n`);

      // Execute via our existing command system
      runCommand(command, $terminalShell || undefined).then(result => {
        const output = result.stdout || '';
        const error = result.stderr || '';

        // Write output to terminal
        if (output) terminal.write(output);
        if (error) {
          // Write error in red
          terminal.write(`\x1b[31m${error}\x1b[0m`);
        }

        // Show exit code if non-zero
        if (result.exit_code !== 0) {
          terminal.write(`\x1b[33mProcess exited with code ${result.exit_code}\x1b[0m\r\n`);
        }

        // New prompt
        terminal.write('\r\n\x1b[32m$ \x1b[0m');
      }).catch(error => {
        terminal.write(`\x1b[31mError: ${error}\x1b[0m\r\n`);
        terminal.write('\r\n\x1b[32m$ \x1b[0m');
      });
    } else {
      // Fallback to old method if terminal not ready
      executeLegacy(command);
    }
  }

  function executeLegacy(cmd: string) {
    // Existing legacy implementation
    const trimmed = cmd.trim();
    if (!trimmed) return;

    // Handle clear natively
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

    // Execute command
    (async () => {
      let output: CommandOutput;
      try {
        output = await runCommand(trimmed, $terminalShell || undefined);
      } catch (e) {
        output = { stdout: '', stderr: `Failed to run: ${e}`, exit_code: -1 };
      }

      entries = entries.map(e =>
        e.id === id ? { ...e, output, running: false } : e
      );
      scrollBottom();
    })();
  }

  function scrollBottom() {
    if (termEl) requestAnimationFrame(() => { termEl.scrollTop = termEl.scrollHeight; });
  }

  function clearOutput() {
    entries = [];
    if (terminal) {
      terminal.clear();
      terminal.write('\r\n\x1b[32m$ \x1b[0m');
    }
  }

  function removeEntry(id: number) {
    entries = entries.filter(e => e.id !== id);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      execute(cmdInput);
    }
    // Let xterm handle other keys when ready
    if (terminal && isTerminalReady && e.key !== 'Enter') {
      // Don't prevent default for most keys when terminal is active
      return;
    }
    // Prevent form submission for Enter in input (when not using terminal)
    if (e.key === 'Enter' && !isTerminalReady) {
      e.preventDefault();
    }
  }

  function runAgain(cmd: string) {
    cmdInput = cmd;
    execute(cmd);
  }

  function fmtTime(ts: number): string {
    return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  }

  // Shell selection
  async function selectShell(shell: string) {
    terminalShell.set(shell);
    try {
      await setSetting('terminalShell', shell);
      // Restart terminal with new shell
      if (terminal) {
        terminal.dispose();
        terminal = null;
        fitAddon = null;
        isTerminalReady = false;
        await new Promise(resolve => setTimeout(resolve, 100)); // Brief delay
        await initializeTerminal();
      }
    } catch (error) {
      notify(`Failed to set shell: ${error}`, 'error');
    }
  }

  async function setCustomShell() {
    try {
      const { open: dialogOpen } = await import('@tauri-apps/plugin-dialog');
      const selected = await dialogOpen({
        multiple: false,
        directory: false,
      });
      if (selected) {
        await selectShell(selected);
        notify(`Terminal shell set to custom path`, 'success');
      }
    } catch (e) {
      notify(`Failed to select custom shell: ${e}`, 'error');
    }
  }

  let shellMenuOpen = false;

  function toggleShellMenu() {
    shellMenuOpen = !shellMenuOpen;
  }

  // Lifecycle
  onMount(async () => {
    // Load available shells for dropdown
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

    // Initialize terminal
    await initializeTerminal();

    // Subscribe to pending commands from LSP
    const unsub = pendingTerminalCommand.subscribe((cmd) => {
      if (cmd) {
        executeCommand(cmd);
        pendingTerminalCommand.set(null);
      }
    });

    // Cleanup on destroy
    return () => {
      if (unsub) unsub();
      if (terminal) {
        terminal.dispose();
      }
    };
  });

  // Destroy cleanup
  onDestroy(() => {
    if (terminal) {
      terminal.dispose();
    }
  });
</script>

<!-- Terminal Container -->
<div class="terminal-panel">
  <div class="term-toolbar">
    <span class="term-label">TERMINAL</span>
    <!-- Shell picker -->
    <div class="term-shell-picker" on:click={toggleShellMenu} on:keydown role="button" tabindex="0" title="Select shell">
      <span class="term-shell-name">
        {#if $terminalShell}
          {$terminalShell.split('/').pop() || $terminalShell}
        {:else}
          default
        {/if}
      </span>
      <ChevronDown size="10" />
    </div>

    {#if shellMenuOpen}
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

  <!-- Terminal Output -->
  <div class="term-output" bind:this={termEl}>
    {#if !isTerminalReady}
      <div class="term-loading">Initializing terminal...</div>
    {:else}
      <!-- xterm.js will render here -->
    {/if}
  </div>

  <!-- Input Bar -->
  <div class="term-input-bar">
    <span class="term-prompt">$</span>
    <input
      type="text"
      class="term-input"
      bind:value={cmdInput}
      on:keydown={handleKeydown}
      placeholder="Type a command…"
      autofocus
      {#if isTerminalDisabled}disabled{/if}
    />
    <button class="term-run-btn" on:click={() => executeCommand(cmdInput)} disabled={!cmdInput.trim()}><Play size="10" /></button>
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
    display: flex;
    flex-direction: column;
  }

  .term-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg-elev);
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
    padding: 0;
    min-height: 0;
    position: relative;
    background: #000000; /* Terminal background */
    overflow: hidden;
  }

  .term-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-faint);
    font-size: 11px;
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
    background: #000000;
    color: #ffffff;
    caret-color: #ffffff;
  }

  .term-input::placeholder {
    color: #808080;
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

  .term-run-btn:disabled { color: #808080; cursor: default; }
  .term-run-btn:hover:not(:disabled) { color: var(--accent-emphasis); }

  /* Shell menu */
  .term-shell-menu {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    z-index: 1000;
    width: 200px;
    max-height: 250px;
    overflow-y: auto;
  }

  .term-shell-opt {
    width: 100%;
    text-align: left;
    padding: 8px 12px;
    font-size: 12px;
    color: var(--text);
    background: transparent;
    border: none;
    cursor: pointer;
  }

  .term-shell-opt:hover {
    background: var(--bg-hover);
  }

  .term-shell-opt.active {
    background: var(--bg-hover);
    border-left: 3px solid var(--accent);
  }

  .term-shell-path {
    font-size: 10px;
    color: var(--text-faint);
    display: block;
    margin-top: 2px;
    font-family: var(--font-mono);
  }

  /* Resize handle */
  .term-resize-handle {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 4px;
    background: repeating-linear-gradient(
      45deg,
      transparent,
      transparent 2px,
      rgba(255,255,255,0.1) 2px,
      rgba(255,255,255,0.1) 4px
    );
    cursor: ns-resize;
    z-index: 10;
  }
</style>