import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
export const buildDir = path.resolve(here, '../website/build');
export const base = '/conductor/';
const types = {
  '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css',
  '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.xml': 'application/xml',
  '.woff2': 'font/woff2', '.woff': 'font/woff', '.ico': 'image/x-icon', '.txt': 'text/plain', '.webp': 'image/webp',
};

export function startServer(port = 0) {
  const server = http.createServer((req, res) => {
    let p = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    if (!p.startsWith(base)) { res.writeHead(404); res.end('not found'); return; }
    p = p.slice(base.length - 1);
    const candidates = [p, p + '.html', path.join(p, 'index.html')];
    for (const c of candidates) {
      const f = path.join(buildDir, c);
      if (!f.startsWith(buildDir)) continue;
      if (fs.existsSync(f) && fs.statSync(f).isFile()) {
        res.writeHead(200, {'content-type': types[path.extname(f)] ?? 'application/octet-stream'});
        fs.createReadStream(f).pipe(res);
        return;
      }
    }
    res.writeHead(404, {'content-type': 'text/html'});
    const nf = path.join(buildDir, '404.html');
    res.end(fs.existsSync(nf) ? fs.readFileSync(nf) : 'not found');
  });
  return new Promise((resolve) => server.listen(port, '127.0.0.1', () => resolve({server, url: `http://127.0.0.1:${server.address().port}`})));
}
