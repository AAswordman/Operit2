import assert from 'node:assert/strict';
import test from 'node:test';
import { createDiskHarness } from './disk-files.mjs';
import { openPlugin } from './plugin-entry-harness.mjs';
import { plain } from './runtime.mjs';

const summary = i => ({ id: 'loading-' + i, title: 'Chat ' + i, updatedAt: '0', displayOrder: i,
  workspaceId: null, workspaceName: null, locked: false, pinned: false, group: null });
const context = chats => ({ input: { view: 'characters' }, chatSidebar: { chats, currentChatId: null, activeStreamingChatIds: [] } });

/** Exercises real registration, file initialization and catalog projection; only extension IO is adapted. */
function sidebarPlugin(disk, readExtension) {
  return openPlugin({ ...disk, globals: { ...disk.globals, Tools: {
    Files: disk.files, Storage: disk.storage, Chat: { readExtension },
    // The sidebar must work without these unrelated management-page dependencies.
    SoftwareSettings: new Proxy({}, { get(_target, name) { throw new Error('Sidebar requested unrelated directory: ' + String(name)); } }),
  } } });
}

test('sidebar reads extensions with bounded overlap and no model, speech or tool directory requests', async t => {
  const disk = await createDiskHarness(t), requests = [];
  let active = 0, maximum = 0;
  const plugin = sidebarPlugin(disk, async target => {
    requests.push(plain(target)); active++; maximum = Math.max(maximum, active);
    try { await new Promise(resolve => setImmediate(resolve)); return null; }
    finally { active--; }
  });
  const chats = Array.from({ length: 25 }, (_, i) => summary(i));
  const result = plain(await plugin.sidebar(context(chats)));
  assert.equal(maximum, 8, 'Do not serialize round trips or enqueue an unbounded number of host requests');
  assert.equal(active, 0);
  assert.deepEqual(requests, chats.map(chat => ({ kind: 'chat', chatId: chat.id })));
  assert.deepEqual(result.sections.find(section => section.id === 'unbound').chats, chats);
  assert.ok(result.sections.some(section => section.id === 'card:default'));
});

test('sidebar refresh reads authoritative extensions again even when chat IDs are unchanged', async t => {
  const disk = await createDiskHarness(t);
  let extension = null, reads = 0;
  const plugin = sidebarPlugin(disk, async () => { reads++; return extension; });
  const request = context([summary(1)]);
  const first = plain(await plugin.sidebar(request));
  assert.equal(first.sections.find(section => section.id === 'unbound').chats.length, 1);
  extension = { version: 1, selection: 'card:default' };
  const second = plain(await plugin.sidebar(request));
  assert.equal(second.sections.find(section => section.id === 'card:default').chats[0].id, 'loading-1');
  assert.equal(second.sections.find(section => section.id === 'unbound').chats.length, 0);
  assert.equal(reads, 2);
});

test('bounded sidebar reads propagate storage and invalid selection failures instead of classifying them as unbound', async t => {
  const disk = await createDiskHarness(t), failure = new Error('Extension storage failed');
  let extension = null, error = failure;
  const plugin = sidebarPlugin(disk, async () => { if (error !== null) throw error; return extension; });
  const request = context([summary(1), summary(2)]);
  await assert.rejects(plugin.sidebar(request), err => err === failure);
  error = null; extension = { version: 1, selection: 'card:missing' };
  await assert.rejects(plugin.sidebar(request), /exactly one stored record/);
  extension = { version: 99, selection: 'card:default' };
  await assert.rejects(plugin.sidebar(request), /Unsupported chat marker version/);
});
