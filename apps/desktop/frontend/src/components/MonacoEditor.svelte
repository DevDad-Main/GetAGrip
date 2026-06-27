<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as monaco from 'monaco-editor';
  import { executeQueryV2, type QueryResultDto } from '$lib/tauri';
  import {
    resultSets, activeResultSetId, statusText, schemaCache,
    nextResultSetId, type ResultSet,
  } from '$lib/stores';

  export let sql = '';
  export let profileId: string | null = null;
  export let tabId = '';
  export let onSqlChange: (sql: string) => void = () => {};
  export let onReady: (runQuery: () => void) => void = () => {};

  let editor: monaco.editor.IStandaloneCodeEditor | null = null;
  let containerEl: HTMLDivElement | undefined;

  monaco.editor.defineTheme('darcula', {
    base: 'vs-dark',
    inherit: true,
    rules: [
      { token: 'keyword', foreground: 'cc7832', fontStyle: 'bold' },
      { token: 'type', foreground: 'a9b7c6' },
      { token: 'string', foreground: '6a8759' },
      { token: 'number', foreground: '6897bb' },
      { token: 'comment', foreground: '808080', fontStyle: 'italic' },
      { token: 'operator', foreground: 'ffc66d' },
      { token: 'identifier', foreground: 'a9b7c6' },
    ],
    colors: {
      'editor.background': '#2b2b2b',
      'editor.foreground': '#bbbbbb',
      'editor.lineHighlightBackground': '#2a2d2e',
      'editor.selectionBackground': '#2f4050',
      'editorCursor.foreground': '#aaaaaa',
      'editorLineNumber.foreground': '#606366',
      'editorLineNumber.activeForeground': '#a9b7c6',
      'editorIndentGuide.background': '#3a3a3a',
      'editorIndentGuide.activeBackground': '#555555',
      'editorWidget.background': '#313335',
      'editorWidget.border': '#4a4e51',
      'editorSuggestWidget.background': '#313335',
      'editorSuggestWidget.border': '#4a4e51',
      'editorSuggestWidget.foreground': '#bbbbbb',
      'editorSuggestWidget.selectedBackground': '#2f4050',
      'editorSuggestWidget.highlightForeground': '#4a9eff',
      'minimap.background': '#2b2b2b',
      'minimapSlider.background': '#4a4e5180',
      'minimapSlider.hoverBackground': '#5a5e6180',
      'scrollbar.shadow': '#00000000',
      'scrollbarSlider.background': '#4a4e5180',
      'scrollbarSlider.hoverBackground': '#5a5e6180',
      'scrollbarSlider.activeBackground': '#6a6e7180',
    },
  });

  // Sync external sql prop changes into the editor (e.g. tab switching)
  $: if (editor && sql !== editor.getValue()) {
    editor.setValue(sql);
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

    monaco.languages.registerCompletionItemProvider('sql', {
      triggerCharacters: ['.', ' '],
      provideCompletionItems(model, position) {
        const textUntilPosition = model.getValueInRange({
          startLineNumber: 1, startColumn: 1,
          endLineNumber: position.lineNumber, endColumn: position.column,
        });
        const word = model.getWordUntilPosition(position);
        const range = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        };

        const suggestions: monaco.languages.CompletionItem[] = [];
        const cache = $schemaCache;

        const dotMatch = textUntilPosition.match(/(\w+)\.\s*$/i);
        if (dotMatch) {
          const tableName = dotMatch[1];
          for (const [db, tables] of Object.entries(cache.tablesByDb)) {
            if (tables.includes(tableName)) {
              const columns = cache.columnsByTable[`${db}.${tableName}`] ?? [];
              for (const col of columns) {
                suggestions.push({
                  label: col, kind: monaco.languages.CompletionItemKind.Field,
                  insertText: col, range, detail: 'column',
                });
              }
            }
          }
        } else {
          const allTables = Object.values(cache.tablesByDb).flat();
          for (const table of allTables) {
            suggestions.push({
              label: table, kind: monaco.languages.CompletionItemKind.Class,
              insertText: table, range, detail: 'table',
            });
          }
          const keywords = ['SELECT', 'FROM', 'WHERE', 'ORDER BY', 'GROUP BY', 'HAVING', 'JOIN', 'LEFT JOIN', 'INNER JOIN', 'ON', 'INSERT INTO', 'UPDATE', 'DELETE FROM', 'CREATE TABLE', 'ALTER TABLE', 'DROP TABLE', 'TOP', 'DISTINCT', 'AS', 'AND', 'OR', 'NOT', 'IN', 'LIKE', 'BETWEEN', 'IS NULL', 'IS NOT NULL', 'EXISTS', 'UNION', 'OFFSET', 'FETCH'];
          for (const kw of keywords) {
            suggestions.push({
              label: kw, kind: monaco.languages.CompletionItemKind.Keyword,
              insertText: kw, range,
            });
          }
        }

        return { suggestions };
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
        if (newSets.length > 0) activeResultSetId.set(newSets[0].id);
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
