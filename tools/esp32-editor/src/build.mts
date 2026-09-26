import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {
  copyFile,
  mkdir,
  mkdtemp,
  readdir,
  readFile,
  rm,
  stat,
  unlink,
  utimes,
  writeFile,
} from 'node:fs/promises';
import {existsSync, rmSync} from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import './compile-layout.mts';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const here = fileURLToPath(new URL('../', import.meta.url));
const ui = path.join(root, 'apps/esp32/lvgl_port');
const out = path.join(here, 'generated');
const firmwareTarget = 'xtensa-esp32-espidf';
const firmwareRoot = path.join(root, 'apps/esp32');
const shortTargetDir = path.join(path.parse(root).root, 'esp32');
const firmwareTargetDir = path.resolve(shortTargetDir);
const firmware = process.argv.includes('--firmware');
const lvglFlag = flagValue('--lvgl');
const emsdkSetting = flagValue('--emsdk') ?? process.env.EMSDK;
if (!emsdkSetting) throw new Error('EMSDK is required; set it to the installed Emscripten 4.0.14 directory or pass --emsdk');
const emsdk = path.resolve(emsdkSetting);

const exports = [
  '_operit_store_validate',
  '_simulator_init',
  '_simulator_frame',
  '_simulator_generation',
  '_simulator_heap_used',
  '_simulator_touch',
  '_operit_lvgl_pump',
  '_operit_lvgl_navigate_home',
  '_operit_lvgl_navigate_apps',
  '_operit_lvgl_set_connection',
  '_operit_lvgl_set_expression',
  '_operit_lvgl_set_pairing_code',
  '_operit_lvgl_set_space_state',
  '_operit_lvgl_set_chat_preview',
  '_operit_lvgl_set_chat_screen',
  '_operit_lvgl_set_chat_task',
  '_operit_lvgl_chat_draft',
  '_operit_lvgl_set_chat_draft',
  '_operit_lvgl_chat_send_result',
  '_operit_lvgl_submit_chat',
  '_operit_lvgl_set_theme',
  '_operit_lvgl_theme_index',
  '_operit_lvgl_round_icons',
  '_operit_lvgl_current_page',
  '_operit_lvgl_debug_tree',
  '_operit_lvgl_debug_snapshot',
  '_operit_lvgl_debug_tap',
  '_operit_lvgl_debug_swipe',
  '_operit_lvgl_layout_clear',
  '_operit_lvgl_layout_add',
  '_operit_lvgl_layout_geometry',
  '_operit_lvgl_layout_bind',
  '_operit_lvgl_layout_page_meta',
  '_operit_lvgl_layout_style',
];

/** Returns the value following a CLI flag. */
function flagValue(flag: string): string | undefined {
  const index = process.argv.indexOf(flag);
  if (index < 0) return undefined;
  const value = process.argv[index + 1];
  if (!value || value.startsWith('-')) throw new Error(flag + ' 需要路径');
  return value;
}

/** Lists files in a directory whose names end with the given suffix, sorted. */
async function filesWithSuffix(dir: string, suffix: string): Promise<string[]> {
  return (await readdir(dir))
    .filter((name) => name.endsWith(suffix))
    .sort()
    .map((name) => path.join(dir, name));
}

/** Recursively lists C sources under a directory. */
async function cSources(dir: string): Promise<string[]> {
  const found: string[] = [];
  for (const entry of await readdir(dir, {withFileTypes: true})) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) found.push(...await cSources(full));
    else if (entry.name.endsWith('.c')) found.push(full);
  }
  return found;
}

/** SHA-256 hex digest of the concatenation of the given files. */
async function hashFiles(files: string[]): Promise<string> {
  const digest = createHash('sha256');
  for (const file of files) digest.update(await readFile(file));
  return digest.digest('hex');
}

/** SHA-256 hex digest of a string or buffer. */
function sha256(value: string | Buffer): string {
  return createHash('sha256').update(value).digest('hex');
}

