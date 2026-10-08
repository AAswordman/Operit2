import assert from 'node:assert/strict';
import test from 'node:test';
import { openBrowserFile, browserFileMimeType } from '../../apps/flutter/app/web/runtime/src/browser_file_open.ts';

function globals(context, values) {
  const original = new Map(Object.keys(values).map(key => [key, Object.getOwnPropertyDescriptor(globalThis, key)]));
  for (const [key, value] of Object.entries(values)) Object.defineProperty(globalThis, key, { value, writable: true, configurable: true });
  context.after(() => { for (const [key, descriptor] of original) {
    if (descriptor) Object.defineProperty(globalThis, key, descriptor); else delete globalThis[key];
  } });
}

function fixture(context) {
  const events = new Map();
  const recorded = { blob: null, revoked: [], closed: 0, timers: new Map(), nodes: [], modals: [] };
  function element(tag) {
    const node = { tag, style: {}, attributes: {}, children: [], setAttribute(key, value) { this.attributes[key] = value; },
      append(...children) { this.children.push(...children); }, replaceChildren(...children) { this.children = children; },
      remove() { this.removed = true; }, showModal() { recorded.modals.push(this); } };
    recorded.nodes.push(node);
    return node;
  }
  const doc = { createElement: element, body: element('body') };
  const popup = { document: doc, opener: {}, closed: false,
    close() { recorded.closed++; this.closed = true; }, addEventListener(name, callback) { events.set('popup.' + name, callback); } };
  const win = { open(url, target) { assert.equal(url, 'about:blank'); assert.equal(target, '_blank'); return popup; },
    addEventListener(name, callback) { events.set(name, callback); }, removeEventListener(name) { events.delete(name); } };
  let timerId = 0;
  globals(context, { window: win, document: doc, HTMLDialogElement: class {}, URL: {
    createObjectURL(blob) { recorded.blob = blob; return 'blob:operit-test'; },
    revokeObjectURL(url) { recorded.revoked.push(url); },
  }, setInterval(callback) { const id = ++timerId; recorded.timers.set(id, callback); return id; },
  clearInterval(id) { recorded.timers.delete(id); }, setTimeout(callback) { const id = ++timerId; recorded.timers.set(id, callback); return id; },
  clearTimeout(id) { recorded.timers.delete(id); } });
  return { win, doc, popup, events, recorded };
}

test('MIME types support documents, media, Unicode names, and unknown binary files', () => {
  assert.equal(browserFileMimeType('/work/报告.PDF'), 'application/pdf');
  assert.equal(browserFileMimeType('/work/index.html'), 'text/html');
  assert.equal(browserFileMimeType('/work/测试.JPG'), 'image/jpeg');
  assert.equal(browserFileMimeType('/work/movie.mp4'), 'video/mp4');
  assert.equal(browserFileMimeType('/work/report.docx'), 'application/vnd.openxmlformats-officedocument.wordprocessingml.document');
  assert.equal(browserFileMimeType('/work/no-extension'), 'application/octet-stream');
});

test('opens actual binary bytes, isolates active content, and preserves a named download', async context => {
  const { popup, recorded, events } = fixture(context);
  const bytes = new Uint8Array([0, 255, 12, 34]);
  await openBrowserFile('/workspace/报告 & 100%.PDF', bytes);
  assert.equal(popup.opener, null);
  assert.equal(popup.document.title, '报告 & 100%.PDF');
  assert.equal(recorded.blob.type, 'application/pdf');
  assert.deepEqual(new Uint8Array(await recorded.blob.arrayBuffer()), bytes);
  const [download, frame] = popup.document.body.children;
  assert.equal(download.href, 'blob:operit-test');
  assert.equal(download.download, '报告 & 100%.PDF');
  assert.equal(frame.src, 'blob:operit-test');
  assert.equal(frame.attributes.sandbox, 'allow-scripts allow-downloads');
  assert.equal(frame.referrerPolicy, 'no-referrer');
  assert.deepEqual(recorded.revoked, []);
  events.get('popup.pagehide')();
  assert.deepEqual(recorded.revoked, ['blob:operit-test']);
  assert.equal(recorded.timers.size, 0);
  assert.equal(events.has('pagehide'), false);
});

test('zero-byte files are real files and Blob resources survive until the window closes', async context => {
  const { popup, recorded } = fixture(context);
  await openBrowserFile('/empty.txt', new Uint8Array());
  assert.equal(recorded.blob.size, 0);
  popup.closed = true;
  [...recorded.timers.values()][0]();
  assert.deepEqual(recorded.revoked, ['blob:operit-test']);
});

test('popup blocking requests a user click, and opens synchronously in that activation', async context => {
  const { win, popup, recorded } = fixture(context);
  win.open = () => null;
  const pending = openBrowserFile('/workspace/index.html', new Uint8Array([1]));
  assert.equal(recorded.modals.length, 1);
  assert.equal(recorded.blob, null);
  win.open = () => popup;
  recorded.modals[0].children[1].onclick();
  await pending;
  assert.equal(recorded.modals[0].removed, true);
  assert.equal(recorded.blob.type, 'text/html');
});

test('cancel, timeout, and a second popup denial all propagate as failures', async context => {
  const { win, recorded } = fixture(context);
  win.open = () => null;
  let pending = openBrowserFile('/one.txt', new Uint8Array([1]));
  recorded.modals.at(-1).children[2].onclick();
  await assert.rejects(pending, /cancelled/);
  pending = openBrowserFile('/two.txt', new Uint8Array([1]));
  [...recorded.timers.values()][0]();
  await assert.rejects(pending, /timed out/);
  pending = openBrowserFile('/three.txt', new Uint8Array([1]));
  recorded.modals.at(-1).children[1].onclick();
  await assert.rejects(pending, /blocked/);
  assert.equal(recorded.blob, null);
  assert.equal(recorded.timers.size, 0);
});

test('presentation failure closes its window and revokes its URL', async context => {
  const { popup, recorded } = fixture(context);
  popup.document.createElement = () => { throw new Error('document unavailable'); };
  await assert.rejects(openBrowserFile('/file.pdf', new Uint8Array([1])), /document unavailable/);
  assert.deepEqual(recorded.revoked, ['blob:operit-test']);
  assert.equal(recorded.closed, 1);
});

test('invalid requests and missing browser capabilities are not successful no-ops', async context => {
  const { win } = fixture(context);
  await assert.rejects(openBrowserFile('', new Uint8Array()), /Invalid/);
  await assert.rejects(openBrowserFile('/a\0b', new Uint8Array()), /Invalid/);
  await assert.rejects(openBrowserFile('/a', []), /Invalid/);
  win.open = () => null;
  delete globalThis.HTMLDialogElement;
  await assert.rejects(openBrowserFile('/file', new Uint8Array()), /allow popups/);
  delete globalThis.window;
  await assert.rejects(openBrowserFile('/file', new Uint8Array()), /requires a browser window/);
});
