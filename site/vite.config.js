import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// base relativa: o site funciona servido na raiz ou numa subpasta (ex.: GitHub Pages).
// fs.allow: a página de termos lê docs/termos-de-uso.md, fora da pasta do site.
export default defineConfig({ plugins: [react()], base: './', server: { fs: { allow: ['..'] } } });
