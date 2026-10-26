import assert from "node:assert/strict";
import test from "node:test";
import { provider, plain } from "./harness.mjs";

/** Opens an authenticated session against the actual provider implementation. */
async function open(p, callerPackage = "consumer", chatId = "chat") { return p.call("session.open", { chatId, profile: "default" }, callerPackage); }

/** Feeds one authoritative assistant snapshot using the declared wire schema. */
function feed(p, session, snapshot, final = false, caller = "consumer", messageTimestamp = 10) { return p.call("session.feed", { sessionId: session.sessionId, messageTimestamp, snapshot, final }, caller); }

test("named settings persist and unknown profiles never select another profile", async () => {
  const p = provider(), initial = await p.call("settings.get", { profile: "default" });
  await assert.rejects(p.call("settings.get", { profile: "missing" }), /does not exist/);
  await p.call("settings.set", { profile: "character-alice", settings: { ...initial, charDelayMs: 17 } });
  assert.equal((await p.call("settings.get", { profile: "character-alice" })).charDelayMs, 17);
  assert.equal(p.flushes(), 1);
  await assert.rejects(p.call("settings.set", { profile: "bad", settings: { ...initial, charDelayMs: -1 } }), /charDelayMs/);
});

test("session ownership is enforced for feeding and closing", async () => {
  const p = provider(), session = await open(p);
  await assert.rejects(feed(p, session, "hello! Next", false, "intruder"), /another package/);
  await assert.rejects(p.call("session.close", { sessionId: session.sessionId }, "intruder"), /another package/);
  assert.equal((await feed(p, session, "hello! Next")).length, 1);
  await p.call("session.close", { sessionId: session.sessionId });
  await assert.rejects(feed(p, session, "hello! Next"), /does not exist/);
});

test("emotion selection produces one actual extracted image and remains stable across snapshots", async () => {
  const p = provider(), settings = await p.call("settings.get", { profile: "default" });
  await p.call("settings.set", { profile: "default", settings: { ...settings, enableEmoticons: true } });
  const session = await open(p);
  const first = await feed(p, session, "你好。<emotion>happy</emotion>后");
  assert.equal(first.length, 2); assert.equal(first[1].kind, "image"); assert.equal(first[1].emotion, "happy");
  assert.match(first[1].source, /^\/host\/resources\/emoji_happy_/);
  assert.deepEqual(plain(await feed(p, session, "你好。<emotion>happy</emotion>后面")), []);
  assert.equal(p.resources.length, 1);
  await assert.rejects(feed(p, session, "你好。<emotion>happy</emotion>后面。<emotion>invented</emotion>"), /No image is configured/);
});

test("explicit custom categories resolve without using bundled resources", async () => {
  const p = provider(), initial = await p.call("settings.get", { profile: "default" });
  await p.call("settings.set", { profile: "default", settings: { ...initial, enableEmoticons: true, emojis: { wave: ["https://images.example/wave.png"] } } });
  const session = await open(p);
  assert.deepEqual(plain(await feed(p, session, "<emotion>wave</emotion>", true)), [{ kind: "image", source: "https://images.example/wave.png", emotion: "wave" }]);
  assert.equal(p.resources.length, 0);
});

test("prompt rules are confined to the active chat and retain its configured settings snapshot", async () => {
  const p = provider(), initial = await p.call("settings.get", { profile: "default" });
  await p.call("settings.set", { profile: "default", settings: { ...initial, enableSelfie: true, selfiePrompt: "silver hair", enableEmoticons: true } });
  const session = await open(p);
  await p.call("settings.set", { profile: "default", settings: { ...initial, customPrompt: "changed" } });
  const event = { event: "after_compose_system_prompt", eventPayload: { chatId: "chat", systemPrompt: "role prompt" } };
  const result = await p.service.composePrompt(event);
  assert.match(result.systemPrompt, /^role prompt/); assert.match(result.systemPrompt, /silver hair/); assert.match(result.systemPrompt, /<emotion>/);
  assert.doesNotMatch(result.systemPrompt, /changed/);
  assert.equal(await p.service.composePrompt({ ...event, eventPayload: { ...event.eventPayload, chatId: "other" } }), undefined);
  await p.call("session.close", { sessionId: session.sessionId });
  assert.equal(await p.service.composePrompt(event), undefined);
});

test("different assistant messages preserve their own sealed prefixes", async () => {
  const p = provider(), session = await open(p);
  await feed(p, session, "第一句。", true, "consumer", 10);
  assert.deepEqual(plain(await feed(p, session, "第一句。", false, "consumer", 10)), []);
  assert.deepEqual(plain(await feed(p, session, "第二句。", true, "consumer", 20)), [{ kind: "text", text: "第二句。" }]);
  await assert.rejects(feed(p, session, "改动。", false, "consumer", 10), /already finalized/);
});
