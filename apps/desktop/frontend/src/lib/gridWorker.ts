/// <reference lib="webworker" />

let rows: Record<string, unknown>[] = [];
let colNames: string[] = [];
let filterText = '';
let sortColumn: string | null = null;
let sortDirection: 'asc' | 'desc' | null = null;

function computeFiltered(): Record<string, unknown>[] {
  if (!filterText?.trim()) return rows;
  const q = filterText.toLowerCase();
  return rows.filter((row) =>
    colNames.some((name) => {
      const val = row[name];
      return val != null && String(val).toLowerCase().includes(q);
    }),
  );
}

function computeSorted(data: Record<string, unknown>[]): Record<string, unknown>[] {
  if (!sortColumn || !sortDirection) return data;
  return data.sort((a, b) => {
    const av = a[sortColumn!];
    const bv = b[sortColumn!];
    if (av == null && bv == null) return 0;
    if (av == null) return sortDirection === 'asc' ? 1 : -1;
    if (bv == null) return sortDirection === 'asc' ? -1 : 1;
    const sa = String(av);
    const sb = String(bv);
    const n = !isNaN(Number(sa)) && !isNaN(Number(sb));
    const cmp = n ? Number(sa) - Number(sb) : sa.localeCompare(sb);
    return sortDirection === 'asc' ? cmp : -cmp;
  });
}

self.onmessage = (e: MessageEvent) => {
  const msg = e.data;
  switch (msg.type) {
    case 'setData': {
      rows = msg.rows;
      colNames = msg.colNames;
      filterText = '';
      sortColumn = null;
      sortDirection = null;
      self.postMessage({ type: 'dataLoaded', totalRows: rows.length, colNames });
      break;
    }
    case 'setFilter': {
      filterText = msg.filterText;
      self.postMessage({ type: 'filterChanged', totalFiltered: computeFiltered().length });
      break;
    }
    case 'setSort': {
      sortColumn = msg.sortColumn;
      sortDirection = msg.sortDirection;
      const data = computeSorted(computeFiltered());
      self.postMessage({ type: 'sortChanged', totalFiltered: data.length });
      break;
    }
    case 'getRows': {
      const data = computeSorted(computeFiltered());
      const slice = data.slice(msg.startIndex, msg.endIndex);
      self.postMessage({
        type: 'rows',
        rows: slice,
        startIndex: msg.startIndex,
        endIndex: msg.endIndex,
        totalFiltered: data.length,
      });
      break;
    }
    case 'getAllRows': {
      const data = computeSorted(computeFiltered());
      self.postMessage({ type: 'allRows', rows: data, colNames });
      break;
    }
  }
};
