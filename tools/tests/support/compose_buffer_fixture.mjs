import assert from 'node:assert/strict';

/** Reads the production arena only for assertions outside the measured producer. */
export function readComposeBuffer(bytes) {
  assert.ok(ArrayBuffer.isView(bytes), 'Compose must submit a binary frame');
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  assert.equal(view.getUint32(0, true), 0x31425043);
  assert.equal(view.getUint32(4, true), 1);
  assert.equal(view.getUint32(8, true), bytes.byteLength);
  const strings = new Map();
  /** Reads an explicitly requested value while checking its descriptor boundaries. */
  function read(offset, depth = 0) {
    assert.ok(offset >= 32 && offset % 8 === 0 && offset + 16 <= bytes.byteLength && depth <= 128);
    const tag = view.getUint32(offset, true), count = view.getUint32(offset + 4, true), data = view.getUint32(offset + 8, true);
    switch (tag) {
      case 0: return null;
      case 1: return false;
      case 2: return true;
      case 3: return view.getFloat64(offset + 8, true);
      case 4: {
        if (strings.has(offset)) return strings.get(offset);
        assert.ok(data + count * 2 <= bytes.byteLength);
        let result = '';
        for (let index = 0; index < count; index++) result += String.fromCharCode(view.getUint16(data + index * 2, true));
        strings.set(offset, result); return result;
      }
      case 5: return Array.from({length:count}, (_, index) => read(view.getUint32(data + index * 4, true), depth + 1));
      case 6: return Object.fromEntries(Array.from({length:count}, (_, index) => [read(view.getUint32(data + index * 8, true), depth + 1), read(view.getUint32(data + index * 8 + 4, true), depth + 1)]));
      case 7: return {__composeUndefined:true};
      case 8: return {__actionId:read(data, depth + 1)};
      case 9: return {__composeRef:[read(data, depth + 1),count]};
      default: throw Error('Unknown Compose value tag');
    }
  }
  const response = {navigationCommands:read(view.getUint32(24, true))};
  const transaction = view.getUint32(16, true), result = view.getUint32(20, true);
  if (transaction) {
    const [reset, revision, rootId, node, programs, sources, removed] = read(transaction);
    response.update = {reset, revision, root:{instanceId:rootId,node},programs,instances:sources.map(([id,programId,values,patches]) => ({id,programId,values,patches:patches.map(([node,field,value]) => ({node,field,value}))})),removed};
  }
  if (result) response.actionResult = read(result);
  return {phase:view.getUint32(12, true) ? 'final' : 'intermediate',response};
}
