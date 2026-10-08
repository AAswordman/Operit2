import test from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import net from 'node:net';
import {mkdtemp, rm, stat} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';

test('editor starts real TCP device, serves firmware UI, persists token and stops listener', {timeout: 120000}, async t => {
  const dir = await mkdtemp(path.join(tmpdir(), 'operit-sim-api-'));
  process.env.OPERIT_SIM_STATE_DIR = dir;
  process.env.OPERIT_SIM_BIND = '127.0.0.1:0';
  const {simulatorRoute, stopSimulator} = await import('../src/api/simulator-api.mts');
  const server = http.createServer((req, res) => void simulatorRoute(req, res, new URL(req.url, 'http://localhost')));
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(async () => {
    stopSimulator();
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
    await rm(dir, {recursive: true, force: true, maxRetries: 10, retryDelay: 100});
  });
  const base = `http://127.0.0.1:${server.address().port}`;
  assert.equal((await fetch(base + '/api/simulator/state', {headers: {Origin: 'https://example.com'}})).status, 403);
  const state = async () => (await fetch(base + '/api/simulator/state')).json();
  async function ready() {
    const deadline = Date.now() + 90000;
    while (Date.now() < deadline) {
      const current = await state();
      if (current.ready) return current;
      if (!current.running) throw Error(current.output);
      await new Promise(resolve => setTimeout(resolve, 200));
    }
    throw Error('Simulator did not become ready');
  }
  assert.equal((await fetch(base + '/api/simulator/start', {method: 'POST'})).status, 202);
  // Startup creates persistent files asynchronously before the child is spawned.
  await new Promise(resolve => setTimeout(resolve, 100));
  const first = await ready();
  assert.equal(first.device.chat.connected, false);
  assert.equal(first.device.memory.profile, 'esp32-2432s028');
  assert.equal(first.device.memory.kind, 'configured-limits');
  assert.equal(first.device.memory.maxPeerMessageBytes, 8192);
  assert.equal(first.device.memory.liveTelemetry, false);
  assert.equal(first.device.memory.freeHeap, undefined);
  assert.equal(first.device.memory.largest8BitBlock, undefined);
  const memory = await (await fetch(base + '/api/simulator/memory')).json();
  assert.deepEqual(memory, first.device.memory);
  assert.match(first.token, /^[0-9a-f]{48}$/);
  const runtimeData = path.join(dir, 'runtime');
  const runtimeExecutable = path.join(dir, 'operit-esp32-simulator' +
    (process.platform === 'win32' ? '.exe' : ''));
  assert((await stat(runtimeData)).isDirectory(), 'runtime data must not collide with the executable');
  assert((await stat(runtimeExecutable)).isFile(), 'the child executable has its own path');

  const invalidImage = await fetch(base + '/api/simulator/send-image', {method:'POST',headers:{'Content-Type':'text/plain'},body:'not an image'});
  assert.equal(invalidImage.status,400);
  assert.match((await invalidImage.json()).error,/PNG\/JPEG/);
  const disconnectedImage = await fetch(base + '/api/simulator/send-image', {
    method:'POST',headers:{'Content-Type':'image/png'},body:Buffer.from([137,80,78,71,13,10,26,10]),
  });
  assert.equal(disconnectedImage.status,400);
  assert.match((await disconnectedImage.json()).error,/尚未连接/);
  const oversizedImage = await fetch(base + '/api/simulator/send-image', {
    method:'POST',headers:{'Content-Type':'image/png'},body:Buffer.alloc(512 * 1024 + 1),
  });
  assert.equal(oversizedImage.status,400);
  assert.match((await oversizedImage.json()).error,/512 KiB/);
  for (const action of ['edge_space_approve', 'edge_space_reject']) {
    for (const identity of [{}, {requestId:'A',assignmentVersion:-1},
      {requestId:'A',assignmentVersion:1.5}, {requestId:' ',assignmentVersion:0}]) {
      const response = await fetch(base + '/api/simulator/action', {method:'POST',
        headers:{'Content-Type':'application/json'}, body:JSON.stringify({action,...identity})});
      assert.equal(response.status,400);
      assert.match((await response.json()).error,/申请编号或审批版本/);
    }
  }
  assert.equal((await state()).ready,true, 'invalid review actions must not kill the simulator');
  const leave = await fetch(base + '/api/simulator/action', {method:'POST',
    headers:{'Content-Type':'application/json'}, body:JSON.stringify({action:'edge_space_leave'})});
  assert.equal(leave.status,200);
  assert.equal((await state()).token, first.token);
  const retired = await fetch(base + '/api/simulator/action', {method:'POST',
    headers:{'Content-Type':'application/json'}, body:JSON.stringify({action:'edge_image:1:a'})});
  assert.equal(retired.status,400);
  assert.match((await retired.json()).error,/Unknown simulator action/);
  const action = await fetch(base + '/api/simulator/action', {method: 'POST', headers: {'Content-Type': 'application/json'}, body: '{"action":"edge_unpair"}'});
  assert.equal(action.status, 200);
  assert.equal((await state()).device.paired, false);
  // Debug queries are fulfilled by the visible UI host, not fabricated by Rust.
  const debug = fetch(base + '/api/simulator/debug/tree');
  let commands = [];
  while (!commands.length) {
    await new Promise(resolve => setTimeout(resolve, 10));
    commands = await (await fetch(base + '/api/simulator/debug/commands')).json();
  }
  assert.equal(commands[0].command, 'tree');
  const rendered = {page: 'Pairing', nodes: [{id: 'actual-ui-node'}]};
  await fetch(base + '/api/simulator/debug/result', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({id: commands[0].id, value: rendered})});
  assert.deepEqual(await (await debug).json(), rendered);
  await fetch(base + '/api/simulator/stop', {method: 'POST'});
  assert.equal((await state()).running, false);
  await new Promise(resolve => setTimeout(resolve, 200));
  const [host, port] = first.device.address.split(':');
  await new Promise((resolve, reject) => {
    const socket = net.connect({host, port: Number(port)});
    socket.once('error', resolve);
    socket.once('connect', () => {socket.destroy(); reject(Error('stopped device still listens'));});
  });
  await fetch(base + '/api/simulator/start', {method: 'POST'});
  await new Promise(resolve => setTimeout(resolve, 100));
  assert.equal((await ready()).token, first.token);
});
