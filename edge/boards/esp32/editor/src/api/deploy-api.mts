import {spawn} from 'node:child_process';
import {readFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import type {IncomingMessage, ServerResponse} from 'node:http';
import {packLayout} from '../layout/package-layout.mts';
import {isRecord, errorMessage} from '../layout/project-model.mts';
import {buildFlashPlan, executeFlashPlan, listSerialPorts} from '../device-tools.mts';

interface FlashState {
  running: boolean;
  output: string;
  error: string | null;
}

interface DeviceCapabilities {
  protocol: number;
  board: string;
  dynamicLayout: boolean;
  imagePreview: boolean;
}

let flash: FlashState = {running: false, output: '', error: null};

/** Runs a process and returns combined stdout/stderr, throwing on non-zero exit. */
function run(command: string, args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {windowsHide: true});
    let output = '';
    child.stdout.on('data', (b: Buffer) => {
      output = (output + b).slice(-8000);
      if (flash.running) flash.output = output;
    });
    child.stderr.on('data', (b: Buffer) => {
      output = (output + b).slice(-8000);
      if (flash.running) flash.output = output;
    });
    child.on('error', reject);
    child.on('close', (code) => {
      if (code === 0) resolve(output);
      else reject(new Error(output || '命令失败 ' + code));
    });
  });
}

/** Validates a device HTTP origin and returns the request URL. */
function deviceUrl(base: string, requestPath: string): URL {
  const url = new URL(base);
  if (
    url.protocol !== 'http:' ||
    url.username ||
    url.password ||
    url.search ||
    url.hash ||
    !['', '/'].includes(url.pathname)
  ) {
    throw new Error('填写设备 HTTP 地址，例如 http://192.168.1.50');
  }
  const p = url.hostname.split('.').map(Number);
  const local = ['localhost', '127.0.0.1'].includes(url.hostname);
  const lan =
    p.length === 4 &&
    p.every((n) => Number.isInteger(n) && n >= 0 && n <= 255) &&
    (p[0] === 10 || (p[0] === 192 && p[1] === 168) || (p[0] === 172 && p[1] >= 16 && p[1] <= 31));
  if (!local && !lan) throw new Error('设备地址需为本机或局域网 IPv4 地址');
  return new URL(requestPath, url);
}

/** Calls a device HTTP JSON endpoint. */
async function device(base: string, requestPath: string): Promise<DeviceCapabilities> {
  const res = await fetch(deviceUrl(base, requestPath), {redirect: 'error', signal: AbortSignal.timeout(15000)});
  const body = await res.text();
  if (!res.ok) {
    throw new Error(
      res.status === 404
        ? '设备未提供 UI 能力接口'
        : `设备 HTTP ${res.status}: ${body.slice(0, 200)}`,
    );
  }
  return JSON.parse(body) as DeviceCapabilities;
}

/** JSON response helper for deploy HTTP routes. */
function reply(res: ServerResponse, status: number, value: unknown): void {
  res.writeHead(status, {'Content-Type': 'application/json', 'Cache-Control': 'no-store'});
  res.end(JSON.stringify(value));
}

/** Reads a JSON request body with a hard size cap. */
async function readJson(req: IncomingMessage): Promise<unknown> {
  const chunks: Buffer[] = [];
  let size = 0;
  for await (const chunk of req) {
    const buffer = typeof chunk === 'string' ? Buffer.from(chunk) : chunk;
    size += buffer.length;
    if (size > 128 * 1024) throw new Error('请求超过 128 KiB');
    chunks.push(buffer);
  }
  return JSON.parse(Buffer.concat(chunks).toString('utf8')) as unknown;
}

/** Serves project export, capability discovery and firmware flashing. */
export async function deployRoute(req: IncomingMessage, res: ServerResponse, url: URL): Promise<boolean> {
  if (!url.pathname.startsWith('/api/deploy/')) return false;
  try {
    if (req.method === 'GET' && url.pathname === '/api/deploy/flash') {
      reply(res, 200, flash);
      return true;
    }
    if (req.method === 'GET' && url.pathname === '/api/deploy/ports') {
      reply(res, 200, {ports: await listSerialPorts()});
      return true;
    }
    if (req.method === 'GET' && url.pathname === '/api/deploy/device') {
      const address = url.searchParams.get('address');
      if (!address) throw new Error('填写设备 HTTP 地址，例如 http://192.168.1.50');
      reply(res, 200, await device(address, '/ui/capabilities'));
      return true;
    }
    if (req.method !== 'POST') {
      reply(res, 405, {error: 'POST required'});
      return true;
    }
    if (['/api/deploy/layout', '/api/deploy/usb-layout'].includes(url.pathname)) {
      reply(res, 422, {error: '当前固定自绘 UI 不支持动态布局部署；可导出项目或更新固件'});
      return true;
    }
    const input = await readJson(req);
    if (!isRecord(input)) throw new Error('请求必须是对象');
    if (url.pathname === '/api/deploy/package') {
      const data = packLayout(input.document);
      res.writeHead(200, {
        'Content-Type': 'application/octet-stream',
        'Content-Disposition': 'attachment; filename="operit-ui.oui"',
        'Cache-Control': 'no-store',
      });
      res.end(data);
      return true;
    }
    if (url.pathname === '/api/deploy/flash') {
      if (flash.running) throw new Error('正在烧录，请等待完成');
      if (input.resetData !== undefined && typeof input.resetData !== 'boolean') throw new Error('resetData 必须是布尔值');
      if (input.resetData === true && input.confirmReset !== true) {
        throw new Error('清空设备会删除 Wi-Fi、配对身份、空间状态和布局；必须明确确认');
      }
      const ports = await listSerialPorts();
      if (typeof input.port !== 'string' || !ports.some(port => port.port === input.port)) {
        throw new Error('串口已断开，请刷新串口列表');
      }
      if (flash.running) throw new Error('另一项串口任务已开始，请等待完成');
      const resetData = input.resetData === true;
      const plan = await buildFlashPlan(input.port, {resetData, confirmReset: input.confirmReset === true});
      if (flash.running) throw new Error('另一项串口任务已开始，请等待完成');
      flash = {running: true, output: resetData ? '正在清空设备并安装基础固件' : '正在清理程序分区并更新基础固件（保留 NVS/布局）', error: null};
      void (async () => {
        try {
          await executeFlashPlan(plan, run);
          flash.output = resetData
            ? '清空并安装完成；请重新配置 Wi-Fi 和配对，旧身份/空间状态/布局已删除'
            : '基础运行时更新完成；程序分区已清理，Wi-Fi、配对、空间状态和布局已保留';
        } catch (error) {
          flash.error = errorMessage(error);
        } finally {
          flash.running = false;
        }
      })();
      reply(res, 202, flash);
      return true;
    }
    reply(res, 404, {error: 'Unknown deployment action'});
  } catch (error) {
    reply(res, 400, {error: errorMessage(error)});
  }
  return true;
}
