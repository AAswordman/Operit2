'use strict';
const assert = require('node:assert/strict');
const { Worker, isMainThread, parentPort, workerData, threadId } = require('node:worker_threads');
const workers = 4;
const iterations = 10000;

if (isMainThread) {
  const shared = new SharedArrayBuffer(8);
  const state = new Int32Array(shared);
  const pool = [];
  const timeout = setTimeout(() => {
    console.error('worker probe timed out');
    process.exitCode = 1;
    for (const worker of pool) worker.terminate();
  }, 15000);
  const jobs = Array.from({ length: workers }, (_, index) => new Promise((resolve, reject) => {
    const worker = new Worker(__filename, { workerData: { shared, index, iterations } });
    pool.push(worker);
    let result;
    worker.on('message', message => {
      if (message.ready) {
        if (Atomics.add(state, 0, 1) + 1 === workers) {
          Atomics.store(state, 0, workers + 1);
          Atomics.notify(state, 0);
        }
      } else {
        result = message;
      }
    });
    worker.on('error', reject);
    worker.on('exit', code => {
      if (code !== 0 || !result) reject(new Error(`worker ${index} exited ${code} without result`));
      else resolve(result);
    });
  }));
  Promise.all(jobs).then(results => {
    clearTimeout(timeout);
    assert.equal(new Set(results.map(result => result.threadId)).size, workers);
    for (const result of results) {
      assert.equal(result.sum, (iterations - 1) * iterations / 2);
    }
    assert.equal(Atomics.load(state, 1), workers * iterations);
    console.log(JSON.stringify({ test: 'workers', ok: true, workers, increments: Atomics.load(state, 1) }));
  }).catch(error => {
    clearTimeout(timeout);
    console.error(error);
    process.exitCode = 1;
    for (const worker of pool) worker.terminate();
  });
} else {
  const state = new Int32Array(workerData.shared);
  parentPort.postMessage({ ready: true });
  while (Atomics.load(state, 0) !== workers + 1) {
    const observed = Atomics.load(state, 0);
    if (observed !== workers + 1) Atomics.wait(state, 0, observed, 1000);
  }
  let sum = 0;
  for (let i = 0; i < workerData.iterations; ++i) {
    sum += i;
    Atomics.add(state, 1, 1);
  }
  parentPort.postMessage({ threadId, sum });
}
