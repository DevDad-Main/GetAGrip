<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { detectAvailableShells } from '$lib/tauri';
  import { pendingTerminalCommand, terminalShell, availableShells } from '$lib/stores';
  import { notify } from '$lib/toast';
  import { getSettings, setSetting } from '$lib/tauri';

  import { Terminal } from 'xterm';
  import 'xterm/css/xterm.css';
  import { FitAddon } from 'xterm-addon-fit';
  import { startPty as startPtyTauri, stopPty, ptyInput, ptyResize, readPtyOutput } from '$lib/tauri';
  import { ChevronDown, Trash2 } from 'lucide-svelte';

  let termEl: HTMLDivElement;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let isTerminalReady = false;
  let ptyReady = false;
  let shellMenuOpen = false;
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  function fitWithGuard() {
    if (!fitAddon || !terminal || !termEl) return;
    const h = termEl.clientHeight;
    if (h < 30) return;
    try {
      fitAddon.fit();
      if (ptyReady && terminal.cols > 1 && terminal.rows > 1) {
        ptyResize(terminal.cols, terminal.rows);
      }
    } catch (e) {
      console.error('fit failed:', e);
    }
  }

  async function pollPtyOutput() {
    if (!terminal || !ptyReady) return;
    try {
      const data = await readPtyOutput();
      if (data && data.length > 0) {
        terminal.write(data);
      }
    } catch (e) {
      console.error('pollPtyOutput failed:', e);
    }
  }

  async function initializeTerminal() {
    if (!termEl || isTerminalReady) return;

    try {
      terminal = new Terminal({
        cursorBlink: true,
        fontSize: 14,
        fontFamily: "'JetBrainsMono Nerd Font', 'Fira Code', 'JetBrains Mono', 'Menlo', 'Consolas', 'Monaco', monospace",
        theme: {
          background: '#1e1e1e',
          foreground: '#cccccc',
          cursor: '#ffffff',
          selectionBackground: '#264f78',
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
        },
        allowTransparency: false,
        macOptionIsMeta: true,
      });

      fitAddon = new FitAddon();
      terminal.loadAddon(fitAddon);
      terminal.open(termEl);

      resizeObserver = new ResizeObserver(() => {
        fitWithGuard();
      });
      resizeObserver.observe(termEl);
      setTimeout(() => fitWithGuard(), 50);

      terminal.onData((data) => {
        if (ptyReady) {
          ptyInput(data);
        }
      });

      // Start polling for PTY output
      pollTimer = setInterval(pollPtyOutput, 30);

      await startPty();
      isTerminalReady = true;
      terminal.focus();
    } catch (error) {
      console.error('Failed to initialize terminal:', error);
      notify(`Failed to initialize terminal: ${error}`, 'error');
    }
  }

  async function startPty() {
    if (!terminal) return;

    try {
      const shell = $terminalShell || (await getDefaultShell());
      await startPtyTauri(shell);
      ptyReady = true;
    } catch (error) {
      console.error('Failed to start PTY:', error);
      notify(`Failed to start PTY: ${error}`, 'error');
    }
  }

  async function getDefaultShell(): Promise<string> {
    try {
      const settings = await getSettings();
      const saved = (settings as any).terminalShell as string | undefined;
      if (saved) return saved;

      const shells = await detectAvailableShells();
      const prefs = ['fish', 'zsh', 'bash', 'sh'];
      for (const shell of prefs) {
        if (shells[shell]) {
          return shells[shell];
        }
      }

      const first = Object.values(shells)[0];
      return first || '/bin/sh';
    } catch (error) {
      console.error('Failed to get default shell:', error);
      return '/bin/sh';
    }
  }

  function clearOutput() {
    if (terminal) {
      terminal.clear();
    }
  }

  async function selectShell(shell: string) {
    terminalShell.set(shell);
    try {
      await setSetting('terminalShell', shell);
      if (terminal) {
        stopPty().catch(() => {});
        if (pollTimer) {
          clearInterval(pollTimer);
          pollTimer = null;
        }
        terminal.dispose();
        terminal = null;
        fitAddon = null;
        isTerminalReady = false;
        ptyReady = false;
        await new Promise(resolve => setTimeout(resolve, 100));
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

  function toggleShellMenu() {
    shellMenuOpen = !shellMenuOpen;
  }

  onMount(() => {
    (async () => {
      try {
        const shells = await detectAvailableShells();
        availableShells.set(shells);
      } catch { /* ignore */ }

      try {
        const settings = await getSettings();
        const saved = (settings as any).terminalShell as string | undefined;
        if (saved) terminalShell.set(saved);
      } catch { /* ignore */ }

      await initializeTerminal();
    })();

    const unsub = pendingTerminalCommand.subscribe((cmd) => {
      if (cmd) {
        if (ptyReady && terminal) {
          ptyInput(cmd + '\n');
        }
        pendingTerminalCommand.set(null);
      }
    });

    return () => {
      if (unsub) unsub();
    };
  });

  onDestroy(() => {
    if (pollTimer) clearInterval(pollTimer);
    if (resizeObserver) resizeObserver.disconnect();
    if (terminal) {
      terminal.dispose();
      terminal = null;
    }
    stopPty().catch(() => {});
  });
</script>

<div class="terminal-panel">
  <div class="term-toolbar">
    <span class="term-label">TERMINAL</span>
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
    <button class="term-btn" on:click={clearOutput} title="Clear"><Trash2 size="10" /></button>
  </div>

  <div class="term-output" bind:this={termEl}>
    {#if !isTerminalReady}
      <div class="term-loading">Initializing terminal...</div>
    {/if}
  </div>
</div>

<style>
  .terminal-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1e1e1e;
    overflow: hidden;
  }

  .term-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 10px;
    border-bottom: 1px solid #333;
    flex-shrink: 0;
    background: #252526;
    position: relative;
    z-index: 10;
  }

  .term-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 1px;
    color: #888;
  }

  .term-toolbar-spacer {
    flex: 1;
  }

  .term-shell-picker {
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
    color: #aaa;
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 3px;
  }

  .term-shell-picker:hover {
    background: #333;
    color: #ccc;
  }

  .term-shell-name {
    max-width: 80px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .term-shell-menu {
    position: absolute;
    top: 100%;
    left: 80px;
    margin-top: 2px;
    background: #252526;
    border: 1px solid #444;
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
    z-index: 1000;
    width: 220px;
    max-height: 260px;
    overflow-y: auto;
  }

  .term-shell-opt {
    width: 100%;
    text-align: left;
    padding: 6px 12px;
    font-size: 12px;
    color: #ccc;
    background: transparent;
    border: none;
    cursor: pointer;
    font-family: inherit;
  }

  .term-shell-opt:hover {
    background: #333;
  }

  .term-shell-opt.active {
    background: #333;
    border-left: 3px solid #0078d4;
  }

  .term-shell-path {
    font-size: 10px;
    color: #666;
    display: block;
    margin-top: 1px;
    font-family: 'Fira Code', 'JetBrains Mono', monospace;
  }

  .term-btn {
    border: none;
    background: transparent;
    color: #888;
    cursor: pointer;
    padding: 2px;
    display: flex;
  }

  .term-btn:hover { color: #ccc; }

  .term-output {
    flex: 1;
    min-height: 100px;
    position: relative;
    overflow: hidden;
  }

  .term-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #666;
    font-size: 11px;
  }
</style>