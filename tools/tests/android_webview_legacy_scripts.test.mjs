import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const native = readFileSync(new URL(
  '../../apps/flutter/thirdparty/webview_flutter_android/android/src/main/java/io/flutter/plugins/webviewflutter/WebViewUserScripts.java',
  import.meta.url,
), 'utf8');

/** Evaluates the production Java string expression to obtain the injected JavaScript. */
function guarded(identifier, source) {
  const method = native.match(/private static String guardScript\(String identifier, String source\) \{\s*return ([\s\S]*?);\r?\n  \}/);
  assert.ok(method, 'production lifecycle script must be available');
  return vm.runInNewContext(method[1], { identifier, source });
}

/** Creates a page with a distinct document and a retained JavaScript window. */
function page() {
  const context = vm.createContext({ document: {}, calls: [] });
  context.window = context;
  return context;
}

test('navigation callbacks execute each registered script once per document', () => {
  const context = page();
  const first = guarded('operit-user-script-1', 'window.calls.push("first");');
  const second = guarded('operit-user-script-2', 'window.calls.push("second");');
  for (let callback = 0; callback < 3; callback += 1) {
    vm.runInContext(first, context);
    vm.runInContext(second, context);
  }
  assert.deepEqual([...context.calls], ['first', 'second']);
  context.document = {};
  vm.runInContext(first, context);
  vm.runInContext(second, context);
  assert.deepEqual([...context.calls], ['first', 'second', 'first', 'second']);
});

test('script failures cannot cause repeated side effects or block later registrations', () => {
  const context = page();
  const broken = guarded('operit-user-script-1', 'window.calls.push("before error"); throw new Error("broken");');
  const next = guarded('operit-user-script-2', 'window.calls.push("healthy");');
  assert.throws(() => vm.runInContext(broken, context), /broken/);
  vm.runInContext(next, context);
  vm.runInContext(broken, context);
  vm.runInContext(next, context);
  assert.deepEqual([...context.calls], ['before error', 'healthy']);
});

test('source trailing comments and early returns cannot corrupt the injection wrapper', () => {
  const context = page();
  vm.runInContext(guarded('operit-user-script-1', 'window.calls.push("comment"); // trailing comment'), context);
  vm.runInContext(guarded('operit-user-script-2', 'window.calls.push("return"); return;'), context);
  assert.deepEqual([...context.calls], ['comment', 'return']);
});
