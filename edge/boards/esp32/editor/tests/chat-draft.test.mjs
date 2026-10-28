import test from 'node:test';
import assert from 'node:assert/strict';
import {draftPreview,checkedDraft} from '../web/chat-draft.ts';

test('a display preview is bounded UTF-8 while the original draft stays complete',()=>{
  const text='完整长文本🙂'.repeat(600)+'末尾标记';
  assert.equal(checkedDraft(text),text);
  const preview=draftPreview(text);
  assert(Buffer.byteLength(preview)<=511);
  assert(!preview.includes('�'));
  assert(text.startsWith(preview));
  assert(text.endsWith('末尾标记'));
  assert.throws(()=>checkedDraft('字'.repeat(6000)),/16384/);
});
