import { sveltekit } from '@sveltejs/kit/vite';
import svelteOptions from './svelte-options.ts';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [tailwindcss(), sveltekit(svelteOptions)],
  server: { host: '127.0.0.1' },
  test: { include: ['src/**/*.test.ts'] }
});
