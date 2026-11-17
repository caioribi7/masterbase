import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  // Nada de data: URI: a CSP do servidor só aceita arquivos do próprio site.
  build: { assetsInlineLimit: 0 },
  server: { proxy: { '/api': 'http://127.0.0.1:8080' } },
});
