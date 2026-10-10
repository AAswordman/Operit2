import { composeStreamFixture } from '../../../../../tools/tests/support/compose_stream_fixture.mjs';
import assert from 'node:assert/strict';
import test from 'node:test';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
import { readFile, access, writeFile } from 'node:fs/promises';
import { buildUiScreenScripts } from '../scripts/build.mjs';
import { packageRuntime, sdkScript } from './package-runtime.mjs';
import { composeNodes, keyedNode } from './selector-render.mjs';

let modules;
test.before(async () => { modules = await buildUiScreenScripts(); });
const summary = (id, changes = {}) => ({ id, title: '对话 ' + id, updatedAt: '1791059541109', displayOrder: Number(id.slice(1)) || 0,
  workspaceId: null, workspaceName: null, locked: false, pinned: false, group: id === "c1" || id === "c2" ? "行程分组" : null, ...changes });
function context(chats, changes = {}) { return { input: { view: 'characters' }, chatSidebar: { chats, currentChatId: 'c1', activeStreamingChatIds: [], ...changes } }; }
function catalog(chats) { return { view: 'characters', sections: [
  { id: 'card:travel', title: '旅行角色', avatarUri: null, kind: 'card', selection: 'card:travel', chats,
    conversationGroups: [{ id: '1', name: '行程分组', pinned: true, displayOrder: 0, chats: chats.slice(0, 2) }], ungroupedChats: chats.slice(2) },
  { id: 'card:empty', title: '没有对话的角色', avatarUri: null, kind: 'card', selection: 'card:empty', chats: [], conversationGroups: [], ungroupedChats: [] },
] }; }
/** Real SDK serialization, callbacks and persistent screen state; only IPC/file/chat transports are labelled test adapters. */
function mount(current, data, adapters = {}) {
  const calls = [], globals = { ToolPkg: {
    ipc: { call: async (name, request, options) => { calls.push({ name, request, options }); if (name === 'character-sidebar.catalog') return adapters.catalog ? adapters.catalog(request) : data;
      assert.equal(name, 'character-memory.domain');
      if (adapters.domain) return adapters.domain(request);
      throw new Error('No labelled domain adapter supplied'); } },
    readResource: async (key, path) => { calls.push({ resource: key, path }); return '/test/operit-avatar.png'; },
  } };
  globals.Tools = { Files: { readBinary: adapters.readBinary ?? (async path => ({ path, size: 12, contentBase64: 'iVBORw0KGgo=' })) }, ...(adapters.chat ? { Chat: adapters.chat } : {}) };
  const compiledModules = { ...modules };
  const runtime = packageRuntime(compiledModules, globals);
  runtime.context.module = { exports: {} };
  runtime.context.exports = runtime.context.module.exports;
  vm.runInContext(readFileSync(new URL('../../../../../core/crates/plugin/sdk/src/toolpkg/vendor/acorn.js', import.meta.url), 'utf8'), runtime.context);
  runtime.context.__operitAcorn = runtime.context.module.exports;
  vm.runInContext(readFileSync(new URL('../../../../../core/crates/plugin/sdk/src/toolpkg/ToolPkgComposeDslCompiler.js', import.meta.url), 'utf8'), runtime.context);
  for (const [modulePath, bytes] of Object.entries(compiledModules)) {
    compiledModules[modulePath] = runtime.context.OperitComposeCompiler.compile(Buffer.from(bytes).toString('utf8'), 'com.operit.character_cards:' + modulePath);
  }
  runtime.context.module = { exports: runtime.load('dist/ui/chat-sidebar/index.ui.js') };
  for (const file of ['ToolPkgComposeDslRetained.js', 'ToolPkgComposeDslReactive.js']) {
    vm.runInContext(readFileSync(new URL('../../../../../core/crates/plugin/sdk/src/toolpkg/' + file, import.meta.url), 'utf8'), runtime.context);
  }
  vm.runInContext(sdkScript('ToolPkgComposeDslBridge.rs'), runtime.context);
  vm.runInContext(sdkScript('ToolPkgComposeDslRuntimeScript.rs').replaceAll('{{', '{').replaceAll('}}', '}').replace('{script}', ''), runtime.context);
  composeStreamFixture(runtime.context).adapt();
  const options = { packageName: 'com.operit.character_cards', executionContextKey: 'native-sidebar-test' };
  const first = runtime.context.__operit_render_compose_dsl({ ...options, state: current });
  const session = { first, calls,
    bundle() { return runtime.context.__operit_compose_bundle; },
    update(next) { return runtime.context.__operit_render_compose_dsl({ ...options, __operit_update_inputs: true, __operit_input_state: next }); },
    render() { return runtime.context.__operit_render_compose_dsl({ ...options, __operit_update_inputs: true }); },
    dispatch(action, payload) { return runtime.context.__operit_dispatch_compose_dsl_action({ ...options, actionId: action.__actionId, __action_payload: payload }); },
    load() { return session.dispatch(first.tree.props.onLoad); },
    async click(tree, key) { return session.dispatch(keyedNode(tree, key).props.onClick); },
  };
  return session;
}

