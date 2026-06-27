<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as monaco from 'monaco-editor';
  import { executeQueryV2, requestCompletion, type QueryResultDto } from '$lib/tauri';
  import {
    resultSets, activeResultSetId, statusText,
    nextResultSetId, resultsPanelHeight, activeTheme, type ResultSet,
  } from '$lib/stores';

  export let sql = '';
  export let profileId: string | null = null;
  export let tabId = '';
  export let onSqlChange: (sql: string) => void = () => {};
  export let onReady: (runQuery: () => void) => void = () => {};

  import { THEMES, type ThemeDef } from '$lib/themes';

  const SQL_KEYWORDS = ['SELECT','FROM','WHERE','JOIN','LEFT JOIN','INNER JOIN','ON','AND','OR','NOT','IN','EXISTS','BETWEEN','LIKE','IS','NULL','AS','INSERT INTO','VALUES','UPDATE','SET','DELETE FROM','CREATE TABLE','ALTER TABLE','DROP TABLE','ORDER BY','GROUP BY','HAVING','LIMIT','OFFSET','UNION','DISTINCT','TOP','CASE','WHEN','THEN','ELSE','END','ASC','DESC','BEGIN','COMMIT','ROLLBACK'];

  let editor: monaco.editor.IStandaloneCodeEditor | null = null;
  let containerEl: HTMLDivElement | undefined;

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

  // Pre-define all themes
  THEMES.forEach((t) => defineMonacoTheme(t));

  // Sync external sql prop changes into the editor (e.g. tab switching)
  $: if (editor && sql !== editor.getValue()) {
    editor.setValue(sql);
  }

  // Switch Monaco theme when activeTheme store changes
  $: if (editor && $activeTheme) {
    monaco.editor.setTheme($activeTheme);
  }

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
      quickSuggestions: true,
      suggestOnTriggerCharacters: true,
      tabCompletion: 'on',
      automaticLayout: true,
    });

    editor.onDidChangeModelContent(() => {
      const value = editor?.getValue() ?? '';
      sql = value;
      onSqlChange?.(value);
    });

    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
      handleRunQuery();
    });

    editor.addCommand(monaco.KeyCode.Escape, () => {
      editor?.trigger('keyboard', 'escape', null);
    });

    const kindMap: Record<string, monaco.languages.CompletionItemKind> = {
      table: monaco.languages.CompletionItemKind.Class,
      view: monaco.languages.CompletionItemKind.Class,
      column: monaco.languages.CompletionItemKind.Field,
      function: monaco.languages.CompletionItemKind.Function,
      keyword: monaco.languages.CompletionItemKind.Keyword,
      schema: monaco.languages.CompletionItemKind.Module,
      alias: monaco.languages.CompletionItemKind.Variable,
    };

    const TRIGGERS = [...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ', '.', ' ', '_'];

    monaco.languages.registerCompletionItemProvider('sql', {
      triggerCharacters: TRIGGERS,
      async provideCompletionItems(model, position) {
        const word = model.getWordUntilPosition(position);
        const range = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        };

        // Fallback keywords when no datasource or Rust call fails
        const fallback = SQL_KEYWORDS.map((kw) => ({
          label: kw,
          kind: monaco.languages.CompletionItemKind.Keyword,
          insertText: kw + ' ',
          detail: '',
          range,
          sortText: '5000',
          filterText: kw,
        }));

        if (!profileId) {
          return { suggestions: fallback };
        }

        try {
          const sql = model.getValue();
          const resp = await requestCompletion({
            connection_id: profileId,
            sql,
            cursor_line: position.lineNumber,
            cursor_column: position.column,
          });

          const suggestions: monaco.languages.CompletionItem[] = resp.suggestions.map((item) => ({
            label: item.label,
            kind: kindMap[item.kind] ?? monaco.languages.CompletionItemKind.Text,
            insertText: item.insert_text ?? item.label,
            detail: item.detail,
            documentation: item.documentation,
            range,
            sortText: String(1000 - item.score).padStart(4, '0'),
            filterText: item.label,
          }));

          return { suggestions: suggestions.length > 0 ? suggestions : fallback };
        } catch {
          return { suggestions: fallback };
        }
      },
    });

    editor.layout();
    editor.focus();
    // Sometimes the webview hasn't finished layout yet — retry
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

<style>
  .monaco-container {
    width: 100%;
    height: 100%;
    min-height: 100px;
    overflow: hidden;
  }
</style>
