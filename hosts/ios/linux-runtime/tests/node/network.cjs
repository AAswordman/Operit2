'use strict';
const assert = require('node:assert/strict');
const net = require('node:net');
const dgram = require('node:dgram');
const { once } = require('node:events');

(async () => {
  const timer = setTimeout(() => { console.error('network probe timed out'); process.exit(1); }, 10000);
  const server = net.createServer(socket => {
    socket.setNoDelay(true);
    socket.once('data', data => {
      assert.equal(data.toString(), 'tcp-request');
      socket.end('tcp-response');
    });
  });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  const client = net.createConnection({ host: '127.0.0.1', port: server.address().port });
  await once(client, 'connect');
  client.write('tcp-request');
  let reply = '';
  client.on('data', data => { reply += data; });
  await once(client, 'end');
  assert.equal(reply, 'tcp-response');
  client.destroy();
  await new Promise(resolve => server.close(resolve));

  const receiver = dgram.createSocket('udp4');
  const sender = dgram.createSocket('udp4');
  receiver.bind(0, '127.0.0.1');
  await once(receiver, 'listening');
  const message = once(receiver, 'message');
  await new Promise((resolve, reject) => sender.send(Buffer.from('udp-request'), receiver.address().port, '127.0.0.1', error => error ? reject(error) : resolve()));
  assert.equal((await message)[0].toString(), 'udp-request');
  sender.close(); receiver.close();
  clearTimeout(timer);
  console.log(JSON.stringify({ test: 'network', ok: true, tcp: true, udp: true }));
})().catch(error => { console.error(error); process.exitCode = 1; });
