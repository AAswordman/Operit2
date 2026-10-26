import assert from "node:assert/strict";
import test from "node:test";
import { load, plain } from "./harness.mjs";
const defaults = load("settings").defaultSettings();
const { splitSnapshot, SentenceSession } = load("sentences");

/** Extracts readable text for segmentation expectations. */
function texts(source, final = true, settings = defaults) { return plain(splitSnapshot(source, final, settings)).map(segment => segment.kind === "text" ? segment.text : segment); }

test("stable sentences arrive incrementally and finalization flushes only the unfinished tail", () => {
  const stream = new SentenceSession(defaults);
  assert.deepEqual(plain(stream.feed("你好。后", false)), [{ kind: "text", text: "你好。" }]);
  assert.deepEqual(plain(stream.feed("你好。后面", false)), []);
  assert.deepEqual(plain(stream.feed("你好。后面！再见", false)), [{ kind: "text", text: "后面！" }]);
  assert.deepEqual(plain(stream.feed("你好。后面！再见", true)), [{ kind: "text", text: "再见" }]);
});

test("every source prefix preserves irreversible segments across Markdown and emotion token boundaries", () => {
  const source = '你好！<emotion>happy</emotion>访问 https://example.com/a?q=1.2。然后看 [帮助](https://example.com/a_(b))。\n```ts\nconst a = "不要拆。";\n```\n![自拍](https://example.com/image.png)再见！';
  const settings = { ...defaults, enableEmoticons: true }, stream = new SentenceSession(settings), all = [];
  for (let end = 0; end <= source.length; end++) all.push(...stream.feed(source.slice(0, end), false));
  all.push(...stream.feed(source, true));
  assert.deepEqual(plain(all), plain(splitSnapshot(source, true, settings)));
  assert.equal(all.filter(segment => segment.kind === "emotion").length, 1);
  assert.equal(all.filter(segment => segment.kind === "image").length, 1);
});

test("Chinese, English, quotes, decimals, emails and URLs preserve valid boundaries", () => {
  assert.deepEqual(texts('你好？！“真的吗？”再见。'), ['你好？！', '“真的吗？”', '再见。']);
  assert.deepEqual(texts('The value is 3.14. Email a@example.com. Next!'), ['The value is 3.14.', 'Email a@example.com.', 'Next!']);
  assert.deepEqual(texts('查看 https://example.com/a。好的。'), ['查看 https://example.com/a。', '好的。']);
});

test("fences, tables, formulas, inline code and Markdown images remain atomic", () => {
  assert.deepEqual(texts('前面\n```ts\nconst a = "a.b。";\n```\n后面。'), ['前面', '```ts\nconst a = "a.b。";\n```', '后面。']);
  assert.deepEqual(texts('$$a.b+c。$$结束。'), ['$$a.b+c。$$', '结束。']);
  assert.deepEqual(texts('| A | B |\n|---|---|\n|。|。|\n\n好了。'), ['| A | B |\n|---|---|\n|。|。|', '好了。']);
  assert.deepEqual(texts('`a.b。` 还在一句里。'), ['a.b。 还在一句里。']);
  assert.deepEqual(texts('![开心](https://example.com/a_(b).png)'), [{ kind: 'image', source: 'https://example.com/a_(b).png', emotion: null }]);
});

test("structured list lines and everyday multiplication remain readable", () => {
  assert.deepEqual(texts('* 第一项\n* 第二项\n完成。'), ['第一项', '第二项', '完成。']);
  assert.deepEqual(texts('2 * 3 = 6。'), ['2 * 3 = 6。']);
});

test("punctuation removal keeps ellipses and protected blocks intact", () => {
  const settings = { ...defaults, removePunctuation: true };
  assert.deepEqual(texts('你好。想想...再见！', true, settings), ['你好', '想想...再见']);
});

test("emotion markup is held while incomplete and emitted separately when enabled", () => {
  const settings = { ...defaults, enableEmoticons: true };
  assert.deepEqual(texts('你好。<emot', false, settings), ['你好。']);
  assert.deepEqual(texts('你好。<emotion>happy</emotion>', true, settings), ['你好。', { kind: 'emotion', category: 'happy' }]);
  assert.deepEqual(texts('你好。<emotion>happy</emotion>'), ['你好。']);
});

test("internal markup is never delivered and malformed terminal structures fail explicitly", () => {
  assert.deepEqual(texts('<think>private</think>你好。<tool name="x">secret</tool>'), ['你好。']);
  assert.throws(() => texts('<think>private'), /Unclosed internal/);
  assert.throws(() => texts('```ts\nprivate'), /Unclosed fenced/);
  assert.throws(() => texts('<emotion>happy'), /Unclosed emotion/);
});

test("revisions of already delivered sentences are rejected", () => {
  const stream = new SentenceSession(defaults);
  stream.feed('先发送。后', false);
  assert.throws(() => stream.feed('改变了。后', false), /already delivered/);
});

test("protected link targets, URL underscores and inline source retain their literal content", () => {
  assert.deepEqual(texts('打开 [文档](https://example.com/a_(b))。'), ['打开 文档 (https://example.com/a_(b))。']);
  assert.deepEqual(texts('看 https://example.com/my_favorite_file。'), ['看 https://example.com/my_favorite_file。']);
  assert.deepEqual(texts('执行 `const x = a_b_c;`。'), ['执行 const x = a_b_c;。']);
});
