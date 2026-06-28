<script lang="ts">
  /**
   * Database-driver identity glyphs.
   *
   * Each driver gets a distinct, recognizable mark (not a generic cylinder):
   *   postgres  → elephant silhouette
   *   mysql     → the classic dolphin "Sakila" mark
   *   sqlite    → stacked slabs (file/db)
   *   mssql     → MS "quadrants" window mark
   *   oracle    → the red "sun" ring
   *   mongodb   → the leaf
   *   redis     → the keyhole logo
   *   generic   → a plain database cylinder
   *
   * Rendered as inline SVG so we can theme via currentColor and keep one
   * crisp glyph at any size. The `driver` prop accepts the lowercase driver
   * string from ConnectionProfile.driver.
   */

  export let driver: string;
  export let size = 14;
  export let port = 0;
  export let host = '';

  $: d = resolveDriver(driver ?? '', port, host);

  /**
   * Use the saved driver, but sanity-check against the port + host hints —
   * if the saved driver is "sqlite" but the port is 1433 (MSSQL), 5432 (PG),
   * etc. the profile was created with the wrong driver. Fall back to the
   * port-based heuristic so the user sees the right database glyph.
   */
  function resolveDriver(raw: string, port: number, host: string): string {
    const lower = raw.toLowerCase();
    // If the driver and port agree, trust the driver.
    if (matchesPort(lower, port)) return lower;
    // Otherwise, try to infer from port (and host for SQLite paths).
    if (host && /\.(db|sqlite|sqlite3)$/i.test(host)) return 'sqlite';
    if (host && /:[\d.]+\//.test(host)) {
      // Looks like a URL — parse the scheme.
      const m = host.match(/^([a-z]+):\/\//i);
      if (m) return m[1].toLowerCase();
    }
    return portToDriver(port) ?? lower;
  }

  function matchesPort(driver: string, port: number): boolean {
    switch (driver) {
      case 'postgres': return port === 5432;
      case 'mysql': return port === 3306;
      case 'mssql': return port === 1433;
      case 'oracle': return port === 1521;
      case 'mongodb': return port === 27017;
      case 'redis': return port === 6379;
      case 'sqlite': return port === 0;
      default: return true;
    }
  }

  function portToDriver(port: number): string | null {
    switch (port) {
      case 5432: return 'postgres';
      case 3306: return 'mysql';
      case 1433: return 'mssql';
      case 1521: return 'oracle';
      case 27017: return 'mongodb';
      case 6379: return 'redis';
      default: return null;
    }
  }
</script>

<span class="driver-icon" data-driver={driver} style="--di-size: {size}px">
  {#if d === 'postgres'}
    <!-- Postgres: simplified elephant head — trunk, tusk, ear -->
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <path d="M7 3c-2 0-3 1.5-3 4 0 1.5.5 2.5 1 3-.5 2-.5 4 0 6 .5 1.5 1.5 3 3 4"/>
      <path d="M17 3c2 0 3 1.5 3 4 0 1.5-.5 2.5-1 3 .5 2 .5 4 0 6-.5 1.5-1.5 3-3 4"/>
      <path d="M10 6c0 1-.5 2-1 2.5M14 6c0 1 .5 2 1 2.5"/>
      <path d="M8 14c-.5 1 0 2 1 2.5M16 14c.5 1 0 2-1 2.5"/>
      <path d="M10 19v1.5M14 19v1.5"/>
    </svg>
  {:else if d === 'mysql'}
    <!-- MySQL: simplified dolphin arc + tail — clean curve -->
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <path d="M4 18c1-4 4-8 8-9 2-.5 4 0 5 1"/>
      <path d="M17 10c1 1 2 3 1.5 5"/>
      <path d="M4 18c0-1-.5-2 0-3"/>
      <path d="M9 9c-.5-2 0-4 1.5-5.5"/>
      <circle cx="11" cy="7" r="0.8" fill="currentColor"/>
      <path d="M15 15c0 2-.5 3.5-2 4.5"/>
    </svg>
  {:else if d === 'sqlite'}
    <!-- SQLite: file + feather quill — the SQLite logo mark -->
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <path d="M6 2h8l6 6v14H6V2z"/>
      <path d="M14 2v6h6"/>
      <path d="M10 18c1-2 3-4 5-5"/>
      <path d="M12 16c.5-1 1.5-2 3-3"/>
    </svg>
  {:else if d === 'mssql'}
    <!-- SQL Server: stacked cylinders (the actual SSMS icon style) -->
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <ellipse cx="12" cy="5" rx="7" ry="2.5"/>
      <path d="M5 5v4.5c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5V5"/>
      <path d="M5 9.5v4.5c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5V9.5"/>
      <path d="M5 14v4c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5v-4"/>
      <path d="M5 18v1c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5v-1"/>
    </svg>
  {:else if d === 'oracle'}
    <!-- Oracle: bold horizontal pill / eye shape — the Oracle "O" -->
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <rect x="3" y="7" width="18" height="10" rx="5"/>
    </svg>
  {:else if d === 'mongodb'}
    <!-- MongoDB: diamond leaf — the Mongo leaf diamond mark -->
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <path d="M12 2c-4 3-6 7-6 11 0 3 2 6 6 8 4-2 6-5 6-8 0-4-2-8-6-11z"/>
      <path d="M12 2v19"/>
      <path d="M9 10l3-2 3 2"/>
      <path d="M8 14l4-2 4 2"/>
    </svg>
  {:else if d === 'redis'}
    <!-- Redis: stacked rhombus — the Redis diamond/speed layers -->
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <path d="M3 8l9 4 9-4"/>
      <path d="M3 12l9 4 9-4"/>
      <path d="M3 16l9 4 9-4"/>
      <path d="M3 8v4M21 8v4M3 12v4M21 12v4"/>
    </svg>
  {:else}
    <!-- Generic: database cylinder -->
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <ellipse cx="12" cy="6" rx="7" ry="3"/>
      <path d="M5 6v6c0 1.7 3.1 3 7 3s7-1.3 7-3V6"/>
      <path d="M5 12v6c0 1.7 3.1 3 7 3s7-1.3 7-3v-6"/>
    </svg>
  {/if}
</span>

<style>
  .driver-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: var(--di-size);
    height: var(--di-size);
    color: var(--text-muted);
  }
  /* Per-driver accent tints — subtle, not loud. */
  .driver-icon[data-driver='postgres'] { color: #336791; }
  .driver-icon[data-driver='mysql'] { color: #00758f; }
  .driver-icon[data-driver='sqlite'] { color: #003b57; }
  .driver-icon[data-driver='mssql'] { color: #e24857; }
  .driver-icon[data-driver='oracle'] { color: #f80000; }
  .driver-icon[data-driver='mongodb'] { color: #4db33d; }
  .driver-icon[data-driver='redis'] { color: #d82c20; }
</style>
