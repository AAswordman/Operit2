import test from 'node:test';
import assert from 'node:assert/strict';
import {processSpeech} from '../../../../../plugins/packages/examples/device_speech_processor/src/processor.ts';

const packet = value => ({kind:'Changed',value});
function fixture(events, delay = false) {
  const sent = []; let closed = false;
  const channel = {
    async *[Symbol.asyncIterator]() { yield packet({type:'ready',protocol:2}); yield* events; },
    async send(value) { if (delay) await new Promise(resolve => setImmediate(resolve)); sent.push(value); },
    async close() { closed = true; },
  };
  return {channel, sent, get closed() { return closed; }};
}
function* recording(seconds, id = 'capture') {
  yield packet({type:'stt.begin',id,sampleRate:16000,channels:1,bitsPerSample:16});
  const length = 32000 * seconds;
  for (let offset=0;offset<length;offset+=4096)
    yield packet({type:'stt.data',id,audio:Uint8Array.from({length:Math.min(4096,length-offset)},(_,i)=>(offset+i)%251)});
  yield packet({type:'stt.end',id});
}
test('actual Core processor accepts five minutes and returns complete long UTF-8 text in bounded chunks',async()=>{
  const text='完整识别🙂'.repeat(600);
  const f=fixture(recording(300),true);
  let calls=0;
  await processSpeech(f.channel,{
    async recognize(audio) {
      calls++;
      const view=new DataView(audio.buffer,audio.byteOffset,audio.byteLength);
      assert.equal(audio.length,44+32000*300);
      assert.equal(view.getUint32(40,true),32000*300);
      assert.equal(view.getUint32(24,true),16000);
      for(let i=44;i<audio.length;i++) assert.equal(audio[i],(i-44)%251);
      return {text};
    },
    async synthesize(){throw Error('unexpected TTS');},
  });
  assert.equal(calls,1);
  assert.equal(f.sent.filter(p=>p.type==='stt.text').map(p=>p.text).join(''),text);
  assert(f.sent.filter(p=>p.type==='stt.text').every(p=>Buffer.byteLength(p.text)<=2048));
  assert.deepEqual(f.sent.at(-1),{type:'stt.result',id:'capture',text:''});
  assert(f.closed);
});
test('overlong recording fails and the next recording can succeed without recognizing the prefix',async()=>{
  function* events(){yield* recording(301);yield* recording(1,'next');}
  const f=fixture(events());let calls=0;
  await processSpeech(f.channel,{async recognize(){calls++;return {text:'ok'};},async synthesize(){throw Error('unexpected');}});
  assert.equal(calls,1);
  assert(f.sent.some(p=>p.id==='capture'&&p.type==='error'&&p.error.includes('300 seconds')));
  assert(!f.sent.some(p=>p.id==='capture'&&p.type==='stt.result'));
  assert.equal(f.sent.at(-1).id,'next');
});
test('long TTS preserves all text, ordered segments and exact bytes under a slow sender',async()=>{
  const text='朗读文本🙂'.repeat(220), inputs=[];
  const f=fixture([packet({type:'tts',id:'tts',text})],true);
  await processSpeech(f.channel,{async recognize(){throw Error('unexpected');},async synthesize(input){
    assert(Buffer.byteLength(input)<=1024); inputs.push(input);
    return {segments:[{contentType:'audio/wav',audio:Array.from({length:9500},(_,i)=>i%251)}]};
  }});
  assert.equal(inputs.join(''),text);
  const segments=f.sent.filter(p=>p.type==='tts.segment');
  assert.deepEqual(segments.map(p=>p.index),inputs.map((_,i)=>i));
  const audio=Buffer.concat(f.sent.filter(p=>p.type==='tts.data').map(p=>Buffer.from(p.audio)));
  assert.deepEqual(audio,Buffer.from(Array.from({length:inputs.length*9500},(_,i)=>(i%9500)%251)));
  assert(f.sent.filter(p=>p.type==='tts.data').every(p=>p.audio.length<=4096));
  assert.equal(f.sent.at(-1).type,'tts.end');
});
test('oversized recognition cannot publish a successful partial result',async()=>{
  const f=fixture(recording(1));
  await processSpeech(f.channel,{async recognize(){return {text:'字'.repeat(6000)};},async synthesize(){throw Error('unexpected');}});
  assert.deepEqual(f.sent.map(p=>p.type),['error']);
});
