import test from 'node:test';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import http from 'node:http';
import {mkdtemp, mkdir, readFile, writeFile, rm, readdir, realpath} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {randomUUID} from 'node:crypto';
import {performDeviceAction} from '../../web/device-actions.ts';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));

async function buildCli() {
  if (process.env.OPERIT_SIM_TEST_CLI) return path.resolve(process.env.OPERIT_SIM_TEST_CLI);
  const child = spawn('cargo', ['build', '--locked', '--manifest-path', 'apps/cli/Cargo.toml', '--bin', 'operit2', '--target-dir', path.join(root, 'apps/cli/target')],
    {cwd: root, windowsHide: true});
  let output = '';
  for (const stream of [child.stdout, child.stderr]) stream.on('data', chunk => { output = (output + chunk).slice(-12000); });
  await new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', code => code === 0 ? resolve() : reject(Error(`Core CLI build failed: ${output}`)));
  });
  return path.join(root, 'apps/cli/target/debug/operit2' + (process.platform === 'win32' ? '.exe' : ''));
}

class CoreSession {
  constructor(executable, config, bindAddress) {
    this.queue = []; this.waiter = null; this.output = ''; this.closed = false;
    this.child = spawn(executable, ['cli', '--json', 'link', 'session', 'tcp', '--bind', bindAddress, '--no-discovery'],
      {cwd: root, windowsHide: true, env: {...process.env, OPERIT_CLI_CONFIG_DIR: config}});
    this.child.stderr.on('data', chunk => { this.output = (this.output + chunk).slice(-12000); });
    createInterface({input: this.child.stdout}).on('line', line => {
      let value;
      try { value = JSON.parse(line); } catch { this.output = (this.output + line + '\n').slice(-12000); return; }
      if (this.waiter) { const pending = this.waiter; this.waiter = null; pending.resolve(value); }
      else this.queue.push(value);
    });
    this.exit = new Promise(resolve => this.child.once('exit', (code, signal) => {
      this.closed = true;
      this.waiter?.reject(Error(`Core session exited (${code ?? signal}): ${this.output}`));
      this.waiter = null; resolve();
    }));
    this.child.once('error', error => { this.closed = true; this.waiter?.reject(error); });
  }
  next(timeout = 60000) {
    if (this.queue.length) return Promise.resolve(this.queue.shift());
    if (this.closed) return Promise.reject(Error(`Core session is closed: ${this.output}`));
    assert.equal(this.waiter, null, 'only one Core command may be outstanding');
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.waiter = null;
        this.child.kill();
        reject(Error(`Core response timed out; no mutation was retried: ${this.output}`));
      }, timeout);
      this.waiter = {resolve: value => { clearTimeout(timer); resolve(value); },
        reject: error => { clearTimeout(timer); reject(error); }};
    });
  }
  async command(args) {
    assert(!this.closed);
    this.child.stdin.write(JSON.stringify(args) + '\n');
    const value = await this.next();
    if (value?.error) throw Error(value.error);
    return value;
  }
  async stop() {
    if (this.closed) return;
    this.child.stdin.write('["quit"]\n');
    const timer = setTimeout(() => this.child.kill(), 15000);
    await this.exit; clearTimeout(timer);
  }
}

async function files(directory, prefix = '') {
  const result = [];
  for (const item of await readdir(directory, {withFileTypes: true})) {
    const relative = path.join(prefix, item.name);
    if (item.isDirectory()) result.push(...await files(path.join(directory, item.name), relative));
    else result.push(relative.replaceAll('\\', '/'));
  }
  return result;
}

