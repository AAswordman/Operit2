import test from 'node:test';
import {decodePacks, paintScene} from '../../web/scene-renderer.ts';
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
          // XML tool results also arrive as user turns; do not mistake them
          // for the user's latest request when selecting the fixture response.
          const lastUserIndex = input.messages.findLastIndex(message =>
            message.role === 'user' && !String(message.content).includes('status=') &&
            /SIM_(CHAT|RECONNECT|CORE_PLUGIN)/.test(JSON.stringify(message.content)));
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
      const plugins=device.plugins ?? {};
      call('set_plugin_category',['number'],[plugins.category==='exclusive'?1:0]);
      (plugins.items ?? []).slice(0,6).forEach((item,index)=>{
        call('set_plugin',['number','string','string','number','number'],[index,item.id,item.name,
          item.status==='probing'?1:item.status==='success'?2:item.status==='failure'?3:0,item.latencyMs??0]);
        call('set_plugin_test',['number','number','number','string'],[index,
          item.toolStatus==='probing'?1:item.toolStatus==='success'?2:item.toolStatus==='failure'?3:0,
          item.toolLatencyMs??0,item.testError??'']);
      });
      const details=plugins.details??{};
      call('set_plugin_details',['string','string','string','number','number','number','string'],
        [details.id??'',details.description??'',(details.tools??[]).join('\n'),details.toolOffset??0,details.toolTotal??0,details.loading?1:0,details.error??'']);
      call('set_plugin_testing',['number'],[plugins.testing?1:0]);
      call('finish_plugins',['number','number','number','number','string'],
        [(plugins.items??[]).length,plugins.offset??0,plugins.total??0,plugins.loading?1:0,plugins.error??'']);
      call('set_chat_screen', ['string'], [device.chatScreen]);
      call('set_chat_task', ['string'], [device.chatTask]);
      call('set_scene', ['number','number','number'], [device.scene?.active ? 1 : 0, device.scene?.revision ?? 0, 0]);
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
    const edgePortArgs={node_id:node,interface_info:{pluginId:'device.status',action:'read'},args:{}};
    await assert.rejects(()=>core.command(['core','tool','exec','edge_execute',JSON.stringify(edgePortArgs)]),
      'plugin port API must not pair, route to a guessed address, or grant access to an unadmitted node');
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
    const initialChat = await waitDevice('automatic remote chat provisioning after admission',
      device => device.chat.connected && device.chat.chatId && !device.chat.initializing && !device.chat.error);
    assert.match(initialChat.chat.chatId, /^[0-9a-f-]{36}$/i, 'first admitted session has a valid nonempty chat ID');
    t.diagnostic('first chat was provisioned through Core before message/state subscriptions opened');
    const edgePort=await core.command(['core','tool','exec','edge_execute',JSON.stringify(edgePortArgs)]);
    assert.equal(edgePort.success,true);assert.equal(edgePort.result.nodeId,node);
    assert.equal(edgePort.result.data.boardId,'ESP32-2432S028-SIM');
    await assert.rejects(()=>core.command(['core','tool','exec','edge_execute',JSON.stringify({...edgePortArgs,
      interface_info:{pluginId:'device.status',action:'undeclared'}})]),/not declared|not found/i);
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
    const fixtureMetadata = {name:'edge_ports_fixture', description:'Isolated hardware SDK integration test',
      enabledByDefault:true, tools:[{name:'probe', description:'Read a declared Edge native action',
        parameters:[{name:'node_id',description:'Explicit Edge node',type:'string',required:true}]}]};
    await writeFile(fixture, `/* METADATA
${JSON.stringify(fixtureMetadata)}
*/
      exports.probe = async function(params) {
        const response = await tools.edge.execute(params.node_id, {pluginId:'device.status',action:'read'}, {});
        let unsupported = '';
        try { await tools.io.execute(params.node_id, {port:'serial',operation:'read'}, {}); }
        catch(error) { unsupported = String(error.message || error); }
        let missingHost = '';
        try { await tools.io.execute(params.node_id, {port:'gpio',operation:'read'}, {pin:2}); }
        catch(error) { missingHost = String(error.message || error); }
        return {nodeId:response.nodeId, boardId:response.data.boardId, sameAlias:tools === Tools, unsupported, missingHost};
      };
    `);
    await core.command(['core','package','import',fixture]);
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
    t.diagnostic('Real Core JS package -> tools.edge/tools.io -> existing tool runtime -> authenticated Edge action passed');

    // The same market-compatible ToolPkg ships as an opt-in "More packages"
    // asset, not an auto-installed built-in or an ESP JavaScript runtime.
    const more = await core.command(['core','package','more']);
    const candidate = more.find(item => item.name === 'com.operit.edge_pixel_pet');
    assert(candidate, 'pixel pet must ship in the Core More packages catalog');
    assert.equal(candidate.type, 'toolpkg'); assert.equal(candidate.loaded, false);
    assert(!(await core.command(['core','package','list'])).some(item => item.name === 'com.operit.edge_pixel_pet'),
      'bundled pet must not be automatically installed');
    const loaded = await core.command(['core','package','load','com.operit.edge_pixel_pet']);
    assert.match(loaded.message, /Successfully imported/);
    const installed = (await core.command(['core','package','list'])).find(item => item.name === 'com.operit.edge_pixel_pet');
    assert(installed); assert.equal(installed.enabled, false); assert.equal(installed.enabledByDefault, false);
    await core.command(['core','plugin','enable','com.operit.edge_pixel_pet']);
    const pet = async tool => {
      const reply=await core.command(['core','package','exec',`edge_pixel_pet:${tool}`,JSON.stringify({node_id:node})]);
      assert.equal(reply.success,true);
      const value=reply.result?.value ?? reply.result;
      return typeof value==='string' ? JSON.parse(value) : value;
    };
    const nativeScene = async(action,args) => {
      assert(Buffer.byteLength(JSON.stringify({v:1,...args}))<=1024);
      const reply=await core.command(['core','tool','exec','edge_execute',JSON.stringify({node_id:node,interface_info:{pluginId:'display.scene',action},args:{v:1,...args}})]);
      const data=reply.result.data;assert(Buffer.byteLength(JSON.stringify(data))<=4096);return data;
    };
    try {
      const caps=(await nativeScene('capabilities',{})).result;
      assert.equal(caps.cache.persistent,false);assert.equal(caps.screen.width,320);
      const started=await pet('start_pet');assert.equal(started.taps,0);await update();assert.equal(screen().page,'edge_scene');
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
          const state=await pet('pet_status'); // Core state only, never Edge input polling
          if(predicate(state)) return state;
          await new Promise(resolve=>setTimeout(resolve,100));
        }
        throw new Error('Edge-initiated scene callback did not persist on Core');
      };
      const notified=await waitPet(state=>state.taps===1);assert.equal(notified.closed,false);
      const again=await pet('pet_status');assert.equal(again.taps,1);
      const renew=await pet('renew_pet');assert.equal(renew.closed,false);
      // Burst + exit must drain bounded push batches, not a Core polling loop.
      for(let i=0;i<15;i++) {ui._simulator_touch(144,146,1);ui._simulator_touch(144,146,0);await Promise.all(actions.splice(0));}
      await tap('scene_exit');await update();assert.notEqual(screen().page,'edge_scene');
      const exited=await waitPet(state=>state.closed==='system.exit');assert.equal(exited.taps,16);assert.equal(exited.lostEvents,false);
      assert.equal((await pet('renew_pet')).closed,'system.exit','keepalive never reopens an exited scene');
      const restarted=await pet('start_pet');assert.equal(restarted.taps,16,'business data remains on Core across display leases');
      await pet('close_pet');await waitPet(state=>state.closed==='lease.closed');await update();assert.equal(screen().page,'Chat');
      assert.equal((await nativeScene('capabilities',{})).result.cache.packs.length,1,'cached original assets reused');
      call('scene_painter',['number'],[0]);ui.removeFunction(painter);
    } finally {
      await core.command(['core','plugin','disable','com.operit.edge_pixel_pet']);
      await core.command(['core','package','delete','edge_pixel_pet']);
    }
    t.diagnostic('Bundled More-package ToolPkg -> real Core tools.edge -> shared native display.scene: chunked SHA-verified assets, local strip rendering, Edge-initiated touch/exit events without Core event polling, Core-owned deduplication, protected exit and cache reuse passed');


    const provider = await core.command(['core', 'model', 'provider-create', 'Simulator deterministic provider',
      'OPENAI_GENERIC', base + '/v1/chat/completions']);
    await core.command(['core', 'model', 'provider-set-key', provider.providerId, 'isolated-test-key']);
    await core.command(['core', 'model', 'provider-model-create', provider.providerId, 'sim-model']);
    await core.command(['core', 'model', 'use', provider.providerId, 'sim-model']);
    // Create through the real rendered button; send via the editor's public API.
    await update(); call('navigate_home'); await tap('sidebar_toggle'); await tap('sidebar_settings'); await tap('settings_device'); await tap('edge_new');
    const created = await waitDevice('Edge-created chat', device => device.chat.chatId && device.chat.chatId !== initialChat.chat.chatId && !device.chat.sending);
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
    // Exercise the same Edge send/Binding route with a real bundled Core
    // ToolPkg. Inspect structured persisted parts, not just a model's claim.
    const beforePlugin = providerRequests.length;
    await post('/api/simulator/send', {text: 'SIM_CORE_PLUGIN: call the Core daily-life date plugin'});
    const pluginReply = await waitDevice('Core plugin result on Edge', device =>
      device.chat.messages.some(message => message.text.includes('SIM_CORE_PLUGIN_OK ')) &&
      device.chat.messages.some(message => message.text.includes('\x1eS|daily_life:get_current_date\x1f')) && !device.chat.generating);
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
    assert(screen().toolCards.some(card => card.icon === 'plugin' && card.name === 'daily_life:get_current_date' && card.status === '成功'),
      'real Core tool execution must render as a named plugin card with success status');
    assert(!JSON.stringify(pluginReply.chat.messages).includes('\\"timestamp\\"'), 'raw tool result JSON must not enter Edge display rows');
    t.diagnostic('Edge chat triggered the real Core daily_life plugin; named success card rendered without raw tool payloads');
    await update();await tap('sidebar_toggle');await tap('sidebar_plugins');
    const pluginList=await waitDevice('enabled Core plugins in Edge page',device=>
      device.plugins?.items?.some(item=>item.id==='com.operit.daily_life')&&!device.plugins.loading);
    const enabledPlugins=(await core.command(['core','plugin','list'])).filter(item=>item.enabled);
    assert.deepEqual(pluginList.plugins.items.map(item=>item.id).sort(),enabledPlugins.map(item=>item.name).sort());
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
    assert.equal(batch.plugins.failed,0);
    const uiOnlyIndex=batch.plugins.items.findIndex(item=>item.id==='com.operit.thinking_guidance');
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
