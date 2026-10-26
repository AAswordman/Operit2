const test = require('node:test');
const assert = require('node:assert/strict');
const {processSpeech} = require('../dist/processor');
function fixture(packets, host) {
  const sent=[];let closed=false;
  const channel={async *[Symbol.asyncIterator](){for(const value of packets)yield{kind:'Changed',value};},
    async send(value){sent.push(value);},async close(){closed=true;}};
  return {sent,run:()=>processSpeech(channel,host),closed:()=>closed};
}
test('STT frames native PCM as WAV and returns actual recognition result with request identity',async()=>{
  let audio;
  const f=fixture([{id:'stt',type:'stt.begin',sampleRate:16000,channels:1,bitsPerSample:16},
    {id:'stt',type:'stt.data',audio:new Uint8Array([0,128,255,127])},{id:'stt',type:'stt.end'}],
    {async recognize(bytes){audio=bytes;return{text:'识别出的文字'};},synthesize(){throw Error('unexpected TTS');}});
  await f.run();assert.equal(Buffer.from(audio).subarray(0,4).toString(),'RIFF');
  assert.equal(new DataView(audio.buffer).getUint32(24,true),16000);
  assert.deepEqual([...audio.slice(44)],[0,128,255,127]);
  assert.deepEqual(f.sent,[{type:'stt.result',id:'stt',text:'识别出的文字'}]);assert(f.closed());
});
test('TTS returns all segments and exact bytes in bounded binary blocks',async()=>{
  const bytes=Array.from({length:10000},(_,i)=>i%256);let text;
  const f=fixture([{id:'tts',type:'tts',text:'朗读设备回复'}],{recognize(){throw Error('unexpected STT');},
    async synthesize(value){text=value;return{segments:[{audio:bytes,contentType:'audio/wav'},{audio:[0,128,255],contentType:'audio/mpeg'}]};}});
  await f.run();assert.equal(text,'朗读设备回复');
  const data=f.sent.filter(p=>p.type==='tts.data');assert(data.every(p=>p.audio instanceof Uint8Array&&p.audio.length<=4096));
  assert.deepEqual([...Buffer.concat(data.map(p=>p.audio))],[...bytes,0,128,255]);
  assert.deepEqual(f.sent.filter(p=>p.type==='tts.segment').map(p=>p.contentType),['audio/wav','audio/mpeg']);
  assert.deepEqual(f.sent.at(-1),{type:'tts.end',id:'tts'});assert(f.closed());
});
test('both provider errors return to device; cancellation and malformed PCM do not call recognition',async()=>{
  let recognition=0;
  const f=fixture([{id:'one',type:'tts',text:'error'},
    {id:'two',type:'stt.begin',sampleRate:16000,channels:1,bitsPerSample:16},
    {id:'two',type:'stt.data',audio:new Uint8Array([1,2])},{id:'two',type:'stt.end'},
    {id:'three',type:'stt.begin',sampleRate:16000,channels:1,bitsPerSample:16},{id:'three',type:'cancel'},
    {id:'four',type:'stt.begin',sampleRate:16000,channels:1,bitsPerSample:16},{id:'four',type:'stt.data',audio:new Uint8Array([1])}],
    {async synthesize(){throw Error('synthesis unavailable');},async recognize(){recognition++;throw Error('recognition unavailable');}});
  await f.run();assert.equal(recognition,1);assert.equal(f.sent.length,3);assert(f.sent.every(p=>p.type==='error'));
  assert.match(f.sent[0].error,/synthesis unavailable/);assert.match(f.sent[1].error,/recognition unavailable/);assert.match(f.sent[2].error,/PCM/);
});