test('native role sidebar restores rounded nested legacy layout without timestamps or browser nodes', async () => {
  const chats = [summary('c1', { pinned: true }), summary('c2', { locked: true }), summary('c3')];
  const screen = mount(context(chats, { activeStreamingChatIds: ['c2'] }), catalog(chats));
  const { tree } = await screen.load();
  if (process.env.OPERIT_NATIVE_SIDEBAR_FIXTURE) await writeFile(process.env.OPERIT_NATIVE_SIDEBAR_FIXTURE, JSON.stringify(tree, null, 2) + '\n');
  assert.equal(tree.type, 'Column');
  assert.equal(tree.props.fillMaxWidth, true);
  assert.equal(tree.props.fillMaxSize, undefined);
  assert.equal(tree.props.height, undefined);
  const content = keyedNode(tree, 'native-sidebar-history');
  assert.equal(content.type, 'Column');
  assert.equal(content.props.weight, undefined);
  assert.equal(content.props.height, undefined);
  assert.equal(composeNodes(tree, node => node.type === 'LazyColumn').length, 0);
  assert.equal(composeNodes(tree, node => node.type === 'WebView').length, 0);
  assert.equal(composeNodes(tree, node => node.type === 'Text' && node.props.text === '1791059541109').length, 0);
  assert.equal(keyedNode(tree, 'sidebar-create-bar').props.height, 34);
  assert.equal(keyedNode(tree, 'sidebar-create-bar').props.shape.type, 'pill');
  assert.equal(keyedNode(tree, 'sidebar-avatar-card:travel').props.width, 22);
  assert.equal(keyedNode(tree, 'sidebar-avatar-card:travel').props.uri, 'data:image/png;base64,iVBORw0KGgo=');
  assert.equal(keyedNode(tree, 'sidebar-section-card:travel').props.paddingStart, 20);
  assert.equal(composeNodes(tree, node => node.props.key === 'sidebar-section-card:empty').length, 0);
  const group = keyedNode(tree, 'sidebar-group-1');
  assert.equal(group.props.paddingStart, 46);
  assert.equal(composeNodes(group, node => node.props.backgroundShape?.cornerRadius === 12).length, 1);
  const chat = keyedNode(tree, 'sidebar-chat-c1');
  assert.equal(chat.type, 'Row');
  assert.equal(chat.props.height, 34); assert.equal(chat.props.paddingStart, 56);
  assert.equal(keyedNode(tree, 'sidebar-chat-rail-c1').props.width, 20);
  const selected = keyedNode(tree, 'sidebar-chat-hit-c1');
  assert.equal(selected.props.modifier.__modifierOps[0].args[1].cornerRadius, 8);
  const combined = selected.props.modifier.__modifierOps.find(op => op.name === 'combinedClickable').args[0];
  assert.ok(combined.onLongClick.__actionId);
  assert.equal(keyedNode(tree, 'sidebar-status-c2').props.running, true);
  const draggable = keyedNode(tree, 'sidebar-chat-drag-c1');
  assert.equal(draggable.type, 'Draggable');
  assert.equal(draggable.props.data, 'c1');
  assert.ok(draggable.slots.feedback);
  const target = keyedNode(tree, 'sidebar-chat-drop-c1');
  assert.equal(target.type, 'DragTarget');
  assert.deepEqual(Array.from(target.props.acceptedData), ['c2', 'c3']);
  const swipe = keyedNode(tree, 'sidebar-chat-swipe-c1');
  assert.equal(swipe.type, 'SwipeActions');
  assert.ok(swipe.props.onStartAction.__actionId); assert.ok(swipe.props.onEndAction.__actionId);
  assert.equal(keyedNode(tree, 'sidebar-group-menu-1').type, 'PopupMenu');
  assert.equal(keyedNode(tree, 'sidebar-group-drop-1').type, 'DragTarget');
  assert.equal(keyedNode(tree, 'sidebar-category-drop-card:travel').type, 'DragTarget');
  const title = composeNodes(tree, node => node.type === 'Text' && node.props.text === '旅行角色')[0];
  assert.equal(title.props.weight, undefined);
  assert.equal(composeNodes(tree, node => node.type === 'GradientRule')[0].props.weight, 1);
  assert.equal(composeNodes(tree, node => /Conversation/.test(node.type)).length, 0);
  assert.equal(composeNodes(tree, node => node.type === 'HoverRegion' && node.props.onHover).length, 0, 'Hover must not call JS');
  assert.deepEqual(JSON.parse(JSON.stringify((await screen.dispatch(combined.onClick)).actionResult)), { type: 'toolpkg.chat.activate', chatId: 'c1' });
});

