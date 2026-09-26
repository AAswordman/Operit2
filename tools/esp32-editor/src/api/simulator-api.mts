import {spawn} from 'node:child_process';
import type {ChildProcessWithoutNullStreams} from 'node:child_process';
import {createInterface} from 'node:readline';
import {mkdir, readFile, writeFile, copyFile} from 'node:fs/promises';
import {randomBytes} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {pathToFileURL} from 'node:url';
import path from 'node:path';
import {networkInterfaces} from 'node:os';
import type {IncomingMessage, ServerResponse} from 'node:http';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const stateDir = process.env.OPERIT_SIM_STATE_DIR
  ? pathToFileURL(path.resolve(process.env.OPERIT_SIM_STATE_DIR) + path.sep)
  : new URL('../../generated/simulator/', import.meta.url);
let child: ChildProcessWithoutNullStreams | null = null;
let starting = false;
let ready = false;
let output = '';
let token = '';
let sequence = 0;
let lifecycle = 0;
let previousExit: Promise<void> = Promise.resolve();
const pending = new Map<number, {resolve: (value: unknown) => void; reject: (error: Error) => void}>();

function advertisedAddress(address: string): string {
  const [host, port] = address.split(':');
  if (host !== '0.0.0.0' && host !== '::') return address;
  const preferred = process.env.OPERIT_SIM_ADVERTISE?.trim();
  if (preferred) return `${preferred}:${port}`;

  // Windows commonly exposes Hyper-V/WSL/VPN and benchmark interfaces before
  // the actual LAN adapter. Never advertise the RFC 2544 test range or a
  // link-local address: those addresses are not reachable by the other device
  // and used to make the pairing page show e.g. 198.18.0.1:18765.
  const candidates: Array<{address: string; rank: number}> = [];
  for (const entries of Object.values(networkInterfaces())) {
    for (const entry of entries ?? []) {
      if (entry.family !== 'IPv4' || entry.internal) continue;
      const octets = entry.address.split('.').map(Number);
      if (octets.length !== 4 || octets.some(value => !Number.isInteger(value) || value < 0 || value > 255)) continue;
      const [first, second] = octets;
      if (first === 127 || first === 0 || (first === 169 && second === 254)) continue;
      if (first === 198 && second >= 18 && second <= 19) continue;
      const isPrivate = first === 10 || (first === 192 && second === 168) ||
        (first === 172 && second >= 16 && second <= 31);
      // Prefer ordinary RFC1918 LAN addresses, then other non-test IPv4
      // addresses. The lexical tie-break keeps the result deterministic.
      candidates.push({address: entry.address, rank: isPrivate ? 0 : 1});
    }
  }
  candidates.sort((left, right) => left.rank - right.rank || left.address.localeCompare(right.address));
  return candidates[0] ? `${candidates[0].address}:${port}` : address;
}

function failPending(): void {
  for (const request of pending.values()) request.reject(new Error('模拟设备已停止'));
  pending.clear();
}

async function start(): Promise<void> {
  if (child || starting) return;
  starting = true;
  const generation = ++lifecycle;
  try {
    await previousExit;
    await mkdir(stateDir, {recursive: true});
    const tokenPath = new URL('token', stateDir);
    try { token = await readFile(tokenPath, 'utf8'); }
    catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
      token = randomBytes(24).toString('hex');
      await writeFile(tokenPath, token, {mode: 0o600, flag: 'wx'});
    }
    if (generation !== lifecycle) return;
    output = '';
    ready = false;
    // Spawn the binary directly so stop and editor shutdown own the real process.
    const build = spawn('cargo', ['build', '--manifest-path', 'tools/esp32-editor/simulator/Cargo.toml',
      '--target-dir', fileURLToPath(new URL('../../simulator/target/', import.meta.url))], {
      cwd: root, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'],
    });
    child = build;
    const append = (chunk: Buffer): void => { output = (output + chunk.toString()).slice(-12000); };
    build.stdout.on('data', append);
    build.stderr.on('data', append);
    await new Promise<void>((resolve, reject) => {
      build.once('error', reject);
      build.once('close', code => code === 0 ? resolve() : reject(new Error('模拟器编译失败，请查看日志')));
    });
    if (child !== build) return;
    const executable = fileURLToPath(new URL('../../simulator/target/debug/operit-esp32-simulator' +
      (process.platform === 'win32' ? '.exe' : ''), import.meta.url));
    const runPath = fileURLToPath(new URL('runtime' + (process.platform === 'win32' ? '.exe' : ''), stateDir));
    await copyFile(executable, runPath);
    if (generation !== lifecycle) return;
    const runtime = spawn(runPath, [], {cwd: root, windowsHide: true,
      env: {...process.env, OPERIT_SIM_TOKEN: token,
        OPERIT_SIM_STATE: fileURLToPath(new URL('pairing.json', stateDir))},
    });
    child = runtime;
    previousExit = new Promise(resolve => runtime.once('close', () => resolve()));
    runtime.stdin.on('error', e => { output = (output + e.message).slice(-12000); failPending(); });
    runtime.stderr.on('data', append);
    createInterface({input: runtime.stdout}).on('line', line => {
      try {
        const message = JSON.parse(line) as {ready?: boolean; id?: number; value?: unknown; error?: string};
        if (message.ready) ready = true;
        if (message.id !== undefined) {
          const request = pending.get(message.id);
          pending.delete(message.id);
          if (message.error) request?.reject(new Error(message.error));
          else request?.resolve(message.value);
        }
      } catch { output = (output + line).slice(-12000); }
    });
    runtime.on('error', e => { output += e.message; });
    runtime.on('close', () => {
      if (child === runtime) { child = null; ready = false; failPending(); }
    });
  } catch (e) { child = null; output += String(e); }
  finally { starting = false; }
}

