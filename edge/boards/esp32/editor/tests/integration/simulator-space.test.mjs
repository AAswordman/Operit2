import test from 'node:test';
import {decodePacks, paintScene} from '../../web/scene-renderer.ts';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import http from 'node:http';
import {mkdtemp, mkdir, readFile, writeFile, rm, realpath} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {randomUUID} from 'node:crypto';
import {performDeviceAction} from '../../web/device-actions.ts';
import {simulatorViewState} from '../../web/device-state.ts';
import {DeviceView} from '../../web/device-view.ts';
import {probeSpeech} from './speech-probe.mjs';

const root = fileURLToPath(new URL('../../../../../../', import.meta.url));
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
    this.child = spawn(executable, ['cli', '--json', 'link', 'session', 'tcp', '--bind', bindAddress, ...(process.env.OPERIT_SIM_TEST_DEVICE_SPACE_ONLY==='1'?[]:['--no-discovery'])],
      {cwd: root, windowsHide: true, env: {...process.env, OPERIT_CLI_CONFIG_DIR: config, ...(process.platform === 'darwin' ? {OPERIT_CLI_SECRET_STORE:'file', OPERIT_PLUGIN_SDK_SOCKET_DIR:path.join(config, 'ipc')} : {})}});
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

// This opt-in integration test uses a real independent Core CLI, the editor's
// real TCP Edge child, and the same C renderer compiled to WASM. No fake approval
// replies, manually written membership, or developer's real profile are used.
test(process.env.OPERIT_SIM_TEST_STANDALONE_ONLY === '1'
  ? 'ESP pairs, syncs preferences and chats after CLI exits and device restarts'
  : process.env.OPERIT_SIM_TEST_DEVICE_SPACE_ONLY === '1'
  ? 'Edge device-space UI pairs, replaces retained credentials, reviews consent and syncs approval after restart'
  : process.env.OPERIT_SIM_TEST_ACCESS_ONLY === '1'
  ? 'Simulated device initiates TCP pairing and Core consent'
  : 'Core and rendered simulator complete pairing, rejoin, cancellation and restart through real UI actions',
  {timeout: 600000}, async t => {
    const temporaryRoot = await realpath(process.platform === 'darwin' ? '/tmp' : tmpdir());
    const directory = await mkdtemp(path.join(temporaryRoot, 'oe-'));
    const edgeDirectory = path.join(directory, 'edge');
    const config = path.join(directory, 'core-config');
    await mkdir(config, {recursive: true});
    const identity = randomUUID();
    await writeFile(path.join(config, 'storage.json'), JSON.stringify({
      runtimeRoot: path.join(directory, 'r'), workspaceRoot: path.join(directory, 'w'),
      activeIdentityId: identity, identities: [{id: identity, name: 'Simulator workflow test', createdAt: Date.now()}],
    }));
    process.env.OPERIT_SIM_STATE_DIR = edgeDirectory;
    process.env.OPERIT_SIM_BIND = process.env.OPERIT_SIM_TEST_DEVICE_SPACE_ONLY==='1'?'0.0.0.0:0':'127.0.0.1:0';
    process.env.OPERIT_SIM_DISCOVERY = process.env.OPERIT_SIM_TEST_DEVICE_SPACE_ONLY==='1'?'true':'false';
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
          if (input.model === 'sim-direct') assert.equal(req.headers.authorization, 'Bearer simulator-direct-key');
          const prompt = JSON.stringify(input.messages);
          // XML tool results also arrive as user turns; do not mistake them
          // for the user's latest request when selecting the fixture response.
          const lastUserIndex = input.messages.findLastIndex(message =>
            message.role === 'user' && !String(message.content).includes('status=') &&
            /SIM_(CHAT|RECONNECT|CORE_PLUGIN|EXTERNAL_SYNC|LONG_INPUT)/.test(JSON.stringify(message.content)));
          const lastUser = input.messages[lastUserIndex];
          const pluginTurn = JSON.stringify(lastUser?.content ?? '').includes('SIM_CORE_PLUGIN');
          let text = prompt.includes('SIM_RECONNECT') ? 'SIM_RECONNECT_OK' : 'SIM_CHAT_OK';
          if (pluginTurn) {
            // The provider requests a tool; only Core may execute it. Never
            // substitute a fixture date or call the plugin from this server.
            const followup = input.messages.slice(lastUserIndex + 1)
              .map(message => typeof message.content === 'string' ? message.content : JSON.stringify(message.content)).join('\n');
            const result = followup.match(/<[^>]+\bname="daily_life:get_current_date"[^>]*\bstatus="success"[^>]*>([\s\S]*?)<\//);
            if (result) {
              const date = JSON.parse(result[1].replace(/^<content>/, ''));
              assert.equal(typeof date.iso, 'string', 'Core tool result must contain the actual date');
              text = 'SIM_CORE_PLUGIN_OK ' + date.iso;
            } else {
              assert(!followup.includes('name="daily_life:get_current_date"'),
                'the plugin must execute successfully before a model follow-up');
              text = '<tool name="daily_life:get_current_date"></tool>';
            }
          }
          if (String(lastUser?.content).includes('SIM_EXTERNAL_SYNC')) {
            text = Array.from({length:90}, (_, index) => 'SYNC_LINE_' + String(index).padStart(3, '0')).join('\n');
          }
          if (String(lastUser?.content).includes('SIM_LONG_INPUT')) text = 'SIM_LONG_INPUT_OK';
          if (input.stream) {
            res.writeHead(200, {'Content-Type': 'text/event-stream'});
            // Send more than one small read without exceeding the per-event limit;
            // the real stream must keep reading through comments to completion.
            if (input.model === 'sim-direct') res.write((': keepalive ' + 'x'.repeat(480) + '\n\n').repeat(20));
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
    let heapPeakBytes = 0;
    t.after(async () => {
      t.diagnostic(`Enforced device heap peak across restarts: ${heapPeakBytes} / 262144 bytes; every sampled state had zero allocation failures`);
      await core?.stop(); stopSimulator();
      server.closeAllConnections(); await new Promise(resolve => server.close(resolve));
      const cleanup = await realpath(directory);
      assert.equal(path.dirname(cleanup), temporaryRoot, 'refuse cleanup outside the temporary parent');
      assert(path.basename(cleanup).startsWith('oe-'), 'refuse cleanup of an unrelated directory');
      await rm(cleanup, {recursive: true, force: true, maxRetries: 20, retryDelay: 100});
    });
    const executable = await buildCli();
    let coreAddress = process.env.OPERIT_SIM_TEST_DEVICE_SPACE_ONLY==='1'?'0.0.0.0:0':'127.0.0.1:0';
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
      if (!response.ok) throw Error((value.error ?? `HTTP ${response.status}`) +
        "; device: " + JSON.stringify(await state()));
      return value;
    };
    /** Checks the actual allocator and guarded task budgets throughout every interaction. */
    const state = async () => {
      const response = await fetch(base + '/api/simulator/state');
      const current = await response.json();
      assert(response.ok, `Simulator state failed: ${JSON.stringify(current)}`);
      if (current.device) {
        const memory = current.device.memory;
        assert.equal(memory.heapCapacityBytes, 256 * 1024);
        assert.equal(memory.mainStackBytes, 16 * 1024);
        assert.equal(memory.asyncStackBytes, 24 * 1024);
        assert.equal(memory.blockingStackBytes, 20 * 1024);
        assert.equal(memory.stackEnforcement, 'fixed-mapping-with-guard-page');
        assert.equal(memory.allocationFailures, 0, JSON.stringify(memory));
        assert(memory.heapPeakBytes <= memory.heapCapacityBytes, JSON.stringify(memory));
        heapPeakBytes = Math.max(heapPeakBytes, memory.heapPeakBytes);
      }
      return current;
    };
    const waitDevice = async (description, predicate, timeout = 60000) => {
      const deadline = Date.now() + timeout;
      let current;
      while (Date.now() < deadline) {
        current = await state();
        assert.equal(current.running, true, `simulator exited: ${JSON.stringify(current)}; Core: ${core?.output}`);
        if (predicate(current.device)) return current.device;
        await sleep(100);
      }
      assert.fail(`${description} timed out: ${JSON.stringify(current.device)}; Core: ${core.output}; Simulator: ${current.output}`);
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
    let directSendId;
    let review;
    ui.onAction = action => {
      if (action === 'edge_send') {
        const text = ui.ccall('operit_ui_chat_draft', 'string', [], []);
        actions.push(post('/api/simulator/send', {text, sendId:directSendId}).catch(error => { actionError = error.message; }));
        return;
      }
      actions.push(performDeviceAction(ui, action, value => post('/api/simulator/action', value), review)
        .catch(error => { actionError = error.message; }));
    };
    const tap = async id => {
      assert.equal(ui.ccall('operit_ui_debug_tap', 'number', ['string'], [id]), 1, `not a clickable UI node: ${id}`);
      await Promise.all(actions.splice(0));
    };
    // Use the browser adapter itself so omitted state fields cannot pass only
    // because an independent integration fixture happened to set them.
    const deviceView = new DeviceView(ui, {update() {}}, (ok, error) =>
      call('chat_send_result', ['number', 'string'], [ok ? 1 : 0, error ?? '']));
    const update = async () => {
      const current = await state();
      assert.equal(current.running, true, `simulator exited: ${JSON.stringify(current)}; Core: ${core?.output}`);
      assert.equal(current.ready, true);
      const device = current.device;
      review = {requestId: device.spaceJoinRequestId, assignmentVersion: device.spaceJoinAssignmentVersion};
      ui._operit_ui_set_connection(1, device.chat.connected ? 1 : 0);
      deviceView.update(simulatorViewState(current));
      call('set_scene', ['number','number','number'], [device.scene?.active ? 1 : 0, device.scene?.revision ?? 0, 0]);
      for (let i = 0; i < 20; i++) ui._operit_ui_pump(20);
      return current;
    };
    const openSpace = async () => {
      call('navigate_home'); await tap('sidebar_toggle'); await tap('sidebar_settings'); await tap('settings_space');
    };
    const openReview = async () => {
      await waitDevice('incoming approval prompt', device => Boolean(device.spaceJoinRequestId));
      await update();
      if(screen().page === 'Space') return;
      // Error diagnostics can be dismissed without losing the captured request.
      call('action_error',['string'],['']); await openSpace(); await tap('space_review');
    };
    const approve = async () => {
      await update(); await openReview(); assert.equal(screen().page, 'Space');
      actionError = ''; await tap('edge_space_approve');
      assert.equal(actionError, '', `approval UI action failed: ${actionError}`);
      await update(); assert.equal(screen().nodes.some(node => node.id === 'edge_space_approve'), false);
      assert(!screen().nodes.some(node => ['error', 'space_join_error'].includes(node.id)), 'successful approval must clear the prior failed-action overlay');
    };
    const leave = async () => {
      await update(); call('navigate_home');
      await tap('sidebar_toggle'); await tap('sidebar_settings'); await tap('settings_space'); await tap('space_manage'); await tap('space_leave');
      assert.equal(screen().page, 'LeaveSpace');
      actionError = ''; await tap('space_leave_confirm'); assert.equal(actionError, '');
      assert.equal((await update()).device.paired, true, 'leaving must not unpair');
    };
    await startCore();
    const first = await startEdge();
    const node = first.device.deviceId;
    if (process.env.OPERIT_SIM_TEST_STANDALONE_ONLY === '1') {
      await probeDeviceAccess();
      const policy = await core.command(['control','show']);
      assert.equal(policy.deviceIdentityIds[node], 'edge', 'approved ESP must have the durable edge identity');
      assert(!policy.roles.edge.capabilities.includes('storage.provide'));
      await core.command(['space','preference-set',node,'chat',JSON.stringify({
        endpoint:base+'/v1/chat/completions',model:'sim-direct',apiKey:'simulator-direct-key',
        systemPrompt:'You are the device assistant.',
      })]);
      await core.command(['space','sync']);
      await waitDevice('synced device preference enables native chat', device => device.chat.mode === 'direct');
      const sendDirect = async (prompt, expected, sendId) => {
        await update(); call('navigate_home');
        await tap('sidebar_toggle'); await tap('sidebar_chat'); await update();
        await tap('conversation_0');
        assert.equal(screen().page, 'Chat');
        directSendId = sendId;
        call('set_chat_draft', ['string'], [prompt]);
        actionError = ''; await tap('edge_send'); assert.equal(actionError, '');
        const value = await waitDevice('native streamed reply', device =>
          device.chatSendResult?.sendId === sendId && !device.chat.generating &&
          device.chat.messages.some(message => message.text === expected));
        assert.equal(value.chat.error, null);
        assert.equal(value.chatSendResult.ok, true);
        await update();
        assert(screen().nodes.find(node => node.id === 'chat_text')?.text.includes(expected), 'native reply reaches the rendered chat screen');
        const memory = (await state()).device.memory;
        t.diagnostic(`${sendId}: heap peak=${memory.heapPeakBytes}/${memory.heapCapacityBytes}, failures=${memory.allocationFailures}`);
        return value;
      };
      await sendDirect('SIM_CHAT before disconnect', 'SIM_CHAT_OK', 'direct-before');
      assert.equal(providerRequests.length, 1);
      await core.stop();
      assert.equal(core.closed, true, 'the CLI process really exited');
      await waitDevice('all peer links disconnected', device => device.activePeers.length === 0);
      const standalone = await sendDirect('SIM_RECONNECT after CLI exit', 'SIM_RECONNECT_OK', 'direct-after');
      assert.equal(standalone.chat.mode, 'direct');
      assert.equal(providerRequests.length, 2);
      assert.equal(providerRequests[1].model, 'sim-direct');
      assert(providerRequests[1].messages.some(message => message.content === 'SIM_CHAT_OK'), 'ESP keeps its own bounded context');
      if (process.env.OPERIT_SIM_TEST_SCREENSHOT) {
        const pointer = ui._simulator_frame();
        await writeFile(process.env.OPERIT_SIM_TEST_SCREENSHOT, ui.HEAPU8.subarray(pointer, pointer+320*240*2));
      }
      const extraTurns = process.env.OPERIT_SIM_TEST_MEMORY_STRESS === '1' ? 12 : 0;
      for (let turn = 0; turn < extraTurns; turn++) {
        await sendDirect(`SIM_RECONNECT round ${turn} ${'x'.repeat(160)}`, 'SIM_RECONNECT_OK', `stress-${turn}`);
        assert.equal(providerRequests.length, 3 + turn, 'each completed turn reached the real HTTP provider');
      }
      if (extraTurns) {
        assert(!providerRequests.at(-1).messages.some(message => message.content === 'SIM_CHAT before disconnect'),
          'continued standalone chat retires old turns instead of accumulating unlimited context');
      }
      const directId = standalone.chat.chatId;
      await post('/api/simulator/stop'); await startEdge();
      await waitDevice('direct configuration restored without CLI', device => device.chat.mode === 'direct');
      const rebooted = await sendDirect('SIM_CHAT after ESP restart without CLI', 'SIM_CHAT_OK', 'direct-reboot');
      assert.notEqual(rebooted.chat.chatId, directId, 'transient context is not a hidden chat database');
      assert.equal(providerRequests.length, 3 + extraTurns);
      const persisted = await (await fetch(base + '/api/simulator/storage')).json();
      assert(persisted.some(file => file.startsWith('runtime/node_preferences/')), persisted.join(', '));
      assert(!persisted.some(file => /(?:\.db(?:-|$)|\.sqlite|(?:^|\/)(?:sync|plugins|extensions)(?:\/|$))/.test(file)),
        'ESP must not receive a chat database, business journal, plugin or file replica: '+persisted.join(', '));
      t.diagnostic('CLI → real TCP pairing → Core approval → formal edge identity → encrypted preference sync → native chat → CLI process exit → standalone streamed chat → ESP restart → standalone chat passed');
      return;
    }
    if (process.env.OPERIT_SIM_TEST_DEVICE_SPACE_ONLY === '1') {
      const coreNode=(await core.command(['space','show'])).members[0];
      await update(); await openSpace(); await tap('space_pair');
      assert.equal(screen().page,'PairDiscovery');
      let found=await waitDevice('real mDNS discovery',d=>d.deviceSpace.phase===2);
      let index=found.deviceSpace.candidateIds.indexOf(coreNode);
      for(let attempt=0;index<0&&attempt<3;attempt++) {
        await update();await tap('pair_scan_again');
        found=await waitDevice('repeat discovery',d=>d.deviceSpace.phase===2);
        index=found.deviceSpace.candidateIds.indexOf(coreNode);
      }
      assert(index>=0,`isolated Core must be discovered: ${JSON.stringify(found.deviceSpace)}`);
      await update(); await tap('space_peer_'+index);
      await waitDevice('code-entry state',d=>d.deviceSpace.phase===3);
      await update();assert.equal(screen().page,'PairCodeEntry');
      const prompt=(await core.command(['prompts'])).find(p=>p.peerNodeId===node);
      assert(prompt,'Core receives the screen-initiated request');
      const digits=async code=>{for(const digit of code)await tap('pair_digit_'+digit);await tap('pair_digit_submit');};
      const wrong=prompt.confirmationCode==='000000'?'111111':'000000';
      await digits(wrong);
      await waitDevice('code failure remains retryable',d=>d.deviceSpace.phase===3&&d.deviceSpace.status!=='请输入对端显示的六位配对码');
      assert.deepEqual(await core.command(['peers']),[],'bad code never pairs');
      await update();for(let i=0;i<6;i++)await tap('pair_digit_delete');
      await digits(prompt.confirmationCode);
      await waitDevice('outgoing consent',d=>d.deviceSpace.pendingJoin);
      let incoming=(await core.command(['space','requests','incoming'])).find(r=>r.applicantDeviceId===node);
      assert(incoming?.canApprove);assert.equal(incoming.status,'pending');
      await update();assert.equal(screen().page,'SpaceManage');
      assert.equal(ui.ccall('operit_ui_debug_tap','number',['string'],['edge_unpair']),0);
      await tap('space_join'); // This row cancels a pending request.
      await waitDevice('cancelled consent',d=>!d.deviceSpace.pendingJoin&&d.deviceSpace.phase===0);
      assert.equal((await core.command(['space','requests','incoming'])).some(r=>r.requestId===incoming.requestId),false);
      await update();await tap('space_manage');await tap('space_join');
      await waitDevice('new consent',d=>d.deviceSpace.pendingJoin);
      incoming=(await core.command(['space','requests','incoming'])).find(r=>r.applicantDeviceId===node);
      assert(incoming?.canApprove);
      await post('/api/simulator/stop');await startEdge();
      await waitDevice('pending consent recovered after restart',d=>d.deviceSpace.pendingJoin);
      await core.command(['space','approve',incoming.requestId,String(incoming.assignmentVersion)]);
      const joined=await waitDevice('silent approval refresh',d=>d.deviceSpace.status==='已加入设备空间'&&!d.deviceSpace.pendingJoin);
      assert(joined.paired);assert((await core.command(['space','show'])).members.includes(node),'Core membership confirms the Edge joined');
      // Exercise the reverse path too: Core forgets locally, Edge retains its
      // old outbound record, then a new rendered Edge code replaces it.
      await leave();
      await core.command(['unpair',node]);
      assert((await update()).device.paired,'Core unpair remains one-sided');
      const renewed=await core.command(['pair-start',node,(await state()).device.address,'tcp']);
      await update();assert.notEqual(screen().page,'Pairing','incoming code does not steal the current page');
      await openSpace();await tap('space_pair_code');
      const freshCode=screen().nodes.find(n=>n.id==='pairing_code').text;
      assert.match(freshCode,/^\d{6}$/);
      await core.command(['pair-finish',renewed.pairingId,freshCode]);
      const reviewRequest=await core.command(['space','join',node]);
      await waitDevice('Edge assigned approval',d=>d.spaceJoinRequestId===reviewRequest.requestId);
      await approve();
      assert.equal((await core.command(['space','refresh',reviewRequest.requestId])).status,'joined');
      await leave();
      const rejected=await core.command(['space','join',node]);
      await waitDevice('Edge rejection prompt',d=>d.spaceJoinRequestId===rejected.requestId);
      await update();await openReview();await tap('edge_space_reject');
      assert.equal((await core.command(['space','refresh',rejected.requestId])).status,'rejected');
      const restored=await core.command(['space','join',node]);
      await waitDevice('incoming approval before restart',d=>d.spaceJoinRequestId===restored.requestId);
      await post('/api/simulator/stop');await startEdge();
      await waitDevice('incoming approval after restart',d=>d.spaceJoinRequestId===restored.requestId);
      await approve();
      assert.equal((await core.command(['space','refresh',restored.requestId])).status,'joined');
      t.diagnostic('Real C UI covers mDNS, code retry, outgoing cancel, both approval directions, restart recovery and one-sided Core unpair with retained Edge pairing replacement');
      return;
    }
    if (process.env.OPERIT_SIM_TEST_ACCESS_ONLY === '1') {
      await probeDeviceAccess();
      const peers = await core.command(['peers']);
      const membership = await core.command(['space', 'show']);
      assert(membership.members.includes(node));
      await post('/api/simulator/stop');
      await startEdge();
      const coreNode = membership.members.find(member => member !== node);
      assert.equal(typeof coreNode, 'string');
      const restored = await waitDevice('authenticated peer restored after restart', device =>
        device.paired && device.activePeers.includes(coreNode));
      // DeviceSpace's prompt is transient UI state. Verify the durable Router scope instead.
      const access = await post('/api/simulator/core-access', {step: 'request', nodeId: coreNode});
      assert.equal(access.joined, true, 'persisted local control state authorizes the existing Space');
      assert.equal(access.request, null, 'restart does not submit another join request');
      assert.equal(restored.deviceId, node, 'restart preserves the admitted device identity');
      assert.deepEqual(await core.command(['peers']), peers, 'restart preserves pairing credentials');
      assert.deepEqual(await core.command(['space', 'show']), membership, 'restart preserves durable membership');
      t.diagnostic(`admission and restart: heap peak=${heapPeakBytes}/262144, allocation failures=0`);
      return;
    }
    const edgePortArgs={node_id:node,interface_info:{target:'edge.status',method:'read'},args:{}};
    await assert.rejects(()=>core.command(['core','tool','exec','edge_execute',JSON.stringify(edgePortArgs)]),
      'plugin port API must not pair, route to a guessed address, or grant access to an unadmitted node');
    const pairing = await core.command(['pair-start', node, first.device.address, 'tcp']);
    await update(); await openSpace(); await tap('space_pair_code');
    assert.equal(screen().page, 'Pairing');
    const code = screen().nodes.find(node => node.id === 'pairing_code')?.text;
    assert.match(code, /^\d{6}$/, 'read the pairing code from the actual rendered screen');
    await core.command(['pair-finish', pairing.pairingId, code]);
    assert.equal((await update()).device.paired, true);
    const credentials = await core.command(['peers']);
    const request = await core.command(['space', 'join', node]);
    assert.equal(request.status, 'pending');
    // Exercise storage faults through the real pairing/approval path.
    const pressureBefore = (await state()).device.storage;
    assert.equal(pressureBefore.logicalBudgetBytes, 7000);
    for (const [configuration, error] of [
      [{otherNamespaceEntries: 630}, /NVS capacity exhausted/],
      [{otherNamespaceEntries: 213, failAfterMutations: 0}, /Injected NVS write failure/],
    ]) {
      await post('/api/simulator/storage-control', configuration);
      await update(); await openReview();
      actionError = ''; await tap('edge_space_approve');
      assert.match(actionError, error, 'storage failure must be reported by the approval UI');
      assert.deepEqual(await core.command(['peers']), credentials, 'failed admission must retain pairing');
      const retained = await update();
      assert.equal(retained.device.paired, true);
      assert.equal(retained.device.spaceJoinRequestId, request.requestId, 'failed save must retain the review');
      assert.equal(retained.device.storage.runtimeEntries, pressureBefore.runtimeEntries, 'failed save must not leak staged NVS entries');
    }
    await post('/api/simulator/storage-control', {otherNamespaceEntries: 213, failAfterMutations: -1});
    await approve();
    assert.equal((await core.command(['space', 'refresh', request.requestId])).status, 'joined');
    t.diagnostic('NVS exhaustion and interrupted write surfaced through approval UI; retry admitted the same paired Core');
    const former = await core.command(['space', 'show']);
    assert(former.members.includes(node));
    t.diagnostic('paired using the C-rendered screen code; first UI approval completed');
    const initialChat = await waitDevice('automatic remote chat provisioning after admission',
      device => device.chat.connected && device.chat.chatId && !device.chat.initializing && !device.chat.error);
    assert.match(initialChat.chat.chatId, /^[0-9a-f-]{36}$/i, 'first admitted session has a valid nonempty chat ID');
    t.diagnostic('first chat was provisioned through Core before message/state subscriptions opened');
    await probeSpeech({core, node, directory, post, waitDevice, t});
    const edgePort=await core.command(['core','tool','exec','edge_execute',JSON.stringify(edgePortArgs)]);
    assert.equal(edgePort.success,true);assert.equal(edgePort.result.nodeId,node);
    assert.equal(edgePort.result.data.boardId,'ESP32-2432S028-SIM');
    await assert.rejects(()=>core.command(['core','tool','exec','edge_execute',JSON.stringify({...edgePortArgs,
      interface_info:{target:'edge.status',method:'undeclared'}})]),/unsupported|not found/i);
    await assert.rejects(()=>core.command(['core','tool','exec','edge_execute',JSON.stringify({...edgePortArgs,
      interface_info:{target:'core.internal',methodName:'erase'}})]),/interface/i);
    await assert.rejects(()=>core.command(['core','tool','exec','io_execute',JSON.stringify({node_id:node,
      interface_info:{port:'serial',operation:'read'},args:{}})]),/serial|unsupported/i);
    await assert.rejects(()=>core.command(['core','tool','exec','io_execute',JSON.stringify({node_id:node,
      interface_info:{port:'gpio',operation:'read'},args:{pin:2}})]),/unavailable|not available|missing|not configured|not installed|unsupported/i,
      'simulator without a DeviceIoHost must not pretend it read a physical GPIO');
    t.diagnostic('Core plugin hardware executors route to the real Edge native action and reject unpaired nodes, undeclared actions, RPC injection, unsupported serial and absent GPIO Host');

    // Execute the public APIs from a real Core JavaScript package, not just
    // through CLI built-in tool calls or a mocked JS execution host.
    const fixture = path.join(directory, 'edge_ports_fixture.js');
    const fixtureMetadata = {name:'edge_ports_fixture', apiVersion:'2.0.0', description:'Isolated hardware SDK integration test',
      enabledByDefault:true, tools:[{name:'probe', description:'Read a declared Edge native action',
        parameters:[{name:'node_id',description:'Explicit Edge node',type:'string',required:true}]}]};
    await writeFile(fixture, `/* METADATA
${JSON.stringify(fixtureMetadata)}
*/
      exports.probe = async function(params) {
        const response = await Tools.device(params.node_id).Edge.execute({target:'edge.status',method:'read'}, {});
        let unsupported = '';
        try { await Tools.device(params.node_id).Io.execute({port:'serial',operation:'read'}, {}); }
        catch(error) { unsupported = String(error.message || error); }
        let missingHost = '';
        try { await Tools.device(params.node_id).Io.execute({port:'gpio',operation:'read'}, {pin:2}); }
        catch(error) { missingHost = String(error.message || error); }
        return {nodeId:response.nodeId, boardId:response.data.boardId, sameAlias:tools === Tools, unsupported, missingHost};
      };
    `);
    await core.command(['core','package','import',fixture]);
    await core.command(['core','package','enable','edge_ports_fixture']);
    try {
      const probe = await core.command(['core','package','exec','edge_ports_fixture:probe',JSON.stringify({node_id:node})]);
      assert.equal(probe.success,true);
      const payload = probe.result?.value ?? probe.result;
      const data = typeof payload === 'string' ? JSON.parse(payload) : payload;
      assert.equal(data.nodeId,node); assert.equal(data.boardId,'ESP32-2432S028-SIM');
      assert.equal(data.sameAlias,true); assert.match(data.unsupported,/serial|unsupported/i);
      assert.match(data.missingHost,/device I.O Host API is not installed/i);
    } finally {
      await core.command(['core','package','delete','edge_ports_fixture']);
    }
    t.diagnostic('Real Core JS package -> Tools.device(nodeId).Edge/Io -> existing tool runtime -> authenticated Edge action passed');

    // The same market-compatible ToolPkg ships as an opt-in "More packages"
    // asset, not an auto-installed built-in or an ESP JavaScript runtime.
    const more = await core.command(['core','package','more']);
    const candidate = more.find(item => item.name === 'com.operit.edge_pixel_pet');
    assert(candidate, 'pixel pet must ship in the Core More packages catalog');
    assert.equal(candidate.type, 'toolpkg'); assert.equal(candidate.loaded, false);
    assert(!(await core.command(['core','plugin','list'])).some(item => item.name === 'com.operit.edge_pixel_pet'),
      'bundled pet must not be automatically installed');
    const loaded = await core.command(['core','package','load','com.operit.edge_pixel_pet']);
    assert.match(loaded.message, /Successfully imported/);
    const installed = (await core.command(['core','plugin','list'])).find(item => item.name === 'com.operit.edge_pixel_pet');
    assert(installed); assert.equal(installed.enabled, false); assert.equal(installed.subpackages, 1);
    const subpackage = await core.command(['core','package','show','edge_pixel_pet']);
    assert.equal(subpackage.enabledByDefault, true);
    await core.command(['core','plugin','enable','com.operit.edge_pixel_pet']);
    const pet = async tool => {
      const reply=await core.command(['core','package','exec',`edge_pixel_pet:${tool}`,JSON.stringify({node_id:node})]);
      assert.equal(reply.success,true);
      const value=reply.result?.value ?? reply.result;
      return typeof value==='string' ? JSON.parse(value) : value;
    };
    const nativeScene = async(action,args) => {
      assert(Buffer.byteLength(JSON.stringify({v:1,...args}))<=1024);
      const reply=await core.command(['core','tool','exec','edge_execute',JSON.stringify({node_id:node,interface_info:{target:'edge.scene',method:action},args:{v:1,...args}})]);
      const data=reply.result.data;assert(Buffer.byteLength(JSON.stringify(data))<=4096);return data;
    };
    try {
      const caps=(await nativeScene('capabilities',{})).result;
      assert.equal(caps.cache.persistent,true);assert.equal(caps.screen.width,320);
      const systemPageBeforeScene = screen().page;
      const started=await pet('apply_configuration');assert(started.revision);await update();assert.equal(screen().page,'edge_scene');
      const view=await (await fetch(base+'/api/simulator/scene-view')).json();
      assert(view.active);assert.equal(view.layers.length,3);assert.equal(view.packs.length,1);
      assert(!JSON.stringify(view).includes('lease'),'browser view must not leak lease handles');
      const assets=decodePacks(view.packs);let paintedStrips=0;
      const painter=ui.addFunction((p,len,y,rows,tick)=>{paintScene(view,assets,ui.HEAPU8.subarray(p,p+len),y,rows,tick);paintedStrips++;},'viiiiii');
      call('scene_painter',['number'],[painter]);call('set_scene',['number','number','number'],[1,view.revision,view.elapsed]);
      for(let i=0;i<20;i++)ui._operit_ui_pump(0);assert(paintedStrips>0);
      // Bottom-centre pet anchor: opaque body at region-local (144,122).
      ui._simulator_touch(144,146,1);ui._simulator_touch(144,146,0);await Promise.all(actions.splice(0));
      const waitPet = async(predicate) => {
        const deadline=Date.now()+20000;
        while(Date.now()<deadline) {
          const state=(await nativeScene('state',{})).result; // Explicit device state query; input handling remains local
          if(predicate(state)) return state;
          await new Promise(resolve=>setTimeout(resolve,100));
        }
        throw new Error('Device-local scene state did not update');
      };
      const notified=await waitPet(state=>state.taps===1);assert.equal(notified.active,true);
      const again=(await nativeScene('state',{})).result;assert.equal(again.taps,1);
      // Burst + exit are handled on the device; the host only reads explicit snapshots.
      for(let i=0;i<15;i++) {ui._simulator_touch(144,146,1);ui._simulator_touch(144,146,0);await Promise.all(actions.splice(0));}
      await tap('scene_exit');await update();assert.notEqual(screen().page,'edge_scene');
      const exited=await waitPet(state=>state.active===false);assert.equal(exited.taps,16);
      assert.equal((await nativeScene('state',{})).result.active,false);
      assert.equal((await nativeScene('config.read',{})).result.enabled,true);
      await pet('apply_configuration');
      await pet('clear_configuration');await waitPet(state=>state.active===false);await update();assert.equal(screen().page,systemPageBeforeScene);
      assert.equal((await nativeScene('capabilities',{})).result.cache.packs.length,1,'cached original assets reused');
      call('scene_painter',['number'],[0]);ui.removeFunction(painter);
    } finally {
      await core.command(['core','plugin','disable','com.operit.edge_pixel_pet']);
      await core.command(['core','package','delete','edge_pixel_pet']);
    }
    t.diagnostic('Bundled More-package ToolPkg -> real Core Tools.device(nodeId).Edge -> shared native edge.scene: chunked SHA-verified assets, local strip rendering, device-local touch/exit actions, protected exit and cache reuse passed');


    const provider = await core.command(['core', 'model', 'provider-create', 'Simulator deterministic provider',
      'OPENAI_GENERIC', base + '/v1/chat/completions']);
    await core.command(['core', 'model', 'provider-set-key', provider.providerId, 'isolated-test-key']);
    await core.command(['core', 'model', 'provider-model-create', provider.providerId, 'sim-model']);
    await core.command(['core', 'model', 'use', provider.providerId, 'sim-model']);
    // Create through the real rendered button; send via the editor's public API.
    await update(); call('navigate_home'); await tap('sidebar_toggle'); await tap('sidebar_settings'); await tap('settings_device'); await tap('edge_new');
    const created = await waitDevice('Edge-created chat', device => device.chat.chatId && device.chat.chatId !== initialChat.chat.chatId && !device.chat.sending);
    const chatId = created.chat.chatId;
    await post('/api/simulator/send', {text: 'SIM_CHAT', sendId: 'browser-fixture-send'});
    const firstReply = await waitDevice('first streamed reply', device =>
      device.chat.messages.some(message => message.text === 'SIM_CHAT_OK') && !device.chat.generating);
    assert(firstReply.chat.messages.some(message => message.sender === 'user' && message.text === 'SIM_CHAT'));
    assert.equal(providerRequests.length, 1);
    const receipt = (await state()).device.chatSendResult;
    assert.equal(receipt.ok, true);
    assert.equal(receipt.sendId, 'browser-fixture-send');
    const observers = await Promise.all([state(), state()]);
    for (const observer of observers) assert.deepEqual(observer.device.chatSendResult, receipt,
      'debug/state observers must not steal the browser send acknowledgement');
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
    assert.equal(initialChat.chat.conversations, null, 'metadata must not load before list browsing');
    const coreSelectionBefore = await core.command(['core','chat','current']);
    let lastCreatedId=chatId;
    for(let i=0;i<3;i++) {
      await update();await tap('sidebar_toggle');await tap('sidebar_chat');await tap('edge_new');
      const extra=await waitDevice('another Edge-created conversation',device=>device.chat.chatId!==lastCreatedId&&!device.chat.initializing&&!device.chat.sending);
      lastCreatedId=extra.chat.chatId;
    }
    const browse = async () => {
      await update();call('navigate_home');await tap('sidebar_toggle');await tap('sidebar_chat');
      const listed=await waitDevice('bounded conversation metadata',device=>!device.chat.conversationsLoading&&device.chat.conversations?.length>=5&&!device.chat.conversationsError);
      await update();assert.equal(screen().page,'Conversations');return listed.chat.conversations.length;
    };
    const selectRendered = async target => {
      let hit;
      for(let page=0;page<8;page++) {
        hit=screen().nodes.find(n=>n.action==='edge_select:'+target);if(hit)break;
        await tap('conversations_next');await update();
      }
      assert(hit,`missing rendered conversation ${target}`);
      await tap(hit.id);
      await waitDevice('selected Edge conversation',device=>device.chat.chatId===target&&!device.chat.initializing);
      await update();assert.equal(screen().page,'Chat');
    };
    const listedCount=await browse();assert.equal(screen().nodes.filter(n=>/^conversation_/.test(n.id)).length,3);
    assert(screen().nodes.some(n=>n.selected===true),'current conversation is visibly marked');
    await tap('conversations_next');await update();
    assert.equal(screen().nodes.filter(n=>/^conversation_/.test(n.id)).length,Math.min(3,listedCount-3));
    await tap('conversations_prev');await update();
    await selectRendered(initialChat.chat.chatId);
    assert(!(await state()).device.chat.messages.some(m=>m.text==='SIM_CHAT_OK'),'switching does not retain another chat transcript');
    await browse();await selectRendered(chatId);
    await waitDevice('selected chat history restored',device=>device.chat.messages.some(m=>m.text==='SIM_CHAT_OK'));
    assert.deepEqual(await core.command(['core','chat','current']),coreSelectionBefore,'Edge selection must not change Core desktop selection');
    assert.equal(providerRequests.length,1,'browsing and selection must not resend messages');
    await assert.rejects(()=>post('/api/simulator/action',{action:'edge_select:missing-chat'}),/不存在|刷新/);
    assert.equal((await state()).device.chat.chatId,chatId);
    t.diagnostic('rendered list/new/pagination/current mark/switch and history restoration passed; Core UI selection and provider send count unchanged');

    // Exercise the same Edge send/Binding route with a real bundled Core
    // ToolPkg. Inspect structured persisted parts, not just a model's claim.
    const beforePlugin = providerRequests.length;
    await post('/api/simulator/send', {text: 'SIM_CORE_PLUGIN: call the Core daily-life date plugin'});
    const pluginReply = await waitDevice('Core plugin result on Edge', device =>
      device.chat.messages.some(message => message.text.includes('SIM_CORE_PLUGIN_OK ')) &&
      device.chat.messages.some(message => message.text.includes('【daily_life:get_current_date】')) && !device.chat.generating);
    assert.equal(pluginReply.chat.error, null);
    assert.equal(pluginReply.chat.sending, false);
    assert.equal(providerRequests.length - beforePlugin, 2, 'tool request and model follow-up must both run');
    const pluginStored = await core.command(['core', 'chat', 'show', chatId]);
    const pluginMessage = pluginStored.messages.find(message => message.parts.some(part =>
      part.kind === 'tool_result' && part.toolName === 'daily_life:get_current_date'));
    assert(pluginMessage, 'real plugin result must be persisted in the Core chat');
    const toolResult = pluginMessage.parts.find(part => part.kind === 'tool_result' && part.toolName === 'daily_life:get_current_date');
    assert.equal(toolResult.attributes.status, 'success');
    const pluginDate = JSON.parse(toolResult.content);
    assert.equal(new Date(pluginDate.timestamp).toISOString(), pluginDate.iso);
    assert(pluginReply.chat.messages.some(message => message.text.includes('SIM_CORE_PLUGIN_OK ' + pluginDate.iso)),
      'Edge must display the actual Core plugin result');
    assert(pluginMessage.completedAt > 0 && pluginMessage.contentStream === null, 'plugin turn must finish and persist');
    assert(pluginMessage.parts.some(part => part.kind === 'tool_call' && part.toolName === 'daily_life:get_current_date'));
    await update();
    assert(screen().nodes.find(node => node.id === 'chat_text').text.includes('SIM_CORE_PLUGIN_OK'),
      'plugin reply must reach the rendered Edge UI');
    while (screen().chatScrollLine > 0 &&
      !screen().nodes.find(node => node.id === 'chat_text').text.includes('daily_life:get_current_date')) {
      assert.equal(ui.ccall('operit_ui_debug_swipe', 'number', ['string'], ['down']), 1);
    }
    assert(screen().nodes.find(node => node.id === 'chat_text').text.includes('daily_life:get_current_date'),
      'Core projects the public tool name as scrollable ordinary display text');
    assert(!JSON.stringify(pluginReply.chat.messages).includes('\\"timestamp\\"'), 'raw tool result JSON must not enter Edge display rows');
    t.diagnostic('Edge chat triggered the real Core daily_life plugin; public name rendered without tool state or payloads');
    await update();await tap('sidebar_toggle');await tap('sidebar_plugins');
    const pluginList=await waitDevice('enabled Core plugins in Edge page',device=>
      device.plugins?.items?.some(item=>item.id==='com.operit.daily_life')&&!device.plugins.loading);
    const enabledPlugins=(await core.command(['core','plugin','list'])).filter(item=>item.enabled);
    assert.equal(pluginList.plugins.total, enabledPlugins.length);
    const displayedPlugins = pluginList.plugins.items.map(item => item.id);
    let pluginPage = pluginList.plugins;
    while (pluginPage.offset + pluginPage.items.length < pluginPage.total) {
      await update(); await tap('plugins_next');
      const offset = pluginPage.offset + 6;
      pluginPage = (await waitDevice('next bounded plugin page', device =>
        !device.plugins.loading && device.plugins.offset === offset)).plugins;
      assert(pluginPage.items.length > 0 && pluginPage.items.length <= 6);
      displayedPlugins.push(...pluginPage.items.map(item => item.id));
    }
    assert.deepEqual(displayedPlugins.sort(), enabledPlugins.map(item => item.name).sort());
    while (pluginPage.offset > 0) {
      await update(); await tap('plugins_prev');
      const offset = pluginPage.offset - 6;
      pluginPage = (await waitDevice('previous bounded plugin page', device =>
        !device.plugins.loading && device.plugins.offset === offset)).plugins;
    }
    assert(!pluginList.plugins.items.some(item=>item.id==='com.operit.workflow'),'disabled plugins are not shown');
    await update();const pluginNodes=screen().nodes.filter(item=>item.id.startsWith('plugin_probe_'));
    assert.equal(new Set(pluginNodes.map(item=>item.rect.x)).size,2);
    const pluginIndex=pluginList.plugins.items.findIndex(item=>item.id==='com.operit.daily_life');
    await tap('plugin_probe_'+pluginIndex);
    assert.equal(screen().pluginDialogOpen,true);
    const metadata=await waitDevice('bounded Core package details',device=>device.plugins.details?.id==='com.operit.daily_life'&&!device.plugins.details.loading);
    assert(metadata.plugins.details.description.length>0);
    assert.equal(metadata.plugins.details.tools.length,3,'tools are paginated, not copied wholesale');
    assert(metadata.plugins.details.toolTotal>=4);
    assert(metadata.plugins.details.tools.every(tool=>typeof tool==='string'&&!tool.includes('function')));
    await update();assert.match(screen().nodes.find(n=>n.id==='plugin_info').text,/包 ID: com.operit.daily_life/);
    await tap('plugin_tools_next');
    await waitDevice('second bounded Core tool page',device=>!device.plugins.details.loading&&device.plugins.details.toolOffset===3);
    await update();await tap('plugin_tools_prev');
    await waitDevice('first Core tool page restored',device=>!device.plugins.details.loading&&device.plugins.details.toolOffset===0);
    await update();await tap('plugin_test_connection');
    const probe=await waitDevice('Core plugin availability probe',device=>
      device.plugins.items[pluginIndex]?.status==='success');
    assert(probe.plugins.items[pluginIndex].latencyMs>=0);
    await update();await tap('plugin_test_tool');
    const toolProbe=await waitDevice('real plugin diagnostic tool call',device=>
      device.plugins.items[pluginIndex]?.toolStatus==='success');
    assert(toolProbe.plugins.items[pluginIndex].toolLatencyMs>=0);
    await update();assert(screen().nodes.find(n=>n.id==='plugin_tool_result').text.includes('成功'));
    await tap('plugin_dialog_close');
    await tap('plugins_test_all');
    const batch=await waitDevice('all enabled plugin connection functions executed',device=>
      !device.plugins.testing&&device.plugins.tested===enabledPlugins.length);
    assert(batch.plugins.failed >= batch.plugins.items.filter(item=>item.status==='failure').length);
    assert(batch.plugins.failed <= batch.plugins.tested);
    assert(batch.plugins.items.every(item=>item.status==='success'||(item.status==='failure'&&item.testError)),
      'each enabled plugin must report its actual diagnostic result, including optional dependencies');
    assert.equal(batch.plugins.items.find(item=>item.id==='com.operit.daily_life').status,'success');
    let uiOnlyPage = batch.plugins;
    while (!uiOnlyPage.items.some(item => item.id === 'com.operit.thinking_guidance') &&
      uiOnlyPage.offset + uiOnlyPage.items.length < uiOnlyPage.total) {
      await update(); await tap('plugins_next');
      const offset = uiOnlyPage.offset + 6;
      uiOnlyPage = (await waitDevice('UI-only plugin page', device =>
        !device.plugins.loading && device.plugins.offset === offset)).plugins;
    }
    const uiOnlyIndex = uiOnlyPage.items.findIndex(item => item.id === 'com.operit.thinking_guidance');
    assert(uiOnlyIndex >= 0);
    await update();await tap('plugin_probe_'+uiOnlyIndex);
    const noTool=await waitDevice('UI-only plugin metadata has no business tools',device=>
      device.plugins.details?.id==='com.operit.thinking_guidance'&&!device.plugins.details.loading);
    assert.equal(noTool.plugins.details.toolTotal,0);
    await update();assert(!screen().nodes.some(n=>n.id==='plugin_test_tool'));
    assert(!screen().nodes.find(n=>n.id==='plugin_info').text.includes('工具:'));
    await tap('plugin_dialog_close');
    await tap('plugins_exclusive');
    await waitDevice('exclusive tab is an explicit empty category',device=>device.plugins.category==='exclusive'&&!device.plugins.loading&&device.plugins.total===0);
    await update();assert.equal(screen().pluginCategory,'exclusive');
    assert.match(screen().nodes.find(n=>n.id==='plugins_empty').text,/专属/);
    await tap('plugins_general');
    await waitDevice('general tab restores enabled Core packages',device=>device.plugins.category==='general'&&!device.plugins.loading&&device.plugins.total===enabledPlugins.length);
    await update();
    // Stale enabled row must turn red if the Core disabled it in the meantime.
    await core.command(['core','plugin','disable','com.operit.daily_life']);
    await update();await tap('plugin_probe_'+pluginIndex);
    await waitDevice('disabled package metadata reports bounded error',device=>device.plugins.details?.id==='com.operit.daily_life'&&!device.plugins.details.loading);
    await update();await tap('plugin_test_connection');
    const failedProbe=await waitDevice('disabled plugin probe failure',device=>
      device.plugins.items[pluginIndex]?.status==='failure');
    assert(failedProbe.plugins.items[pluginIndex].latencyMs>=0);
    // A package change may also reopen Core chat watches. Paint the exact
    // acknowledged probe projection rather than racing that next session.
    failedProbe.plugins.items.forEach((item,index)=>call('set_plugin',
      ['number','string','string','number','number'],[index,item.id,item.name,
        item.status==='success'?2:item.status==='failure'?3:0,item.latencyMs??0]));
    call('finish_plugins',['number','number','number','number','string'],
      [failedProbe.plugins.items.length,failedProbe.plugins.offset,failedProbe.plugins.total,0,'']);
    call('set_plugin_test',['number','number','number','string'],[pluginIndex,0,0,'']);
    if(screen().pluginDialogOpen)call('debug_tap',['string'],['plugin_dialog_close']);
    const failedNode=screen().nodes.find(item=>item.id==='plugin_probe_'+pluginIndex);
    assert(failedNode,'failed plugin row must stay visible: '+JSON.stringify(screen()));
    assert.equal(failedNode.probeState,3);
    await core.command(['core','plugin','enable','com.operit.daily_life']);
    await tap('plugins_refresh');
    await waitDevice('plugin refresh clears old probe latency',device=>!device.plugins.loading&&
      device.plugins.items.some(item=>item.id==='com.operit.daily_life'&&item.status==='untested'));
    t.diagnostic('exclusive/general tabs, bounded package ID/description/tools, hidden empty tools and pagination verified; existing Core diagnostic exports report real RTT');
    call('navigate_home');
    const requestsBeforeRestart = providerRequests.length;
    await core.stop();
    await waitDevice('chat offline after Core stops', device => !device.chat.connected);
    await startCore();
    await waitDevice('automatic chat reconnect after Core restart', device => device.chat.connected && device.chat.chatId === chatId && !device.chat.initializing && !device.chat.error);
    await post('/api/simulator/send', {text: 'SIM_RECONNECT'});
    const secondReply = await waitDevice('reply after reconnect', device =>
      device.chat.messages.some(message => message.text === 'SIM_RECONNECT_OK') && !device.chat.generating);
    assert(secondReply.chat.messages.some(message => message.text === 'SIM_CHAT_OK'), 'previous history must survive reconnect');
    assert.equal(providerRequests.length, requestsBeforeRestart + 1);
    assert.deepEqual(await core.command(['peers']), credentials, 'Core restart must not require pairing again');
    t.diagnostic('Core stop produced offline; restart restored the same chat and a second streamed round-trip without re-pairing');

    // A different UI (the real Core CLI) writes the selected chat. The Edge
    // observes the owner subscription and pages one long message without replay.
    const requestsBeforeExternal = providerRequests.length;
    await core.command(['core', 'chat', 'send', '--chat', chatId, 'SIM_EXTERNAL_SYNC']);
    let external = await waitDevice('Core-originated long message reaches the Edge', device =>
      device.chat.messages.some(message => message.text.includes('SYNC_LINE_089')) && !device.chat.generating);
    const seenLines = new Set();
    let turns = 0;
    while (true) {
      const text = external.chat.messages.map(message => message.text).join('\n');
      assert(Buffer.byteLength(text) <= 640 + external.chat.messages.length, 'one bounded device display window');
      for (const match of text.matchAll(/SYNC_LINE_\d{3}/g)) seenLines.add(match[0]);
      if (seenLines.size === 90) break;
      assert(external.chat.hasOlder && turns++ < 20, 'the complete long message must be reachable');
      const previous = JSON.stringify(external.chat.messages);
      await post('/api/simulator/action', {action:'edge_history_older'});
      external = await waitDevice('older byte/line page of the same message', device =>
        JSON.stringify(device.chat.messages) !== previous);
    }
    assert.deepEqual([...seenLines].sort(), Array.from({length:90}, (_, index) => 'SYNC_LINE_' + String(index).padStart(3, '0')));
    while (external.chat.hasNewer) {
      assert(turns-- > 0);
      const previous = JSON.stringify(external.chat.messages);
      await post('/api/simulator/action', {action:'edge_history_newer'});
      external = await waitDevice('restore the latest long-message page', device =>
        JSON.stringify(device.chat.messages) !== previous);
    }
    assert(external.chat.messages.some(message => message.text.includes('SYNC_LINE_089')));
    assert.equal(providerRequests.length, requestsBeforeExternal + 1, 'sync and pagination must not replay the send');
    t.diagnostic('Core CLI wrote the active chat; Edge subscription received it and recovered all 90 lines through bounded pages without resending');
    const longInput = 'SIM_LONG_INPUT\n' + '流式长文本🙂'.repeat(850) + '\n完整末尾标记';
    assert(Buffer.byteLength(longInput) > 8192 && Buffer.byteLength(longInput) <= 16384);
    const beforeLong = providerRequests.length;
    await post('/api/simulator/send', {text:longInput, sendId:'long-utf8-input'});
    await waitDevice('complete long text reaches the existing Core generation pipeline',device=>
      device.chatSendResult?.sendId==='long-utf8-input' && device.chatSendResult.ok &&
      device.chat.messages.some(message=>message.text==='SIM_LONG_INPUT_OK') && !device.chat.generating);
    assert.equal(providerRequests.length,beforeLong+1);
    assert(providerRequests.at(-1).messages.some(message=>
      JSON.stringify(message.content).includes(longInput.replaceAll('\n','\\n')) ||
      (typeof message.content==='string' && message.content.includes(longInput))),
      'provider must receive every UTF-8 character including the final marker');
    const persistedLong = await core.command(['core','chat','show',chatId]);
    assert(JSON.stringify(persistedLong).includes('完整末尾标记'));
    await assert.rejects(post('/api/simulator/send',{text:'字'.repeat(6000)}),/16384/);
    assert.equal(providerRequests.length,beforeLong+1,'rejected oversized input must not start a turn');
    t.diagnostic('Long UTF-8 input crossed the 8 KiB frame boundary in 2 KiB chunks; Core provider and history retained the final marker exactly once');
    await leave();
    assert.deepEqual(await core.command(['space', 'show']), former, 'the same Core retains the previous Space');
    const cancelled = await core.command(['space', 'join', node]);
    await update(); await openReview(); assert.equal(screen().page, 'Space');
    assert.equal((await core.command(['space', 'cancel', cancelled.requestId])).status, 'cancelled');
    // Cancel won just before a tap on a still-visible approval prompt. A normal
    // decision error must NOT terminate the simulated device or its listener.
    actionError = ''; await tap('edge_space_approve');
    assert.match(actionError, /当前没有待审批|no longer awaiting|not.*awaiting/i,
      `stale UI approval must return an ordinary error without killing the simulator: ${actionError}`);
    await waitDevice('cancelled review disappears after the shared review poll', device => device.spaceJoinPrompt === '');
    t.diagnostic('Core cancelled the same-Core rejoin; stale approval tap did not crash the simulator');
    const second = await core.command(['space', 'join', node]);
    // Before the next UI poll the rendered button still belongs to the
    // CANCELLED request. It must not silently approve this new submission.
    actionError = ''; await tap('edge_space_approve');
    assert(actionError, 'a stale approval must not be retargeted to the next pending request');
    assert.equal((await core.command(['space', 'refresh', second.requestId])).status, 'pending');
    t.diagnostic('a stale approval for cancelled A did not approve the newly submitted B');
    await update(); await openReview();
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
    await update(); await openReview(); assert.equal(screen().page, 'Space');
    await core.stop(); await post('/api/simulator/stop');
    await startCore();
    const restarted = await startEdge(); assert.equal(Object.hasOwn(restarted, 'token'), false);
    await update(); await openReview(); assert.equal(screen().page, 'Space', 'approval prompt survives process restart');
    assert.equal((await core.command(['space', 'cancel', pending.requestId])).status, 'cancelled');
    await waitDevice('cancelled persisted review disappears', device => device.spaceJoinPrompt === '');
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
    await waitDevice('offline cancellation removes the review', device => device.spaceJoinPrompt === '');
    await update(); assert.equal(screen().nodes.some(value => value.id === 'edge_space_approve'), false);
    const afterOfflineCancel = await core.command(['space', 'join', node]);
    await approve();
    assert.equal((await core.command(['space', 'refresh', afterOfflineCancel.requestId])).status, 'joined');
    assert.deepEqual(await core.command(['peers']), credentials, 'pairing credentials must not be replaced');
    const edgeFiles = await (await fetch(base + '/api/simulator/storage')).json();
    assert(edgeFiles.some(file => file.startsWith('runtime/link_access/')), 'inspect the logical device records');
    assert(!edgeFiles.some(file => /^sync\//.test(file)), 'non-storage Edge must not create business replication journals');
    assert(!edgeFiles.some(file => /(^|\/)(chat|chats|messages|blobs|models)\//.test(file)), 'no business data copied to Edge storage');
    assert.equal(screen().heapBytes, 0); assert(screen().staticBytes < 10 * 1024);
    t.diagnostic('offline cancellation intent survived both restarts; refresh retried it and reapplication completed with pairing intact and no business replica');

    async function probeDeviceAccess() {
    // A simulated device may initiate onboarding itself, with the Core as
    // the reviewer. Reuse TCP pairing and the ordinary Space approval records.
    if ((await state()).device.paired) {
      await leave();
      await core.command(['unpair', node]);
      await post('/api/simulator/action', {action:'edge_unpair'});
    }
    await core.command(['space','leave']);
    const targetCore = (await core.command(['space','show'])).members.find(id => id !== node);
    const access = await post('/api/simulator/core-access', {step:'request', nodeId:targetCore,
      address:coreAddress});
    assert.equal(access.pairing.peerNodeId, targetCore);
    const duplicate = await post('/api/simulator/core-access', {step:'request', nodeId:targetCore});
    assert.equal(duplicate.pairing.pairingId, access.pairing.pairingId);
    const prompt = (await core.command(['prompts'])).find(value => value.pairingId === access.pairing.pairingId);
    assert(prompt, 'Core receives the simulator-initiated pairing request');
    await assert.rejects(post('/api/simulator/core-access', {step:'confirm',
      pairingId:access.pairing.pairingId, confirmationCode:'123'}), /六位/);
    const submission = await post('/api/simulator/core-access', {step:'confirm',
      pairingId:access.pairing.pairingId, confirmationCode:prompt.confirmationCode});
    assert.equal(submission.pairing, null);
    assert.equal(submission.request.status, 'pending');
    const incoming = (await core.command(['space','requests','incoming']))
      .find(value => value.requestId === submission.request.requestId);
    assert(incoming?.canApprove, 'the real Core has a pending approval for the simulator');
    await core.command(['space','approve',incoming.requestId,String(incoming.assignmentVersion)]);
    const joined = await post('/api/simulator/core-access', {step:'refresh',requestId:incoming.requestId});
    assert.equal(joined.request.status, 'joined');
    assert.equal((await post('/api/simulator/core-access', {step:'request',nodeId:targetCore})).joined, true);
    await state();
    t.diagnostic('simulated device initiated TCP pairing and a Core-reviewed permission request');
    }
    await probeDeviceAccess();


  });