test('native group collapse keeps the category and other conversations visible', async () => {
  const chats = [summary('c1'), summary('c2'), summary('c3')], screen = mount(context(chats), catalog(chats));
  const first = await screen.load();
  const groupHit = composeNodes(keyedNode(first.tree, 'sidebar-group-1'), node => node.props.onClick)[0];
  const collapsed = await screen.dispatch(groupHit.props.onClick);
  assert.equal(composeNodes(collapsed.tree, node => node.props.key === 'sidebar-chat-c1').length, 0);
  assert.equal(keyedNode(collapsed.tree, 'sidebar-chat-c3').type, 'Row');
  assert.equal(keyedNode(collapsed.tree, 'sidebar-section-card:travel').type, 'Column');
});

test('native previews retain current pinned and streaming rows, and search finds hidden rows', async () => {
  const chats = Array.from({ length: 10 }, (_, i) => summary('c' + i, { pinned: i === 6 }));
  const data = catalog(chats); data.sections[0].conversationGroups = []; data.sections[0].ungroupedChats = chats;
  const screen = mount(context(chats, { currentChatId: 'c8', activeStreamingChatIds: ['c9'] }), data);
  const loaded = await screen.load();
  assert.equal(composeNodes(loaded.tree, node => /^sidebar-chat-c\d+$/.test(node.props.key ?? '')).length, 7);
  assert.equal(keyedNode(loaded.tree, 'sidebar-chat-c8').type, 'Row');
  assert.equal(keyedNode(loaded.tree, 'sidebar-chat-c9').type, 'Row');
  assert.equal(composeNodes(loaded.tree, node => node.props.key === 'sidebar-chat-c5').length, 0);
  const opened = await screen.click(loaded.tree, 'sidebar-search-toggle');
  const search = keyedNode(opened.tree, 'sidebar-search');
  const filtered = await screen.dispatch(search.props.onValueChange, '对话 c5');
  assert.equal(keyedNode(filtered.tree, 'sidebar-chat-c5').type, 'Row');
  assert.equal(composeNodes(filtered.tree, node => /^sidebar-chat-c\d+$/.test(node.props.key ?? '')).length, 1);
});

test('obsolete sidebar browser files, registration and archive builder are removed', async () => {
  for (const path of ['web/sidebar.ts', 'web/features/sidebar/controller.ts', 'resources/character-sidebar.html']) {
    await assert.rejects(access(new URL('../' + path, import.meta.url)), { code: 'ENOENT' });
  }
  const manifest = JSON.parse(await readFile(new URL('../manifest.json', import.meta.url), 'utf8'));
  assert.equal(manifest.resources.some(resource => resource.key === 'character_sidebar_html'), false);
  const build = await readFile(new URL('../scripts/build.mjs', import.meta.url), 'utf8');
  assert.doesNotMatch(build, /buildSidebarScript|createSidebarHtmlDocument|web\/sidebar\.ts|character-sidebar\.html/);
});


test('generic long press and swipe callbacks render plugin-owned actions and confirmation dialogs', async () => {
  const chats = [summary('c1'), summary('c2'), summary('c3')], screen = mount(context(chats), catalog(chats));
  const {tree} = await screen.load();
  const hit = keyedNode(tree, 'sidebar-chat-hit-c1');
  const combined = hit.props.modifier.__modifierOps.find(op => op.name === 'combinedClickable').args[0];
  const pressed = await screen.dispatch(combined.onLongClick);
  assert.ok(keyedNode(pressed.tree, 'sidebar-edit-dialog'));
  for (const label of ['编辑名称', '上移', '下移', '置顶', '锁定', '删除'])
    assert.ok(composeNodes(pressed.tree, node => node.type === 'Text' && node.props.text === label).length > 0, label);
  const rename = await screen.dispatch(keyedNode(tree, 'sidebar-chat-swipe-c1').props.onStartAction);
  assert.equal(keyedNode(rename.tree, 'sidebar-group-name').props.value, '对话 c1');
  const deletion = await screen.dispatch(keyedNode(tree, 'sidebar-chat-swipe-c1').props.onEndAction);
  assert.ok(composeNodes(deletion.tree, node => node.type === 'Text' && node.props.text.includes('确认删除对话')).length > 0);
});

