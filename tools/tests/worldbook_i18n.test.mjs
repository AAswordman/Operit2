import assert from 'node:assert/strict';
import test from 'node:test';
import { loadModule, plain } from '../../plugins/packages/buildin/character_cards/tests/runtime.mjs';

/** Loads real services and tools against one retained storage owner and a changeable host locale. */
function runtime() {
  let language = 'en-US';
  const files = new Map(), results = [];
  const globals = {
    getLang: () => language,
    complete(value) { results.push(plain(value)); },
    ToolPkg: { getConfigDir: () => '/worldbook', async callDependency() { return []; } },
    Tools: { Files: {
      async exists(path) { return { exists: files.has(path) }; }, async mkdir() {},
      async read(path) { return { content: files.get(path) }; },
      async write(path, content) { files.set(path, content); }, async deleteFile(path) { files.delete(path); },
    } },
  };
  return {
    service: loadModule('../../external/worldbook/src/shared/worldbook_service.ts', globals),
    tools: loadModule('../../external/worldbook/src/packages/worldbook_tools.ts', globals),
    i18n: loadModule('../../external/worldbook/src/i18n/index.ts', globals),
    files, results, setLanguage(value) { language = value; },
  };
}

test('import warnings, entry errors and tool results follow the host locale without translating stored content', async () => {
  const r = runtime();
  const imported = await r.service.importWorldBookEntries({ content: JSON.stringify({ entries: {
    0: { comment: '用户名称', content: '用户原文', key: ['关键词'], keysecondary: ['extra'] },
  } }) });
  assert.equal(imported.imported_count, 1);
  assert.match(imported.warnings.join('\n'), /secondary keywords/);
  assert.doesNotMatch(imported.warnings.join('\n'), /\p{Script=Han}/u);
  assert.equal(imported.entries[0].content, '用户原文');
  await assert.rejects(r.service.getWorldBookEntry('missing'), error => error.message === 'Entry not found: missing');
  await r.tools.create_entry({ name: '用户条目', content: '原始正文' });
  assert.equal(r.results.at(-1).message, 'Entry created');
  assert.equal(r.results.at(-1).entry.name, '用户条目');
  const entryId = r.results.at(-1).entry.id;
  r.setLanguage('zh-CN');
  await r.tools.toggle_entry({ id: entryId });
  assert.match(r.results.at(-1).message, /已禁用/);
  await assert.rejects(r.service.getWorldBookEntry('missing'), error => error.message === '条目不存在: missing');
  assert.equal(JSON.parse(r.files.get('/worldbook/entries.json')).find(entry => entry.id === entryId).content, '原始正文');
});

test('variable snippet descriptions, labels and example paths are localized with stable template IDs', () => {
  const r = runtime();
  const english = r.i18n.currentWorldBookI18n().contentSnippets;
  assert.ok(english.length > 0);
  assert.ok(english.every(snippet => !/\p{Script=Han}/u.test(JSON.stringify(snippet))));
  assert.match(english.find(snippet => snippet.id === 'get_message_variable').content, /stat_data\.system\.points/);
  r.setLanguage('zh-CN');
  const chinese = r.i18n.currentWorldBookI18n().contentSnippets;
  assert.deepEqual(plain(english.map(snippet => snippet.id)), plain(chinese.map(snippet => snippet.id)));
  assert.match(chinese.find(snippet => snippet.id === 'get_message_variable').content, /stat_data\.系统\.积分/);
});
