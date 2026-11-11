// Serves web/apps/<app>/dist at /<app>/ on 127.0.0.1 (a trusted bridge host).
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const apps = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../apps');
const port = Number(process.argv[2] || 4317);
const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.woff2': 'font/woff2', '.json': 'application/json', '.svg': 'image/svg+xml' };
createServer(async (req, res) => {
  const url = new URL(req.url ?? '/', 'http://x');
  const [, app, ...rest] = url.pathname.split('/');
  const file = path.normalize(path.join(apps, app ?? '', 'dist', rest.join('/') || 'index.html'));
  if (!file.startsWith(apps)) return res.writeHead(403).end();
  try {
    const body = await readFile(file);
    res.writeHead(200, { 'content-type': types[path.extname(file)] ?? 'application/octet-stream' });
    res.end(body);
  } catch {
    res.writeHead(404).end('not found');
  }
}).listen(port, '127.0.0.1', () => console.log(`apps on http://127.0.0.1:${port}/`));
