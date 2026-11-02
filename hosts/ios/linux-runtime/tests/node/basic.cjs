'use strict';
const originalTitle = process.title;
process.title = 'operit-arm64-basic';
if (process.title !== 'operit-arm64-basic') throw new Error('process title was not updated');
process.title = originalTitle;
const assert = require('node:assert/strict');
assert.equal(process.arch, 'arm64');
assert.equal(process.platform, 'linux');
console.log(JSON.stringify({ test: 'basic', ok: true, version: process.version, arch: process.arch }));