/** Hash of shared LVGL C sources plus the layout JSON. */
async function sharedHash(): Promise<string> {
  return hashFiles([
    ...await filesWithSuffix(ui, '.c'),
    ...await filesWithSuffix(ui, '.h'),
    path.join(root, 'apps/esp32/ui/layout.json'),
  ]);
}

/** Locates the ESP-IDF managed LVGL tree used by the firmware build. */
async function firmwareLvgl(): Promise<string> {
  const buildRoot = path.join(firmwareTargetDir, firmwareTarget, 'release', 'build');
  if (!existsSync(buildRoot)) throw new Error('LVGL source not found; build the ESP32 target once or pass --lvgl /path/to/lvgl (v9.3.0)');
  for (const entry of await readdir(buildRoot, {withFileTypes: true})) {
    if (!entry.isDirectory() || !entry.name.startsWith('esp-idf-sys-')) continue;
    const candidate = path.join(buildRoot, entry.name, 'out', 'managed_components', 'lvgl__lvgl');
    if (existsSync(candidate)) return candidate;
  }
  throw new Error('LVGL source not found; pass --lvgl /path/to/lvgl (v9.3.0)');
}

/** Resolves the Emscripten compiler command for this host. */
function emccCommand(sdk: string): string[] {
  const dir = path.join(sdk, 'upstream/emscripten');
  if (!existsSync(path.join(dir, 'emcc.py'))) throw new Error('Install/activate Emscripten 4.0.14, or pass --emsdk');
  if (process.platform === 'win32') return [path.join(dir, 'emcc.bat')];
  return [path.join(dir, 'emcc')];
}

/** Spawns a process and returns combined status and captured output. */
function run(
  command: string[],
  options: {env?: NodeJS.ProcessEnv; cwd?: string; capture?: boolean} = {},
): Promise<{code: number | null; stdout: string; stderr: string}> {
  const file = command[0];
  if (!file) throw new Error('empty command');
  const args = command.slice(1);
  const windowsBat = process.platform === 'win32' && file.toLowerCase().endsWith('.bat');
  return new Promise((resolve, reject) => {
    const child = spawn(
      windowsBat ? 'cmd.exe' : file,
      windowsBat ? ['/d', '/s', '/c', file, ...args] : args,
      {
        env: options.env,
        cwd: options.cwd,
        windowsHide: true,
      },
    );
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk: Buffer) => {
      stdout += chunk.toString();
      if (!options.capture) process.stdout.write(chunk);
    });
    child.stderr.on('data', (chunk: Buffer) => {
      stderr += chunk.toString();
      if (!options.capture) process.stderr.write(chunk);
    });
    child.on('error', reject);
    child.on('close', (code) => resolve({code, stdout, stderr}));
  });
}

/** Maps items with a bounded number of concurrent workers. */
async function mapLimit<T, R>(items: T[], limit: number, fn: (item: T) => Promise<R>): Promise<R[]> {
  const results = new Array<R>(items.length);
  let next = 0;
  async function worker(): Promise<void> {
    while (next < items.length) {
      const index = next;
      next += 1;
      results[index] = await fn(items[index]);
    }
  }
  await Promise.all(Array.from({length: Math.min(limit, items.length)}, () => worker()));
  return results;
}

