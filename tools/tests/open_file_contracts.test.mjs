import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
const source = path => readFileSync(new URL('../../' + path, import.meta.url), 'utf8');
const android = 'apps/flutter/app/android/app/src/main/';

test('Android open_file reaches the JNI owner for native paths and document capabilities', () => {
  const rust = source('hosts/android/src/filesystem.rs');
  const open = rust.slice(rust.indexOf('fn openFile'), rust.indexOf('fn shareFile'));
  assert.match(open, /validatePath/);
  assert.match(open, /document_filesystem::call/);
  assert.match(open, /"operation": "openFile"/);
  assert.doesNotMatch(open, /requires the Flutter/);
  const backend = source(android + 'kotlin/app/operit/AndroidDocumentFileSystem.kt');
  assert.match(backend, /"openFile" ->/);
  assert.match(backend, /target\.input\(\)\.use/);
  assert.match(backend, /opener\(target\.file, target\.tree,.*target\.documentId\(\)/);
});

test('Android shares only a staged file or the granted SAF document, never file://', () => {
  const owner = source(android + 'kotlin/app/operit/AndroidFileOpener.kt');
  for (const api of ['FileProvider.getUriForFile', 'buildDocumentUriUsingTree', 'FLAG_GRANT_READ_URI_PERMISSION', 'ClipData.newRawUri', 'Intent.ACTION_VIEW', 'contentResolver.getType', 'context.startActivity(intent)'])
    assert.ok(owner.includes(api), api);
  assert.match(owner, /class OperitOpenFileProvider : FileProvider\(\)/);
  assert.doesNotMatch(owner, /Uri\.fromFile|FLAG_GRANT_WRITE_URI_PERMISSION|resolveActivity/);
  const paths = source(android + 'res/xml/open_file_paths.xml');
  assert.match(paths, /cache-path.*path="open_files\/"/);
  assert.doesNotMatch(paths, /root-path|external-path|path="\."/);
  const manifest = source(android + 'AndroidManifest.xml');
  assert.match(manifest, /OperitOpenFileProvider"[\s\S]*?android:exported="false"[\s\S]*?grantUriPermissions="true"/);
});

test('iOS uses retained UIKit presentation through the owner bridge rather than spawning open', () => {
  const assembly = source('apps/flutter/native/operit-flutter-bridge/src/platform_runtime/ios.rs');
  assert.match(assembly, /IosFileSystemHost::fromFileOpener[\s\S]*?ownerFileOpen/);
  const swift = source('apps/flutter/app/ios/Runner/AppleRuntimeChannel.swift');
  for (const api of ['case "ownerFileOpen"', 'UIDocumentInteractionControllerDelegate', 'fileController = controller', 'presentPreview(animated: true)', 'presentOpenInMenu(', 'isReadableFile', 'foregroundActive'])
    assert.ok(swift.includes(api), api);
  const rust = source('hosts/apple/src/tools/fs/mod.rs');
  const open = rust.slice(rust.indexOf('fn openFile'), rust.indexOf('fn shareFile'));
  assert.match(open, /return opener\(path\)/);
  assert.match(open, /#\[cfg\(target_os = "macos"\)\][\s\S]*?Command::new\("open"\)/);
});

test('Web reads bytes in its worker, then routes presentation through the UI module', () => {
  const bridge = source('apps/flutter/app/web/runtime/src/operit_runtime_bridge.ts');
  assert.match(bridge, /filePresentation: registerMainHostModule\([\s\S]*?openFile: openBrowserFile/);
  assert.match(bridge, /openFile\(path: string\): void[\s\S]*?storageHasFile[\s\S]*?host\.filePresentation\.openFile\(path, storageRead\(filePrefix, path\)\)/);
  assert.doesNotMatch(bridge, /openFile\(\) \{\}/);
  assert.ok(source('apps/flutter/app/hook/build.dart').includes("'browser_file_open.js'"));
});

test('desktop openers use literal paths and OpenHarmony retains its ability owner', () => {
  const windows = source('hosts/windows/src/tools/fs/mod.rs');
  const open = windows.slice(windows.indexOf('fn openFile'), windows.indexOf('fn shareFile'));
  assert.match(open, /ShellExecuteW/);
  assert.doesNotMatch(open, /Command::new\("cmd"\)/);
  assert.match(source('hosts/linux/src/tools/fs/mod.rs'), /Command::new\("xdg-open"\)\.arg\(path\)/);
  assert.match(source('apps/flutter/native/operit-flutter-bridge/src/platform_runtime/ohos.rs'), /Arc::new\(ownerFileOpen\)/);
  assert.match(source('apps/flutter/app/ohos/entry/src/main/ets/entryability/OperitRuntimeChannel.ets'), /private async ownerFileOpen[\s\S]*?ACTION_VIEW_DATA[\s\S]*?FLAG_AUTH_READ_URI_PERMISSION/);
});


test('opening a file is a presentation side effect, not a write to its read-only mount', () => {
  const executor = source('core/crates/tool/services/src/tools/defaultTool/standard/StandardFileSystemTools.rs');
  assert.match(executor, /FileSystemToolOperation::OpenFile => ToolAccessSpec \{\s*effect: ToolEffect::WRITE,\s*boundary: ToolBoundary::FilePath \{\s*effect: ToolEffect::READ/);
});
