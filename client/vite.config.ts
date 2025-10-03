import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    port: 5173,
    host: '0.0.0.0',
    proxy: {
      '/auth': 'http://localhost:8080',
      '/inputs': 'http://localhost:8080',
      '/ws': 'http://localhost:8080',
      '/rtc': 'http://localhost:8080'
    }
  }
});
