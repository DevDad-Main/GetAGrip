<script lang="ts">
  import { showResults, resultColumns, resultRows, resultElapsedMs, resultRowsAffected } from '$lib/stores';

  const MAX_ROWS = 5000;

  function isNull(val: unknown): boolean {
    return val === null || val === undefined;
  }

  function isNumber(val: unknown): boolean {
    return typeof val === 'number';
  }

  function formatValue(val: unknown): string {
    if (isNull(val)) return 'NULL';
    if (typeof val === 'boolean') return val ? 'true' : 'false';
    if (typeof val === 'object') return JSON.stringify(val);
    return String(val);
  }
</script>

{#if $showResults}
  <section class="results">
    <div class="results-header">
      <span class="results-title">RESULTS</span>
      <span class="results-meta">
        {$resultRows.length} rows — {$resultElapsedMs}ms
        {#if $resultRowsAffected > 0}
          — {$resultRowsAffected} affected
        {/if}
      </span>
    </div>
    <div class="results-body">
      {#if $resultColumns.length === 0}
        <div class="results-empty">No columns returned.</div>
      {:else}
        <div class="results-scroll">
          <table class="results-table">
            <thead>
              <tr>
                {#each $resultColumns as col}
                  <th title={col.db_type}>{col.name}</th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each $resultRows.slice(0, MAX_ROWS) as row, idx (idx)}
                <tr class:stripe={idx % 2 === 1}>
                  {#each $resultColumns as col}
                    <td class:null={isNull(row[col.name])} class:number={isNumber(row[col.name])}>
                      {formatValue(row[col.name])}
                    </td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
          {#if $resultRows.length > MAX_ROWS}
            <div class="results-truncated">
              Showing first {MAX_ROWS} of {$resultRows.length} rows.
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </section>
{/if}

<style>
  .results {
    border-top: 1px solid var(--border);
    background: var(--bg);
    height: 220px;
    min-height: 100px;
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .results-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 12px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 1px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .results-title {
    color: var(--text-muted);
  }
  .results-meta {
    font-weight: 400;
    letter-spacing: 0;
    color: var(--text-faint);
  }
  .results-body {
    flex: 1;
    overflow: auto;
    min-height: 0;
    position: relative;
  }
  .results-scroll {
    min-width: min-content;
  }
  .results-table {
    border-collapse: collapse;
    font-size: 12px;
    font-family: var(--font-mono);
    width: max-content;
    min-width: 100%;
  }
  .results-table th {
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
  }
  .results-table td {
    padding: 3px 10px;
    border-bottom: 1px solid var(--border);
    border-right: 1px solid var(--border);
    color: var(--text);
    white-space: nowrap;
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .results-table tr.stripe td {
    background: var(--bg-stripe);
  }
  .results-table tr:hover td {
    background: var(--bg-hover);
  }
  .results-table td.null {
    color: var(--text-faint);
    font-style: italic;
  }
  .results-table td.number {
    text-align: right;
    color: var(--info);
  }
  .results-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-faint);
    font-size: 12px;
  }
  .results-truncated {
    padding: 6px 12px;
    font-size: 11px;
    color: var(--warning);
    background: var(--bg-elev);
    border-top: 1px solid var(--border);
  }
</style>
