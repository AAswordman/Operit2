import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp, mkdir, writeFile, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import http from 'node:http';
import {buildFlashPlan, executeFlashPlan, selectSerialPort, validateFirmwareImages, xtensaCompilerEnvironment} from '../src/device-tools.mts';
import {deviceMain, deviceDebugArgs} from '../src/device.mts';
import {deployRoute} from '../src/api/deploy-api.mts';

const partitions = [
  ['nvs', 1, 2, 0x9000, 0x6000], ['phy_init', 1, 1, 0xf000, 0x1000],
  ['factory', 0, 0, 0x10000, 0x3d0000], ['ui_layout', 1, 0x82, 0x3e0000, 0x10000],
];
function images() {
  const table = Buffer.alloc(0xc00, 255);
  partitions.forEach(([name, type, subtype, offset, size], i) => {
    const at = i * 32;
    table.fill(0, at, at + 32);
    table.writeUInt16LE(0x50aa, at); table[at + 2] = type; table[at + 3] = subtype;
    table.writeUInt32LE(offset, at + 4); table.writeUInt32LE(size, at + 8);
    table.write(name, at + 12);
  });
  const app = Buffer.alloc(64); app[0] = 0xe9;
  return {table, app, bootloader: Buffer.from(app)};
}
async function fixture(t) {
  const dist = await mkdtemp(path.join(tmpdir(), 'operit-flash-test-'));
  t.after(async () => {
    const absolute = path.resolve(dist);
    assert.equal(path.dirname(absolute), path.resolve(tmpdir()));
    assert.ok(path.basename(absolute).startsWith('operit-flash-test-'));
    await rm(absolute, {recursive: true, force: true});
  });
  const {table, app, bootloader} = images();
  await Promise.all([writeFile(path.join(dist, 'partition-table.bin'), table),
    writeFile(path.join(dist, 'operit-esp32.bin'), app), writeFile(path.join(dist, 'bootloader.bin'), bootloader)]);
  return dist;
}

test('automatic port selection uses the sole USB port, never Bluetooth or multiple boards', () => {
  const usb = {port: 'COM24', name: 'CH340', usb: true};
  const bluetooth = {port: 'COM3', name: 'Bluetooth', usb: false};
  assert.equal(selectSerialPort([bluetooth, usb]), 'COM24');
  assert.equal(selectSerialPort([usb], 'com24'), 'COM24');
  assert.throws(() => selectSerialPort([bluetooth]), /唯一 USB/);
  assert.throws(() => selectSerialPort([usb, {...usb, port: 'COM25'}]), /唯一 USB/);
  assert.throws(() => selectSerialPort([usb], 'COM27'), /未连接/);
});

test('normal update erases the complete application partition but never data partitions', async t => {
  const dist = await fixture(t);
  const plan = await buildFlashPlan('COM24', {dist});
  assert.equal(plan.length, 4);
  assert.equal(plan[0].args[0], 'erase-region');
  assert.deepEqual(plan[0].args.slice(-2), ['0x10000', '0x3d0000']);
  assert.deepEqual(plan.slice(1).map(p => p.args.slice(-2)), [
    ['0x1000', path.join(dist, 'bootloader.bin')], ['0x8000', path.join(dist, 'partition-table.bin')],
    ['0x10000', path.join(dist, 'operit-esp32.bin')],
  ]);
  assert.equal(plan.filter(p => p.args[0].startsWith('erase')).length, 1);
  assert.ok(plan.every(p => p.args.includes('COM24') && p.args.includes('--non-interactive')));
});

test('fresh install requires confirmation and erases all flash, including old identities', async t => {
  const dist = await fixture(t);
  await assert.rejects(buildFlashPlan('COM24', {dist, resetData: true}), /必须明确确认/);
  const plan = await buildFlashPlan('COM24', {dist, resetData: true, confirmReset: true});
  assert.equal(plan[0].args[0], 'erase-flash');
  assert.equal(plan.length, 4);
});

test('device is kept in bootloader until the final write, not rebooted between erase and writes', async t => {
  const dist = await fixture(t);
  for (const resetData of [false, true]) {
    const plan = await buildFlashPlan('COM24', {dist, resetData, confirmReset: true});
    assert.deepEqual(plan.map(p => p.args[p.args.indexOf('--after') + 1]),
      ['no-reset', 'no-reset', 'no-reset', 'hard-reset']);
  }
});

test('corrupt or incompatible partition table and oversized images are rejected before erasure', () => {
  const {table, app, bootloader} = images();
  validateFirmwareImages(table, app, bootloader);
  for (const at of [2, 3, 4, 8, 12, 28, 64 + 4, 96 + 8]) {
    const invalid = Buffer.from(table); invalid[at] ^= 1;
    assert.throws(() => validateFirmwareImages(invalid, app, bootloader));
  }
  assert.throws(() => validateFirmwareImages(table.subarray(0, 32), app, bootloader));
  assert.throws(() => validateFirmwareImages(table, Buffer.alloc(64), bootloader));
  const large = Buffer.alloc(0x3d0001); large[0] = 0xe9;
  assert.throws(() => validateFirmwareImages(table, large, bootloader), /超出/);
  assert.throws(() => validateFirmwareImages(table, app, Buffer.alloc(0x7001)), /过大/);
});

