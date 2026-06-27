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
    const widgetHeight = 340; // approximate max height
    const below = containerRect.top + coords.top + 20;
    // If would overflow viewport, flip above cursor
    const top = (below + widgetHeight > window.innerHeight)
      ? containerRect.top + coords.top - widgetHeight - 8
      : below;
    suggestPos = {
      top: Math.max(4, top),
      left: Math.min(containerRect.left + coords.left, window.innerWidth - 500),
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

  let cacheChecked = false;

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

    // Auto-refresh metadata cache once per session (cleared on rebuild)
    if (!cacheChecked) {
      cacheChecked = true;
      try {
        const { refreshMetadata } = await import('$lib/tauri');
        await refreshMetadata({ connection_id: profileId });
      } catch { /* silent */ }
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
      quickSuggestions: true,
      suggestOnTriggerCharacters: true,
      wordBasedSuggestions: false,
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

    // ── Hover provider ─────────────────────────────────────────────────

    const fnDocs: Record<string, string> = {
      COUNT: '**COUNT(expr)**\n\nReturns the number of rows or non-null values.\n\nAggregate function.',
      SUM: '**SUM(expr)**\n\nReturns the sum of all values.\n\nAggregate function.',
      AVG: '**AVG(expr)**\n\nReturns the average of all values.\n\nAggregate function.',
      MIN: '**MIN(expr)**\n\nReturns the minimum value.\n\nAggregate function.',
      MAX: '**MAX(expr)**\n\nReturns the maximum value.\n\nAggregate function.',
      COALESCE: '**COALESCE(val1, val2, ...)**\n\nReturns the first non-null argument.\n\nScalar function.',
      NULLIF: '**NULLIF(expr1, expr2)**\n\nReturns NULL if expr1 = expr2, otherwise expr1.\n\nScalar function.',
      CAST: '**CAST(expr AS type)**\n\nConverts an expression to a specified data type.\n\nConversion function.',
      CONVERT: '**CONVERT(type, expr)**\n\nConverts an expression to a specified data type (MSSQL).\n\nConversion function.',
      CONCAT: '**CONCAT(str1, str2, ...)**\n\nConcatenates two or more strings.\n\nString function.',
      CONCAT_WS: '**CONCAT_WS(separator, str1, str2, ...)**\n\nConcatenates strings with a separator.\n\nString function.',
      UPPER: '**UPPER(str)**\n\nConverts a string to uppercase.\n\nString function.',
      LOWER: '**LOWER(str)**\n\nConverts a string to lowercase.\n\nString function.',
      TRIM: '**TRIM(str)**\n\nRemoves leading and trailing spaces.\n\nString function.',
      LEN: '**LEN(str)**\n\nReturns the length of a string.\n\nString function.',
      SUBSTRING: '**SUBSTRING(str, start, length)**\n\nReturns part of a string.\n\nString function.',
      REPLACE: '**REPLACE(str, old, new)**\n\nReplaces occurrences of a substring.\n\nString function.',
      CHARINDEX: '**CHARINDEX(substr, str)**\n\nReturns the starting position of a substring.\n\nString function.',
      GETDATE: '**GETDATE()**\n\nReturns the current date and time.\n\nDate function.',
      GETUTCDATE: '**GETUTCDATE()**\n\nReturns the current UTC date and time.\n\nDate function.',
      DATEADD: '**DATEADD(datepart, number, date)**\n\nAdds an interval to a date.\n\nDate function.',
      DATEDIFF: '**DATEDIFF(datepart, start, end)**\n\nReturns the difference between two dates.\n\nDate function.',
      DATEPART: '**DATEPART(datepart, date)**\n\nReturns a specific part of a date.\n\nDate function.',
      YEAR: '**YEAR(date)**\n\nReturns the year from a date.\n\nDate function.',
      MONTH: '**MONTH(date)**\n\nReturns the month from a date.\n\nDate function.',
      DAY: '**DAY(date)**\n\nReturns the day from a date.\n\nDate function.',
      STRING_AGG: '**STRING_AGG(expr, separator)**\n\nConcatenates values from multiple rows.\n\nAggregate function.',
      FORMAT: '**FORMAT(value, format)**\n\nFormats a value with a .NET format string.\n\nString function.',
      ROW_NUMBER: '**ROW_NUMBER() OVER (ORDER BY ...)**\n\nNumbers the output of a result set.\n\nWindow function.',
      RANK: '**RANK() OVER (ORDER BY ...)**\n\nRanks rows with gaps for ties.\n\nWindow function.',
      DENSE_RANK: '**DENSE_RANK() OVER (ORDER BY ...)**\n\nRanks rows without gaps for ties.\n\nWindow function.',
      LEAD: '**LEAD(expr, offset, default) OVER (...)**\n\nAccesses a subsequent row\'s value.\n\nWindow function.',
      LAG: '**LAG(expr, offset, default) OVER (...)**\n\nAccesses a previous row\'s value.\n\nWindow function.',
    };

    monaco.languages.registerHoverProvider('sql', {
      provideHover(model, position) {
        const word = model.getWordAtPosition(position);
        if (!word) return null;
        const upper = word.word.toUpperCase();
        if (fnDocs[upper]) {
          return {
            range: {
              startLineNumber: position.lineNumber,
              endLineNumber: position.lineNumber,
              startColumn: word.startColumn,
              endColumn: word.endColumn,
            },
            contents: [{ value: fnDocs[upper] }],
          };
        }
        return null;
      },
    });

    // ── Signature help provider ─────────────────────────────────────────

    const fnSignatures: Record<string, monaco.languages.SignatureInformation> = {
      COUNT: { label: 'COUNT(expression)', documentation: 'Returns the number of rows', parameters: [{ label: 'expression', documentation: 'Column name, *, or 1' }] },
      SUM: { label: 'SUM(expression)', documentation: 'Returns the sum of all values', parameters: [{ label: 'expression', documentation: 'Numeric column or expression' }] },
      COALESCE: { label: 'COALESCE(value1, value2, ...)', documentation: 'Returns the first non-null argument', parameters: [{ label: 'value1', documentation: 'First value to check' }, { label: 'value2', documentation: 'Fallback value' }] },
      CONCAT: { label: 'CONCAT(string1, string2, ...)', documentation: 'Concatenates strings', parameters: [{ label: 'string1', documentation: 'First string' }, { label: 'string2', documentation: 'Second string' }] },
      SUBSTRING: { label: 'SUBSTRING(string, start, length)', documentation: 'Returns part of a string', parameters: [{ label: 'string', documentation: 'The source string' }, { label: 'start', documentation: 'Start position (1-based)' }, { label: 'length', documentation: 'Number of characters to return' }] },
      REPLACE: { label: 'REPLACE(string, old, new)', documentation: 'Replaces occurrences', parameters: [{ label: 'string', documentation: 'The source string' }, { label: 'old', documentation: 'Substring to find' }, { label: 'new', documentation: 'Replacement string' }] },
      DATEADD: { label: 'DATEADD(datepart, number, date)', documentation: 'Adds an interval to a date', parameters: [{ label: 'datepart', documentation: 'year, month, day, hour, etc.' }, { label: 'number', documentation: 'Value to add' }, { label: 'date', documentation: 'The source date' }] },
      DATEDIFF: { label: 'DATEDIFF(datepart, start, end)', documentation: 'Returns the difference between dates', parameters: [{ label: 'datepart', documentation: 'year, month, day, hour, etc.' }, { label: 'start', documentation: 'Start date' }, { label: 'end', documentation: 'End date' }] },
      DATEPART: { label: 'DATEPART(datepart, date)', documentation: 'Returns a specific part of a date', parameters: [{ label: 'datepart', documentation: 'year, month, day, hour, etc.' }, { label: 'date', documentation: 'The source date' }] },
      CAST: { label: 'CAST(expression AS type)', documentation: 'Converts an expression to a type', parameters: [{ label: 'expression', documentation: 'Value to convert' }] },
      CONVERT: { label: 'CONVERT(type, expression)', documentation: 'Converts an expression (MSSQL)', parameters: [{ label: 'type', documentation: 'Target data type' }, { label: 'expression', documentation: 'Value to convert' }] },
    };

    monaco.languages.registerSignatureHelpProvider('sql', {
      signatureHelpTriggerCharacters: ['(', ','],
      signatureHelpRetriggerCharacters: [','],
      provideSignatureHelp(model, position) {
        const textUntilPos = model.getValueInRange({
          startLineNumber: position.lineNumber,
          startColumn: 1,
          endLineNumber: position.lineNumber,
          endColumn: position.column - 1,
        });

        // Find the function name before the opening paren
        const match = textUntilPos.match(/(\w+)\s*\([^)]*$/);
        if (!match) return null;

        const fnName = match[1].toUpperCase();
        const sig = fnSignatures[fnName];
        if (!sig) return null;

        // Count commas before cursor to determine active parameter
        const afterParen = textUntilPos.slice(textUntilPos.lastIndexOf('(') + 1);
        const activeParam = afterParen.split(',').length - 1;

        return {
          value: {
            signatures: [sig],
            activeSignature: 0,
            activeParameter: Math.min(activeParam, (sig.parameters?.length ?? 1) - 1),
          },
          dispose: () => {},
        };
      },
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
