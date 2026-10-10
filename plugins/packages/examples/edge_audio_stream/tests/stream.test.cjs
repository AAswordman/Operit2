const { test, afterEach } = require("node:test");
const assert = require("node:assert/strict");
const path = require("node:path");
const { decodePcm, transcribeEdge } = require(process.env.OPERIT_AUDIO_EXAMPLE_MODULE || path.join(__dirname, "../build/stream.js"));
afterEach(() => { delete global.Tools; });
const format = { encoding: "pcm_s16le", sampleRateHz: 16000, channels: 1 };
function setup(blocks) {
  const events = [];
  global.Tools = { Edge: {
    async openAudioInput(node, options) { events.push(["open", node, options]); return { streamId: "stream", format }; },
    async readAudioInput(id) { assert.equal(id, "stream"); return blocks.shift(); },
    async closeAudioInput(id) { events.push(["close", id]); return { value: true }; },
  } };
  return events;
}
function block(bytes, sequence = 0, sampleOffset = 0) {
  return { dataBase64: Buffer.from(bytes).toString("base64"), byteLength: bytes.length, sequence, sampleOffset, pending: false, done: false };
}
function adapter(events) {
  return {
    async start(value) { events.push(["start", value]); },
    async write(value) { assert.ok(value instanceof Uint8Array); events.push(["pcm", Array.from(value)]); },
    async finish() { events.push(["finish"]); return "recognized"; },
    async abort(error) { events.push(["abort", error.message]); },
  };
}
test("bounded PCM decoding preserves signed samples and full blocks", () => {
  const bytes = Uint8Array.from({ length: 4096 }, (_, i) => (i * 17) & 255);
  assert.deepEqual(decodePcm(Buffer.from(bytes).toString("base64"), bytes.length), bytes);
  assert.throws(() => decodePcm("AA==", 2), /length mismatch/);
  assert.throws(() => decodePcm("??", 1), /Invalid PCM/);
});
test("stream forwards ordered PCM, skips idle polls, finishes and closes", async () => {
  const events = setup([{ pending: true }, block([0,128,255,127]), block([7,0],1,2), { done: true, error: null, sequence: 2, sampleOffset: 3 }]);
  const value = await transcribeEdge("edge", { inputId: "mic0", maxDurationMs: 1000 }, adapter(events));
  assert.equal(value, "recognized");
  assert.deepEqual(events.filter(e => e[0] === "pcm"), [["pcm", [0,128,255,127]], ["pcm", [7,0]]]);
  assert.deepEqual(events.slice(-2), [["finish"], ["close", "stream"]]);
});
test("capture error aborts the STT session and closes Edge input", async () => {
  const events = setup([block([3,0]), { done: true, error: "mic overrun", sequence: 1, sampleOffset: 1 }]);
  await assert.rejects(transcribeEdge("edge", { inputId: "mic0" }, adapter(events)), /mic overrun/);
  assert.deepEqual(events.slice(-2), [["abort", "mic overrun"], ["close", "stream"]]);
  assert.ok(!events.some(e => e[0] === "finish"));
});
test("provider failure preserves the original error and always closes", async () => {
  const events = setup([block([3,0])]);
  const stt = adapter(events);
  stt.write = async () => { throw new Error("provider disconnected"); };
  stt.abort = async () => { throw new Error("abort failed"); };
  await assert.rejects(transcribeEdge("edge", { inputId: "mic0" }, stt), /provider disconnected/);
  assert.deepEqual(events.at(-1), ["close", "stream"]);
});
