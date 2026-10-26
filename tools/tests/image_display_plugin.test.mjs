import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const packageRoot = new URL('../../plugins/packages/buildin/image_display/', import.meta.url);
const directory = '/app/data/plugin_data/space/com.operit.image_display';

/** Loads the packaged entry with observable prompt registration and storage access. */
function runtime(getSpaceDataDir) {
  const hooks = [];
  const exports = {};
  vm.runInNewContext(readFileSync(new URL('dist/main.js', packageRoot), 'utf8'), {
    exports,
    ToolPkg: {
      getSpaceDataDir,
      /** Captures the only capability this plugin registers. */
      registerSystemPromptComposeHook(hook) { hooks.push(hook); },
    },
  });
  return { exports, hooks };
}

/** Creates a prompt event in the same shape as the Core dispatch contract. */
function event(useEnglish) {
  return {
    eventName: 'after_compose_system_prompt',
    eventPayload: { systemPrompt: 'existing system prompt', useEnglish },
  };
}

test('space-installed image plugin registers without accessing files', () => {
  const manifest = JSON.parse(readFileSync(new URL('manifest.json', packageRoot), 'utf8'));
  assert.equal(manifest.installation_scope, 'space');
  assert.deepEqual(manifest.subpackages, []);
  const fixture = runtime(() => { throw new Error('Registration must not access storage'); });
  assert.equal(fixture.exports.registerToolPkg(), true);
  assert.equal(fixture.hooks.length, 1);
  assert.equal(fixture.hooks[0].function, fixture.exports.onSystemPromptCompose);
});

test('both locales preserve the prompt and teach the actual shared Markdown path', () => {
  for (const useEnglish of [false, true]) {
    const actualDirectory = directory + '-fixture';
    const fixture = runtime(() => actualDirectory);
    const result = fixture.exports.onSystemPromptCompose(event(useEnglish));
    assert.ok(result.systemPrompt.startsWith('existing system prompt\n\n'));
    assert.ok(result.systemPrompt.includes(`](${actualDirectory}/image-unique.png)`));
    assert.match(result.systemPrompt, useEnglish ? /Confirm success/ : /确认成功/);
    assert.match(result.systemPrompt, useEnglish ? /Keep existing images/ : /保留已有图片/);
  }
});

test('other prompt stages do not access storage or change the prompt', () => {
  const fixture = runtime(() => { throw new Error('Unexpected storage access'); });
  const request = event(false);
  request.eventName = 'before_compose_system_prompt';
  assert.equal(fixture.exports.onSystemPromptCompose(request), null);
});

test('missing prompts and shared storage errors remain explicit failures', () => {
  const fixture = runtime(() => { throw new Error('Shared storage denied'); });
  assert.throws(() => fixture.exports.onSystemPromptCompose(event(false)), /Shared storage denied/);
  const request = event(false);
  delete request.eventPayload.systemPrompt;
  assert.throws(() => fixture.exports.onSystemPromptCompose(request), /requires a system prompt/);
});