// This opt-in integration test uses a real independent Core CLI, the editor's
// real TCP Edge child, and the same C renderer compiled to WASM. No fake approval
// replies, manually written membership, or developer's real profile are used.
test('Core and rendered simulator complete pairing, rejoin, cancellation and restart through real UI actions',
  {timeout: 600000}, async t => {
    const temporaryRoot = await realpath(tmpdir());
    const directory = await mkdtemp(path.join(temporaryRoot, 'operit-simulator-space-'));
    const edgeDirectory = path.join(directory, 'edge');
    const config = path.join(directory, 'core-config');
    await mkdir(config, {recursive: true});
    const identity = randomUUID();
    await writeFile(path.join(config, 'storage.json'), JSON.stringify({
      runtimeRoot: path.join(directory, 'core', 'runtime'), workspaceRoot: path.join(directory, 'core', 'workspaces'),
      activeIdentityId: identity, identities: [{id: identity, name: 'Simulator workflow test', createdAt: Date.now()}],
    }));
    process.env.OPERIT_SIM_STATE_DIR = edgeDirectory;
    process.env.OPERIT_SIM_BIND = '127.0.0.1:0';
    const {simulatorRoute, stopSimulator} = await import('../../src/api/simulator-api.mts');
    // Only the model provider boundary is deterministic. Pairing, admission,
    // chat creation, sends, persistence, watches and renderer remain real.
    const providerRequests = [];
    const server = http.createServer((req, res) => {
      if (req.url === '/v1/chat/completions') {
        void (async () => {
          let body = '';
          for await (const chunk of req) body += chunk;
          const input = JSON.parse(body);
          providerRequests.push(input);
          const prompt = JSON.stringify(input.messages);
          const text = prompt.includes('SIM_RECONNECT') ? 'SIM_RECONNECT_OK' : 'SIM_CHAT_OK';
          if (input.stream) {
            res.writeHead(200, {'Content-Type': 'text/event-stream'});
            for (const content of [text.slice(0, 4), text.slice(4)]) {
              res.write('data: ' + JSON.stringify({id: 'sim-chat', object: 'chat.completion.chunk',
                choices: [{index: 0, delta: {content}, finish_reason: null}]}) + '\n\n');
              await sleep(30);
            }
            res.end('data: ' + JSON.stringify({id: 'sim-chat', choices: [{index: 0, delta: {}, finish_reason: 'stop'}]}) +
              '\n\ndata: [DONE]\n\n');
          } else {
            res.writeHead(200, {'Content-Type': 'application/json'});
            res.end(JSON.stringify({id: 'sim-chat', choices: [{index: 0,
              message: {role: 'assistant', content: text}, finish_reason: 'stop'}]}));
          }
        })().catch(error => { res.writeHead(500); res.end(String(error)); });
        return;
      }
      void simulatorRoute(req, res, new URL(req.url, 'http://localhost')).then(handled => {
        if (!handled) { res.writeHead(404); res.end(); }
      });
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const base = `http://127.0.0.1:${server.address().port}`;
    let core;
    t.after(async () => {
      await core?.stop(); stopSimulator();
      server.closeAllConnections(); await new Promise(resolve => server.close(resolve));
      const cleanup = await realpath(directory);
      assert.equal(path.dirname(cleanup), temporaryRoot, 'refuse cleanup outside the temporary parent');
      assert(path.basename(cleanup).startsWith('operit-simulator-space-'), 'refuse cleanup of an unrelated directory');
      await rm(cleanup, {recursive: true, force: true, maxRetries: 20, retryDelay: 100});
    });
    const executable = await buildCli();
    let coreAddress = '127.0.0.1:0';
    const startCore = async () => {
      core = new CoreSession(executable, config, coreAddress);
      const ready = await core.next();
      assert.equal(ready.listening, true, `Core did not listen: ${JSON.stringify(ready)}`);
      coreAddress = ready.bindAddress;
    };
    const post = async (route, input) => {
      const response = await fetch(base + route, {method: 'POST', headers: {'Content-Type': 'application/json'},
        body: input === undefined ? undefined : JSON.stringify(input)});
      const value = await response.json();
      if (!response.ok) throw Error(value.error ?? `HTTP ${response.status}`);
      return value;
    };
    const state = async () => (await fetch(base + '/api/simulator/state')).json();
    const waitDevice = async (description, predicate, timeout = 60000) => {
      const deadline = Date.now() + timeout;
      let current;
      while (Date.now() < deadline) {
        current = await state();
        assert.equal(current.running, true, `simulator exited: ${current.output}`);
        if (predicate(current.device)) return current.device;
        await sleep(100);
      }
      assert.fail(`${description} timed out: ${JSON.stringify(current.device)}`);
    };
    const startEdge = async () => {
      await post('/api/simulator/start');
      const deadline = Date.now() + 180000;
      while (Date.now() < deadline) {
        const value = await state();
        if (value.ready) {
          // Preserve discovered listener addresses across restart, as the real
          // device does. Do not rely on OS ephemeral-port reuse or discovery.
          process.env.OPERIT_SIM_BIND = value.device.address;
          return value;
        }
        if (!value.running) throw Error(value.output);
        await sleep(100);
      }
      throw Error('Simulator startup timed out');
    };
    globalThis.window ??= {};
    const {default: createUi} = await import('../../generated/ui.mjs');
    const ui = await createUi({wasmBinary: await readFile(new URL('../../generated/ui.wasm', import.meta.url))});
    assert.equal(ui._simulator_init(), 1);
    const call = (name, types = [], values = []) => ui.ccall('operit_ui_' + name, null, types, values);
    const screen = () => JSON.parse(ui.ccall('operit_ui_debug_snapshot', 'string', [], []));
    const actions = [];
    let actionError = '';
    let review;
    ui.onAction = action => {
      actions.push(performDeviceAction(ui, action, value => post('/api/simulator/action', value), review)
        .catch(error => { actionError = error.message; }));
    };
    const tap = async id => {
      assert.equal(ui.ccall('operit_ui_debug_tap', 'number', ['string'], [id]), 1, `not a clickable UI node: ${id}`);
      await Promise.all(actions.splice(0));
    };
    const update = async () => {
      const current = await state();
      assert.equal(current.running, true, `simulator exited: ${current.output}`);
      assert.equal(current.ready, true);
      const device = current.device;
      review = {requestId: device.spaceJoinRequestId, assignmentVersion: device.spaceJoinAssignmentVersion};
      ui._operit_ui_set_connection(1, device.chat.connected ? 1 : 0);
      call('set_chat_identity', ['string', 'string'], [device.chat.chatId ?? '', device.chatPreview]);
      const messages=(device.chat.messages ?? []).filter(message=>message.text?.trim()).slice(-12);
      messages.forEach((message,index)=>call('set_message',['number','number','string'],
        [index,message.sender==='user'?1:0,message.text]));
      call('finish_messages',['number'],[messages.length]);
      call('set_chat_screen', ['string'], [device.chatScreen]);
      call('set_chat_task', ['string'], [device.chatTask]);
      call('set_paired', ['number'], [device.paired ? 1 : 0]);
      call('set_pairing_code', ['string'], [device.pairingCode]);
      call('set_space_join_prompt', ['string', 'number'], [device.spaceJoinPrompt, 0]);
      for (let i = 0; i < 20; i++) ui._operit_ui_pump(20);
      return current;
    };
    const approve = async () => {
      await update(); assert.equal(screen().page, 'Space');
      actionError = ''; await tap('edge_space_approve');
      assert.equal(actionError, '', `approval UI action failed: ${actionError}`);
      await update(); assert.equal(screen().nodes.some(node => node.id === 'edge_space_approve'), false);
      assert(!screen().nodes.some(node => ['error', 'space_join_error'].includes(node.id)), 'successful approval must clear the prior failed-action overlay');
    };
    const leave = async () => {
      await update(); call('navigate_home');
      await tap('sidebar_toggle'); await tap('sidebar_settings'); await tap('settings_space'); await tap('space_leave');
      assert.equal(screen().page, 'LeaveSpace');
      actionError = ''; await tap('space_leave_confirm'); assert.equal(actionError, '');
      assert.equal((await update()).device.paired, true, 'leaving must not unpair');
    };
    await startCore();
    const first = await startEdge();
    const node = first.device.deviceId;
    const pairing = await core.command(['pair-start', node, first.device.address, 'tcp', '--token', first.token]);
    await update();
    assert.equal(screen().page, 'Pairing');
    const code = screen().nodes.find(node => node.id === 'pairing_code')?.text;
    assert.match(code, /^\d{6}$/, 'read the pairing code from the actual rendered screen');
    await core.command(['pair-finish', pairing.pairingId, code]);
    assert.equal((await update()).device.paired, true);
    const credentials = await core.command(['peers']);
    const request = await core.command(['space', 'join', node]);
    assert.equal(request.status, 'pending');
    await approve();
    assert.equal((await core.command(['space', 'refresh', request.requestId])).status, 'joined');
    const former = await core.command(['space', 'show']);
    assert(former.members.includes(node));
    t.diagnostic('paired using the C-rendered screen code; first UI approval completed');
    await waitDevice('automatic chat route installation after admission', device => device.chat.connected);
    const provider = await core.command(['core', 'model', 'provider-create', 'Simulator deterministic provider',
      'OPENAI_GENERIC', base + '/v1/chat/completions']);
    await core.command(['core', 'model', 'provider-set-key', provider.providerId, 'isolated-test-key']);
    await core.command(['core', 'model', 'provider-model-create', provider.providerId, 'sim-model']);
    await core.command(['core', 'model', 'use', provider.providerId, 'sim-model']);
    // Create through the real rendered button; send via the editor's public API.
    await update(); call('navigate_home'); await tap('sidebar_toggle'); await tap('sidebar_settings'); await tap('settings_device'); await tap('edge_new');
    const created = await waitDevice('Edge-created chat', device => device.chat.chatId && !device.chat.sending);
    const chatId = created.chat.chatId;
    await post('/api/simulator/send', {text: 'SIM_CHAT'});
    const firstReply = await waitDevice('first streamed reply', device =>
      device.chat.messages.some(message => message.text === 'SIM_CHAT_OK') && !device.chat.generating);
    assert(firstReply.chat.messages.some(message => message.sender === 'user' && message.text === 'SIM_CHAT'));
    assert.equal(providerRequests.length, 1);
    await update();
    const painted=screen();
    assert.equal(painted.page,'Chat');
    const faceRect=painted.nodes.find(node=>node.id==='home_face').rect;
    const chatNode=painted.nodes.find(node=>node.id==='chat_text');
    assert(faceRect.x+faceRect.w<chatNode.rect.x,'real reply must be displayed beside the expression');
    assert(chatNode.text.includes('SIM_CHAT_OK'),`actual streamed reply must reach the rendered right column: ${JSON.stringify({node:chatNode,screen:firstReply.chatScreen,messages:firstReply.chat.messages})}`);
    const stored = await core.command(['core', 'chat', 'show', chatId]);
    assert(JSON.stringify(stored).includes('SIM_CHAT_OK'), 'reply must be stored on Core, not fabricated by simulator');
    t.diagnostic('automatic Space chat route, rendered new-chat action, Edge send and real Core streamed reply passed (local deterministic provider)');
    await core.stop();
    await waitDevice('chat offline after Core stops', device => !device.chat.connected);
    await startCore();
    await waitDevice('automatic chat reconnect after Core restart', device => device.chat.connected && device.chat.chatId === chatId);
    await post('/api/simulator/send', {text: 'SIM_RECONNECT'});
    const secondReply = await waitDevice('reply after reconnect', device =>
      device.chat.messages.some(message => message.text === 'SIM_RECONNECT_OK') && !device.chat.generating);
    assert(secondReply.chat.messages.some(message => message.text === 'SIM_CHAT_OK'), 'previous history must survive reconnect');
    assert.equal(providerRequests.length, 2);
    assert.deepEqual(await core.command(['peers']), credentials, 'Core restart must not require pairing again');
    t.diagnostic('Core stop produced offline; restart restored the same chat and a second streamed round-trip without re-pairing');
    await leave();
    assert.deepEqual(await core.command(['space', 'show']), former, 'the same Core retains the previous Space');
    const cancelled = await core.command(['space', 'join', node]);
    await update(); assert.equal(screen().page, 'Space');
    assert.equal((await core.command(['space', 'cancel', cancelled.requestId])).status, 'cancelled');
    // Cancel won just before a tap on a still-visible approval prompt. A normal
    // decision error must NOT terminate the simulated device or its listener.
    actionError = ''; await tap('edge_space_approve');
    assert.match(actionError, /当前没有待审批|no longer awaiting|not.*awaiting/i,
      `stale UI approval must return an ordinary error without killing the simulator: ${actionError}`);
    assert.equal((await state()).device.spaceJoinPrompt, '');
    t.diagnostic('Core cancelled the same-Core rejoin; stale approval tap did not crash the simulator');
    const second = await core.command(['space', 'join', node]);
    // Before the next UI poll the rendered button still belongs to the
    // CANCELLED request. It must not silently approve this new submission.
    actionError = ''; await tap('edge_space_approve');
    assert(actionError, 'a stale approval must not be retargeted to the next pending request');
    assert.equal((await core.command(['space', 'refresh', second.requestId])).status, 'pending');
    t.diagnostic('a stale approval for cancelled A did not approve the newly submitted B');
    await update();
    await assert.rejects(post('/api/simulator/action', {action: 'edge_space_approve',
      requestId: review.requestId, assignmentVersion: review.assignmentVersion + 1}));
    assert.equal((await core.command(['space', 'refresh', second.requestId])).status, 'pending', 'stale assignment cannot approve');
    actionError = ''; await tap('edge_space_reject'); assert.equal(actionError, '');
    assert.equal((await core.command(['space', 'refresh', second.requestId])).status, 'rejected');
    const afterReject = await core.command(['space', 'join', node]);
    await approve();
    assert.equal((await core.command(['space', 'refresh', afterReject.requestId])).status, 'joined');
    t.diagnostic('same Core successfully rejoined after leaving and cancelling');
    await leave();
    const pending = await core.command(['space', 'join', node]);
    await update(); assert.equal(screen().page, 'Space');
    await core.stop(); await post('/api/simulator/stop');
    await startCore();
    const restarted = await startEdge(); assert.equal(restarted.token, first.token);
    await update(); assert.equal(screen().page, 'Space', 'approval prompt survives process restart');
    assert.equal((await core.command(['space', 'cancel', pending.requestId])).status, 'cancelled');
    await update(); assert.equal(screen().nodes.some(node => node.id === 'edge_space_approve'), false);
    const third = await core.command(['space', 'join', node]);
    await approve();
    assert.equal((await core.command(['space', 'refresh', third.requestId])).status, 'joined');
    t.diagnostic('both processes restarted; the persisted pending request was cancelled and a new application approved');
    await leave();
    const offline = await core.command(['space', 'join', node]);
    await update(); await post('/api/simulator/stop');
    // Offline cancellation must report non-delivery while persisting intent.
    // Restart both processes, then ordinary refresh must retry the cancellation.
    await assert.rejects(core.command(['space', 'cancel', offline.requestId]));
    const queued = (await core.command(['space', 'requests', 'outgoing'])).find(value => value.requestId === offline.requestId);
    assert.equal(queued.status, 'pending', 'offline cancellation must not fabricate a completed status');
    await core.stop(); await startCore(); await startEdge();
    assert.equal((await core.command(['space', 'refresh', offline.requestId])).status, 'cancelled',
      'ordinary refresh after both restarts must retry the persisted cancellation intent');
    await update(); assert.equal(screen().nodes.some(value => value.id === 'edge_space_approve'), false);
    const afterOfflineCancel = await core.command(['space', 'join', node]);
    await approve();
    assert.equal((await core.command(['space', 'refresh', afterOfflineCancel.requestId])).status, 'joined');
    assert.deepEqual(await core.command(['peers']), credentials, 'pairing credentials must not be replaced');
    const edgeFiles = await files(path.join(edgeDirectory, 'runtime'));
    assert(!edgeFiles.some(file => /^sync\//.test(file)), 'non-storage Edge must not create business replication journals');
    assert(!edgeFiles.some(file => /(^|\/)(chat|chats|messages|blobs|models)\//.test(file)), 'no business data copied to Edge storage');
    assert.equal(screen().heapBytes, 0); assert(screen().staticBytes < 10 * 1024);
    t.diagnostic('offline cancellation intent survived both restarts; refresh retried it and reapplication completed with pairing intact and no business replica');
  });
