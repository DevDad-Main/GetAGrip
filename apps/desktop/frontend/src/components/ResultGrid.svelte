<script lang="ts">
  import type { ResultSet } from '$lib/stores';
  import { resultSets } from '$lib/stores';
  import { exportResult, saveExport, type ExportInput, type ExportColumn } from '$lib/tauri';
  import { save } from '@tauri-apps/plugin-dialog';
  import { notify } from './Toast.svelte';
  import { Copy, Download, ChevronDown, ArrowUp, ArrowDown } from 'lucide-svelte';

  export let result: ResultSet;

  const MAX_ROWS = 5000;
  let exportMenuOpen = false;

  $: filtered = computeFiltered(result.rows, result.columns, result.filterText);
  $: sorted = computeSorted(filtered, result.columns, result.sortColumn, result.sortDirection);

  function computeFiltered(rows: Record<string, unknown>[], cols: Record<string, unknown>[], filter: string): Record<string, unknown>[] {
    if (!filter?.trim()) return rows;
    const q = filter.toLowerCase();
    return rows.filter((row) => cols.some((col) => {
      const val = row[String(col.name)];
      return val != null && String(val).toLowerCase().includes(q);
    }));
  }

  function computeSorted(rows: Record<string, unknown>[], cols: Record<string, unknown>[], sortCol: string | null, sortDir: 'asc' | 'desc' | null): Record<string, unknown>[] {
    if (!sortCol || !sortDir) return rows;
    return [...rows].sort((a, b) => {
      const av = a[sortCol];
      const bv = b[sortCol];
      if (av == null && bv == null) return 0;
      if (av == null) return sortDir === 'asc' ? 1 : -1;
      if (bv == null) return sortDir === 'asc' ? -1 : 1;
      const sa = String(av);
      const sb = String(bv);
      const n = !isNaN(Number(sa)) && !isNaN(Number(sb));
      const cmp = n ? Number(sa) - Number(sb) : sa.localeCompare(sb);
      return sortDir === 'asc' ? cmp : -cmp;
    });
  }

  function isNull(val: unknown): boolean { return val === null || val === undefined; }
  function isNumber(val: unknown): boolean { return typeof val === 'number'; }

  const MAX_CELL_DISPLAY = 120;
  const MAX_CELL_COPY = 200;

  function truncate(s: string, max: number): string {
    if (s.length > max) return s.slice(0, max) + '\u2026';
    return s;
  }

  function formatValue(val: unknown): string {
    if (isNull(val)) return 'NULL';
    if (typeof val === 'boolean') return val ? 'true' : 'false';
    if (typeof val === 'object') return truncate(JSON.stringify(val), MAX_CELL_DISPLAY);
    return truncate(String(val), MAX_CELL_DISPLAY);
  }

  function copyValue(val: unknown): string {
    if (isNull(val)) return '';
    if (typeof val === 'boolean') return val ? 'true' : 'false';
    if (typeof val === 'object') {
      const s = JSON.stringify(val);
      if (s.includes('\t') || s.includes('\n') || s.includes('\r')) {
        return truncate(s.replace(/\t/g, ' ').replace(/\r?\n/g, ' '), MAX_CELL_COPY);
      }
      return truncate(s, MAX_CELL_COPY);
    }
    let s = String(val);
    if (s.includes('\t') || s.includes('\n') || s.includes('\r')) {
      s = s.replace(/\t/g, ' ').replace(/\r?\n/g, ' ');
    }
    return truncate(s, MAX_CELL_COPY);
  }

  function handleSort(colName: string) {
    resultSets.update((rs) =>
      rs.map((r) => {
        if (r.id !== result.id) return r;
        const newDir = r.sortColumn === colName && r.sortDirection === 'asc' ? 'desc' : 'asc';
        return { ...r, sortColumn: colName, sortDirection: newDir };
      }),
    );
  }

  function handleCopy() {
    try {
      const cols = result.columns.map((c: Record<string, unknown>) => String(c.name));
      const lines = [cols.join('\t')];
      for (const row of sorted.slice(0, MAX_ROWS)) {
        lines.push(cols.map((c) => copyValue(row[c])).join('\t'));
      }
      navigator.clipboard.writeText(lines.join('\n'))
        .then(() => notify('Copied to clipboard', 'success'))
        .catch(() => notify('Copy failed', 'error'));
    } catch (e) {
      console.error('copy failed:', e);
      notify('Copy failed', 'error');
    }
  }

  async function handleExport(format: string, download: boolean) {
    try {
      const columns: ExportColumn[] = result.columns.map((c: Record<string, unknown>, i: number) => ({
        name: String(c.name), col_type: String(c.col_type ?? 'string'),
        db_type: String(c.db_type ?? ''), nullable: Boolean(c.nullable ?? true), ordinal: i,
      }));
      const rows: unknown[][] = sorted.slice(0, MAX_ROWS).map((row: Record<string, unknown>) =>
        columns.map((c) => row[c.name]),
      );
      const outFmt = format === 'tsv' ? 'tsv' : format;
      const input: ExportInput = { format: outFmt, columns, rows, include_header: true };
      const output = await exportResult(input);

      if (download) {
        const ext = format === 'tsv' ? 'tsv' : format === 'markdown' ? 'md' : format;
        const filePath = await save({
          defaultPath: `query_result.${ext}`,
          filters: [{ name: format.toUpperCase(), extensions: [ext] }],
        });
        if (filePath) {
          await saveExport(input, filePath);
          notify(`Saved to ${filePath}`, 'success');
        }
      } else {
        navigator.clipboard.writeText(output)
          .then(() => notify(`Copied ${format.toUpperCase()} to clipboard`, 'success'))
          .catch(() => notify('Copy failed', 'error'));
      }
    } catch (e) {
      console.error('export failed:', e);
      const msg = e instanceof Error ? e.message : typeof e === 'string' ? e : JSON.stringify(e);
      notify(`Export failed: ${msg}`, 'error');
    }
  }
