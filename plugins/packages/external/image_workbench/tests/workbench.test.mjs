import assert from 'node:assert/strict';
import test from 'node:test';
import { host, plain, screen, toolRuntime } from './runtime.mjs';

/** Finds a native action by its direct text or composed label. */
function button(page, text) {
  return page.nodes().find(node => ['Button', 'OutlinedButton', 'TextButton'].includes(node.type) && (node.props.text === text || [...node.children, ...Object.values(node.slots).flat()].some(child => child.props.text === text)));
}

/** Sends a controlled text edit through the actual compiled input callback. */
async function edit(page, label, value) {
  const node = page.nodes().find(node => node.type === 'TextField' && node.props.label === label);
  assert.ok(node, `Missing field: ${label}`);
  await page.dispatch(node, 'onValueChange', value);
}

test('production Compose renders the six-provider page and keeps provider form state separate', async () => {
  const io = host([], { OPENAI_API_KEY: 'configured' }), page = await screen(io);
  assert.ok(page.nodes().some(node => node.props.text === '生图工作台'));
  for (const title of ['OpenAI', '通义千问', '智谱', 'xAI / Grok', 'MiniMax', 'Nano Banana']) assert.ok(button(page, title), title);
  await edit(page, '模型', 'custom-openai-model');
  await page.dispatch(button(page, 'MiniMax'));
  assert.ok(page.nodes().some(node => node.type === 'TextField' && node.props.value === 'image-01'));
  assert.ok(page.nodes().some(node => node.props.text?.includes('本地参考图会上传到 Beeimg')));
  await page.dispatch(button(page, 'OpenAI'));
  assert.ok(page.nodes().some(node => node.type === 'TextField' && node.props.value === 'custom-openai-model'));
  assert.equal(page.fixture.errors.length, 0);
});

test('workbench uses the real unified tool service and renders its saved VFS image', async () => {
  const io = host([{ data: [{ b64_json: 'aQ==' }] }], { OPENAI_API_KEY: 'configured' });
  const tools = await toolRuntime(io), calls = [];
  const page = await screen(io, {}, async (name, input) => {
    calls.push({ name, input: plain(input) });
    assert.equal(name, 'image_workbench:generate_image');
    return tools.generate_image(input);
  });
  await edit(page, '提示词', 'a garden');
  await page.dispatch(button(page, '开始生图'));
  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0].input, { provider: 'openai', prompt: 'a garden', model: 'gpt-image-1' });
  assert.ok(page.nodes().some(node => node.type === 'Image' && node.props.uri === io.writes[0].path));
  await page.dispatch(button(page, '复制结果引用'));
  assert.equal(io.clipboard[0], `![生成的图片](${io.writes[0].path})`);
  assert.equal(page.fixture.errors.length, 0);
});

test('settings write the unified namespace and generation failures remain visible without an image', async () => {
  const io = host([]), page = await screen(io, {}, async () => { throw new Error('service rejected this request'); });
  await page.dispatch(button(page, '配置 API Key 与接口'));
  const secret = page.nodes().find(node => node.type === 'TextField' && node.props.label === 'OpenAI API Key');
  assert.equal(secret.props.isPassword, true);
  await edit(page, 'OpenAI API Key', 'saved-key');
  await page.dispatch(button(page, '保存配置'));
  assert.equal(io.environment.OPENAI_API_KEY, 'saved-key');
  await edit(page, '提示词', 'a garden');
  await page.dispatch(button(page, '开始生图'));
  assert.ok(page.nodes().some(node => node.props.text?.endsWith('service rejected this request')));
  assert.equal(page.nodes().some(node => node.type === 'Image'), false);
});

test('xAI video controls invoke the video tool and switching providers restores image mode', async () => {
  const io = host([], { XAI_API_KEY: 'configured' }), calls = [];
  const page = await screen(io, {}, async (name, input) => { calls.push({ name, input: plain(input) }); throw new Error('video fixture'); });
  await page.dispatch(button(page, 'xAI / Grok'));
  await page.dispatch(button(page, '生成视频'));
  await edit(page, '提示词', 'a garden');
  await edit(page, '视频时长（秒）', '6');
  await page.dispatch(button(page, '开始生成视频'));
  assert.equal(calls[0].name, 'image_workbench:generate_video');
  assert.equal(calls[0].input.duration, 6);
  await page.dispatch(button(page, 'OpenAI'));
  assert.ok(button(page, '开始生图'));
  assert.equal(page.nodes().some(node => node.type === 'TextField' && node.props.label === '视频时长（秒）'), false);
});
