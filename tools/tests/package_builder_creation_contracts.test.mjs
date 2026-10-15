import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import test from 'node:test';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const require = createRequire(path.join(root, 'plugins/packages/buildin/workflow/package.json'));
const ts = require('typescript');
const guideDirectory = 'plugins/docs/package_builder/';
const skillDirectory = 'plugins/skills/external/PackageBuilder/';
const promptPath = 'apps/flutter/app/lib/ui/features/packages/market/PluginCreationIntent.dart';
const editorPath = 'plugins/packages/buildin/operit_editor.ts';
const documents = [
  guideDirectory + 'PLUGIN_CREATION_WORKFLOW.md',
  guideDirectory + 'SCRIPT_DEV_GUIDE.md',
  guideDirectory + 'TOOLPKG_FORMAT_GUIDE.md',
  skillDirectory + 'SKILL.md',
];

/** Reads one current repository source as UTF-8. */
function source(relative) {
  return readFileSync(path.join(root, relative), 'utf8').replace(/\r\n/g, '\n');
}

/** Loads the real editor source without stale built assets or platform services. */
function editor(exec = async () => { throw new Error('Unexpected editor host call'); }) {
  const context = vm.createContext({ exports: {}, Tools: { SoftwareSettings: { exec } } });
  const result = ts.transpileModule(source(editorPath), {
    compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS },
    reportDiagnostics: true,
    fileName: editorPath,
  });
  assert.deepEqual(result.diagnostics, []);
  vm.runInContext(result.outputText, context);
  return context.exports;
}

