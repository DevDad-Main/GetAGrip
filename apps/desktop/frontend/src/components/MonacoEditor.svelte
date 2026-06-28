<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as monaco from 'monaco-editor';
  import { executeQueryV2, requestCompletion, requestDiagnostics, type QueryResultDto, type CompletionItem } from '$lib/tauri';
  import {
    resultSets, activeResultSetId, statusText, diagnostics as diagStore,
    nextResultSetId, resultsPanelHeight, activeTheme, type ResultSet,
    metadataRefreshed, jumpToPosition,
  } from '$lib/stores';
  import CustomSuggestWidget from './CustomSuggestWidget.svelte';
  import HoverWidget from './HoverWidget.svelte';

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
  let suggestMatchWord = '';
  let lastCompletionPos: monaco.Position | null = null;
  let completionWordStartCol = 0;

  // Custom hover state
  let hoverVisible = false;
  let hoverContent = '';
  let hoverX = 0;
  let hoverY = 0;

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

  // Jump to diagnostics position
  $: if (editor && $jumpToPosition) {
    const pos = $jumpToPosition;
    editor.setPosition({ lineNumber: pos.line, column: pos.column });
    editor.revealLineInCenter(pos.line);
    editor.focus();
    jumpToPosition.set(null);
  }

  // ── suggest widget position helper ────────────────────────────────────

  function updateSuggestPosition(pos: monaco.Position) {
    if (!editor || !containerEl) return;
    const coords = editor.getScrolledVisiblePosition(pos);
    const containerRect = containerEl.getBoundingClientRect();
    const widgetHeight = 480; // approximate max height
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
    suggestMatchWord = word?.word ?? '';

    suggestItems = items;
    suggestActive = -1;
    updateSuggestPosition(pos);
    suggestVisible = true;
    lastCompletionPos = pos;
  }

  function hideSuggest() {
    suggestVisible = false;
    suggestItems = [];
    suggestActive = -1;
    suggestMatchWord = '';
    lastCompletionPos = null;
    completionWordStartCol = 0;
  }

  let cacheChecked = false;
  let diagTimer: ReturnType<typeof setTimeout> | null = null;
  let completionReqId = 0;
  let completionTimer: ReturnType<typeof setTimeout> | null = null;
  const COMPLETION_DEBOUNCE_MS = 150;

  function runDiagnostics() {
    if (!editor) return;

    clearTimeout(diagTimer ?? undefined);
    diagTimer = setTimeout(async () => {
      try {
        const model = editor?.getModel();
        if (!model) return;
        const sql = model.getValue();
        if (!sql.trim()) {
          monaco.editor.setModelMarkers(model, 'sql-diagnostics', []);
          diagStore.set([]);
          return;
        }
        // Run diagnostics even without datasource (syntax errors still work)
        const resp = await requestDiagnostics({
          connection_id: profileId ?? '__no_datasource__',
          sql,
        });
        diagStore.set(resp.diagnostics);
        const markers: monaco.editor.IMarkerData[] = resp.diagnostics.map((d) => ({
          severity: d.severity === 'error' ? monaco.MarkerSeverity.Error
            : d.severity === 'warning' ? monaco.MarkerSeverity.Warning
            : monaco.MarkerSeverity.Hint,
          message: d.message + (d.hint ? `\n${d.hint}` : ''),
          startLineNumber: d.line,
          startColumn: d.column,
          endLineNumber: d.end_line ?? d.line,
          endColumn: (d.end_column && d.end_column < 9999) ? d.end_column
            : Math.min(d.column + 20, 9999),
        }));
        monaco.editor.setModelMarkers(model, 'sql-diagnostics', markers);

      } catch (e) {
        console.error('diag error:', e);
      }
    }, 500);
  }

  // Re-run diagnostics when datasource connects or metadata refreshes
  $: if (profileId) {
    runDiagnostics();
  }
  $: if ($metadataRefreshed > 0) {
    runDiagnostics();
  }

  async function triggerCompletion(pos?: monaco.Position) {
    if (!editor) return;
    const model = editor.getModel();
    if (!model) return;
    const position = pos ?? editor.getPosition();
    if (!position) return;

    // Debounce: rapid typing should not fire a request per char
    if (completionTimer) {
      clearTimeout(completionTimer);
      completionTimer = null;
    }
    completionTimer = setTimeout(() => {
      completionTimer = null;
      void doCompletion(position);
    }, COMPLETION_DEBOUNCE_MS);
  }

  async function doCompletion(position: monaco.Position) {
    const reqId = ++completionReqId;

    // Skip if we're still on the same word (cursor moved but text didn't change)
    const model = editor?.getModel();
    const word = model?.getWordUntilPosition(position);
    if (
      suggestVisible &&
      position.lineNumber === lastCompletionPos?.lineNumber &&
      word?.startColumn === completionWordStartCol &&
      word?.word === suggestMatchWord
    ) {
      return;
    }

    if (!profileId) {
      if (reqId === completionReqId) {
        const fallback: CompletionItem[] = SQL_KEYWORDS.map((kw) => ({
          label: kw,
          kind: 'keyword',
          detail: '',
          score: 50,
        }));
        showSuggest(fallback, position);
      }
      return;
    }

    if (!cacheChecked) {
      cacheChecked = true;
      try {
        const { refreshMetadata } = await import('$lib/tauri');
        await refreshMetadata({ connection_id: profileId });
        metadataRefreshed.set(Date.now());
      } catch { /* silent */ }
    }

    try {
      const text = editor?.getModel()?.getValue() ?? '';
      const resp = await requestCompletion({
        connection_id: profileId,
        sql: text,
        cursor_line: position.lineNumber,
        cursor_column: position.column,
      });
      if (reqId === completionReqId) {
        showSuggest(resp.suggestions, position);
      }
    } catch {
      if (reqId === completionReqId) {
        hideSuggest();
      }
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
      if (suggestActive <= 0) {
        suggestActive = -1;
      } else {
        suggestActive--;
      }
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
      hover: { above: false },
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
      runDiagnostics();
    });

    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
      hideSuggest();
      handleRunQuery();
    });

    // Keyboard interceptor for custom suggest widget
    editor.onKeyDown((e) => {
      if (handleSuggestKey(e)) return;
    });

    // ── Custom hover (replaces Monaco hover provider) ────────────────────

    const fnDocs: Record<string, string> = {
      COUNT: '<strong>COUNT(expr)</strong><br><em>Returns the number of rows or non-null values.</em><br>Aggregate function.',
      SUM: '<strong>SUM(expr)</strong><br><em>Returns the sum of all values.</em><br>Aggregate function.',
      AVG: '<strong>AVG(expr)</strong><br><em>Returns the average of all values.</em><br>Aggregate function.',
      MIN: '<strong>MIN(expr)</strong><br><em>Returns the minimum value.</em><br>Aggregate function.',
      MAX: '<strong>MAX(expr)</strong><br><em>Returns the maximum value.</em><br>Aggregate function.',
      COALESCE: '<strong>COALESCE(val1, val2, ...)</strong><br><em>Returns the first non-null argument.</em><br>Scalar function.',
      NULLIF: '<strong>NULLIF(expr1, expr2)</strong><br><em>Returns NULL if expr1 = expr2, otherwise expr1.</em><br>Scalar function.',
      CAST: '<strong>CAST(expr AS type)</strong><br><em>Converts an expression to a specified data type.</em><br>Conversion function.',
      CONVERT: '<strong>CONVERT(type, expr)</strong><br><em>Converts an expression to a specified data type (MSSQL).</em><br>Conversion function.',
      CONCAT: '<strong>CONCAT(str1, str2, ...)</strong><br><em>Concatenates two or more strings.</em><br>String function.',
      CONCAT_WS: '<strong>CONCAT_WS(sep, str1, str2, ...)</strong><br><em>Concatenates strings with a separator.</em><br>String function.',
      UPPER: '<strong>UPPER(str)</strong><br><em>Converts a string to uppercase.</em><br>String function.',
      LOWER: '<strong>LOWER(str)</strong><br><em>Converts a string to lowercase.</em><br>String function.',
      TRIM: '<strong>TRIM(str)</strong><br><em>Removes leading and trailing spaces.</em><br>String function.',
      LEN: '<strong>LEN(str)</strong><br><em>Returns the length of a string.</em><br>String function.',
      SUBSTRING: '<strong>SUBSTRING(str, start, length)</strong><br><em>Returns part of a string.</em><br>String function.',
      REPLACE: '<strong>REPLACE(str, old, new)</strong><br><em>Replaces occurrences of a substring.</em><br>String function.',
      CHARINDEX: '<strong>CHARINDEX(substr, str)</strong><br><em>Returns the starting position of a substring.</em><br>String function.',
      GETDATE: '<strong>GETDATE()</strong><br><em>Returns the current date and time.</em><br>Date function.',
      GETUTCDATE: '<strong>GETUTCDATE()</strong><br><em>Returns the current UTC date and time.</em><br>Date function.',
      DATEADD: '<strong>DATEADD(datepart, number, date)</strong><br><em>Adds an interval to a date.</em><br>Date function.',
      DATEDIFF: '<strong>DATEDIFF(datepart, start, end)</strong><br><em>Returns the difference between two dates.</em><br>Date function.',
      DATEPART: '<strong>DATEPART(datepart, date)</strong><br><em>Returns a specific part of a date.</em><br>Date function.',
      YEAR: '<strong>YEAR(date)</strong><br><em>Returns the year from a date.</em><br>Date function.',
      MONTH: '<strong>MONTH(date)</strong><br><em>Returns the month from a date.</em><br>Date function.',
      DAY: '<strong>DAY(date)</strong><br><em>Returns the day from a date.</em><br>Date function.',
      STRING_AGG: '<strong>STRING_AGG(expr, separator)</strong><br><em>Concatenates values from multiple rows.</em><br>Aggregate function.',
      FORMAT: '<strong>FORMAT(value, format)</strong><br><em>Formats a value with a .NET format string.</em><br>String function.',
      ROW_NUMBER: '<strong>ROW_NUMBER() OVER (ORDER BY ...)</strong><br><em>Numbers the output of a result set.</em><br>Window function.',
      RANK: '<strong>RANK() OVER (ORDER BY ...)</strong><br><em>Ranks rows with gaps for ties.</em><br>Window function.',
      DENSE_RANK: '<strong>DENSE_RANK() OVER (ORDER BY ...)</strong><br><em>Ranks rows without gaps for ties.</em><br>Window function.',
      LEAD: '<strong>LEAD(expr, offset, default) OVER (...)</strong><br><em>Accesses a subsequent row\'s value.</em><br>Window function.',
      LAG: '<strong>LAG(expr, offset, default) OVER (...)</strong><br><em>Accesses a previous row\'s value.</em><br>Window function.',
    };

    editor.onMouseMove((e) => {
      if (!e.target.position) {
        hoverVisible = false;
        return;
      }
      const model = editor?.getModel();
      if (!model) return;

      // Function docs only — diagnostics use Monaco's built-in hover
      const word = model.getWordAtPosition(e.target.position);
      if (word) {
        const upper = word.word.toUpperCase();
        if (fnDocs[upper]) {
          hoverContent = fnDocs[upper];
          hoverX = e.event.posx + 12;
          hoverY = e.event.posy - 8;
          hoverVisible = true;
          return;
        }
      }
      hoverVisible = false;
    });

    editor.onMouseLeave(() => {
      hoverVisible = false;
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
          endColumn: position.column,
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

    // Refresh completions when the cursor moves within the same line (e.g. typing
    // "Sel" then moving right to fix "Sal"), or dismiss if moved before the word
    // start or to a different line. Use onDidType for character-driven refresh so
    // we don't re-fetch on every arrow-key move.
    editor.onDidChangeCursorPosition((e) => {
      if (!suggestVisible) return;
      const cur = e.position;
      if (cur.lineNumber !== lastCompletionPos?.lineNumber || cur.column < completionWordStartCol) {
        hideSuggest();
        return;
      }
      // Same line, forward movement without typing — the word under cursor
      // hasn't changed, so don't re-fetch. onDidType will re-trigger when the
      // user actually types something at this position.
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
    if (completionTimer) clearTimeout(completionTimer);
    if (diagTimer) clearTimeout(diagTimer);
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
  matchWord={suggestMatchWord}
  on:select={(e) => handleSuggestSelect(e.detail)}
  on:close={hideSuggest}
  on:change={(e) => suggestActive = e.detail}
/>

<HoverWidget visible={hoverVisible} content={hoverContent} x={hoverX} y={hoverY} />

<style>
  .monaco-container {
    width: 100%;
    height: 100%;
    min-height: 100px;
    overflow: hidden;
  }
</style>
