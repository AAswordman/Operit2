'use strict';
const assert = require('node:assert/strict');
const { pbkdf2, pbkdf2Sync } = require('node:crypto');
const jobs = 8;
const timeout = setTimeout(() => {
  console.error('libuv threadpool probe timed out');
  process.exit(1);
}, 15000);
const expected = pbkdf2Sync('operit', 'arm64-probe', 1000, 32, 'sha256');
Promise.all(Array.from({ length: jobs }, () => new Promise((resolve, reject) => {
  pbkdf2('operit', 'arm64-probe', 1000, 32, 'sha256', (error, result) => {
    if (error) reject(error);
    else resolve(result);
  });
}))).then(results => {
  clearTimeout(timeout);
  for (const result of results) assert.deepEqual(result, expected);
  console.log(JSON.stringify({ test: 'threadpool', ok: true, jobs, completed: results.length }));
}).catch(error => {
  clearTimeout(timeout);
  console.error(error);
  process.exitCode = 1;
});
