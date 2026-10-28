import {simulatorBuildArguments, simulatorExecutable} from '../simulator-build.mts';
import {requestUi, takeUiCommands, completeUiCommand} from './ui-debug.mts';
import {spawn} from 'node:child_process';
import type {ChildProcessWithoutNullStreams} from 'node:child_process';
import {createInterface} from 'node:readline';
import {createServer} from 'node:net';
import {randomUUID} from 'node:crypto';
import type {Server, Socket} from 'node:net';
import {mkdir, copyFile, unlink} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {pathToFileURL} from 'node:url';
import path from 'node:path';
import {networkInterfaces} from 'node:os';
import type {IncomingMessage, ServerResponse} from 'node:http';

async function jsonBody(req: IncomingMessage, limit: number, message: string): Promise<unknown> {
  const chunks: Buffer[] = []; let length = 0;
  for await (const chunk of req) {
    const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    length += bytes.length;
    if (length > limit) throw new Error(message);
    chunks.push(bytes);
  }
  return JSON.parse(Buffer.concat(chunks).toString('utf8')) as unknown;
}

const root = fileURLToPath(new URL('../../../../../../', import.meta.url));
const stateDir = process.env.OPERIT_SIM_STATE_DIR
  ? pathToFileURL(path.resolve(process.env.OPERIT_SIM_STATE_DIR) + path.sep)
  : new URL('../../generated/simulator/', import.meta.url);
let child: ChildProcessWithoutNullStreams | null = null;
let ipc: Socket | null = null;
let ipcServer: Server | null = null;
const ipcSockets = new Set<Socket>();
let starting = false;
let ready = false;
let output = '';
let sequence = 0;
let lifecycle = 0;
let previousExit: Promise<void> = Promise.resolve();
const pending = new Map<number, {resolve: (value: unknown) => void; reject: (error: Error) => void}>();

/** Releases the editor connection and its single-use loopback listener. */
function closeIpc(): void {
  ipc = null;
  for (const connection of ipcSockets) connection.destroy();
  ipcSockets.clear();
  ipcServer?.close();
  ipcServer = null;
}

/** Accepts one simulator connection and routes unchanged JSON RPC responses. */
async function listenForIpc(generation: number, token: string): Promise<string> {
  const server = createServer(socket => {
    if (generation !== lifecycle || ipc) { socket.destroy(); return; }
    let authenticated = false;
    ipcSockets.add(socket);
    socket.setNoDelay(true);
    /** Terminates failed reads and writes while reporting the active connection error. */
    const fail = (error: Error): void => {
      if (ipc === socket) output = (output + error.message).slice(-12000);
      socket.destroy();
    };
    socket.on('error', fail);
    socket.on('close', () => {
      ipcSockets.delete(socket);
      if (ipc !== socket) return;
      ipc = null;
      ready = false;
      failPending();
    });
    createInterface({input: socket}).on('error', fail).on('line', line => {
      if (generation !== lifecycle) { socket.destroy(); return; }
      try {
        const message = JSON.parse(line) as {ipcToken?: string; ready?: boolean; id?: number; value?: unknown; error?: string};
        if (!authenticated) {
          if (message.ipcToken !== token || ipc) { socket.destroy(); return; }
          authenticated = true;
          ipc = socket;
          server.close();
          if (ipcServer === server) ipcServer = null;
          return;
        }
        if (ipc !== socket) return;
        if (message.ready) ready = true;
        if (message.id !== undefined) {
          const request = pending.get(message.id);
          pending.delete(message.id);
          if (message.error) request?.reject(new Error(message.error));
          else request?.resolve(message.value);
        }
      } catch (error) {
        output = (output + String(error)).slice(-12000);
        socket.destroy();
      }
    });
  });
  ipcServer = server;
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  if (!address || typeof address === 'string') throw new Error('模拟器 IPC 监听失败');
  return `127.0.0.1:${address.port}`;
}

/** Selects the externally reachable address advertised by the simulated device. */
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

/** Rejects outstanding commands when their simulator connection ends. */
function failPending(): void {
  for (const request of pending.values()) request.reject(new Error('模拟设备已停止'));
  pending.clear();
}

/** Windows may briefly retain an executable mapping after the old child closes.
 * Retry only transient sharing/permission errors; never hide other copy failures. */
export async function copySimulatorBinary(source: string, destination: string,
  copy: (source: string, destination: string) => Promise<void> = copyFile): Promise<void> {
  for (let attempt = 0; ; attempt += 1) {
    try { await copy(source, destination); return; }
    catch (error) {
      const code = (error as NodeJS.ErrnoException).code;
      if (attempt >= 9 || !['EBUSY', 'EPERM', 'EACCES'].includes(code ?? '')) throw error;
      await new Promise(resolve => setTimeout(resolve, 100));
    }
  }
}

