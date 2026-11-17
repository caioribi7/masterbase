import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// base relativa: o site funciona servido na raiz ou numa subpasta (ex.: GitHub Pages).
export default defineConfig({ plugins: [react()], base: './' });
