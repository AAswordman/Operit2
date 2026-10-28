import assert from 'node:assert/strict';
import path from 'node:path';
import {readFile, writeFile} from 'node:fs/promises';
import ts from 'typescript';

// A deterministic processor verifies the real authenticated TCP Watch/Push and
// binary codec. It does not substitute for microphone, speaker or STT quality tests.
export async function probeSpeech({core, node, directory, post, waitDevice, t}) {
  const fixture = path.join(directory, 'edge_speech_fixture.js');
  const metadata = {name:'edge_speech_fixture', apiVersion:'2.0.0', description:'Isolated device speech transport test',
    enabledByDefault:true, tools:[{name:'probe',description:'Verify bounded speech packets',
      parameters:[{name:'node_id',description:'Paired Edge node',type:'string',required:true},
        {name:'mode',description:'Disconnect or roundtrip',type:'string',required:false}]}]};
  const source = await readFile(new URL('../../../../../../plugins/packages/examples/device_speech_processor/src/processor.ts',import.meta.url),'utf8');
  const processor = ts.transpileModule(source.replaceAll('export ',''), {compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
  await writeFile(fixture, `/* METADATA\n${JSON.stringify(metadata)}\n*/
    ${processor}
    exports.probe = async function(params) {
      const channel = await Tools.device(params.node_id, {timeoutMs:120000}).transport.open({
        target:'operit.device.speech', method:'results', property:'requests', args:{session:'network-speech-probe'}
      });
      if (params.mode === 'long') {
        let complete = false, length = 0, checksum = 0;
        const adapter = {
          async *[Symbol.asyncIterator]() { for await (const event of channel) { yield event; if (complete) break; } },
          async send(packet) { await channel.send(packet); if (packet.type === 'tts.end') complete = true; },
          close: () => channel.close(),
        };
        await processSpeech(adapter, {
          recognize: async audio => {
            const view = new DataView(audio.buffer, audio.byteOffset, audio.length);
            length = view.getUint32(40,true);
            if (length !== 32000*65 || audio.length !== length+44) throw new Error('Long WAV length mismatch');
            for(let i=44;i<audio.length;i++) { if(audio[i] !== ((i-44)%251)) throw new Error('Long PCM altered'); checksum=(checksum+audio[i])>>>0; }
            return {text:'长语音识别🙂'.repeat(550)+'完整结尾'};
          },
          synthesize: async text => ({segments:[{contentType:'audio/wav',audio:Array.from({length:12288},(_,i)=>i%251)}]}),
        });
        return {length,checksum};
      }
      let recording, completed = 0;
      const received = [];
      try {
        for await (const event of channel) {
          const p = event.value;
          if (p.type === 'ready') continue;
          if (p.type === 'stt.begin') {
            if (params.mode === 'disconnect') return {disconnected:true};
            recording = {id:p.id, length:0, checksum:0};
          }
          else if (p.type === 'stt.data') {
            if (recording?.id !== p.id || !(p.audio instanceof Uint8Array)) throw new Error('PCM framing or binary type lost');
            recording.length += p.audio.length;
            for (const byte of p.audio) recording.checksum = (recording.checksum + byte) >>> 0;
          } else if (p.type === 'stt.end') {
            if (recording?.id !== p.id) throw new Error('STT request changed');
            received.push(recording); recording = undefined;
            await channel.send({type:'stt.result', id:p.id, text:'speech-round-' + received.length});
          } else if (p.type === 'tts') {
            await channel.send({type:'tts.segment', id:p.id, index:0, contentType:'audio/wav'});
            // More than the device's four queue slots: exercise real backpressure.
            for (let block=0; block<6; block++) {
              const audio = new Uint8Array(4096);
              for (let i=0;i<audio.length;i++) audio[i]=(i+block+completed)&255;
              await channel.send({type:'tts.data',id:p.id,audio});
            }
            await channel.send({type:'tts.end',id:p.id});
            if (++completed === 3) break;
          } else throw new Error('Unexpected speech request: ' + p.type);
        }
        return {received, completed};
      } finally { await channel.close(); }
    };
  `);
  await core.command(['core','package','import',fixture]);
  await core.command(['core','package','enable','edge_speech_fixture']);
  let pending;
  try {
    pending = core.command(['core','package','exec','edge_speech_fixture:probe',JSON.stringify({node_id:node,mode:'disconnect'})]);
    pending.catch(() => {});
    await waitDevice('first speech processor ready',d=>d.speech.ready);
    await post('/api/simulator/speech',{operation:'begin'});
    assert.equal((await pending).success,true);
    await waitDevice('in-flight speech disconnect reported',d=>!d.speech.ready&&d.speech.phase===0&&Boolean(d.speech.error));
    pending = core.command(['core','package','exec','edge_speech_fixture:probe',JSON.stringify({node_id:node})]);
    // Observe a rejection immediately even if setup fails before awaiting the result.
    pending.catch(() => {});
    await waitDevice('network speech processor ready', d => d.speech.ready);
    const expected = [];
    for (let round=0;round<3;round++) {
      const begin = await post('/api/simulator/speech', {operation:'begin'});
      let length=0, checksum=0;
      for (const size of [4096, 2048]) {
        const pcm=Buffer.from(Array.from({length:size},(_,i)=>(i+round*37)&255));
        length+=pcm.length; for(const byte of pcm) checksum=(checksum+byte)>>>0;
        await post('/api/simulator/speech',{operation:'pcm',requestId:begin.requestId,audio:pcm.toString('base64')});
      }
      expected.push({id:begin.requestId,length,checksum});
      await post('/api/simulator/speech',{operation:'finish',requestId:begin.requestId});
      await waitDevice('network STT result', d=>d.speech.phase===0 && d.speech.text==='speech-round-'+(round+1));
      // A stale HTTP capture packet must fail without replacing the finished task.
      await assert.rejects(post('/api/simulator/speech',{operation:'pcm',requestId:begin.requestId,audio:'AAA='}),/expired|changed/);
      const request = await post('/api/simulator/speech',{operation:'tts',text:'transport round '+round});
      const packets=[];
      await waitDevice('network TTS completion',d=>{
        packets.push(...d.playback);
        return packets.some(p=>p.type==='tts.end'&&p.id===request.requestId);
      });
      assert(packets.every(p=>p.id===request.requestId),'audio crossed request boundary');
      assert.equal(packets[0].type,'tts.segment');
      assert.equal(packets.at(-1).type,'tts.end');
      const audio=Buffer.concat(packets.filter(p=>p.type==='tts.data').map(p=>Buffer.from(p.audio,'base64')));
      const expectedAudio=Buffer.from(Array.from({length:6*4096},(_,i)=>(i%4096+Math.floor(i/4096)+round)&255));
      assert.deepEqual(audio,expectedAudio,'encrypted speech stream altered or lost bytes');
    }
    const reply=await pending;
    assert.equal(reply.success,true,JSON.stringify(reply));
    const value=reply.result?.value??reply.result;
    const data=typeof value==='string'?JSON.parse(value):value;
    assert.deepEqual(data.received,expected);
    assert.equal(data.completed,3);
    await waitDevice('processor closed cleanly',d=>!d.speech.ready);
    pending = core.command(['core','package','exec','edge_speech_fixture:probe',JSON.stringify({node_id:node,mode:'long'})]);
    pending.catch(()=>{});
    await waitDevice('actual speech processor ready for a long capture',d=>d.speech.ready);
    const begin = await post('/api/simulator/speech',{operation:'begin'});
    let checksum = 0;
    const length=32000*65;
    for(let offset=0;offset<length;offset+=4096) {
      const pcm=Buffer.from(Array.from({length:Math.min(4096,length-offset)},(_,i)=>(offset+i)%251));
      for(const byte of pcm) checksum=(checksum+byte)>>>0;
      await post('/api/simulator/speech',{operation:'pcm',requestId:begin.requestId,audio:pcm.toString('base64')});
    }
    await post('/api/simulator/speech',{operation:'finish',requestId:begin.requestId});
    const text='长语音识别🙂'.repeat(550)+'完整结尾';
    await waitDevice('complete long recognition text survives segmented encrypted transport',d=>d.speech.phase===0&&d.speech.text===text);
    const tts=await post('/api/simulator/speech',{operation:'tts',text:'长篇朗读🙂'.repeat(180)});
    const playback=[];
    await waitDevice('segmented long TTS preserves every packet',d=>{
      playback.push(...d.playback);return playback.some(p=>p.type==='tts.end'&&p.id===tts.requestId);
    });
    const segments=playback.filter(p=>p.type==='tts.segment');
    assert(segments.length>1,'actual processor must split provider text requests');
    const audio=Buffer.concat(playback.filter(p=>p.type==='tts.data').map(p=>Buffer.from(p.audio,'base64')));
    assert.deepEqual(audio,Buffer.from(Array.from({length:segments.length*12288},(_,i)=>(i%12288)%251)));
    const longReply=await pending;
    assert.equal(longReply.success,true,JSON.stringify(longReply));
    const longValue=longReply.result?.value??longReply.result;
    assert.deepEqual(typeof longValue==='string'?JSON.parse(longValue):longValue,{length,checksum});
    await waitDevice('actual processor closes after long capture and synthesis',d=>!d.speech.ready);
    t.diagnostic('Actual processor logic over authenticated TCP: 65 seconds PCM byte volume, WAV payload verified, >8 KiB UTF-8 recognition result and multi-segment TTS passed; STT/TTS providers remain deterministic');
    t.diagnostic('Real Core JS processor <-> authenticated TCP Edge: disconnect/reconnect, three STT/TTS rounds, 4096-byte binary blocks, playback backpressure and exact audio bytes passed (deterministic processor)');
  } finally {
    // The Core tool owns this channel and closes it in its own finally clause.
    if (pending) await pending.catch(()=>{});
    await core.command(['core','package','delete','edge_speech_fixture']);
  }
}
