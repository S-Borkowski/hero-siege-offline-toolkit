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
  server: {
    port: 5177,
    strictPort: true,
    fs: { allow: ['..'] },
    // Without this, vite's watcher recurses into `src-tauri/target`, where
    // cargo is writing -- and holding open -- a hundred megabytes of build
    // output. On Windows that surfaces as `EBUSY: resource busy or locked,
    // watch ...hero_siege_toolkit_hub.exe` and takes the dev server down in
    // the middle of the rebuild that caused it.
    watch: { ignored: ['**/src-tauri/**'] },
  },
  build: { outDir: 'dist' },
});
