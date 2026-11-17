import assert from "node:assert/strict";
import test from "node:test";
import { c2c, load, plain } from "./harness.mjs";

test("quotes, cards, voice metadata and author fields are normalized without exposing scene tokens in readable text", () => {
  const frame = c2c();
  Object.assign(frame.d, { message_type: 103, message_scene: { ext: ["msg_idx=REFIDX_own==", "ref_msg_idx=REFIDX_old==", "auth_token=secret"] },
    author: { user_openid: "internal-author", username: "小明", avatar: "https://cdn.example/avatar", bot: false },
    msg_elements: [{ msg_idx: "REFIDX_old==", content: "上一条建议", attachments: [{ url: "https://cdn.example/voice", content_type: "voice", filename: "voice.silk", voice_wav_url: "https://cdn.example/voice.wav", asr_refer_text: "语音内容" }],
      msg_elements: [{ content: "更早的消息" }] }], ark_data: { ark_name: "位置", fields: { title: "公园", jump_url: "https://example.com" } } });
  const event = load("model").inbound(frame), { chatText, messageAttachments } = load("messages");
  assert.equal(event.referenceIndex, "REFIDX_own=="); assert.equal(event.referencedMessageIndex, "REFIDX_old==");
  assert.equal(event.author.name, "小明"); assert.equal(event.messageType, 103);
  assert.equal(event.elements[0].elements[0].text, "更早的消息");
  assert.equal(messageAttachments(event)[0].kind, "audio");
  const readable = chatText(event);
  assert.match(readable, /> 上一条建议/); assert.match(readable, />> 更早的消息/);
  assert.match(readable, /\[语音转写\] 语音内容/); assert.match(readable, /公园/);
  assert.doesNotMatch(readable, /REFIDX|internal-author|auth_token|secret/);
});

test("recursive references and aggregate attachments have explicit limits", () => {
  const frame = c2c(); let parent = frame.d;
  for (let depth = 0; depth < 8; depth++) { parent.msg_elements = [{ content: "nested" }]; parent = parent.msg_elements[0]; }
  assert.throws(() => load("model").inbound(frame), /nested message limit/);
  const many = c2c(); many.d.msg_elements = Array.from({ length: 51 }, () => ({ content: "nested" }));
  assert.throws(() => load("model").inbound(many), /nested message limit/);
  const crowded = c2c(); crowded.d.msg_elements = [{ attachments: Array.from({ length: 10 }, () => ({ url: "https://cdn.example/a", content_type: "image/png" })) }];
  crowded.d.attachments = [{ url: "https://cdn.example/a", content_type: "image/png" }];
  const event = load("model").inbound(crowded);
  assert.throws(() => load("messages").messageAttachments(event), /more than ten attachments/);
});

test("known internal reply envelopes disappear while ordinary text and readable quote content survive", () => {
  const { externalText } = load("messages");
  for (const marker of ["[[reply_to:internal-id]]", "[[reply_to_current]]", "[CQ:reply,id=internal-id]", "[reply_id:internal-id]", '<reply id="internal-id"/>', "[QQ group; sender=internal-id; message=internal-message]"]) {
    assert.equal(externalText(`${marker}你好。`), "你好。");
  }
  assert.equal(externalText('<reply_to sender="internal-id" timestamp="123">上一条消息</reply_to>\n回复'), "上一条消息\n回复");
  assert.equal(externalText("普通 [example] 和 https://example.com/reply?id=123"), "普通 [example] 和 https://example.com/reply?id=123");
});

test("a reply envelope split across streamed sentences never emits its identifier fragments", () => {
  const { ReplyTextStream } = load("messages"), stream = new ReplyTextStream();
  const chunks = [stream.push("[[reply_"), stream.push("to:ROBOT1.0_"), stream.push("internal]]你好。"), stream.push("[CQ:reply,id="), stream.push("private"), stream.push("", true)];
  assert.deepEqual(plain(chunks), ["", "", "你好。", "", "", ""]);
});
