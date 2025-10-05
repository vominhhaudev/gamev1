import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    port: 5173,
    host: '0.0.0.0',
    strictPort: false,
    proxy: {
      '/auth': 'http://localhost:8080',
      '/inputs': 'http://localhost:8080',
      '/ws': 'http://localhost:8080',
      '/rtc': 'http://localhost:8080',
      '/api': 'http://localhost:8080',
      '/healthz': 'http://localhost:8080',
      '/version': 'http://localhost:8080',
      '/metrics': 'http://localhost:8080'
    }
  },
  optimizeDeps: {
    include: ['svelte', '@sveltejs/kit']
  },
  build: {
    sourcemap: true
  }
});
