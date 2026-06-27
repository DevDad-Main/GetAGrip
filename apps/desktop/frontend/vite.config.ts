import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { sveltePreprocess } from 'svelte-preprocess';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte({ preprocess: sveltePreprocess() })],
  // Tauri expects a fixed port during dev; fail fast if it's taken.
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
    target: 'es2022',
    sourcemap: false,
    minify: 'esbuild',
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      $lib: resolve(__dirname, 'src/lib'),
    },
  },
  // Monaco ships CommonJS; Vite needs to pre-bundle it.
  optimizeDeps: {
    include: ['monaco-editor'],
  },
});
