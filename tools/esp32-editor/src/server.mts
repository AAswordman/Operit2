import http from 'node:http';
import type {IncomingMessage, ServerResponse} from 'node:http';
import {aiRoute} from './api/ai-api.mts';
import {deployRoute} from './api/deploy-api.mts';
import {simulatorRoute, stopSimulator} from './api/simulator-api.mts';
import {layoutRoute} from './api/layout-api.mts';
import {readFile, readdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import ts from 'typescript';
import {errorMessage, errorCode} from './layout/project-model.mts';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const editorRoot = path.resolve(root, 'tools/esp32-editor');
const uiRoot = path.resolve(root, 'apps/esp32/ui_port');
const argument = process.argv.indexOf('--port');
const port = Number(argument >= 0 ? process.argv[argument + 1] : process.env.PORT || 8766);
if (!Number.isInteger(port) || port < 1 || port > 65535) {
  throw new Error('Invalid port');
}

const files = new Map<string, string>([
  ['/', 'web/index.html'],
  ['/app.js', 'web/app.ts'],
  ['/device-actions.js', 'web/device-actions.ts'],
  ['/device-state.js', 'web/device-state.ts'],
  ['/scene-renderer.js', 'web/scene-renderer.ts'],
  ['/simulator.js', 'web/simulator.ts'],
  ['/src/layout/project-model.mjs', 'src/layout/project-model.mts'],
  ['/src/layout/geometry.mjs', 'src/layout/geometry.mts'],
  ['/src/layout/routes.mjs', 'src/layout/routes.mts'],
  ['/src/layout/layout-model.mjs', 'src/layout/layout-model.mts'],
  ['/src/layout/package-layout.mjs', 'src/layout/package-layout.mts'],
  ['/src/source/component-context.mjs', 'src/source/component-context.mts'],
  ['/pages.js', 'web/pages.ts'],
  ['/ai.js', 'web/ai.ts'],
  ['/shell.js', 'web/shell.ts'],
  ['/transport.js', 'web/transport.ts'],
  ['/types.js', 'web/types.ts'],
  ['/interactions.js', 'web/interactions.ts'],
  ['/model.js', 'web/model.ts'],
  ['/style.css', 'web/style.css'],
  ['/editor.js', 'web/editor.ts'],
  ['/generated/ui.mjs', 'generated/ui.mjs'],
  ['/generated/ui.wasm', 'generated/ui.wasm'],
  ['/generated/manifest.json', 'generated/manifest.json'],
  ['/deploy.js', 'web/deploy.ts'],
]);

const mime: Record<string, string> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.json': 'application/json',
  '.css': 'text/css; charset=utf-8',
};

let running = false;
let error: string | null = null;
let queued = false;
let output = '';

interface BuildManifest {
  runtimeHash?: string;
}

function toBrowserModule(sourceText: string, fileName: string): string {
  const result = ts.transpileModule(sourceText, {
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ESNext,
      verbatimModuleSyntax: true,
    },
    fileName,
  });
  return result.outputText.replace(/(\bfrom\s+['"][^'"]+)\.mts(['"])/g, '$1.mjs$2');
}

async function hash(): Promise<string> {
  const names = await readdir(uiRoot);
  const hashed = [
    ...names.filter((n: string) => n.endsWith('.c')).sort(),
    ...names.filter((n: string) => n.endsWith('.h')).sort(),
    ...names.filter((n: string) => n.endsWith('.inc')).sort(),
    ...names.filter((n: string) => n.endsWith('.svg')).sort(),
  ];
  const digest = createHash('sha256');
  for (const n of hashed) {
    digest.update(await readFile(path.join(uiRoot, n)));
  }
  const rust = path.resolve(uiRoot, '../src');
  for (const n of (await readdir(rust)).filter((name: string) => name.endsWith('.rs')).sort()) {
    digest.update(await readFile(path.join(rust, n)));
  }
  digest.update(await readFile(path.resolve(uiRoot, '../partitions.csv')));
  return digest.digest('hex');
}

async function status(): Promise<{
  running: boolean;
  error: string | null;
  manifest: BuildManifest | null;
  stale: boolean;
  output: string;
}> {
  let manifest: BuildManifest | null = null;
  try {
    manifest = JSON.parse(await readFile(path.join(editorRoot, 'generated/manifest.json'), 'utf8')) as BuildManifest;
  } catch {
    manifest = null;
  }
  return {
    running,
    error,
    manifest,
    stale: !manifest || manifest.runtimeHash !== await hash(),
    output: output.slice(-3000),
  };
}

