import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const android = 'apps/flutter/app/android/app/';
const source = path => readFileSync(new URL(path, root), 'utf8');
const channel = source(`${android}src/main/kotlin/app/operit/DocumentInputChannel.kt`);

/** Image selection must let users choose a provider without a DocumentsUI-only route. */
test('Android image selection offers image providers through a content chooser', () => {
  const dart = source('apps/flutter/thirdparty/operit_folder_access/lib/operit_folder_access.dart');
  assert.match(dart, /imagesOnly \? 'pickImages' : 'pickFiles'/);
  assert.match(channel, /"pickImages" -> pickDocuments\(result, true, false, listOf\("image\/\*"\), imagesOnly = true\)/);
  const imageBranch = channel.slice(
    channel.indexOf('val intent = if (imagesOnly)'),
    channel.indexOf('} else {', channel.indexOf('val intent = if (imagesOnly)')),
  );
  assert.match(imageBranch, /Intent\.createChooser\(Intent\(Intent\.ACTION_GET_CONTENT\)\.apply \{/);
  assert.match(imageBranch, /addCategory\(Intent\.CATEGORY_OPENABLE\)/);
  assert.match(imageBranch, /addFlags\(Intent\.FLAG_GRANT_READ_URI_PERMISSION\)/);
  assert.match(imageBranch, /type = "image\/\*"/);
  assert.match(imageBranch, /putExtra\(Intent\.EXTRA_ALLOW_MULTIPLE, multiple\)/);
  assert.match(imageBranch, /\}, null\)/);
  assert.doesNotMatch(imageBranch, /ACTION_OPEN_DOCUMENT|PickMultipleVisualMedia|setPackage|SDK_INT|FLAG_GRANT_PERSISTABLE_URI_PERMISSION/);
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
