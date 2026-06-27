export interface ThemeDef {
  value: string;
  label: string;
  bg: string;
  fg: string;
  accent: string;
  isDark: boolean;
}

export const THEMES: ThemeDef[] = [
  { value: 'darcula', label: 'Darcula', bg: '#2b2b2b', fg: '#bbbbbb', accent: '#4a9eff', isDark: true },
  { value: 'catppuccin-mocha', label: 'Catppuccin Mocha', bg: '#1e1e2e', fg: '#cdd6f4', accent: '#cba6f7', isDark: true },
  { value: 'nord', label: 'Nord', bg: '#2e3440', fg: '#d8dee9', accent: '#88c0d0', isDark: true },
  { value: 'one-dark', label: 'One Dark', bg: '#282c34', fg: '#abb2bf', accent: '#61afef', isDark: true },
  { value: 'solarized-dark', label: 'Solarized Dark', bg: '#002b36', fg: '#839496', accent: '#268bd2', isDark: true },
  { value: 'solarized-light', label: 'Solarized Light', bg: '#fdf6e3', fg: '#586e75', accent: '#268bd2', isDark: false },
];

export function findTheme(value: string): ThemeDef {
  return THEMES.find((t) => t.value === value) ?? THEMES[0];
}

function adj(hex: string, amount: number): string {
  const num = parseInt(hex.slice(1), 16);
  const r = Math.min(255, Math.max(0, ((num >> 16) & 0xff) + amount));
  const g = Math.min(255, Math.max(0, ((num >> 8) & 0xff) + amount));
  const b = Math.min(255, Math.max(0, (num & 0xff) + amount));
  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, '0')}`;
}

export function applyAppTheme(t: ThemeDef) {
  const r = document.documentElement;
  const d = t.isDark;

  r.style.setProperty('--bg', t.bg);
  r.style.setProperty('--bg-elev', adj(t.bg, d ? 8 : -8));
  r.style.setProperty('--bg-input', adj(t.bg, d ? 16 : -12));
  r.style.setProperty('--bg-input-focus', adj(t.bg, d ? 24 : -16));
  r.style.setProperty('--bg-hover', adj(t.bg, d ? 4 : -4));
  r.style.setProperty('--bg-stripe', adj(t.bg, d ? 4 : -3));
  r.style.setProperty('--border', adj(t.bg, d ? 12 : -10));
  r.style.setProperty('--border-strong', adj(t.bg, d ? 30 : -20));
  r.style.setProperty('--text', t.fg);
  r.style.setProperty('--text-muted', adj(t.fg, d ? -40 : 40));
  r.style.setProperty('--text-faint', adj(t.fg, d ? -80 : 80));
  r.style.setProperty('--accent', t.accent);
  r.style.setProperty('--accent-emphasis', adj(t.accent, -10));
  r.style.setProperty('--accent-soft', t.accent + '22');
  r.style.setProperty('--success', d ? '#629755' : '#4c8a3d');
  r.style.setProperty('--error', d ? '#bc3c3c' : '#c72525');
  r.style.setProperty('--warning', d ? '#cc7832' : '#b8661a');
  r.style.setProperty('--info', d ? '#6897bb' : '#4d7a9e');
}
