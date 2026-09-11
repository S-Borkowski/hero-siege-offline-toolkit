import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  // `tauri dev` restarts vite on every change, and vite clears the terminal as
  // it starts -- taking the cargo errors that caused the restart with it.
  clearScreen: false,
  plugins: [svelte()],
  // 5176 is HS Offline Tracker's. Both can be running during development.
  //
  // `fs.allow` reaches one level up because `bridge.js` imports
  // `catalog/catalog.json` from the repository root: the browser preview draws
  // the real library without the Rust side running, which is what makes the
  // frontend workable on its own.
  server: { port: 5177, strictPort: true, fs: { allow: ['..'] } },
  build: { outDir: 'dist' },
});
