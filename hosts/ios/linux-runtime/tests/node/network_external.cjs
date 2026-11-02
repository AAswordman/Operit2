'use strict';
const assert = require('node:assert/strict');
const https = require('node:https');
const hostname = process.argv[2] || 'example.com';
const request = https.get({ hostname, path: '/', method: 'HEAD', timeout: 10000 }, response => {
  assert.equal(response.socket.authorized, true);
  assert.ok(response.statusCode >= 200 && response.statusCode < 400);
  console.log(JSON.stringify({ test: 'network-external', ok: true, hostname, tls: response.socket.getProtocol(), status: response.statusCode }));
  response.resume();
});
request.on('timeout', () => request.destroy(new Error('HTTPS timeout')));
request.on('error', error => { console.error(error); process.exitCode = 1; });
