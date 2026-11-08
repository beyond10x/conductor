import { defineConfig, type Plugin } from 'vitest/config';
import vue from '@vitejs/plugin-vue';

// The build the conductor binary embeds and conductord serves (crates/conductor/src/daemon/api.rs,
// story:instance-api): index.html and its
// assets under dist/, served from the root of 127.0.0.1:<daemon port> under the policy
// `default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'`, so the build
// writes no inline script or style and loads nothing from another origin.
//
// `task ui:dev` runs the Vite dev server instead (hot reload of every view): it proxies the data
// and the answers to the running daemon, CONDUCTOR_DAEMON (default its
// http://127.0.0.1:7300), sending the daemon's own Origin, and copies the daemon's per-process
// answer token into the dev page, so the Decisions page answers as it does when embedded.
const target = process.env.CONDUCTOR_DAEMON ?? 'http://127.0.0.1:7300';

/** Puts the running daemon's `<meta name="conductor-token">` into the dev server's page. */
function dashboardToken(): Plugin {
  return {
    name: 'conductor-dashboard-token',
    apply: 'serve',
    async transformIndexHtml(html) {
      try {
        const page = await (await fetch(`${target}/`)).text();
        const meta = page.match(/<meta name="conductor-token"[^>]*>/);
        return meta ? html.replace('</head>', `${meta[0]}\n</head>`) : html;
      } catch {
        return html;
      }
    },
  };
}

export default defineConfig({
  plugins: [vue(), dashboardToken()],
  base: '/',
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    proxy: {
      '/v1': { target, changeOrigin: true, headers: { origin: target } },
    },
  },
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
