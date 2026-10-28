import test from 'node:test';
import assert from 'node:assert/strict';
import {SpeechPcm} from '../web/device-pcm.ts';
for (const rate of [16000, 44100, 48000]) {
  test(`browser PCM stays continuous across worklet boundaries at ${rate} Hz`, () => {
    const input = Float32Array.from({length: rate}, (_, i) => Math.sin(i * 440 * 2 * Math.PI / rate) * 0.5);
    const once = new SpeechPcm(rate), split = new SpeechPcm(rate);
    const expected = [...once.push(input), once.flush()];
    const actual = [];
    for (let i=0; i<input.length; i+=128) actual.push(...split.push(input.slice(i, i+128)));
    actual.push(split.flush());
    assert.deepEqual(Buffer.concat(actual), Buffer.concat(expected));
    assert(Math.abs(Buffer.concat(actual).length - 32000) <= 2);
    assert(actual.slice(0, -1).every(block => block.length === 640));
  });
}
test('PCM clamps signed samples and retains little endian byte order', () => {
  const pcm = new SpeechPcm(16000);
  pcm.push(new Float32Array([-2, 0, 2, 0]));
  assert.deepEqual([...pcm.flush()], [0, 128, 0, 0, 255, 127]);
});