test('plugin menu delegates pin and lock to the real SDK interface, not native business widgets', async () => {
  const chats = [summary('c1'), summary('c2'), summary('c3')], mutations = [];
  const screen = mount(context(chats), catalog(chats), { chat: {
    updatePinned: async (...args) => { mutations.push(['pin', ...args]); return ''; },
    updateLocked: async (...args) => { mutations.push(['lock', ...args]); return ''; },
  }});
  const {tree} = await screen.load();
  const selected = keyedNode(tree, 'sidebar-chat-menu-c1').props.onSelected;
  await screen.dispatch(selected, 1);
  await screen.dispatch(selected, 2);
  assert.deepEqual(mutations, [['pin', 'c1', true], ['lock', 'c1', true]]);
});

test('generic drop commits native membership before canonical order, and refuses stale IDs', async () => {
  const chats = [summary('c1'), summary('c2'), summary('c3')], mutations = [];
  const screen = mount(context(chats), catalog(chats), {
    chat: { updateGroup: async (ids,name) => { mutations.push(['membership', Array.from(ids),name]); return ''; }, reorder: async ids => { mutations.push(['order', Array.from(ids)]); return ''; } },
  });
  const {tree} = await screen.load();
  const dropped = await screen.dispatch(keyedNode(tree, 'sidebar-chat-drop-c1').props.onDrop, 'c3');
  assert.deepEqual(mutations, [
    ['membership', ['c3'], '行程分组'],
    ['order', ['c3', 'c1', 'c2']],
  ]);
  const before = mutations.length;
  await assert.rejects(screen.dispatch(keyedNode(dropped.tree, 'sidebar-chat-drop-c1').props.onDrop, 'missing'), /no longer belongs/);
  assert.equal(mutations.length, before);
});

test('failed native membership does not proceed to order mutation', async () => {
  const chats = [summary('c1'), summary('c2'), summary('c3')], orders = [];
  const screen = mount(context(chats), catalog(chats), {
    chat: { updateGroup: async () => { throw new Error('labelled membership failure'); }, reorder: async ids => { orders.push(ids); return ''; } },
  });
  const {tree} = await screen.load();
  await assert.rejects(screen.dispatch(keyedNode(tree, 'sidebar-chat-drop-c1').props.onDrop, 'c3'), /labelled membership failure/);
  assert.equal(orders.length, 0);
});


test('host input refresh during a pending drag retains the live context and clears busy/loading', async () => {
  const chats = [summary('c1'), summary('c2'), summary('c3')];
  let finishOrder, enteredOrder;
  const started = new Promise(resolve => { enteredOrder = resolve; });
  const order = new Promise(resolve => { finishOrder = resolve; });
  const screen = mount(context(chats), catalog(chats), {
    domain: async () => ({}), catalog: async request => catalog(request.chatSidebar.chats),
    chat: { updateGroup: async () => '', reorder: async () => { enteredOrder(); await order; return ''; } },
  });
  const loaded = await screen.load(), bundle = screen.bundle();
  const dropping = screen.dispatch(keyedNode(loaded.tree, 'sidebar-chat-drop-c1').props.onDrop, 'c3');
  await started;
  const updated = screen.update(context([chats[2], chats[0], chats[1]].map((chat,i) => ({...chat, displayOrder:i}))));
  assert.equal(screen.bundle(), bundle, 'input updates must not replace pending refs/promises with serialized snapshots');
  await screen.dispatch(updated.tree.props.onInputsChanged);
  finishOrder();
  await dropping;
  const final = screen.render();
  assert.equal(final.state['native-sidebar-state'].busy, false);
  assert.equal(final.state['native-sidebar-state'].loading, false);
  for (const chat of chats) assert.ok(keyedNode(final.tree, 'sidebar-chat-' + chat.id));
});

test('concurrent input refresh joins the real pending promise instead of leaving a snapshot marker', async () => {
  const chats = [summary('c1'), summary('c2')];
  let finish, calls = 0;
  const wait = new Promise(resolve => { finish = resolve; });
  const screen = mount(context(chats), catalog(chats), { catalog: async () => { calls++; await wait; return catalog(chats); } });
  const loading = screen.load(), updated = screen.update(context(chats));
  const refresh = screen.dispatch(updated.tree.props.onInputsChanged);
  assert.equal(calls, 1);
  assert.ok(keyedNode(updated.tree, 'native-sidebar-history'));
  assert.equal(composeNodes(updated.tree, node => node.type === 'LoadingIndicator').length, 1);
  finish(); await Promise.all([loading, refresh]);
  assert.equal(screen.render().state['native-sidebar-state'].loading, false);
});


