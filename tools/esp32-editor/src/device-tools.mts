import {spawn} from 'node:child_process';
import {readFile, readdir, access} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {isRecord} from './layout/project-model.mts';

export const editorRoot = fileURLToPath(new URL('../', import.meta.url));
export const repoRoot = path.resolve(editorRoot, '../..');
export const firmwareDist = path.join(repoRoot, 'apps/esp32/dist');
export const firmwareTargetDir = path.join(path.parse(repoRoot).root, 'esp32');
export const firmwareElf = path.join(firmwareTargetDir, 'xtensa-esp32-espidf/release/operit-esp32');
export const configFile = path.join(editorRoot, 'device.local.json');
export interface DeviceConfig {port?: string; emsdk?: string; idf?: string}
export interface SerialPort {port: string; name: string; usb: boolean}
export interface FlashCommand {command: string; args: string[]}

/** Local settings contain machine paths, never device credentials. */
export async function loadDeviceConfig(): Promise<DeviceConfig> {
  let text: string;
  try { text = await readFile(configFile, 'utf8'); }
  catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') return {};
    throw error;
  }
  const value: unknown = JSON.parse(text);
  if (!isRecord(value) || Object.keys(value).some(key => !['port', 'emsdk', 'idf'].includes(key)) ||
      Object.values(value).some(entry => typeof entry !== 'string' || !entry.trim())) {
    throw new Error('device.local.json 只接受非空字符串 port / emsdk / idf');
  }
  return value as DeviceConfig;
}

/** Captures only a bounded tail; monitor can inherit the terminal without a log file. */
export function runTool(command: string, args: string[], options: {
  cwd?: string; env?: NodeJS.ProcessEnv; inherit?: boolean; onOutput?: (tail: string) => void;
} = {}): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd, env: options.env, windowsHide: true,
      stdio: options.inherit ? 'inherit' : ['ignore', 'pipe', 'pipe'],
    });
    let output = '';
    const receive = (chunk: Buffer): void => {
      output = (output + chunk.toString()).slice(-8000);
      options.onOutput?.(output);
    };
    child.stdout?.on('data', receive);
    child.stderr?.on('data', receive);
    child.on('error', reject);
    child.on('close', (code, signal) => {
      if (code === 0) resolve(output);
      else reject(new Error(`${command} 失败 (${signal ?? code})${output ? ': ' + output : ''}`));
    });
  });
}

export async function listSerialPorts(): Promise<SerialPort[]> {
  const output = await runTool('python', ['-c',
    'import json,serial.tools.list_ports; print(json.dumps([{"port":p.device,"name":p.description,"usb":p.vid is not None} for p in serial.tools.list_ports.comports()]))']);
  const ports: unknown = JSON.parse(output);
  if (!Array.isArray(ports) || !ports.every(p => isRecord(p) && typeof p.port === 'string' &&
      typeof p.name === 'string' && typeof p.usb === 'boolean')) throw new Error('串口列表格式错误');
  return ports as SerialPort[];
}

/** Never guess between several boards, or fall back from a disconnected explicit port. */
export function selectSerialPort(ports: SerialPort[], requested?: string): string {
  if (requested) {
    const match = ports.find(p => p.port.toLowerCase() === requested.toLowerCase());
    if (!match) throw new Error(`串口 ${requested} 未连接；运行 npm run ports 检查`);
    return match.port;
  }
  const usb = ports.filter(p => p.usb);
  if (usb.length !== 1) throw new Error('未找到唯一 USB 串口；连接设备，或通过 --port COMxx 明确选择');
  return usb[0]!.port;
}

/** This editor supports one fixed 4-MiB board layout. Validate before any erasure. */
export function validateFirmwareImages(table: Buffer, app: Buffer, bootloader: Buffer): void {
  const expected = [
    {name: 'nvs', type: 1, subtype: 2, offset: 0x9000, size: 0x6000},
    {name: 'phy_init', type: 1, subtype: 1, offset: 0xf000, size: 0x1000},
    {name: 'factory', type: 0, subtype: 0, offset: 0x10000, size: 0x3d0000},
    {name: 'ui_layout', type: 1, subtype: 0x82, offset: 0x3e0000, size: 0x10000},
  ];
  const entries = [];
  if (table.length < 128 || table.length > 0x1000 || table.length % 32 !== 0) throw new Error('分区表长度错误');
  for (let at = 0; at + 32 <= table.length; at += 32) {
    const magic = table.readUInt16LE(at);
    if (magic === 0xffff || magic === 0xebeb) break;
    if (magic !== 0x50aa) throw new Error('分区表格式错误');
    entries.push({name: table.subarray(at + 12, at + 28).toString().split('\0')[0],
      type: table[at + 2], subtype: table[at + 3], offset: table.readUInt32LE(at + 4),
      size: table.readUInt32LE(at + 8)});
    if (table.readUInt32LE(at + 28) !== 0) throw new Error('不支持带 flags 的分区');
  }
  if (JSON.stringify(entries) !== JSON.stringify(expected)) throw new Error('固件分区布局不匹配，拒绝烧录/擦除');
  if (app.length < 24 || app[0] !== 0xe9 || app.length > 0x3d0000) throw new Error('程序镜像无效或超出程序分区');
  if (bootloader.length < 24 || bootloader[0] !== 0xe9 || bootloader.length > 0x7000) throw new Error('bootloader 镜像无效或过大');
}

