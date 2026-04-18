import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const host = (globalThis as any).process?.env?.TAURI_DEV_HOST as string | undefined;

export default defineConfig({
  plugins: [sveltekit(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1421,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1422 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },
});
