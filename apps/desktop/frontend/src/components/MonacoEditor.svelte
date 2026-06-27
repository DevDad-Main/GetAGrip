<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as monaco from 'monaco-editor';
  import { executeQueryV2, requestCompletion, type QueryResultDto, type CompletionItem } from '$lib/tauri';
  import {
    resultSets, activeResultSetId, statusText,
    nextResultSetId, resultsPanelHeight, activeTheme, type ResultSet,
  } from '$lib/stores';
  import CustomSuggestWidget from './CustomSuggestWidget.svelte';

  export let sql = '';
  export let profileId: string | null = null;
  export let tabId = '';
  export let onSqlChange: (sql: string) => void = () => {};
  export let onReady: (runQuery: () => void) => void = () => {};

  import { THEMES, type ThemeDef } from '$lib/themes';

  const SQL_KEYWORDS = ['SELECT','FROM','WHERE','JOIN','LEFT JOIN','INNER JOIN','ON','AND','OR','NOT','IN','EXISTS','BETWEEN','LIKE','IS','NULL','AS','INSERT INTO','VALUES','UPDATE','SET','DELETE FROM','CREATE TABLE','ALTER TABLE','DROP TABLE','ORDER BY','GROUP BY','HAVING','LIMIT','OFFSET','UNION','DISTINCT','TOP','CASE','WHEN','THEN','ELSE','END','ASC','DESC','BEGIN','COMMIT','ROLLBACK'];

  let editor: monaco.editor.IStandaloneCodeEditor | null = null;
  let containerEl: HTMLDivElement | undefined;

  // Custom suggest widget state
  let suggestVisible = false;
  let suggestItems: CompletionItem[] = [];
  let suggestActive = 0;
  let suggestPos = { top: 0, left: 0 };
  let lastCompletionPos: monaco.Position | null = null;
  let completionWordStartCol = 0;

  function defineMonacoTheme(t: ThemeDef) {
    const d = t.isDark;
    const ebg = t.bg;
    const efg = t.fg;
    const elh = adj(ebg, d ? 4 : -4);
    const esel = adj(ebg, d ? 16 : -10);
    const ecur = d ? '#aaaaaa' : '#333333';
    const eln = adj(efg, d ? -90 : 90);
    const elna = adj(efg, d ? -10 : 10);
    const eig = adj(ebg, d ? 20 : -15);
    const eiga = adj(ebg, d ? 40 : -25);
    const ewbg = adj(ebg, d ? 12 : -8);
    const ewb = adj(ebg, d ? 30 : -20);
    const eswfg = d ? '#bbbbbb' : '#333333';
    const eswsel = adj(ebg, d ? 16 : -10);
    const eswhl = t.accent;

    monaco.editor.defineTheme(t.value, {
      base: d ? 'vs-dark' : 'vs',
      inherit: true,
      rules: [
        { token: 'keyword', foreground: d ? 'cc7832' : '0000ff', fontStyle: 'bold' },
        { token: 'type', foreground: efg },
        { token: 'string', foreground: d ? '6a8759' : '008000' },
        { token: 'number', foreground: d ? '6897bb' : '098658' },
        { token: 'comment', foreground: d ? '808080' : '808080', fontStyle: 'italic' },
        { token: 'operator', foreground: d ? 'ffc66d' : '000000' },
        { token: 'identifier', foreground: efg },
      ],
      colors: {
        'editor.background': ebg,
        'editor.foreground': efg,
        'editor.lineHighlightBackground': elh,
        'editor.selectionBackground': esel,
        'editorCursor.foreground': ecur,
        'editorLineNumber.foreground': eln,
        'editorLineNumber.activeForeground': elna,
        'editorIndentGuide.background': eig,
        'editorIndentGuide.activeBackground': eiga,
        'editorWidget.background': ewbg,
        'editorWidget.border': ewb,
        'editorSuggestWidget.background': ewbg,
        'editorSuggestWidget.border': ewb,
        'editorSuggestWidget.foreground': eswfg,
        'editorSuggestWidget.selectedBackground': eswsel,
        'editorSuggestWidget.highlightForeground': eswhl,
        'minimap.background': ebg,
        'minimapSlider.background': ewb + '80',
        'minimapSlider.hoverBackground': ewb,
        'minimapSlider.activeBackground': ewb,
        'scrollbar.shadow': '#00000000',
        'scrollbarSlider.background': ewb + '80',
        'scrollbarSlider.hoverBackground': ewb,
        'scrollbarSlider.activeBackground': ewb,
      },
    });
  }

  function adj(hex: string, amount: number): string {
    const num = parseInt(hex.slice(1), 16);
    const r = Math.min(255, Math.max(0, ((num >> 16) & 0xff) + amount));
    const g = Math.min(255, Math.max(0, ((num >> 8) & 0xff) + amount));
    const b = Math.min(255, Math.max(0, (num & 0xff) + amount));
    return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, '0')}`;
  }

  THEMES.forEach((t) => defineMonacoTheme(t));

  $: if (editor && sql !== editor.getValue()) {
    editor.setValue(sql);
  }

  $: if (editor && $activeTheme) {
    monaco.editor.setTheme($activeTheme);
  }

  // ── suggest widget position helper ────────────────────────────────────

  function updateSuggestPosition(pos: monaco.Position) {
    if (!editor || !containerEl) return;
    const coords = editor.getScrolledVisiblePosition(pos);
    const containerRect = containerEl.getBoundingClientRect();
    suggestPos = {
      top: containerRect.top + coords.top + 20,
      left: containerRect.left + coords.left,
    };
  }

  // ── suggest widget lifecycle ──────────────────────────────────────────

  function showSuggest(items: CompletionItem[], pos: monaco.Position) {
    if (items.length === 0) {
      hideSuggest();
      return;
    }
    const model = editor?.getModel();
    const word = model?.getWordUntilPosition(pos);
    completionWordStartCol = word?.startColumn ?? pos.column;

    suggestItems = items;
    suggestActive = 0;
    updateSuggestPosition(pos);
    suggestVisible = true;
    lastCompletionPos = pos;
  }

  function hideSuggest() {
    suggestVisible = false;
    suggestItems = [];
    suggestActive = 0;
    lastCompletionPos = null;
    completionWordStartCol = 0;
  }

  async function triggerCompletion(pos?: monaco.Position) {
    if (!editor) return;
    const model = editor.getModel();
    if (!model) return;
    const position = pos ?? editor.getPosition();
    if (!position) return;

    if (!profileId) {
      const fallback: CompletionItem[] = SQL_KEYWORDS.map((kw) => ({
        label: kw,
        kind: 'keyword',
        detail: '',
        score: 50,
      }));
      showSuggest(fallback, position);
      return;
    }

    try {
      const text = model.getValue();
      const resp = await requestCompletion({
        connection_id: profileId,
        sql: text,
        cursor_line: position.lineNumber,
        cursor_column: position.column,
      });
      showSuggest(resp.suggestions, position);
    } catch {
      hideSuggest();
    }
  }

  function handleSuggestSelect(item: CompletionItem) {
    if (!editor || !lastCompletionPos) return;
    const insertText = item.insert_text ?? item.label;
    const word = editor.getModel()?.getWordUntilPosition(lastCompletionPos);
    if (word) {
      const range = {
        startLineNumber: lastCompletionPos.lineNumber,
        endLineNumber: lastCompletionPos.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn,
      };
      editor.executeEdits('completion', [{ range, text: insertText }]);
    } else {
      editor.executeEdits('completion', [{
        range: {
          startLineNumber: lastCompletionPos.lineNumber,
          endLineNumber: lastCompletionPos.lineNumber,
          startColumn: lastCompletionPos.column,
          endColumn: lastCompletionPos.column,
        },
        text: insertText,
      }]);
    }
    editor.focus();
    hideSuggest();
  }

  // ── keyboard handler for custom suggest ───────────────────────────────

  function handleSuggestKey(e: monaco.IKeyboardEvent) {
    if (!suggestVisible) return false;

    // Don't capture Ctrl+Enter (run query)
    if (e.ctrlKey || e.metaKey) return false;

    if (e.keyCode === monaco.KeyCode.Escape) {
      hideSuggest();
      e.preventDefault();
      e.stopPropagation();
      return true;
    }
    if (e.keyCode === monaco.KeyCode.DownArrow) {
      suggestActive = Math.min(suggestActive + 1, suggestItems.length - 1);
      e.preventDefault();
      e.stopPropagation();
      return true;
    }
    if (e.keyCode === monaco.KeyCode.UpArrow) {
      suggestActive = Math.max(0, suggestActive - 1);
      e.preventDefault();
      e.stopPropagation();
      return true;
    }
    if (e.keyCode === monaco.KeyCode.Enter || e.keyCode === monaco.KeyCode.Tab) {
      if (suggestActive >= 0 && suggestActive < suggestItems.length) {
        handleSuggestSelect(suggestItems[suggestActive]);
      }
      hideSuggest();
      e.preventDefault();
      e.stopPropagation();
      return true;
    }
    return false;
  }

  // ── main mount ────────────────────────────────────────────────────────

  onMount(() => {
    if (!containerEl) return;

    editor = monaco.editor.create(containerEl, {
      value: sql,
      language: 'sql',
      theme: 'darcula',
      minimap: { enabled: true, scale: 1 },
      fontSize: 13,
      fontFamily: 'JetBrains Mono, Fira Code, Menlo, Consolas, monospace',
      fontLigatures: true,
      lineNumbers: 'on',
      lineNumbersMinChars: 3,
      glyphMargin: false,
      folding: true,
      scrollBeyondLastLine: false,
      wordWrap: 'on',
      renderLineHighlight: 'line',
      smoothScrolling: true,
      cursorBlinking: 'smooth',
      cursorSmoothCaretAnimation: 'on',
      padding: { top: 8, bottom: 8 },
      scrollbar: {
        verticalScrollbarSize: 10,
        horizontalScrollbarSize: 10,
        useShadows: false,
      },
      overviewRulerBorder: false,
      hideCursorInOverviewRuler: true,
      overviewRulerLanes: 0,
      lineDecorationsWidth: 0,
      lineHeight: 20,
      quickSuggestions: false,
      suggestOnTriggerCharacters: false,
      tabCompletion: 'off',
      automaticLayout: true,
    });

    editor.onDidChangeModelContent(() => {
      const value = editor?.getValue() ?? '';
      sql = value;
      onSqlChange?.(value);
    });

    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
      hideSuggest();
      handleRunQuery();
    });

    // Keyboard interceptor for custom suggest widget
    editor.onKeyDown((e) => {
      if (handleSuggestKey(e)) return;
    });

    // Completion provider — calls Rust, returns empty to suppress Monaco widget
    const TRIGGERS = [...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ', '.', ' ', '_'];

    monaco.languages.registerCompletionItemProvider('sql', {
      triggerCharacters: TRIGGERS,
      async provideCompletionItems(_model, position) {
        // Fire-and-forget: trigger our custom widget, tell Monaco to show nothing
        triggerCompletion(position);
        return { suggestions: [] };
      },
    });

    // Ctrl+Space: manual trigger
    editor.addAction({
      id: 'getagrip.triggerSuggest',
      label: 'Trigger SQL Suggestions',
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Space],
      run: () => {
        const pos = editor?.getPosition();
        if (pos) triggerCompletion(pos);
      },
    });

    // Auto-close when cursor moves away from completion context
    editor.onDidChangeCursorPosition((e) => {
      if (!suggestVisible) return;
      const cur = e.position;
      // Different line or moved before completion word start — dismiss
      if (cur.lineNumber !== lastCompletionPos?.lineNumber || cur.column < completionWordStartCol) {
        hideSuggest();
      }
    });

    // Close on editor blur
    editor.onDidBlurEditorText(() => {
      hideSuggest();
    });

    // Close when clicking editor container (outside the widget)
    containerEl.addEventListener('mousedown', () => {
      if (suggestVisible) hideSuggest();
    });

    editor.layout();
    editor.focus();
    setTimeout(() => { editor?.layout(); editor?.focus(); }, 100);
    onReady(handleRunQuery);
  });

  async function handleRunQuery() {
    const currentSql = editor?.getValue() ?? sql;
    if (!currentSql.trim()) return;
    if (!profileId) {
      statusText.set('No data source selected');
      return;
    }

    statusText.set('Running query…');
    try {
      const results: QueryResultDto[] = await executeQueryV2(profileId, currentSql, tabId);

      resultSets.update((rs) => {
        const newSets: ResultSet[] = results.map((r) => ({
          id: nextResultSetId(),
          tabId,
          columns: r.columns as unknown as Record<string, unknown>[],
          rows: r.rows as Record<string, unknown>[],
          elapsedMs: r.elapsed_ms,
          rowsAffected: r.rows_affected,
          pinned: false,
          sortColumn: null,
          sortDirection: null,
          filterText: '',
        }));

        const updated = [...rs, ...newSets];
        if (newSets.length > 0) {
          activeResultSetId.set(newSets[0].id);
          resultsPanelHeight.set(280);
        }
        return updated;
      });

      const totalRows = results.reduce((sum, r) => sum + r.rows.length, 0);
      const totalMs = results.reduce((sum, r) => sum + r.elapsed_ms, 0);
      statusText.set(`${totalRows} rows — ${totalMs}ms`);
    } catch (e: unknown) {
      const msg = String(e);
      statusText.set(`Error: ${msg}`);
    }
  }

  onDestroy(() => {
    editor?.dispose();
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="monaco-container" bind:this={containerEl} on:click={() => editor?.focus()}></div>

<CustomSuggestWidget
  items={suggestItems}
  visible={suggestVisible}
  position={suggestPos}
  activeIndex={suggestActive}
  on:select={(e) => handleSuggestSelect(e.detail)}
  on:close={hideSuggest}
  on:change={(e) => suggestActive = e.detail}
/>

<style>
  .monaco-container {
    width: 100%;
    height: 100%;
    min-height: 100px;
    overflow: hidden;
  }
</style>
