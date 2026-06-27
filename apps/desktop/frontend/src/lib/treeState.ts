import { writable } from 'svelte/store';
import type { ExplorerNode } from './tauri';

// Reactive store for the explorer tree nodes.
// The tree writes to this store after mutating node properties.
export const treeNodes = writable<ExplorerNode[]>([]);
