import {simulatorBuildArguments} from './simulator-build.mts';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import path from 'node:path';

// Reuse the upstream allocator, guarded stacks and integration scenarios.
const directory = fileURLToPath(new URL('../', import.meta.url));
const root = path.resolve(directory, '../../../..');
const firmware = process.argv.slice(2).includes('--firmware');
const npmCli = process.env.npm_execpath;
if (!npmCli) throw new Error('Run npm run test:device-budget --prefix edge/boards/esp32/editor');
const env: NodeJS.ProcessEnv = {...process.env};
for (const name of ['OPERIT_SIM_MEMORY', 'OPERIT_SIM_HEAP_BYTES', 'OPERIT_SIM_TARGET',
  'OPERIT_SIM_TEST_ACCESS_ONLY', 'OPERIT_SIM_TEST_DEVICE_SPACE_ONLY',
  'OPERIT_SIM_TEST_STANDALONE_ONLY', 'OPERIT_SIM_TEST_MEMORY_STRESS', 'OPERIT_SIM_TEST_CLI']) delete env[name];

async function run(label: string, executable: string, args: string[],
  overrides: NodeJS.ProcessEnv = {}): Promise<void> {
  console.log(`\nDevice budget gate: ${label} (upstream fixed 256 KiB Rust heap; guarded task stacks)`);
  const child = spawn(executable, args, {cwd: executable === 'cargo' ? root : directory, env: {...env, ...overrides},
    stdio: 'inherit', windowsHide: true});
  const code = await new Promise<number | null>((resolve, reject) => {
    child.once('error', reject);
    child.once('close', resolve);
  });
  if (code !== 0) throw new Error(`${label} failed (${code}); budget verification did not pass`);
}
await run(firmware ? 'firmware build and C/Wasm preview' : 'C/Wasm preview', process.execPath,
  [npmCli, 'run', firmware ? 'build:firmware' : 'build']);
await run('TypeScript checks', process.execPath, [npmCli, 'run', 'check']);
await run('prepare upstream bounded desktop dependencies', path.join(root, '.venv',
  process.platform === 'win32' ? 'Scripts/python.exe' : 'bin/python'), ['-X', 'utf8', 'prepare-simulator.py']);
await run('Rust firmware-module tests', 'cargo', ['test', '--locked', '--manifest-path', path.join(directory, 'simulator/Cargo.toml')]);
await run('current simulator release build', 'cargo', simulatorBuildArguments);
await run('editor and real heap/stack boundary probes', process.execPath, [npmCli, 'test']);
const cliDirectory = path.join(root, 'apps/cli/target');
const cliExecutable = path.join(cliDirectory, 'debug', 'operit2' + (process.platform === 'win32' ? '.exe' : ''));
await run('current bundled Core plugin assets', path.join(root, '.venv',
  process.platform === 'win32' ? 'Scripts/python.exe' : 'bin/python'),
  ['-X', 'utf8', path.join(root, 'plugins/tools/sync_plugin_packages.py'), '--source', 'runtime', '--no-hot-reload']);
await run('current Core CLI build', 'cargo', ['build', '--locked', '--manifest-path',
  path.join(root, 'apps/cli/Cargo.toml'), '--bin', 'operit2', '--target-dir', cliDirectory]);
const integration = ['--test', '--experimental-strip-types', 'tests/integration/simulator-space.test.mjs'];
await run('isolated pairing, admission and restart', process.execPath, integration,
  {OPERIT_SIM_TEST_ACCESS_ONLY: '1', OPERIT_SIM_TEST_CLI: cliExecutable});
await run('standalone chat, twelve extra turns and restart', process.execPath, integration,
  {OPERIT_SIM_TEST_STANDALONE_ONLY: '1', OPERIT_SIM_TEST_MEMORY_STRESS: '1',
    OPERIT_SIM_TEST_CLI: cliExecutable});
await run('remote chat, admission storage failures and binary speech recovery', process.execPath, integration,
  {OPERIT_SIM_TEST_CLI: cliExecutable});
console.log('\nDevice budget scenarios passed. These simulator budgets do not measure whole-board ESP-IDF/Wi-Fi/TLS/C memory.');