test('live metadata refresh retains rows, menus, search and collapse without a loading pane', async () => {
  const chats = [summary('c1'),summary('c2'),summary('c3')];
  let release, reads=0;
  const pending = new Promise(resolve => {release=resolve;});
  const screen = mount(context(chats),catalog(chats),{catalog:async request => {
    if (++reads > 1) await pending;
    return catalog(request.chatSidebar.chats);
  }});
  const loaded = await screen.load();
  await screen.click(loaded.tree,'sidebar-search-toggle');
  const changed = screen.update(context(chats.map((chat,i) => ({...chat,title:'Updated '+chat.id,updatedAt:'new',displayOrder:3-i}))));
  assert.equal(composeNodes(changed.tree,node => node.type === 'LoadingIndicator').length,0);
  assert.ok(keyedNode(changed.tree,'sidebar-chat-c1'));
  assert.ok(keyedNode(changed.tree,'sidebar-search'));
  assert.equal(composeNodes(changed.tree,node => node.type === 'Text' && node.props.text === 'Updated c1').length,1);
  const refreshing=screen.dispatch(changed.tree.props.onInputsChanged);
  const during=screen.render();
  assert.equal(during.state['native-sidebar-state'].loading,false);
  assert.equal(composeNodes(during.tree,node => node.type === 'LoadingIndicator').length,0);
  assert.ok(keyedNode(during.tree,'sidebar-chat-c1'));
  release(); await refreshing;
});

// The shared toolbar is a generic Material Surface, not a hand-painted business widget.
test('new-chat toolbar uses native Material surface shape with explicit centered spacing', async () => {
  const chats = [summary('c1')], screen = mount(context(chats), catalog(chats));
  const { tree } = await screen.load();
  const surface = keyedNode(tree, 'sidebar-create-bar');
  assert.equal(surface.type, 'Surface');
  assert.equal(surface.props.height, 34);
  assert.equal(surface.props.shape.type, 'pill');
  assert.equal(surface.props.background, undefined);
  const create = keyedNode(tree, 'sidebar-new-chat');
  assert.equal(create.props.horizontalArrangement, 'center');
  assert.equal(create.props.spacing, 6);
});

test('selection and streaming input changes rerender native rows without reloading the catalog', async () => {
  const chats = [summary('c1'), summary('c2')];
  const screen = mount(context(chats), catalog(chats));
  await screen.load();
  const next = screen.update(context(chats, { currentChatId: 'c2', activeStreamingChatIds: ['c2'] }));
  const result = await screen.dispatch(next.tree.props.onInputsChanged);
  assert.equal(screen.calls.filter(call => call.name === 'character-sidebar.catalog').length, 1);
  assert.equal(keyedNode(result.tree, 'sidebar-status-c2').props.running, true);
  assert.equal(keyedNode(result.tree, 'sidebar-status-c2').props.selected, true);
  assert.equal(keyedNode(result.tree, 'sidebar-status-c1').props.selected, false);

  const renamed = chats.map(chat => chat.id === 'c2' ? { ...chat, title: 'Renamed' } : chat);
  const changed = screen.update(context(renamed));
  await screen.dispatch(changed.tree.props.onInputsChanged);
  assert.equal(screen.calls.filter(call => call.name === 'character-sidebar.catalog').length, 2,
    'Actual metadata changes must still reload authoritative classifications');
});

test('custom avatars overlap file reads with bounded concurrency', async () => {
  const chats = Array.from({ length: 17 }, (_, i) => summary('c' + (i + 1), { group: null })), data = catalog(chats);
  data.sections = Array.from({ length: 17 }, (_, i) => ({ ...data.sections[0], id: 'card:avatar-' + i,
    selection: 'card:avatar-' + i, avatarUri: '/test/avatar-' + i + '.png',
    chats: [chats[i]], conversationGroups: [], ungroupedChats: [chats[i]] }));
  let active = 0, maximum = 0, reads = 0;
  const screen = mount(context(chats), data, { readBinary: async () => {
    reads++; active++; maximum = Math.max(maximum, active);
    try { await new Promise(resolve => setImmediate(resolve)); return { size: 12, contentBase64: 'iVBORw0KGgo=' }; }
    finally { active--; }
  } });
  const { tree } = await screen.load();
  assert.equal(reads, 17);
  assert.equal(maximum, 8);
  assert.equal(keyedNode(tree, 'sidebar-avatar-card:avatar-16').props.uri, 'data:image/png;base64,iVBORw0KGgo=');
});