/** Update clears only the program partition; fresh is deliberately destructive. */
export async function buildFlashPlan(port: string, options: {
  resetData?: boolean; confirmReset?: boolean; dist?: string;
} = {}): Promise<FlashCommand[]> {
  if (options.resetData && options.confirmReset !== true) {
    throw new Error('清空设备会删除 Wi-Fi、配对身份、空间状态和布局；必须明确确认');
  }
  const dist = options.dist ?? firmwareDist;
  const [table, app, bootloader] = await Promise.all([
    readFile(path.join(dist, 'partition-table.bin')), readFile(path.join(dist, 'operit-esp32.bin')),
    readFile(path.join(dist, 'bootloader.bin')),
  ]);
  validateFirmwareImages(table, app, bootloader);
  const common = ['--chip', 'esp32', '--port', port, '--baud', '460800', '--skip-update-check', '--non-interactive'];
  const commands: FlashCommand[] = [{command: 'espflash', args: [
    options.resetData ? 'erase-flash' : 'erase-region', ...common, '--after', 'no-reset',
    ...(options.resetData ? [] : ['0x10000', '0x3d0000']),
  ]}];
  const images = [['0x1000', 'bootloader.bin'], ['0x8000', 'partition-table.bin'], ['0x10000', 'operit-esp32.bin']];
  for (const [index, [address, file]] of images.entries()) {
    commands.push({command: 'espflash', args: ['write-bin', ...common,
      '--after', index === images.length - 1 ? 'hard-reset' : 'no-reset', address!, path.join(dist, file!)]});
  }
  return commands;
}

/** Stop at the first failed erase/write; never boot a partly installed image intentionally. */
export async function executeFlashPlan(plan: FlashCommand[], runner = runTool): Promise<void> {
  for (const step of plan) await runner(step.command, step.args);
}

/** Discover the SDK toolchain without hardcoding machine paths or host suffixes.
 * Explicit compiler environment settings take precedence. */
export async function xtensaCompilerEnvironment(firmwareRoot: string,
  env: NodeJS.ProcessEnv = process.env, platform: string = process.platform): Promise<NodeJS.ProcessEnv> {
  const result: NodeJS.ProcessEnv = {};
  const roots = [env.IDF_TOOLS_PATH, path.join(firmwareRoot, '.embuild/espressif')].filter((value): value is string => !!value);
  const bins: string[] = [];
  for (const root of roots) {
    const base = path.join(root, 'tools/xtensa-esp-elf');
    const versions = await readdir(base).catch((error: NodeJS.ErrnoException) => {
      if (error.code === 'ENOENT') return [];
      throw error;
    });
    for (const version of versions.sort().reverse()) bins.push(path.join(base, version, 'xtensa-esp-elf/bin'));
  }
  const suffix = platform === 'win32' ? '.exe' : '';
  bins.push(...(env.PATH ?? '').split(platform === 'win32' ? ';' : ':').filter(Boolean));
  for (const [key, tool] of Object.entries({CC_xtensa_esp32_espidf: 'gcc', AR_xtensa_esp32_espidf: 'ar'})) {
    if (env[key]) continue;
    for (const bin of bins) {
      const candidate = path.join(bin, 'xtensa-esp32-elf-' + tool + suffix);
      try { await access(candidate); result[key] = candidate; break; }
      catch (error) { if (!['ENOENT', 'ENOTDIR'].includes((error as NodeJS.ErrnoException).code ?? '')) throw error; }
    }
  }
  // Clean SDKs can bootstrap their tools on the first Cargo build. Leave missing
  // entries unset so ESP-IDF/cc can use their normal discovery.
  return result;
}