/** Extracts only CLI-style string arrays, preserving escaped JSON arguments. */
function commands(text) {
  return Array.from(text.matchAll(/\[(?:"(?:\\.|[^"\\])*"\s*(?:,\s*)?)+\]/gu),
    match => JSON.parse(match[0]));
}

/** Confirms every include stays in the plugin tree and names an existing resource. */
test('PackageBuilder bundles the current workflow with types and examples', () => {
  const includes = JSON.parse(source(skillDirectory + 'skill.include.json')).includes;
  const workflow = includes.filter(include => include.target === 'references/PLUGIN_CREATION_WORKFLOW.md');
  assert.equal(workflow.length, 1);
  for (const include of includes) {
    const resolved = path.resolve(root, skillDirectory, include.source);
    const relative = path.relative(path.join(root, 'plugins'), resolved);
    assert.equal(path.isAbsolute(relative), false);
    assert.doesNotMatch(relative, /^\.\.(?:[\\/]|$)/u);
    assert.equal(existsSync(resolved), true, include.source);
  }
  assert.match(source(skillDirectory + 'SKILL.md'), /references\/PLUGIN_CREATION_WORKFLOW\.md/u);
});

/** Prevents the authoring materials from reintroducing removed execution routes. */
test('authoring instructions contain no obsolete scripts, source paths, or fixed mobile directory', () => {
  const obsolete = /execute_js\.(?:bat|sh)|run_sandbox_script\.(?:bat|sh)|debug_toolpkg\.(?:py|bat|sh)|debug_run_sandbox_script|sync_example_packages\.py|app\/src\/main|Android\/data\/com\.ai\.assistance\.operit|AAswordman\/Operit\.git|\/sdcard\/Download|手机下载\/Operit|JAVA_BRIDGE_INTERFACE\.md|examples\/(?:quick_start|various_search|various_output|windows_control|template_try|time\.ts)/u;
  for (const file of [...documents, promptPath, editorPath]) {
    assert.doesNotMatch(source(file), obsolete, file);
  }
});

/** Checks that linked documentation can be read from the installed reference directory. */
test('authoring reference links resolve to bundled local documents', () => {
  for (const file of documents.slice(0, 3)) {
    for (const match of source(file).matchAll(/\]\((\.\/[^)#]+)(?:#[^)]*)?\)/gu)) {
      assert.equal(existsSync(path.resolve(root, path.dirname(file), match[1])), true, `${file}: ${match[1]}`);
    }
  }
  const guide = source(guideDirectory + 'SCRIPT_DEV_GUIDE.md');
  for (const example of ['buildin/time.ts', 'buildin/operit_editor.ts', 'external/template_try', 'buildin/workflow']) {
    assert.equal(existsSync(path.join(root, 'plugins/packages', example)), true, example);
    assert.match(guide, new RegExp(`examples/packages/${example.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&')}`, 'u'));
  }
});

/** Ensures AI sees an executable command contract instead of a decorative query. */
test('operit_editor declares required args and no query parameter', () => {
  const metadata = JSON.parse(/\/\* METADATA\s*([\s\S]*?)\*\//u.exec(source(editorPath))[1]);
  assert.equal(metadata.name, 'operit_editor');
  assert.equal(metadata.tools.length, 1);
  assert.equal(metadata.tools[0].name, 'operit_editor');
  assert.deepEqual(metadata.tools[0].parameters.map(({ name, type, required }) => ({ name, type, required })),
    [{ name: 'args', type: 'array', required: true }]);
  assert.match(metadata.tools[0].description.zh, /Tools\.SoftwareSettings\.exec\(args\)/u);
  assert.match(metadata.tools[0].description.zh, /用户确认/u);
});

/** Exercises reads and writes against the exact host SDK entry point used in production. */
test('operit_editor executes commands once and returns the actual host output', async () => {
  const calls = [];
  const exports = editor(async args => {
    calls.push(Array.from(args));
    return `host-output:${JSON.stringify(args)}`;
  });
  assert.deepEqual(Object.keys(exports).sort(), ['main', 'operit_editor']);
  assert.equal(exports.main, exports.operit_editor);
  assert.deepEqual(calls, [], 'module loading must not execute any commands');
  const commands = [
    [], ['package', 'list'], ['package', 'enable', 'sample'],
    ['skill', 'show', 'PackageBuilder'], ['mcp', 'disable', 'sample'],
    ['model', 'function-set', 'chat', 'provider-id', 'model-id'],
    ['prefs', 'thinking', 'on'], ['log', 'package'], ['workspace', 'list'],
  ];
  for (const args of commands) {
    assert.equal(await exports.operit_editor({ args }), `host-output:${JSON.stringify(args)}`);
  }
  assert.deepEqual(calls, commands);
});

/** Preserves JSON payloads, whitespace, quotes, Unicode, and paths as separate arguments. */
test('operit_editor accepts native and JSON arrays without rewriting arguments', async () => {
  const calls = [];
  const exports = editor(async args => { calls.push(Array.from(args)); return ''; });
  const args = ['package', 'exec', 'sample:hello_world', JSON.stringify({
    name: '世界', text: '  a \"quoted\" line\n第二行  ', path: 'C:\\Users\\test folder\\main.ts',
  })];
  assert.equal(await exports.operit_editor({ args }), '');
  assert.equal(await exports.main({ args: JSON.stringify(args) }), '');
  assert.deepEqual(calls, [args, args]);
});

/** Rejects misleading legacy input and malformed arguments before reaching the host. */
test('operit_editor rejects query and invalid args without executing anything', async () => {
  const calls = [];
  const exports = editor(async args => { calls.push(args); return 'must not run'; });
  const inputs = [
    undefined, null, [], 'package list', {}, { query: '启用插件' },
    { query: '启用插件', args: ['package', 'list'] },
    { args: undefined }, { args: null }, { args: 1 }, { args: true },
    { args: 'package list' }, { args: 'null' }, { args: '{}' },
    { args: '["package",]' }, { args: ['package', 1] }, { args: ['package', null] },
    { args: ['package', {}] }, { args: '["package",true]' }, { args: new Array(1) },
  ];
  for (const params of inputs) {
    await assert.rejects(exports.operit_editor(params), /operit_editor:/u);
  }
  assert.deepEqual(calls, []);
});

/** Neither host failures nor diagnostic output may become fabricated success responses. */
test('operit_editor preserves executor failure and never retries or falls back to a guide', async () => {
  for (const message of ['permission denied', 'Core command executor is not configured.', 'package not found']) {
    const failure = new Error(message);
    let calls = 0;
    const exports = editor(async () => { calls++; throw failure; });
    await assert.rejects(exports.operit_editor({ args: ['package', 'list'] }), error => error === failure);
    assert.equal(calls, 1);
  }
  const output = 'stderr: command failed\nstdout: diagnostic details';
  assert.equal(await editor(async () => output).operit_editor({ args: [] }), output);
});

/** Guards the SDK-to-core-command route so the wrapper cannot use an invented tool. */
test('editor host binding routes to the real core command executor', () => {
  assert.match(source('core/crates/plugin/sdk/src/js_sdk/runtime_bindings.rs'),
    /namespace: "SoftwareSettings", method: "exec", tool: BuiltinToolName::ExecuteCliCommand/u);
  const registration = source('core/crates/tool/services/src/tools/ToolRegistration.rs');
  const start = registration.indexOf('BuiltinToolName::ExecuteCliCommand,');
  const end = registration.indexOf('BuiltinToolName::ReadEnvironmentVariable,', start);
  assert.ok(start >= 0 && end > start);
  const executor = registration.slice(start, end);
  assert.match(executor, /serde_json::from_str::<Vec<String>>/u);
  assert.match(executor, /context\.coreCommandExecutor/u);
  assert.match(executor, /executor\(args\)\.await/u);
  assert.match(executor, /Err\(error\) => toolErrorResult\(&tool, error\)/u);
  const proxy = source('core/crates/proxy/local/src/lib.rs');
  assert.match(proxy, /Self::installCoreCommandExecutor\(&mut application\)/u);
  assert.match(proxy, /application\.toolHandler\.setCoreCommandExecutor\(executor\)/u);
  assert.match(proxy, /operit_command_core::run_core_command\(/u);
  assert.match(proxy, /runtime\.sharedCoreCommandRuntime\(\)/u);
  assert.doesNotMatch(proxy, /run_core_command_with_context\(/u);
});

/** Checks documented editor command arity against the current core command surface. */
test('editor command examples include current required arguments', async () => {
  const listed = commands(source(guideDirectory + 'PLUGIN_CREATION_WORKFLOW.md'));
  const arity = new Map([
    ['package.help', 2], ['package.dir', 2], ['package.import', 3], ['package.delete', 3],
    ['package.list', 2], ['package.more', 2], ['package.load', 3], ['package.show', 3],
    ['package.enable', 3], ['package.disable', 3], ['package.use', 3], ['package.exec', 4],
    ['skill', 1], ['skill.dir', 2], ['skill.list', 2], ['skill.show', 3], ['skill.delete', 3], ['skill.load', 3], ['skill.visible', 4], ['skill.errors', 2],
    ['mcp.dir', 2], ['mcp.list', 2], ['mcp.show', 3], ['mcp.enable', 3], ['mcp.disable', 3], ['mcp.start', 3], ['mcp.tools', 3],
    ['model.list', 2], ['model.show', 3], ['model.function-list', 2], ['model.function-show', 3], ['model.function-set', 5],
    ['prefs.show', 2], ['prefs.thinking', 3], ['prefs.stream', 3], ['prefs.media-history', 4], ['prefs.mcp-timeout', 3],
    ['log.show', 2], ['log.package', 2], ['log.path', 2], ['log.clear', 2],
    ['tool', 1], ['tool.list', 3], ['tool.show', 3], ['tool.exec', 4],
    ['workspace', 1], ['workspace.list', 2], ['workspace.commands', 3], ['workspace.run', 4], ['workspace.bind-default', 3],
  ]);
  assert.ok(listed.length > 10);
  for (const args of listed) {
    const key = args.slice(0, 2).join('.');
    assert.equal(args.length, arity.get(key), JSON.stringify(args));
    const rust = source(`core/crates/command/core/src/commands/${args[0]}.rs`);
    if (args.length > 1 && args[1] !== 'help') {
      assert.match(rust, new RegExp(`"${args[1]}"`, 'u'), JSON.stringify(args));
    }
  }
});

/** Verifies workflow commands are explicit installation operations, not shell snippets. */
test('workflow documents installation, observed state, authorized replacement, and skill refresh', () => {
  const text = source(guideDirectory + 'PLUGIN_CREATION_WORKFLOW.md');
  const listed = commands(text);
  for (const expected of [
    ['package', 'import', '<artifact_host_path>'],
    ['package', 'delete', '<package_id>'],
    ['package', 'exec', '<runtime_package_name>:<tool_name>', '{}'],
    ['skill', 'delete', 'PackageBuilder'],
    ['skill', 'load', 'PackageBuilder'],
    ['skill', 'visible', 'PackageBuilder', 'true'],
  ]) {
    assert.ok(listed.some(actual => JSON.stringify(actual) === JSON.stringify(expected)), JSON.stringify(expected));
  }
  assert.match(text, /启用后重新读取.*真实.*enabled/u);
  assert.match(text, /不提供覆盖安装或自动热更新/u);
  assert.match(text, /获得用户确认后执行/u);
  assert.match(text, /不会自动覆盖它的附件/u);
  assert.match(text, /manifest 必须位于 ZIP 根目录/u);
  assert.match(source('plugins/types/files.d.ts'), /function zip\(source: string, destination: string, include_root_directory\?: boolean\)/u);
  assert.match(text, /Tools\.Files\.zip\(sourceVfsPath, artifactVfsPath, false\)/u);
});

/** Checks each embedded command example contains a valid escaped JSON tool payload. */
test('first-script execution example preserves its declared name and JSON parameters', () => {
  const text = source(guideDirectory + 'SCRIPT_DEV_GUIDE.md');
  const exec = commands(text).filter(args => args[0] === 'package' && args[1] === 'exec');
  assert.deepEqual(exec, [['package', 'exec', 'MyNewScript:hello_world', '{"name":"世界"}']]);
  assert.deepEqual(JSON.parse(exec[0][3]), { name: '世界' });
  assert.match(text, /"name": "MyNewScript"/u);
  assert.match(text, /"typeRoots": \["\.\.\/types"\]/u);
  assert.match(text, /"include": \["src\/\*\*\/\*\.ts", "\.\.\/types\/\*\*\/\*\.d\.ts"\]/u);
});

/** Prevents setup from reporting success before reading the actual package state. */
test('creator setup verifies the editor state instead of interpreting status text', () => {
  const text = source('apps/flutter/app/lib/ui/features/packages/screens/QuickPluginCreatorSetupSupport.dart');
  assert.match(text, /\.enablePackage\(packageName: 'operit_editor'\);[\s\S]*\.isPackageEnabled\(packageName: 'operit_editor'\);[\s\S]*if \(!editorEnabled\) \{[\s\S]*throw StateError\([\s\S]*success: true/u);
  assert.doesNotMatch(text, /\.contains\(/u);
  const dialog = source('apps/flutter/app/lib/ui/features/packages/dialogs/QuickPluginCreatorDialog.dart');
  assert.match(dialog, /发送草稿后才开始开发/u);
  assert.match(dialog, /不会自动生成、安装或发布插件/u);
  assert.match(dialog, /准备并前往聊天/u);
});

/** Type-checks the first-script example against the documented config and real bundled types. */
test('first-script TypeScript example passes the documented strict configuration', () => {
  const text = source(guideDirectory + 'SCRIPT_DEV_GUIDE.md');
  const configurations = Array.from(text.matchAll(/```json\n([\s\S]*?)\n```/gu));
  const config = JSON.parse(configurations[1][1]);
  const projectDirectory = path.join(root, 'plugins/__package_builder_type_contract__');
  const virtualFile = path.join(projectDirectory, 'src/my_new_script.ts');
  const chapter = text.slice(text.indexOf('### 步骤 3: 编写主体逻辑'));
  const example = /```typescript\n([\s\S]*?)\n```/u.exec(chapter)[1];
  const converted = ts.convertCompilerOptionsFromJson(config.compilerOptions, projectDirectory);
  assert.deepEqual(converted.errors, []);
  const options = { ...converted.options, noEmit: true };
  const host = ts.createCompilerHost(options);
  const originalGetSourceFile = host.getSourceFile.bind(host);
  /** Supplies only the in-memory example while loading all real declarations normally. */
  host.getSourceFile = (fileName, languageVersion, onError, shouldCreateNewSourceFile) => {
    if (path.resolve(fileName) === virtualFile) {
      return ts.createSourceFile(fileName, example, languageVersion, true);
    }
    return originalGetSourceFile(fileName, languageVersion, onError, shouldCreateNewSourceFile);
  };
  const program = ts.createProgram([virtualFile, path.join(root, 'plugins/types/index.d.ts')], options, host);
  const diagnostics = ts.getPreEmitDiagnostics(program);
  assert.deepEqual(diagnostics.map(diagnostic => ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n')), []);
});

/** Type-checks the actual executable editor against the bundled host SDK declarations. */
test('operit_editor passes strict TypeScript checking with the real SDK types', () => {
  const program = ts.createProgram([path.join(root, editorPath)], {
    target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS,
    strict: true, skipLibCheck: true, noEmit: true, types: [],
  });
  const diagnostics = ts.getPreEmitDiagnostics(program);
  assert.deepEqual(diagnostics.map(diagnostic => ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n')), []);
});

/** Keeps API support and platform scope explicit in every author-facing entry point. */
test('creation materials distinguish v2 portability from incomplete Android-oriented v1 compatibility', async () => {
  for (const file of documents) {
    const text = source(file);
    assert.match(text, /Operit2 的 ToolPkg API 支持版本为 `2\.0\.0`/u, file);
    assert.match(text, /对 `1\.0\.0` 和 `1\.0\.1` 的加载支持并不完整/u, file);
    assert.match(text, /Operit1 完整支持 ToolPkg API `1\.0\.0` 和 `1\.0\.1`/u, file);
    assert.match(text, /主要面向 Android，是旧版 API 形式/u, file);
    assert.match(text, /"api_version": "2\.0\.0"/u, file);
    assert.match(text, /基本所有公共接口.*跨平台兼容/u, file);
    assert.match(text, /平台特异接口.*其他平台/u, file);
  }
  for (const text of [source(promptPath)]) {
    assert.match(text, /显式声明 api_version 为 2\.0\.0/u);
    assert.match(text, /1\.0\.0 和 1\.0\.1 的加载支持并不完整/u);
    assert.match(text, /Operit1 完整支持 1\.0\.0 和 1\.0\.1/u);
    assert.match(text, /主要面向 Android，是旧版 API 形式/u);
    assert.match(text, /平台特异接口.*其他平台/u);
  }
  const manifest = /```json\n([\s\S]*?)\n```/u.exec(source(guideDirectory + 'TOOLPKG_FORMAT_GUIDE.md'));
  const parsed = JSON.parse(manifest[1]);
  assert.equal(parsed.schema_version, 1);
  assert.equal(parsed.api_version, '2.0.0');
  assert.notEqual(parsed.version, parsed.api_version);
  assert.match(source(guideDirectory + 'TOOLPKG_FORMAT_GUIDE.md'), /\| `api_version` \| string \|/u);
  assert.match(source('plugins/docs/V1_COMPATIBILITY.md'), /loading compatibility[\s\S]*is incomplete/u);
});
