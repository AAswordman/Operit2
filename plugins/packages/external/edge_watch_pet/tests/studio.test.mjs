import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';

/** Runs the packaged document with observable image loads and deterministic timers. */
function studio(bridge) {
  const elements = new Map(), buttons = [], timers = [];
  const listeners = {};
  /** Represents the DOM properties used by the offline studio. */
  function element(id) {
    const events = {}, attributes = {};
    return { id, events, attributes, dataset: {}, children: [], style: { setProperty() {} },
      append(...children) { this.children.push(...children); },
      replaceChildren(...children) { this.children = [...children]; },
      addEventListener(name, handler) { events[name] = handler; },
      setAttribute(name, value) { attributes[name] = value; },
      removeAttribute(name) { delete this[name]; }, focus() {} };
  }
  for (const mood of ['working', 'thinking', 'approval', 'completed', 'idle', 'error']) {
    const button = element(mood); button.dataset.mood = mood; buttons.push(button);
  }
  const document = {
    createElement(tag) { return element(tag); },
    getElementById(id) { if (!elements.has(id)) elements.set(id, element(id)); return elements.get(id); },
    querySelectorAll(selector) { return selector === '[data-mood]' ? buttons : []; },
  };
  const html = readFileSync(new URL('../resources/studio.html', import.meta.url), 'utf8');
  runInNewContext(html.match(/<script>([\s\S]*?)<\/script>/)[1], {
    document, window: { addEventListener(name, work) { listeners[name] = work; }, WatchStudio: bridge },
    setTimeout(work, delay) { const timer = { work, delay, cancelled: false }; timers.push(timer); return timer; },
    clearTimeout(timer) { if (timer) timer.cancelled = true; },
  });
  return { elements, buttons, timers, pet: elements.get('pet'), listeners };
}

test('all six previews load their original animation and completion returns to idle', () => {
  const f = studio(); assert.equal(f.pet.src, 'idle.webp');
  for (const button of f.buttons) {
    button.events.click();
    assert.equal(f.pet.src, `${button.dataset.mood}.webp`);
    assert.equal(button.attributes['aria-pressed'], 'true');
  }
  f.buttons.find(button => button.dataset.mood === 'completed').events.click();
  const timer = f.timers.at(-1); assert.equal(timer.delay, 3000);
  timer.work(); assert.equal(f.pet.src, 'idle.webp');
});

/** Flushes bridge continuations without depending on wall-clock timers. */
async function settle() { for (let step = 0; step < 20; step++) await Promise.resolve(); }

const config = { version: 1, accent: 'mint', haptics: true, keepAwake: false };
/** Supplies one identified device with a device-confirmed settings snapshot. */
function watch(nodeId, label = '圆屏手表') { return { target: { nodeId, label }, model: 'Round watch', config }; }

test('automatically connects the single verified watch without any manual input', async () => {
  const only = watch('one'), requests = [];
  const f = studio({ async request(payload) { requests.push(payload); return { ok: true, data: { watches: [only], issues: [], connected: only } }; } });
  await settle();
  assert.equal(requests[0].action, 'discover'); assert.equal(requests.length, 1);
  assert.equal(f.elements.get('watch-name').textContent, '圆屏手表');
  assert.equal(f.elements.get('discovery').hidden, true); assert.equal(f.elements.get('settings').hidden, false);
});

test('lists multiple watches and sends only the clicked identity', async () => {
  const watches = [watch('one', '<客厅手表>'), watch('two', '卧室手表')], requests = [];
  const f = studio({ async request(payload) {
    requests.push(payload);
    return { ok: true, data: payload.action === 'discover' ? { watches, issues: [], connected: null } : watches[1] };
  } });
  await settle();
  const choices = f.elements.get('watch-list').children;
  assert.equal(choices.length, 2); assert.equal(choices[0].children[0].textContent, '<客厅手表>');
  assert.equal(f.elements.get('settings').hidden, true);
  choices[1].events.click(); await settle();
  assert.equal(requests[1].action, 'connect'); assert.equal(requests[1].nodeId, 'two');
  assert.equal(f.elements.get('watch-name').textContent, '卧室手表');
});

test('no eligible watch explains the empty state and refresh discovers a newly connected watch', async () => {
  const only = watch('one'); let count = 0;
  const f = studio({ async request() { count++; return { ok: true, data: count === 1
    ? { watches: [], issues: [{ label: '旧手表', message: '暂未响应' }], connected: null }
    : { watches: [only], issues: [], connected: only } }; } });
  await settle();
  assert.match(f.elements.get('discovery-help').textContent, /没有发现可用手表/);
  assert.equal(f.elements.get('scan-issues').hidden, false);
  f.elements.get('refresh').events.click(); await settle();
  assert.equal(f.elements.get('watch-name').textContent, '圆屏手表'); assert.equal(count, 2);
});

test('switch opens the verified choices even when the saved watch is still online', async () => {
  const watches = [watch('one'), watch('two')];
  const f = studio({ async request() { return { ok: true, data: { watches, issues: [], connected: watches[0] } }; } });
  await settle(); assert.equal(f.elements.get('discovery').hidden, true);
  f.elements.get('switch').events.click(); await settle();
  assert.equal(f.elements.get('discovery').hidden, false); assert.equal(f.elements.get('watch-list').children.length, 2);
});

test('repeated taps cancel the old reaction and resume the selected work', () => {
  const f = studio();
  f.buttons[0].events.click();
  f.elements.get('face').events.click(); assert.equal(f.pet.src, 'approval.webp');
  const first = f.timers.at(-1);
  f.elements.get('face').events.click(); assert.equal(first.cancelled, true);
  const second = f.timers.at(-1); assert.equal(second.delay, 1200);
  second.work(); assert.equal(f.pet.src, 'working.webp');
});

test('changing preview during a reaction prevents an old timer from overriding it', () => {
  const f = studio(); f.elements.get('face').events.click();
  const reaction = f.timers.at(-1);
  f.buttons.find(button => button.dataset.mood === 'error').events.click();
  assert.equal(reaction.cancelled, true); assert.equal(f.pet.src, 'error.webp');
});