</script>

<div class="rg">
  <div class="rg-toolbar">
    <span class="rg-meta">{sorted.length} of {result.rows.length} rows — {result.elapsedMs}ms</span>
    <div class="rg-actions">
      <button class="rg-btn" on:click={handleCopy} title="Copy as TSV"><Copy size="12" /></button>

      <div class="rg-export-wrap">
        <button class="rg-btn" on:click={() => handleExport('csv', true)} title="Download CSV">
          <Download size="12" /> CSV
        </button>
        <button class="rg-btn rg-btn-arrow" on:click|stopPropagation={() => exportMenuOpen = !exportMenuOpen} title="More export options">
          <ChevronDown size="10" />
        </button>
        {#if exportMenuOpen}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="rg-menu" on:click|stopPropagation>
            <button class="rg-menu-item" on:click={() => { handleExport('csv', true); exportMenuOpen = false; }}>Download CSV</button>
            <button class="rg-menu-item" on:click={() => { handleExport('tsv', true); exportMenuOpen = false; }}>Download TSV</button>
            <button class="rg-menu-item" on:click={() => { handleExport('json', true); exportMenuOpen = false; }}>Download JSON</button>
            <div class="rg-menu-sep"></div>
            <button class="rg-menu-item" on:click={() => { handleExport('csv', false); exportMenuOpen = false; }}>Copy CSV to clipboard</button>
            <button class="rg-menu-item" on:click={() => { handleExport('tsv', false); exportMenuOpen = false; }}>Copy TSV to clipboard</button>
            <button class="rg-menu-item" on:click={() => { handleExport('json', false); exportMenuOpen = false; }}>Copy JSON to clipboard</button>
          </div>
        {/if}
      </div>

      <input class="rg-filter" type="text" bind:value={result.filterText} placeholder="Filter…" />
    </div>
  </div>

  <div class="rg-body">
    {#if result.columns.length === 0}
      <div class="rg-empty">No columns returned.</div>
    {:else}
      <div class="rg-scroll">
        <table class="rg-table">
          <thead>
            <tr>
              {#each result.columns as col (col.name)}
                <th title={col.db_type} on:click={() => handleSort(String(col.name))}>
                  <span class="th-label">{col.name}</span>
                  {#if result.sortColumn === String(col.name)}
                    {#if result.sortDirection === 'asc'}
                      <ArrowUp size="10" class="sort-icon" />
                    {:else}
                      <ArrowDown size="10" class="sort-icon" />
                    {/if}
                  {/if}
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each sorted.slice(0, MAX_ROWS) as row, idx (idx)}
              <tr class:stripe={idx % 2 === 1}>
                {#each result.columns as col}
                  <td class:null={isNull(row[col.name])} class:number={isNumber(row[col.name])}>
                    {formatValue(row[col.name])}
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
        {#if sorted.length > MAX_ROWS}
          <div class="rg-truncated">Showing first {MAX_ROWS} of {sorted.length} rows.</div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<svelte:window on:click={() => { exportMenuOpen = false; }} />

<style>
  .rg { display: flex; flex-direction: column; height: 100%; overflow: hidden; }
  .rg-toolbar {
    display: flex; align-items: center; gap: 8px; padding: 4px 8px;
    background: var(--bg); border-bottom: 1px solid var(--border); flex-shrink: 0;
  }
  .rg-meta { font-size: 11px; color: var(--text-muted); }
  .rg-actions { margin-left: auto; display: flex; align-items: center; gap: 4px; }
  .rg-btn {
    font-size: 10px; padding: 2px 6px; display: flex; align-items: center; gap: 2px;
    background: var(--bg-elev); border-color: var(--border);
  }
  .rg-btn:hover { background: var(--bg-input); }
  .rg-btn-arrow {
    padding: 2px 3px; border-left: none;
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0; margin-left: -1px;
  }
  .rg-export-wrap { position: relative; display: flex; }
  .rg-export-wrap > .rg-btn:first-child { border-radius: var(--radius-sm) 0 0 var(--radius-sm); }
  .rg-menu {
    position: absolute; top: 100%; right: 0; margin-top: 2px;
    background: var(--bg-elev); border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm); box-shadow: var(--shadow-md);
    z-index: 100; min-width: 180px; padding: 4px 0;
  }
  .rg-menu-item {
    display: block; width: 100%; text-align: left; padding: 6px 12px;
    font-size: 11px; color: var(--text); background: transparent; border: none; cursor: pointer;
  }
  .rg-menu-item:hover { background: var(--accent-soft); }
  .rg-menu-sep { height: 1px; background: var(--border); margin: 4px 0; }
  .rg-filter { width: 120px; font-size: 11px; padding: 2px 6px; }
  .rg-body { flex: 1; overflow: hidden; min-height: 0; }
  .rg-scroll { overflow: auto; height: 100%; }
  .rg-table {
    border-collapse: collapse; font-size: 12px; font-family: var(--font-mono);
    width: max-content; min-width: 100%;
  }
  .rg-table th {
    position: sticky; top: 0; background: var(--bg-elev); color: var(--text-muted);
    font-weight: 600; text-align: left; padding: 4px 10px;
    border-bottom: 1px solid var(--border); border-right: 1px solid var(--border);
    white-space: nowrap; z-index: 1; cursor: pointer; user-select: none;
  }
  .rg-table th:hover { background: var(--bg-input); }
  .th-label { margin-right: 4px; }
  .sort-icon { opacity: 0.5; flex-shrink: 0; vertical-align: middle; }
  .rg-table td {
    padding: 3px 10px; border-bottom: 1px solid var(--border);
    border-right: 1px solid var(--border); color: var(--text);
    white-space: nowrap; max-width: 300px; overflow: hidden; text-overflow: ellipsis;
  }
  .rg-table tr.stripe td { background: var(--bg-stripe); }
  .rg-table tr:hover td { background: var(--bg-hover); }
  .rg-table td.null { color: var(--text-faint); font-style: italic; }
  .rg-table td.number { text-align: right; color: var(--info); }
  .rg-empty {
    display: flex; align-items: center; justify-content: center;
    height: 100%; color: var(--text-faint); font-size: 12px;
  }
  .rg-truncated {
    padding: 6px 12px; font-size: 11px; color: var(--warning);
    background: var(--bg-elev); border-top: 1px solid var(--border);
  }
</style>
