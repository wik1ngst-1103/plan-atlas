import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig(({ mode }) => ({
  base: './',
  plugins: [svelte()],
  build: { outDir: 'dist' },
  server: {
    proxy: {
      // PLAN_ATLAS_API points the dev proxy at another backend (e.g. a scratch instance).
      '/api': { target: process.env.PLAN_ATLAS_API ?? 'http://127.0.0.1:7420', changeOrigin: true },
    },
  },
  resolve: mode === 'test' ? { conditions: ['browser'] } : undefined,
  test: { environment: 'jsdom', include: ['src/**/*.test.ts'] },
}));
