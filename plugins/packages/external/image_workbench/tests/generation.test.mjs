import assert from 'node:assert/strict';
import test from 'node:test';
import { host, plain, toolRuntime } from './runtime.mjs';

const keys = { OPENAI_API_KEY: 'openai-secret', DASHSCOPE_API_KEY: 'qwen-secret', ZHIPU_API_KEY: 'zhipu-secret', XAI_API_KEY: 'xai-secret', MINIMAX_API_KEY: 'minimax-secret', NANOBANANA_API_KEY: 'nano-secret' };

for (const [provider, responses] of [
  ['openai', [{ data: [{ b64_json: 'aW1hZ2U=' }] }]],
  ['qwen', [{ output: { task_id: 'qwen-job' } }, { output: { task_status: 'RUNNING' } }, { output: { task_status: 'SUCCEEDED', results: [{ url: 'https://images.test/qwen.png' }] } }]],
  ['zhipu', [{ data: [{ url: 'https://images.test/zhipu.png' }] }]],
  ['xai', [{ data: [{ url: 'https://images.test/xai.jpeg' }] }]],
  ['minimax', [{ base_resp: { status_code: 0 }, data: { image_base64: ['aW1hZ2U='] } }]],
  ['nanobanana', [{ code: 0, data: { id: 'nano-job' } }, { code: -22 }, { code: 0, data: { status: 'succeeded', results: [{ url: 'https://images.test/nano.png' }] } }]],
]) {
  test(`${provider} uses the unified public request and VFS asset response`, async () => {
    const io = host(structuredClone(responses), keys), tools = await toolRuntime(io);
    const output = plain(await tools.generate_image({ provider, prompt: '  a quiet garden  ' }));
    assert.equal(output.success, true);
    assert.equal(output.data.provider, provider);
    assert.equal(output.data.prompt, 'a quiet garden');
    assert.equal(output.data.kind, 'image');
    assert.equal(output.data.assets.length, 1);
    assert.equal(io.writes.length, 1);
    assert.match(output.data.assets[0].uri, new RegExp(`^/operitapp/plugins/image_workbench/generations/${provider}/`));
    assert.equal(output.data.markdown, `![生成的图片](${io.writes[0].path})`);
    assert.ok(!JSON.stringify(output).includes('-secret'));
    assert.ok(io.requests.every(request => request.headers.Authorization.endsWith('-secret')));
  });
}

test('count, dimensions and boolean controls map to service parameters and retain every output', async () => {
  const io = host([{ output: { task_id: 'batch' } }, { output: { task_status: 'SUCCEEDED', results: [{ url: 'https://images.test/1.png' }, { url: 'https://images.test/2.png' }] } }], keys);
  const output = await (await toolRuntime(io)).generate_image({ provider: 'qwen', prompt: 'garden', count: 2, size: '1024x1024', optimize_prompt: false, watermark: false, negative_prompt: 'text' });
  assert.deepEqual(io.requests[0].body.parameters, { n: 2, size: '1024*1024', prompt_extend: false, watermark: false, negative_prompt: 'text' });
  assert.equal(output.data.assets.length, 2);
  assert.equal(new Set(io.writes.map(file => file.path)).size, 2);
  const mini = host([{ data: { image_base64: ['aQ==', 'aQ=='] } }], keys);
  const result = await (await toolRuntime(mini)).generate_image({ provider: 'minimax', prompt: 'garden', size: '1024x768', count: 2, optimize_prompt: false, watermark: true });
  assert.equal(mini.requests[0].body.width, 1024);
  assert.equal(mini.requests[0].body.height, 768);
  assert.equal(mini.requests[0].body.prompt_optimizer, false);
  assert.equal(mini.requests[0].body.aigc_watermark, true);
  assert.equal(result.data.assets.length, 2);
});