function build(): void {
  if (running) {
    queued = true;
    return;
  }
  running = true;
  error = null;
  output = '';
  const child = spawn(process.execPath, ['--experimental-strip-types', path.join(editorRoot, 'src/build.mts'), '--firmware'], {
    cwd: editorRoot,
    windowsHide: true,
  });
  const append = (d: Buffer): void => {
    output = (output + d.toString()).slice(-16000);
  };
  child.stdout.on('data', append);
  child.stderr.on('data', append);
  child.on('error', (e: Error) => {
    error = e.message;
    running = false;
  });
  child.on('close', (code: number | null) => {
    running = false;
    if (code !== 0) error = '查看 tools/esp32-editor/generated 的构建日志；' + output.slice(-700);
    if (queued) {
      queued = false;
      build();
    }
  });
}

function isLocalEditor(req: IncomingMessage): boolean {
  const host = req.headers.host;
  const origin = req.headers.origin;
  if (!host || ![`127.0.0.1:${port}`, `localhost:${port}`].includes(host)) return false;
  if (origin && origin !== `http://${host}`) return false;
  return true;
}

const server = http.createServer(async (req: IncomingMessage, res: ServerResponse) => {
  const url = new URL(req.url ?? '/', 'http://localhost');
  try {
    if (!req.method || !['GET', 'HEAD'].includes(req.method)) {
      if (!isLocalEditor(req)) {
        res.writeHead(403);
        res.end();
        return;
      }
    }
    if (await simulatorRoute(req, res, url)) return;
    if (await aiRoute(req, res, url)) return;
    if (await deployRoute(req, res, url)) return;
    if (await layoutRoute(req, res, url)) return;

    if (url.pathname === '/api/build') {
      if (req.method === 'POST') {
        if (!isLocalEditor(req)) {
          res.writeHead(403);
          res.end();
          return;
        }
        if (!running) build();
      } else if (req.method !== 'GET') {
        res.writeHead(405);
        res.end();
        return;
      }
      res.writeHead(200, {'Content-Type': 'application/json', 'Cache-Control': 'no-store'});
      res.end(JSON.stringify(await status()));
      return;
    }

    if (req.method !== 'GET') {
      res.writeHead(405);
      res.end();
      return;
    }

    if (url.pathname === '/api/board') {
      // Fixed renderer palette; no retired widget-theme parser or runtime dependency.
      const themes = [{name: 'Mini', bg: '#101820', surface: '#263849',
        accent: '#e6edf3', muted: '#73818d'}];
      res.writeHead(200, {'Content-Type': 'application/json', 'Cache-Control': 'no-store'});
      res.end(
        JSON.stringify({
          model: 'ESP32-2432S028',
          width: 320,
          height: 240,
          controller: 'ST7789',
          themes,
          mode: 'shared-mini-wasm',
          capabilities: {layoutEditing: false, images: false, keyboard: false, themes: false},
        }),
      );
      return;
    }

    const file = files.get(url.pathname);
    if (!file) {
      res.writeHead(404);
      res.end('Not found');
      return;
    }
    const abs = path.join(editorRoot, file);
    const raw = await readFile(abs);
    const body = /\.[cm]?ts$/.test(file) ? toBrowserModule(raw.toString('utf8'), abs) : raw;
    const ext = path.extname(url.pathname) || '.html';
    const contentType = mime[ext];
    if (!contentType) throw new Error('Unknown MIME type for ' + url.pathname);
    res.writeHead(200, {
      'Content-Type': contentType,
      'Cache-Control': 'no-store',
      'X-Content-Type-Options': 'nosniff',
    });
    res.end(body);
  } catch (e) {
    res.writeHead(errorCode(e) === 'ENOENT' ? 404 : 500, {'Content-Type': 'text/plain; charset=utf-8'});
    res.end(errorMessage(e));
  }
});

server.on('error', (e: Error) => {
  console.error(e);
  process.exit(1);
});
process.on('exit', stopSimulator);
for (const signal of ['SIGINT', 'SIGTERM'] as const) {
  process.on(signal, () => { stopSimulator(); server.close(); process.exit(0); });
}
server.listen(port, '127.0.0.1', () => console.log(`ESP32 editor running at http://127.0.0.1:${port}`));