/** Prepares the bounded desktop Host dependencies and owns the current simulator process. */
async function start(): Promise<void> {
  if (child || starting) return;
  starting = true;
  const generation = ++lifecycle;
  try {
    await previousExit;
    await mkdir(stateDir, {recursive: true});
    if (generation !== lifecycle) return;
    output = '';
    ready = false;
    // Spawn the binary directly so stop and editor shutdown own the real process.
    const python = path.join(root, '.venv', process.platform === 'win32' ? 'Scripts/python.exe' : 'bin/python');
    const prepare = spawn(python, ['-X', 'utf8', 'edge/boards/esp32/editor/prepare-simulator.py'], {
      cwd: root, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'],
    });
    child = prepare;
    await new Promise<void>((resolve, reject) => {
      prepare.once('error', reject);
      prepare.stderr.on('data', chunk => { output = (output + chunk.toString()).slice(-12000); });
      prepare.once('close', code => code === 0 ? resolve() : reject(new Error('模拟器依赖准备失败，请查看日志')));
    });
    if (child !== prepare || generation !== lifecycle) return;
    const build = spawn('cargo', simulatorBuildArguments, {
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
    const executable = simulatorExecutable;
    // Keep the executable separate from the packed storage carrier's runtime/
    // directory. On Unix an extensionless 'runtime' binary occupies that path.
    const runPath = fileURLToPath(new URL('operit-esp32-simulator' +
      (process.platform === 'win32' ? '.exe' : ''), stateDir));
    // Replacing an existing Mach-O inode can retain macOS's cached code
    // signature and kill the next process. The previous child has exited.
    if (process.platform === 'darwin') {
      await unlink(runPath).catch(error => { if (error.code !== 'ENOENT') throw error; });
    }
    await copySimulatorBinary(executable, runPath);
    if (generation !== lifecycle) return;
    const ipcToken = randomUUID();
    const ipcAddress = await listenForIpc(generation, ipcToken);
    if (generation !== lifecycle) { closeIpc(); return; }
    const runtime = spawn(runPath, [], {cwd: root, windowsHide: true,
      env: {...process.env,
        OPERIT_SIM_IPC_ADDRESS: ipcAddress,
        OPERIT_SIM_IPC_TOKEN: ipcToken,
        OPERIT_SIM_STATE_DIR: fileURLToPath(stateDir),
        OPERIT_SIM_DISCOVERY_IP: advertisedAddress('0.0.0.0:0').split(':')[0]},
    });
    child = runtime;
    previousExit = new Promise(resolve => runtime.once('close', () => resolve()));
    runtime.stdout.on('data', append);
    runtime.stderr.on('data', append);
    runtime.on('error', e => { output += e.message; });
    runtime.on('close', (code, signal) => {
      output = (output + `\nSimulator exited (code=${code}, signal=${signal})\n`).slice(-12000);
      if (child === runtime) { child = null; ready = false; closeIpc(); failPending(); }
    });
  } catch (e) { child = null; ready = false; closeIpc(); failPending(); output += String(e); }
  finally { starting = false; }
}

/** Stops the owned process and closes all IPC resources. */
export function stopSimulator(): void {
  lifecycle += 1;
  const processToStop = child;
  child = null;
  ready = false;
  closeIpc();
  failPending();
  processToStop?.kill();
}

/** Sends one correlated command over the current simulator connection. */
async function rpc(command: string, fields = {}, timeoutMs = 5000): Promise<unknown> {
  if (!child || !ready || !ipc) throw new Error('请先启动模拟设备并等待编译完成');
  const id = ++sequence;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(id); reject(new Error('模拟设备响应超时')); }, timeoutMs);
    pending.set(id, {resolve: value => { clearTimeout(timer); resolve(value); },
      reject: error => { clearTimeout(timer); reject(error); }});
    ipc!.write(JSON.stringify({id, command, ...fields}) + '\n');
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
      reply(200, {running: !!child || starting, ready, output, device});
    } else if (url.pathname === '/api/simulator/storage' && req.method === 'GET') {
      reply(200, await rpc('storageRecords'));
    } else if (url.pathname === '/api/simulator/storage-control' && req.method === 'POST') {
      let body = '';
      for await (const chunk of req) { body += String(chunk); if (Buffer.byteLength(body) > 1024) throw new Error('存储测试配置过长'); }
      reply(200, await rpc('storageControl', {configuration: JSON.parse(body)}));
    } else if (url.pathname === '/api/simulator/speech' && req.method === 'POST') {
      const input = await jsonBody(req, 24000, '语音请求过长') as Record<string, unknown>;
      reply(200, await rpc('speech', input));
    } else if (url.pathname === '/api/simulator/core-access'  && req.method === 'POST') {
      let body = '';
      for await (const chunk of req) {
        body += String(chunk);
        if (Buffer.byteLength(body) > 8192) throw new Error('Core 接入请求过长');
      }
      const input = JSON.parse(body) as Record<string, unknown>;
      if (!['request', 'confirm', 'refresh', 'cancel'].includes(String(input.step))) {
        throw new Error('无效的 Core 接入步骤');
      }
      reply(200, await rpc('coreAccess', {step: input.step, nodeId: input.nodeId, address: input.address,
        pairingId: input.pairingId, confirmationCode: input.confirmationCode, requestId: input.requestId}, 30000));
    } else if (url.pathname === '/api/simulator/scene-view' && req.method === 'GET') {
      reply(200, ready ? await rpc('sceneView') : null);
    } else if (url.pathname === '/api/simulator/memory' && req.method === 'GET') {
      reply(200, ready ? await rpc('memory') : null);
    } else if (url.pathname === '/api/simulator/send-image' && req.method === 'POST') {
      const sendId = req.headers['x-operit-send-id'];
      if (sendId !== undefined && (typeof sendId !== 'string' || sendId.length > 64)) throw new Error('无效的发送编号');
      const mimeType = req.headers['content-type']?.split(';', 1)[0]?.trim().toLowerCase();
      if (mimeType !== 'image/png' && mimeType !== 'image/jpeg') throw new Error('只支持 PNG/JPEG 图片');
      const chunks: Buffer[] = []; let total = 0;
      for await (const part of req) {
        const chunk = Buffer.from(part); total += chunk.length;
        if (total > 512 * 1024) throw new Error('图片不能超过 512 KiB');
        chunks.push(chunk);
      }
      if (!total) throw new Error('图片为空');
      const base64 = Buffer.concat(chunks).toString('base64');
      reply(200, await rpc('sendImage', {mimeType, bytes: base64, sendId}));
    } else if (url.pathname === '/api/simulator/debug/commands' && req.method === 'GET') {
      reply(200, takeUiCommands());
    } else if (url.pathname === '/api/simulator/debug/result' && req.method === 'POST') {
      let body = '';
      for await (const chunk of req) {
        body += String(chunk);
        if (Buffer.byteLength(body) > 131072) throw new Error('UI snapshot too large');
      }
      const result = JSON.parse(body) as {id: number; value?: unknown; error?: string};
      completeUiCommand(result.id, result.value, result.error); reply(200, {ok: true});
    } else if (url.pathname === '/api/simulator/debug/tree'  && req.method === 'GET') {
      reply(200, await requestUi('tree'));
    } else if (url.pathname === '/api/simulator/debug/snapshot' && req.method === 'GET') {
      reply(200, await requestUi('snapshot'));
    } else if (['/api/simulator/debug/tap', '/api/simulator/debug/swipe'].includes(url.pathname) && req.method === 'POST') {
      let body = '';
      for await (const chunk of req) {
        body += String(chunk);
        if (Buffer.byteLength(body) > 8192) throw new Error('请求过长');
      }
      const input = JSON.parse(body) as {id?: unknown; direction?: unknown};
      if (url.pathname.endsWith('/tap')) {
        if (typeof input.id !== 'string' || !input.id) throw new Error('缺少控件 id');
        reply(200, await requestUi('tap', {id: input.id}));
      } else {
        if (typeof input.direction !== 'string' || !input.direction) throw new Error('缺少滑动方向');
        reply(200, await requestUi('swipe', {direction: input.direction}));
      }
    } else if (['/api/simulator/action', '/api/simulator/send'].includes(url.pathname) && req.method === 'POST') {
      const input = await jsonBody(req, 100000, '消息过长') as {action?: unknown; text?: unknown; sendId?: unknown; requestId?: unknown; assignmentVersion?: unknown; inputId?: unknown};
      if (url.pathname.endsWith('/send')) {
        if (typeof input.text !== 'string') throw new Error('缺少消息内容');
        if (Buffer.byteLength(input.text) > 16384) throw new Error('消息超过 16384 字节，请拆分发送');
        if (input.sendId !== undefined && (typeof input.sendId !== 'string' || input.sendId.length > 64)) throw new Error('无效的发送编号');
        reply(200, await rpc('send', {text: input.text, sendId: input.sendId}));
        return true;
      }
      if (typeof input.action !== 'string') throw new Error('缺少设备 action');
      const action: Record<string, unknown> = {action: input.action};
      if (input.action === 'edge_space_approve' || input.action === 'edge_space_reject') {
        if (typeof input.requestId !== 'string' || !input.requestId.trim() ||
            typeof input.assignmentVersion !== 'number' || !Number.isSafeInteger(input.assignmentVersion) || input.assignmentVersion < 0) {
          throw new Error('缺少有效的设备空间申请编号或审批版本');
        }
        action.requestId = input.requestId;
        action.assignmentVersion = input.assignmentVersion;
      }
      if (input.action === 'edge_voice' && input.inputId !== undefined) {
        if (typeof input.inputId !== 'string' || input.inputId.length > 128) throw new Error('无效的麦克风输入');
        action.inputId = input.inputId;
      }
      reply(200, await rpc('action', action));
    } else reply(404, {error: 'Unknown simulator endpoint'});
  } catch (e) { reply(400, {error: String(e), running: child !== null, ready, output}); }
  return true;
}
