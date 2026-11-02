'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

(async () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'operit-watch-'));
  const controller = new AbortController();
  let watcher;
  let timer;
  let events = 0;
  try {
    await new Promise((resolve, reject) => {
      watcher = fs.watch(directory, { signal: controller.signal }, (event, filename) => {
        try {
          if (String(filename) !== 'created.txt') return;
          assert.ok(event === 'rename' || event === 'change');
          ++events;
          resolve();
        } catch (error) {
          reject(error);
        }
      });
      watcher.once('error', reject);
      timer = setTimeout(() => reject(new Error('fs.watch did not report file creation')), 5000);
      setImmediate(() => fs.writeFileSync(path.join(directory, 'created.txt'), 'watch-data'));
    });
    const closed = new Promise(resolve => watcher.once('close', resolve));
    controller.abort();
    await closed;
    assert.ok(events > 0);
    console.log(JSON.stringify({ test: 'watch', ok: true, events, closed: true }));
  } finally {
    clearTimeout(timer);
    controller.abort();
    fs.rmSync(directory, { recursive: true, force: true });
  }
})().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
