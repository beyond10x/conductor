// Serves the committed build, crates/conductor/dashboard/dist/, on a loopback port as conductord
// serves it (crates/conductor/src/daemon/api.rs): index.html with the answer token in its
// `<meta name="conductor-token">`, every file under the dashboard's Content-Security-Policy. The
// instance API below `/v1/` is not served here: each page answers it from its own store fixture
// (measure.ts), so this server answers 404 there.
import { createServer, type Server } from 'node:http';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { extname, join, normalize } from 'node:path';
import { DIST_DIR, TOKEN } from './adapter.ts';

/** The policy the daemon serves the app under (crates/conductor/src/dashboard.rs). */
export const CONTENT_SECURITY_POLICY = "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'";

const TYPES: Record<string, string> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.ico': 'image/x-icon',
  '.woff2': 'font/woff2',
};

export function startServer(): Promise<{ server: Server; url: string }> {
  const index = readFileSync(join(DIST_DIR, 'index.html'), 'utf8').replace(
    '</head>',
    `<meta name="conductor-token" content="${TOKEN}"></head>`,
  );
  const server = createServer((request, response) => {
    const path = decodeURIComponent(new URL(request.url ?? '/', 'http://x').pathname);
    const headers = { 'content-security-policy': CONTENT_SECURITY_POLICY };
    if (path === '/' || path === '/index.html') {
      response.writeHead(200, { ...headers, 'content-type': TYPES['.html'] });
      response.end(index);
      return;
    }
    const file = normalize(join(DIST_DIR, path));
    if (!path.startsWith('/v1/') && file.startsWith(DIST_DIR) && existsSync(file) && statSync(file).isFile()) {
      response.writeHead(200, { ...headers, 'content-type': TYPES[extname(file)] ?? 'application/octet-stream' });
      response.end(readFileSync(file));
      return;
    }
    response.writeHead(404, { ...headers, 'content-type': 'text/plain' });
    response.end('not found\n');
  });
  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      const port = typeof address === 'object' && address !== null ? address.port : 0;
      resolve({ server, url: `http://127.0.0.1:${port}` });
    });
  });
}
