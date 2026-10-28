import test from 'node:test';
import assert from 'node:assert/strict';
import net from 'node:net';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {fileURLToPath} from 'node:url';

test('editor serves every browser dependency including microphone and full draft adapters',{timeout:20000},async t=>{
  const probe=net.createServer();probe.listen(0,'127.0.0.1');await once(probe,'listening');
  const port=probe.address().port;await new Promise(resolve=>probe.close(resolve));
  const child=spawn(process.execPath,['--experimental-strip-types','src/server.mts','--port',String(port)],{
    cwd:fileURLToPath(new URL('../',import.meta.url)),windowsHide:true,stdio:['ignore','pipe','pipe'],
  });
  const exited=once(child,'exit');
  t.after(async()=>{child.kill();await exited;});
  let output='';child.stdout.on('data',chunk=>output+=chunk);child.stderr.on('data',chunk=>output+=chunk);
  await Promise.race([new Promise(resolve=>child.stdout.on('data',()=>{if(output.includes('editor running'))resolve();})),
    exited.then(()=>{throw Error(output);})]);
  const base=`http://127.0.0.1:${port}`;
  const seen=new Set(),queue=['/app.js','/simulator.js'];
  while(queue.length){
    const route=queue.shift();if(seen.has(route))continue;seen.add(route);
    const response=await fetch(base+route);
    assert.equal(response.status,200,route);
    assert.match(response.headers.get('content-type'),/javascript/,route);
    const source=await response.text();
    for(const match of source.matchAll(/\bfrom\s+['"]([^'"]+)['"]/g)){
      if(match[1].startsWith('.'))queue.push(new URL(match[1],base+route).pathname);
    }
  }
  assert(seen.has('/device-speech.js'));assert(seen.has('/device-pcm.js'));assert(seen.has('/chat-draft.ts'));
});
