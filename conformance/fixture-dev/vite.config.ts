import { fileURLToPath } from 'node:url';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { FIXTURE_DEV_PORT } from './src/lib/contract.ts';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    host: '127.0.0.1',
    port: FIXTURE_DEV_PORT,
    strictPort: true,
    fs: {
      allow: [fileURLToPath(new URL('..', import.meta.url))],
    },
  },
});