/** Local timestamp with numeric timezone offset, matching the previous build manifest. */
function builtAt(): string {
  const date = new Date();
  const pad = (value: number, width = 2): string => String(value).padStart(width, '0');
  const offset = -date.getTimezoneOffset();
  const sign = offset >= 0 ? '+' : '-';
  const absolute = Math.abs(offset);
  return (
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}` +
    `T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}` +
    `${sign}${pad(Math.floor(absolute / 60))}${pad(absolute % 60)}`
  );
}

const lvgl = lvglFlag ?? await firmwareLvgl();
const sdkconfig = path.join(lvgl, '..', '..', 'build', 'config', 'sdkconfig.h');
if (!existsSync(sdkconfig)) throw new Error('Build ESP32 once first: generated sdkconfig.h is required to share LVGL configuration');
const emcc = emccCommand(emsdk);
await mkdir(out, {recursive: true});
const staging = await mkdtemp(path.join(out, '.build-'));
process.on('exit', () => {
  rmSync(staging, {recursive: true, force: true});
});

const config = (await readFile(sdkconfig, 'utf8'))
  .split(/\r?\n/)
  .filter((line) => line.startsWith('#define CONFIG_LV_'))
  .join('\n');
await writeFile(path.join(staging, 'sdkconfig.h'), config + '\n', 'utf8');
const env = {
  ...process.env,
  EMSDK: emsdk,
  EM_CONFIG: path.join(emsdk, '.emscripten'),
};
const cargoEnv = {
  ...env,
  CARGO_TARGET_DIR: firmwareTargetDir,
  CARGO_WORKSPACE_DIR: firmwareRoot,
};
const flags = [
  '-O2',
  '-DLV_CONF_SKIP',
  '-DLV_KCONFIG_PRESENT',
  '-include',
  path.join(staging, 'sdkconfig.h'),
  '-I' + staging,
  '-I' + path.join(here, 'wasm'),
  '-I' + ui,
  '-I' + lvgl,
];
const version = await readFile(path.join(lvgl, 'lv_version.h'), 'utf8');
const configHash = sha256(config + version + flags.join(' ').replaceAll(staging, '<build>'));
const sources = [
  ...await cSources(path.join(lvgl, 'src')),
  ...await filesWithSuffix(ui, '.c'),
  path.join(here, 'wasm/bridge.c'),
];
const objdir = path.join(out, 'objects');
await mkdir(objdir, {recursive: true});
const expected = new Set(
  sources.flatMap((source) => {
    const stem = sha256(source).slice(0, 20);
    return [stem + '.o', stem + '.sha'];
  }),
);
for (const name of await readdir(objdir)) {
  if (!name.endsWith('.o') && !name.endsWith('.sha')) continue;
  if (expected.has(name)) continue;
  await unlink(path.join(objdir, name));
}

const headers = await filesWithSuffix(ui, '.h');
const timerHeader = await readFile(path.join(here, 'wasm/esp_timer.h'));

/** Compiles one C translation unit when its stamp no longer matches. */
async function compileOne(source: string): Promise<string> {
  const dest = path.join(objdir, sha256(source).slice(0, 20) + '.o');
  const stamp = dest.replace(/\.o$/, '.sha');
  const headerBytes = await Promise.all(
    headers
      .filter((header) => path.basename(header) !== 'layout.generated.h' || path.dirname(source) === ui)
      .map((header) => readFile(header)),
  );
  const signature = sha256(Buffer.concat([await readFile(source), Buffer.from(configHash), ...headerBytes, timerHeader]));
  const stampText = existsSync(stamp) ? await readFile(stamp, 'utf8') : '';
  if (!existsSync(dest) || stampText !== signature) {
    const result = await run([...emcc, ...flags, '-c', source, '-o', dest], {env, capture: true});
    if (result.code !== 0) throw new Error(result.stderr);
    await writeFile(stamp, signature);
  }
  return dest;
}

const sourceHash = await sharedHash();
console.log('Compiling shared LVGL UI to WebAssembly...');
const objects = await mapLimit(sources, 8, compileOne);
const linkArgs = [
  ...objects,
  '-O2',
  '--no-entry',
  '-sMODULARIZE=1',
  '-sEXPORT_ES6=1',
  '-sENVIRONMENT=web',
  '-sALLOW_MEMORY_GROWTH=1',
  '-sEXPORTED_FUNCTIONS=' + JSON.stringify(exports),
  '-sEXPORTED_RUNTIME_METHODS=["ccall","HEAPU8"]',
  '-o',
  path.join(staging, 'ui.mjs'),
];
const responseFile = path.join(staging, 'link.rsp');
await writeFile(responseFile, linkArgs.map((arg) => JSON.stringify(arg)).join('\n'), 'utf8');
const linked = await run([...emcc, '@' + responseFile], {env});
if (linked.code !== 0) throw new Error('emcc link failed');

const runtimeFiles = [
  ...await filesWithSuffix(ui, '.c'),
  ...(await filesWithSuffix(ui, '.h')).filter((file) => path.basename(file) !== 'layout.generated.h'),
  ...await filesWithSuffix(path.join(root, 'apps/esp32/src'), '.rs'),
  path.join(root, 'apps/esp32/partitions.csv'),
];
const manifest: Record<string, unknown> = {
  runtimeHash: await hashFiles(runtimeFiles),
  sourceHash,
  builtAt: builtAt(),
  lvgl: '9.3.0',
  firmwareBuilt: false,
  source: 'apps/esp32/lvgl_port/operit_lvgl.c',
};

if (firmware) {
  console.log('Building ESP32 from the same source...');
  const archive = path.join(lvgl, '..', '..', 'build', 'esp-idf', 'lvgl_port', 'liblvgl_port.a');
  const newest = Math.max(
    ...(await Promise.all(
      (await readdir(ui))
        .filter((name) => ['.c', '.h', '.txt'].includes(path.extname(name)))
        .map(async (name) => (await stat(path.join(ui, name))).mtimeMs),
    )),
  );
  if (!existsSync(archive) || (await stat(archive)).mtimeMs < newest) {
    await utimes(path.join(root, 'apps/esp32/sdkconfig.defaults'), new Date(), new Date());
  }
  const logPath = path.join(out, 'firmware-build.log');
  const cargo = await run(['cargo', 'build', '--release'], {
    cwd: firmwareRoot,
    env: cargoEnv,
    capture: true,
  });
  await writeFile(logPath, cargo.stdout + cargo.stderr);
  const log = await readFile(logPath);
  await writeFile(logPath, log.subarray(Math.max(0, log.length - 256 * 1024)));
  if (cargo.code !== 0) throw new Error('Firmware failed; see generated/firmware-build.log');
  const firmwareElf = path.join(firmwareTargetDir, firmwareTarget, 'release', 'operit-esp32');
  manifest.firmwareBuilt = true;
  manifest.firmwareElf = firmwareElf;
  manifest.firmwareSha256 = sha256(await readFile(firmwareElf));
  const release = path.dirname(firmwareElf);
  const dist = path.join(root, 'apps/esp32/dist');
  await mkdir(dist, {recursive: true});
  const flash = [
    'espflash',
    'save-image',
    '--chip',
    'esp32',
    '--flash-mode',
    'dio',
    '--flash-size',
    '4mb',
    '--skip-update-check',
    '--bootloader',
    path.join(release, 'bootloader.bin'),
    '--partition-table',
    path.join(root, 'apps/esp32/partitions.csv'),
  ];
  const image = await run([...flash, firmwareElf, path.join(dist, 'operit-esp32.bin')]);
  if (image.code !== 0) throw new Error('espflash save-image failed');
  const merged = await run([...flash, '--merge', firmwareElf, path.join(dist, 'operit-esp32-4mb-full.bin')]);
  if (merged.code !== 0) throw new Error('espflash merge failed');
  for (const name of ['bootloader.bin', 'partition-table.bin']) {
    await copyFile(path.join(release, name), path.join(dist, name));
  }
}

if (sourceHash !== await sharedHash()) {
  throw new Error('UI changed during build; rebuild before publishing matching artifacts');
}
await copyFile(path.join(staging, 'ui.wasm'), path.join(out, 'ui.wasm'));
await copyFile(path.join(staging, 'ui.mjs'), path.join(out, 'ui.mjs'));
await writeFile(path.join(out, 'manifest.json'), JSON.stringify(manifest, null, 2), 'utf8');
await rm(staging, {recursive: true, force: true});
console.log('Build complete: ' + sourceHash.slice(0, 12));
