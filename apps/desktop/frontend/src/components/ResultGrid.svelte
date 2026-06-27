<script lang="ts">
  import type { ResultSet } from '$lib/stores';
  import { exportResult, type ExportInput, type ExportColumn } from '$lib/tauri';
  import { ArrowUpDown, Copy, Download } from 'lucide-svelte';

  export let result: ResultSet;

  const MAX_ROWS = 5000;

  function isNull(val: unknown): boolean { return val === null || val === undefined; }
  function isNumber(val: unknown): boolean { return typeof val === 'number'; }

  function formatValue(val: unknown): string {
    if (isNull(val)) return 'NULL';
    if (typeof val === 'boolean') return val ? 'true' : 'false';
    if (typeof val === 'object') return JSON.stringify(val);
    return String(val);
  }

  function handleSort(colName: string) {
    if (result.sortColumn === colName) {
      result.sortDirection = result.sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      result.sortColumn = colName;
      result.sortDirection = 'asc';
    }
  }

  function handleCopy() {
    const rows = result.rows;
    const cols = result.columns.map((c: Record<string, unknown>) => String(c.name));
    const lines = [cols.join('\t')];
    for (const row of rows) {
      lines.push(cols.map((c) => formatValue(row[c])).join('\t'));
    }
    navigator.clipboard.writeText(lines.join('\n')).catch(console.error);
  }

  async function handleExport(format: string) {
    try {
      const columns: ExportColumn[] = result.columns.map((c: Record<string, unknown>, i: number) => ({
        name: String(c.name),
        col_type: String(c.col_type ?? 'string'),
        db_type: String(c.db_type ?? ''),
        nullable: Boolean(c.nullable ?? true),
        ordinal: i,
      }));
      const rows: unknown[][] = result.rows.map((row: Record<string, unknown>) =>
        columns.map((c) => row[c.name])
      );
      const input: ExportInput = { format, columns, rows, include_header: true };
      const output = await exportResult(input);
      navigator.clipboard.writeText(output).catch(console.error);
    } catch (e) {
      console.error('export failed:', e);
    }
  }
</script>

<div class="rg">
  <div class="rg-toolbar">
    <span class="rg-meta">
      {result.rows.length} rows — {result.elapsedMs}ms
    </span>
    <div class="rg-actions">
      <button class="rg-btn" on:click={handleCopy} title="Copy as TSV"><Copy size="12" /></button>
      <button class="rg-btn" on:click={() => handleExport('csv')} title="Copy as CSV"><Download size="12" /> CSV</button>
      <button class="rg-btn" on:click={() => handleExport('json')} title="Copy as JSON"><Download size="12" /> JSON</button>
      <input
        class="rg-filter"
        type="text"
        bind:value={result.filterText}
        placeholder="Filter…"
        on:input={() => {}}
      />
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
                <th
                  title={col.db_type}
                  on:click={() => handleSort(String(col.name))}
                >
                  <span>{col.name}</span>
                  <ArrowUpDown size="10" class="sort-icon" />
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each result.rows.slice(0, MAX_ROWS) as row, idx (idx)}
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
        {#if result.rows.length > MAX_ROWS}
          <div class="rg-truncated">
            Showing first {MAX_ROWS} of {result.rows.length} rows.
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .rg {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .rg-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .rg-meta { font-size: 11px; color: var(--text-muted); }
  .rg-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .rg-btn {
    font-size: 10px;
    padding: 2px 6px;
    display: flex;
    align-items: center;
    gap: 2px;
    background: var(--bg-elev);
    border-color: var(--border);
  }
  .rg-btn:hover { background: var(--bg-input); }
  .rg-filter {
    width: 120px;
    font-size: 11px;
    padding: 2px 6px;
  }
  .rg-body {
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }
  .rg-scroll {
    overflow: auto;
    height: 100%;
  }
  .rg-table {
    border-collapse: collapse;
    font-size: 12px;
    font-family: var(--font-mono);
    width: max-content;
    min-width: 100%;
  }
  .rg-table th {
    position: sticky;
    top: 0;
    background: var(--bg-elev);
    color: var(--text-muted);
    font-weight: 600;
    text-align: left;
    padding: 4px 10px;
    border-bottom: 1px solid var(--border);
    border-right: 1px solid var(--border);
    white-space: nowrap;
    z-index: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .rg-table th:hover { background: var(--bg-input); }
  .sort-icon {
    opacity: 0.3;
    flex-shrink: 0;
  }
  .rg-table td {
    padding: 3px 10px;
    border-bottom: 1px solid var(--border);
    border-right: 1px solid var(--border);
    color: var(--text);
    white-space: nowrap;
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .rg-table tr.stripe td { background: var(--bg-stripe); }
  .rg-table tr:hover td { background: var(--bg-hover); }
  .rg-table td.null { color: var(--text-faint); font-style: italic; }
  .rg-table td.number { text-align: right; color: var(--info); }
  .rg-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-faint);
    font-size: 12px;
  }
  .rg-truncated {
    padding: 6px 12px;
    font-size: 11px;
    color: var(--warning);
    background: var(--bg-elev);
    border-top: 1px solid var(--border);
  }
</style>