test('reference inputs use the shared field and local uploads require an explicit credential', async () => {
  const env = { ...keys, BEEIMG_API_KEY: 'bee-key' };
  const io = host([{ data: { image_base64: ['aQ=='] } }], env);
  await (await toolRuntime(io)).generate_image({ provider: 'minimax', prompt: 'garden', reference_images: ['https://images.test/reference.png', '/selected/portrait.png'] });
  assert.equal(io.uploads[0].files[0].file_path, '/selected/portrait.png');
  assert.deepEqual(io.requests[0].body.subject_reference.map(item => item.image_file), ['https://images.test/reference.png', 'https://images.test/uploaded.png']);
  const missing = host([], keys), tools = await toolRuntime(missing);
  await assert.rejects(tools.generate_image({ provider: 'minimax', prompt: 'garden', reference_images: ['/selected/portrait.png'] }), /Beeimg/);
  assert.equal(missing.uploads.length, 0);
  assert.equal(missing.requests.length, 0);
});

test('capability validation rejects unsupported options before creating a paid generation', async () => {
  const io = host([], keys), tools = await toolRuntime(io);
  for (const input of [{ provider: 'openai', prompt: 'garden', count: 2 }, { provider: 'qwen', prompt: 'garden', count: 1.5 }, { provider: 'minimax', prompt: 'garden', size: '1024x1024', aspect_ratio: '1:1' }, { provider: 'nanobanana', prompt: 'garden', model: 'nano-banana', resolution: '4K' }]) await assert.rejects(tools.generate_image(input));
  assert.equal(io.requests.length, 0);
});

test('HTTP, malformed JSON, failed tasks and failed file writes propagate without another request', async () => {
  for (const response of [{ httpStatus: 401, error: 'bad credential' }, { raw: '{' }]) {
    const io = host([response], keys);
    await assert.rejects((await toolRuntime(io)).generate_image({ provider: 'openai', prompt: 'garden' }));
    assert.equal(io.requests.length, 1); assert.equal(io.writes.length, 0);
  }
  const failed = host([{ output: { task_id: 'failed' } }, { output: { task_status: 'FAILED' } }], keys);
  await assert.rejects((await toolRuntime(failed)).generate_image({ provider: 'qwen', prompt: 'garden' }), /失败/);
  assert.equal(failed.requests.length, 2);
  const disk = host([{ data: [{ b64_json: 'aQ==' }] }], keys);
  disk.tools.Files.writeBinary = async () => ({ successful: false, details: 'disk full' });
  await assert.rejects((await toolRuntime(disk)).generate_image({ provider: 'openai', prompt: 'garden' }), /disk full/);
});

test('only the selected credential is required and list_methods never returns credentials', async () => {
  const io = host([{ data: [{ b64_json: 'aQ==' }] }], { OPENAI_API_KEY: 'one-secret' });
  const tools = await toolRuntime(io), methods = plain(tools.list_methods());
  assert.equal(methods.data.length, 6);
  assert.deepEqual(methods.data.filter(item => item.configured).map(item => item.id), ['openai']);
  assert.ok(!JSON.stringify(methods).includes('one-secret'));
  await tools.generate_image({ provider: 'openai', prompt: 'garden' });
  await assert.rejects(tools.generate_image({ provider: 'zhipu', prompt: 'garden' }), /API Key/);
});

test('compatible endpoint configuration produces one canonical URL and supports explicit URL image responses', async () => {
  for (const base of ['https://compatible.test', 'https://compatible.test/v1/', 'https://compatible.test/v1/images/generations']) {
    const io = host([{ data: [{ url: 'https://images.test/result.png' }] }], { ...keys, OPENAI_API_BASE_URL: base });
    await (await toolRuntime(io)).generate_image({ provider: 'openai', prompt: 'garden' });
    assert.equal(io.requests[0].url, 'https://compatible.test/v1/images/generations');
    assert.equal(io.writes[0].url, 'https://images.test/result.png');
  }
});

test('xAI video shares assets while keeping image references inside the service request', async () => {
  const io = host([{ request_id: 'video-job' }, { status: 'processing' }, { status: 'done', video: { url: 'https://images.test/output.mp4' } }], keys);
  const output = await (await toolRuntime(io)).generate_video({ provider: 'xai', prompt: 'garden', reference_image: '/selected/reference.png', duration: 6 });
  assert.equal(output.data.kind, 'video');
  assert.match(io.requests[0].body.image.url, /^data:image\/png;base64,/);
  assert.equal(io.requests[0].body.duration, 6);
  assert.equal(io.uploads.length, 0);
  assert.match(output.data.assets[0].uri, /\.mp4$/);
});
