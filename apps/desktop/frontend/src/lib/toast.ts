import { writable } from 'svelte/store';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

interface Toast {
  id: number;
  message: string;
  type: ToastType;
}

export interface NotificationEntry {
  id: number;
  message: string;
  type: ToastType;
  timestamp: number;
}

const MAX_HISTORY = 200;
const STORAGE_KEY = 'getagrip-notification-history';

export const toasts = writable<Toast[]>([]);
export const notificationHistory = writable<NotificationEntry[]>([]);
let _id = 0;

function loadHistory(): NotificationEntry[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch { return []; }
}

function saveHistory(entries: NotificationEntry[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(entries));
  } catch { /* quota exceeded — silently drop */ }
}

notificationHistory.set(loadHistory());

export function notify(message: string, type: ToastType = 'info', durationMs = 5000) {
  const id = ++_id;
  toasts.update((t) => [...t, { id, message, type }]);
  setTimeout(() => {
    toasts.update((t) => t.filter((x) => x.id !== id));
  }, durationMs);

  const entry: NotificationEntry = { id, message, type, timestamp: Date.now() };
  notificationHistory.update((h) => {
    const next = [entry, ...h].slice(0, MAX_HISTORY);
    saveHistory(next);
    return next;
  });
}

export function clearNotificationHistory() {
  notificationHistory.set([]);
  saveHistory([]);
}
