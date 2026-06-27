<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as monaco from 'monaco-editor';
  import { executeQuery, type QueryResultDto } from '$lib/tauri';
  import { connectionUrl, resultColumns, resultRows, resultElapsedMs, resultRowsAffected, showResults, statusText, schemaCache } from '$lib/stores';

  export let sql = '';
  export let onSqlChange: (sql: string) => void = () => {};
  export let onReady: (runQuery: () => void) => void = () => {};

  let editor: monaco.editor.IStandaloneCodeEditor | null = null;
  let containerEl: HTMLDivElement | undefined;

  // Define the Darcula theme once
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
      renderWhitespace: 'none',
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
      letterSpacing: 0,
      quickSuggestions: true,
      suggestOnTriggerCharacters: true,
      acceptSuggestionOnEnter: 'on',
      acceptSuggestionOnCommitCharacter: true,
      tabCompletion: 'on',
      wordBasedSuggestions: 'document',
      parameterHints: { enabled: true, cycle: true },
    });

    // Listen for content changes
    editor.onDidChangeModelContent(() => {
      const value = editor?.getValue() ?? '';
      sql = value;
      onSqlChange?.(value);
    });

    // Ctrl+Enter / Cmd+Enter → run query
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
      handleRunQuery();
    });

    // Escape — blur
    editor.addCommand(monaco.KeyCode.Escape, () => {
      editor?.trigger('keyboard', 'escape', null);
    });

    // SQL autocomplete provider
    monaco.languages.registerCompletionItemProvider('sql', {
      triggerCharacters: ['.', ' '],
      provideCompletionItems(model, position) {
        const textUntilPosition = model.getValueInRange({
          startLineNumber: 1,
          startColumn: 1,
          endLineNumber: position.lineNumber,
          endColumn: position.column,
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

        // After a dot → suggest columns of the preceding table
        const dotMatch = textUntilPosition.match(/(\w+)\.\s*$/i);
        if (dotMatch) {
          const tableName = dotMatch[1];
          // Try all databases for this table name
          for (const [db, tables] of Object.entries(cache.tablesByDb)) {
            if (tables.includes(tableName)) {
              const key = `${db}.${tableName}`;
              const columns = cache.columnsByTable[key] ?? [];
              for (const col of columns) {
                suggestions.push({
                  label: col,
                  kind: monaco.languages.CompletionItemKind.Field,
                  insertText: col,
                  range,
                  detail: 'column',
                });
              }
            }
          }
          // Also try the bare table name as key
          if (suggestions.length === 0) {
            const columns = cache.columnsByTable[tableName] ?? [];
            for (const col of columns) {
              suggestions.push({
                label: col,
                kind: monaco.languages.CompletionItemKind.Field,
                insertText: col,
                range,
                detail: 'column',
              });
            }
          }
        } else {
          // Otherwise → suggest tables (from all known databases) + SQL keywords
          const allTables = Object.values(cache.tablesByDb).flat();
          for (const table of allTables) {
            suggestions.push({
              label: table,
              kind: monaco.languages.CompletionItemKind.Class,
              insertText: table,
              range,
              detail: 'table',
            });
          }
          // Common SQL keywords
          const keywords = ['SELECT', 'FROM', 'WHERE', 'ORDER BY', 'GROUP BY', 'HAVING', 'JOIN', 'LEFT JOIN', 'INNER JOIN', 'ON', 'INSERT INTO', 'UPDATE', 'DELETE FROM', 'CREATE TABLE', 'ALTER TABLE', 'DROP TABLE', 'TOP', 'DISTINCT', 'AS', 'AND', 'OR', 'NOT', 'IN', 'LIKE', 'BETWEEN', 'IS NULL', 'IS NOT NULL', 'EXISTS', 'UNION', 'OFFSET', 'FETCH'];
          for (const kw of keywords) {
            suggestions.push({
              label: kw,
              kind: monaco.languages.CompletionItemKind.Keyword,
              insertText: kw,
              range,
            });
          }
        }

        return { suggestions };
      },
    });

    // Focus
    editor.focus();

    // Notify parent that we're ready, passing the run function
    onReady(handleRunQuery);
  });

  async function handleRunQuery() {
    const currentSql = editor?.getValue() ?? sql;
    if (!currentSql.trim()) return;
    const url = $connectionUrl;
    if (!url) {
      statusText.set('Not connected — connect first');
      return;
    }

    statusText.set('Running query…');
    try {
      const result: QueryResultDto = await executeQuery(currentSql, url);
      resultColumns.set(result.columns as unknown as Record<string, unknown>[]);
      resultRows.set(result.rows);
      resultElapsedMs.set(result.elapsed_ms);
      resultRowsAffected.set(result.rows_affected);
      showResults.set(true);
      statusText.set(`${result.rows.length} rows — ${result.elapsed_ms}ms`);
    } catch (e: unknown) {
      const msg = String(e);
      statusText.set(`Error: ${msg}`);
      showResults.set(false);
    }
  }

  onDestroy(() => {
    editor?.dispose();
  });
</script>

<div class="monaco-container" bind:this={containerEl}></div>

<style>
  .monaco-container {
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
</style>
