import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = path => readFileSync(new URL(`../../${path}`, import.meta.url), 'utf8');
const android = 'apps/flutter/app/android/app/src/main/kotlin/app/operit/';

test('Android file presentation is routed to the owner and retains URI read grants', () => {
  const runtime = source('apps/flutter/native/operit-flutter-bridge/src/platform_runtime/android.rs');
  const channel = source(android + 'OwnerSystemCapabilityChannel.kt');
  const documents = source(android + 'AndroidDocumentFileSystem.kt');
  const opener = source(android + 'AndroidFileOpener.kt');
  assert.match(runtime, /AndroidFileSystemHost::withPresentation/);
  assert.match(runtime, /"android_file_presentation"/);
  assert.match(channel, /"android_file_presentation"[\s\S]*fileSystemResourceOperation/);
  assert.match(documents, /"openFile", "shareFile"/);
  assert.match(documents, /target\.input\(\)\.use/);
  assert.match(opener, /Intent\.ACTION_SEND/);
  assert.match(opener, /Intent\.EXTRA_STREAM/);
  assert.match(opener, /FLAG_GRANT_READ_URI_PERMISSION/);
});

test('privileged Android runtime forwards local inference to its owner', () => {
  const runtime = source('apps/flutter/native/operit-flutter-bridge/src/platform_runtime/android.rs');
  const channel = source(android + 'OwnerSystemCapabilityChannel.kt');
  assert.match(runtime, /if module\.is_some\(\)[\s\S]*withLocalInferenceHost\(Arc::new\(FlutterLocalInferenceHost\)\)/);
  for (const method of ['ownerLocalInference', 'transcribeLocalSpeech', 'synthesizeLocalSpeech']) {
    assert.ok(channel.includes(`"${method}"`));
  }
  assert.match(channel, /"resultJson" to response/);
});
