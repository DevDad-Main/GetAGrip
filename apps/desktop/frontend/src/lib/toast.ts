import { writable } from 'svelte/store';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

interface Toast {
  id: number;
  message: string;
  type: ToastType;
}

export const toasts = writable<Toast[]>([]);
let _id = 0;

export function notify(message: string, type: ToastType = 'info', durationMs = 5000) {
  const id = ++_id;
  toasts.update((t) => [...t, { id, message, type }]);
  setTimeout(() => {
    toasts.update((t) => t.filter((x) => x.id !== id));
  }, durationMs);
}
