import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const android = 'apps/flutter/app/android/app/';
const source = path => readFileSync(new URL(path, root), 'utf8');
const channel = source(`${android}src/main/kotlin/app/operit/DocumentInputChannel.kt`);

/** Images must use a distinct method, not just a MIME-filtered document picker. */
test('Android image selection uses the multi-image photo picker contract', () => {
  const dart = source('apps/flutter/app/lib/core/host/SelectedFileInput.dart');
  assert.match(dart, /imagesOnly \? 'pickImages' : 'pickFiles'/);
  assert.match(channel, /"pickImages" -> pickDocuments\(result, true, false, listOf\("image\/\*"\), imagesOnly = true\)/);
  const imageBranch = channel.slice(
    channel.indexOf('val intent = if (imagesOnly)'),
    channel.indexOf('} else {', channel.indexOf('val intent = if (imagesOnly)')),
  );
  // AndroidX handles system/OEM/backported pickers and document fallback, without
  // hard-coding an Android version check that would miss backported pickers.
  assert.match(imageBranch, /ActivityResultContracts\.PickMultipleVisualMedia\(\)\.createIntent\(/);
  assert.match(imageBranch, /\.setMediaType\(ActivityResultContracts\.PickVisualMedia\.ImageOnly\)/);
  assert.doesNotMatch(imageBranch, /Intent\(Intent\.ACTION_OPEN_DOCUMENT\)|SDK_INT|CATEGORY_OPENABLE/);
  assert.match(source(`${android}build.gradle.kts`), /implementation\("androidx\.activity:activity:[^"]+"\)/);
});

/** Ordinary files and snapshot archives must retain their document selection behavior. */
test('Android ordinary files and snapshots keep the document picker', () => {
  assert.match(channel, /"pickFiles" -> pickDocuments\(result, true, false, call\.argument<List<String>>\("mimeTypes"\) \?: emptyList\(\)\)/);
  assert.match(channel, /"pick" -> pickDocuments\(result, false, true, listOf\(/);
  assert.match(channel, /imagesOnly: Boolean = false/);
  assert.match(channel, /} else \{\s*Intent\(Intent\.ACTION_OPEN_DOCUMENT\)\.apply \{/);
  assert.match(channel, /putExtra\(Intent\.EXTRA_ALLOW_MULTIPLE, multiple\)/);
});

/** Both pickers reuse cancellation, URI metadata and lazy, bounded input reads. */
test('Android photo results reuse the bounded document input pipeline', () => {
  assert.match(channel, /try \{\s*val intent = if \(imagesOnly\)/);
  assert.match(channel, /activity\.startActivityForResult\(intent, PICK_DOCUMENT_REQUEST_CODE\)/);
  assert.match(channel, /catch \(error: Exception\) \{\s*pendingPick = null\s*result\.error\("PICK_FAILED"/);
  assert.match(channel, /if \(resultCode != Activity\.RESULT_OK\) \{\s*pick\.result\.success\(if \(pick\.multiple\) emptyList<Map<String, Any\?>>\(\) else null\)/);
  assert.match(channel, /data\?\.data\?\.let\(uris::add\)/);
  assert.match(channel, /uris\.add\(clip\.getItemAt\(i\)\.uri\)/);
  assert.match(channel, /openInputs\[token\] = OpenDocumentInput\(uri\)/);
  assert.match(channel, /MAX_CHUNK_SIZE = 1024 \* 1024/);
  const selection = channel.slice(channel.indexOf('fun onActivityResult('), channel.indexOf('private fun readChunk('));
  assert.doesNotMatch(selection, /openInputStream|readBytes|readAllBytes/);
  assert.match(channel, /activity\.contentResolver\.openInputStream\(input\.uri\)/);
});
