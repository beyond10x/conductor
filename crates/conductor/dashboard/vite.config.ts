import { defineConfig } from 'vitest/config';
import vue from '@vitejs/plugin-vue';

// The build the conductor binary embeds (crates/conductor/src/dashboard.rs): index.html and its
// assets under dist/, served from the root of 127.0.0.1:<port> under the policy
// `default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'`, so the build
// writes no inline script or style and loads nothing from another origin.
export default defineConfig({
  plugins: [vue()],
  base: '/',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    assetsDir: 'assets',
    // No inline module-preload polyfill, no source maps in the committed build.
    modulePreload: { polyfill: false },
    sourcemap: false,
    // Every asset is a file of its own: an inlined data: URL is refused by the policy.
    assetsInlineLimit: 0,
  },
  test: {
    environment: 'jsdom',
    // The views keep their specs beside them, under src/views/__tests__/.
    include: ['tests/**/*.spec.ts', 'src/**/__tests__/**/*.spec.ts'],
  },
});
