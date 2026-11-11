#!/usr/bin/env node
// Serves qa/ on 127.0.0.1 for engine spikes and e2e tests. Usage: node qa/serve.mjs [port=8787]
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = path.dirname(fileURLToPath(import.meta.url));
const port = Number(process.argv[2] || 8787);
const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.gif': 'image/gif', '.txt': 'text/plain; charset=utf-8', '.json': 'application/json' };
createServer(async (req, res) => {
  const url = new URL(req.url, 'http://x');
  let p = path.normalize(path.join(root, decodeURIComponent(url.pathname)));
  if (!p.startsWith(root)) return res.writeHead(403).end();
  if (p.endsWith(path.sep)) p = path.join(p, 'index.html');
  try {
    const body = await readFile(p);
    res.writeHead(200, { 'content-type': types[path.extname(p)] || 'application/octet-stream', 'cache-control': 'no-store' });
    res.end(body);
  } catch {
    res.writeHead(404).end('not found');
  }
}).listen(port, '127.0.0.1', () => console.log(`qa fixtures on http://127.0.0.1:${port}/`));