export function stopSimulator(): void {
  lifecycle += 1;
  const processToStop = child;
  child = null;
  ready = false;
  failPending();
  processToStop?.kill();
}

async function rpc(command: string, fields = {}): Promise<unknown> {
  if (!child || !ready) throw new Error('请先启动模拟设备并等待编译完成');
  const id = ++sequence;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(id); reject(new Error('模拟设备响应超时')); }, 5000);
    pending.set(id, {resolve: value => { clearTimeout(timer); resolve(value); },
      reject: error => { clearTimeout(timer); reject(error); }});
    child!.stdin.write(JSON.stringify({id, command, ...fields}) + '\n');
  });
}

export async function simulatorRoute(req: IncomingMessage, res: ServerResponse, url: URL): Promise<boolean> {
  if (!url.pathname.startsWith('/api/simulator')) return false;
  if (![ `localhost:${req.socket.localPort}`, `127.0.0.1:${req.socket.localPort}` ].includes(req.headers.host ?? '') ||
      (req.headers.origin && req.headers.origin !== `http://${req.headers.host}`) ||
      req.headers['sec-fetch-site'] === 'cross-site') {
    res.writeHead(403).end(); return true;
  }
  const reply = (code: number, value: unknown): void => {
    res.writeHead(code, {'Content-Type': 'application/json', 'Cache-Control': 'no-store'});
    res.end(JSON.stringify(value));
  };
  try {
    if (url.pathname === '/api/simulator/start' && req.method === 'POST') {
      void start(); reply(202, {ok: true});
    } else if (url.pathname === '/api/simulator/stop' && req.method === 'POST') {
      stopSimulator(); reply(200, {ok: true});
    } else if (url.pathname === '/api/simulator/state' && req.method === 'GET') {
      const device = ready ? await rpc('state') : null;
      if (device && typeof device === 'object' && device !== null && 'address' in device) {
        const raw = device as {address?: unknown};
        if (typeof raw.address === 'string') raw.address = advertisedAddress(raw.address);
      }
      reply(200, {running: !!child || starting, ready, output, token: ready ? token : '', device});
    } else if (url.pathname === '/api/simulator/debug/tree' && req.method === 'GET') {
      reply(200, await rpc('debug_tree'));
    } else if (url.pathname === '/api/simulator/debug/snapshot' && req.method === 'GET') {
      reply(200, await rpc('debug_snapshot'));
    } else if (['/api/simulator/debug/tap', '/api/simulator/debug/swipe'].includes(url.pathname) && req.method === 'POST') {
      let body = '';
      for await (const chunk of req) {
        body += String(chunk);
        if (Buffer.byteLength(body) > 8192) throw new Error('请求过长');
      }
      const input = JSON.parse(body) as {id?: unknown; direction?: unknown};
      if (url.pathname.endsWith('/tap')) {
        if (typeof input.id !== 'string' || !input.id) throw new Error('缺少控件 id');
        reply(200, await rpc('debug_tap', {nodeId: input.id}));
      } else {
        if (typeof input.direction !== 'string' || !input.direction) throw new Error('缺少滑动方向');
        reply(200, await rpc('debug_swipe', {direction: input.direction}));
      }
    } else if (['/api/simulator/action', '/api/simulator/send'].includes(url.pathname) && req.method === 'POST') {
      let body = '';
      for await (const chunk of req) {
        body += String(chunk);
        if (Buffer.byteLength(body) > 8192) throw new Error('消息过长');
      }
      const input = JSON.parse(body) as {action?: unknown; text?: unknown};
      if (url.pathname.endsWith('/send')) {
        if (typeof input.text !== 'string') throw new Error('缺少消息内容');
        reply(200, await rpc('send', {text: input.text}));
        return true;
      }
      if (typeof input.action !== 'string') throw new Error('缺少设备 action');
      reply(200, await rpc('action', {action: input.action}));
    } else reply(404, {error: 'Unknown simulator endpoint'});
  } catch (e) { reply(400, {error: String(e)}); }
  return true;
}