test('missing or invalid artifacts cannot produce any runnable erase plan', async t => {
  const dist = await fixture(t);
  await writeFile(path.join(dist, 'operit-esp32.bin'), Buffer.alloc(64));
  await assert.rejects(buildFlashPlan('COM24', {dist}), /镜像无效/);
  await assert.rejects(buildFlashPlan('COM24', {dist: path.join(dist, 'missing')}), /ENOENT/);
});

test('execution stops immediately on every failed erase/write', async t => {
  const plan = await buildFlashPlan('COM24', {dist: await fixture(t)});
  for (let failAt = 0; failAt < plan.length; failAt++) {
    const calls = [];
    await assert.rejects(executeFlashPlan(plan, async (command, args) => {
      calls.push({command, args});
      if (calls.length - 1 === failAt) throw new Error('injected failure');
      return '';
    }), /injected failure/);
    assert.deepEqual(calls, plan.slice(0, failAt + 1));
  }
  const calls = [];
  await executeFlashPlan(plan, async (command, args) => {calls.push({command, args}); return '';});
  assert.deepEqual(calls, plan);
});

test('CLI rejects typos/unsafe combinations rather than silently flashing', async () => {
  await assert.rejects(deviceMain(['flash', '--porrt', 'COM24']), /未知参数/);
  await assert.rejects(deviceMain(['flash', '--port']), /需要参数/);
  await assert.rejects(deviceMain(['dev', '--dry-run']), /不会构建或连接设备/);
  await assert.rejects(deviceMain(['monitor', '--reset-data']), /只能用于/);
  await assert.rejects(deviceMain(['unknown']), /未知命令/);
  await assert.rejects(deviceMain(['flash', '--minimal-ui']), /未知参数/);
  await assert.rejects(deviceMain(['dev', '--lvgl', 'old-sdk']), /未知参数/);
});

test('HTTP destructive requests require confirmation before enumerating or touching hardware', async t => {
  const server = http.createServer((req, res) => deployRoute(req, res, new URL(req.url, 'http://localhost')));
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => server.close());
  const base = 'http://127.0.0.1:' + server.address().port;
  for (const body of [{port: 'COM24', resetData: true}, {port: 'COM24', resetData: 'true', confirmReset: true}]) {
    const res = await fetch(base + '/api/deploy/flash', {method: 'POST', body: JSON.stringify(body)});
    assert.equal(res.status, 400);
    assert.match((await res.json()).error, /必须明确确认|布尔值/);
  }
  const state = await (await fetch(base + '/api/deploy/flash')).json();
  assert.equal(state.running, false);
});


test('hardware debug entrypoints reject flash flags and debug arguments on non-debug actions', async () => {
  await assert.rejects(deviceMain(['screen', '--reset-data']), /只能用于/);
  await assert.rejects(deviceMain(['screen', '--minimal-ui']), /未知参数/);
  await assert.rejects(deviceMain(['screen', '--dry-run']), /只支持 flash/);
  await assert.rejects(deviceMain(['flash', '--id', 'button:any']), /只支持真机调试/);
  await assert.rejects(deviceMain(['screen', '--timeout']), /需要参数/);
  await assert.rejects(deviceMain(['health', '--reset-data']), /只能用于/);
  await assert.rejects(deviceMain(['health', '--dry-run']), /只支持 flash/);
  await assert.rejects(deviceMain(['health', '--minimal-ui']), /未知参数/);
});

test('compiler discovery respects SDK location, host suffix and explicit overrides', async t => {
  const root = await mkdtemp(path.join(tmpdir(), 'operit-toolchain-test-'));
  t.after(() => rm(root, {recursive: true, force: true}));
  const bin = path.join(root, '.embuild/espressif/tools/xtensa-esp-elf/local-version/xtensa-esp-elf/bin');
  await mkdir(bin, {recursive: true});
  for (const suffix of ['', '.exe']) for (const tool of ['gcc', 'ar'])
    await writeFile(path.join(bin, 'xtensa-esp32-elf-' + tool + suffix), '');
  const linux = await xtensaCompilerEnvironment(root, {}, 'linux');
  const windows = await xtensaCompilerEnvironment(root, {}, 'win32');
  assert.equal(linux.CC_xtensa_esp32_espidf, path.join(bin, 'xtensa-esp32-elf-gcc'));
  assert.equal(windows.AR_xtensa_esp32_espidf, path.join(bin, 'xtensa-esp32-elf-ar.exe'));
  const explicit = await xtensaCompilerEnvironment(root, {CC_xtensa_esp32_espidf: 'custom-cc'}, 'win32');
  assert.equal(explicit.CC_xtensa_esp32_espidf, undefined);
  assert.equal(explicit.AR_xtensa_esp32_espidf, windows.AR_xtensa_esp32_espidf);
  assert.deepEqual(await xtensaCompilerEnvironment(path.join(root, 'absent'), {}, 'linux'), {});
});

test('UART and bridge drafts use the debug handler and retain text including an empty draft', () => {
  for (const connection of [['--port', 'COM27'], ['--bridge', 'http://127.0.0.1:8767']]) {
    for (const text of ['链路测试', '']) {
      assert.deepEqual(deviceDebugArgs('draft', connection, new Map([['--text', text], ['--timeout', '12']])),
        ['draft', ...connection, '--timeout', '12', '--text', text]);
    }
    assert.deepEqual(deviceDebugArgs('tap', connection, new Map([['--id', 'edge_send']])),
      ['tap', ...connection, '--id', 'edge_send']);
  }
  assert.throws(() => deviceDebugArgs('monitor', ['--port', 'COM27'], new Map()), /未知调试命令/);
});
