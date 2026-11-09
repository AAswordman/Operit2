// The launcher prepends the production character-memory VectorIndex adapter.
(function() {
  'use strict';
  var checks = 0;
  function check(condition, label) {
    checks++;
    if (!condition) throw new Error(label);
  }
  function throws(run, kind, label) {
    try { run(); } catch (error) { check(error instanceof kind, label + ': ' + error); return; }
    throw new Error(label + ': expected an exception');
  }
  function scalarsAndReferences() {
    check(typeof WebAssembly === 'object', 'WASM is exposed in the jitless engine');
    var scalarBytes = __operitWasmFixture('scalar').bytes;
    check(WebAssembly.validate(scalarBytes), 'scalar fixture validates');
    var scalar = new WebAssembly.Instance(new WebAssembly.Module(scalarBytes), {
      env: {
        identity: function(value) { if (value && value.fail) throw new Error('import failure'); return value; },
        multi: function(value) { return [value, 73]; }
      }
    }).exports;
    check(scalar.store(109) === 109, 'i32 scalar result');
    check(new DataView(scalar.memory.buffer).getInt32(0, true) === 109, 'retained memory state');
    check(scalar.i64(-9007199254740993n) === -9007199254740993n, 'lossless negative i64');
    check(scalar.i64(9223372036854775807n) === 9223372036854775807n, 'i64 maximum');
    check(scalar.f32(3) === 1.5 && scalar.f64(3) === 3.25, 'floating point scalars');
    check(scalar.indirect(21) === 42, 'indirect function call');
    check(scalar.simd(2147483647) === 2147483647, 'SIMD lane round trip');
    throws(function() { scalar.trap(0); }, WebAssembly.RuntimeError, 'unreachable traps');
    throws(function() { scalar.identity({ fail: true }); }, Error, 'JS import exception');
    check(scalar.storeAt(65532) === 1 && scalar.load(65532) === 0x12345678, 'last complete memory word is accessible');
    check(scalar.loadSimd(0) === 109, 'SIMD memory load');
    for (var address of [-1, 65533, 65536, 2147483647]) {
      throws(function() { scalar.load(address); }, WebAssembly.RuntimeError, 'native memory load bounds');
      throws(function() { scalar.storeAt(address); }, WebAssembly.RuntimeError, 'native memory store bounds');
    }
    throws(function() { scalar.loadSimd(65521); }, WebAssembly.RuntimeError, 'native SIMD memory bounds');
    check(scalar.load(65532) === 0x12345678, 'trapped store does not partially overwrite memory');
    var oldBuffer = scalar.memory.buffer;
    check(scalar.grow(1) === 1, 'memory growth returns prior page count');
    check(oldBuffer.byteLength === 0 && scalar.memory.buffer.byteLength === 131072, 'grown buffer replaces and detaches old memory');
    check(new DataView(scalar.memory.buffer).getInt32(0, true) === 109, 'growth preserves contents');
    check(scalar.grow(3) === -1, 'memory growth respects maximum');
    var values = [undefined, null, 17, -21, {}, [], '中文reference', 9007199254740993n, Symbol('reference')];
    for (var value of values) {
      check(scalar.identity(value) === value, 'imported externref identity');
      var pair = scalar.multi(value);
      check(pair[0] === value && pair[1] === 73, 'mixed imported multi-value reference result');
    }
    var references = new WebAssembly.Instance(new WebAssembly.Module(__operitWasmFixture('references').bytes)).exports;
    var structure = references.newStruct(19);
    var array = references.newArray();
    check(references.getStruct(structure) === 19, 'Smi struct field');
    gc(); gc();
    for (var index = 0; index < 80; index++) {
      var young = { index: index, payload: new Array(20).fill('young object') };
      references.setStruct(structure, young);
      references.fillArray(array, young);
      gc();
      check(references.getStruct(structure) === young, 'uncompressed struct field and GC barrier');
      for (var item = 0; item < 4; item++) check(references.getArray(array, item) === young, 'reference array fill and GC barrier');
    }
    for (var replacement of [null, 0, -1, 'string field']) {
      references.fillArray(array, replacement);
      references.setStruct(structure, replacement);
      check(references.getArray(array, 3) === replacement && references.getStruct(structure) === replacement, 'full-width tagged reference replacement');
    }
    throws(function() { references.getArray(array, 4); }, WebAssembly.RuntimeError, 'reference array bounds');
  }
  async function memoryPackage() {
    globalThis.ToolPkg = { wasm: __operitCreateToolPkgWasmApi(function() { return 'character_cards'; }, false) };
    var raw = await ToolPkg.wasm.open('usearch', {
      env: { emscripten_notify_memory_growth: function() {} },
      wasi_snapshot_preview1: { clock_time_get: function() { return 0; }, fd_close: function() { return 8; } }
    });
    raw.call('_initialize');
    function i32(value) { return { type: 'i32', value: value }; }
    var pointer = raw.call('malloc', [i32(16)]);
    raw.writeMemory(pointer, new Uint8Array([1, 3, 5, 7]));
    var priorMemory = raw.memory();
    var large = raw.call('malloc', [i32(priorMemory.byteLength + 65536)]);
    check(large > 0 && raw.memory().byteLength > priorMemory.byteLength, 'real module memory growth');
    check(raw.readMemory(pointer, 4).join(',') === '1,3,5,7', 'ToolPkg memory survives growth');
    throws(function() { raw.call('not_declared', []); }, Error, 'declared export allowlist');
    throws(function() { raw.readMemory(-1, 2); }, RangeError, 'memory range validation');
    raw.call('free', [i32(large)]);
    raw.call('free', [i32(pointer)]);
    raw.close();
    throws(function() { raw.memory(); }, Error, 'closed instance cannot access memory');
    var index = await VectorIndex.open(4);
    var first = 9007199254740993n;
    var second = 9007199254740995n;
    index.add(first, [1, 0, 0, 0]);
    index.add(second, [0, 1, 0, 0]);
    index.add(3n, [0.9, 0.1, 0, 0]);
    check(index.search([1, 0, 0, 0], 3)[0].key === first, 'real USearch recall and 64-bit keys');
    check(index.search([1, 0, 0, 0], 3, [second])[0].key === second, 'memory library allow filter');
    check(index.search([1, 0, 0, 0], 3, undefined, [first]).every(function(item) { return item.key !== first; }), 'memory exclusion filter');
    var scores = index.score([1, 0, 0, 0], [first, second]);
    check(Math.abs(scores[0] - 1) < 0.001 && Math.abs(scores[1]) < 0.001, 'memory scoring');
    index.remove(first);
    check(index.search([1, 0, 0, 0], 3).every(function(item) { return item.key !== first; }), 'memory deletion updates index');
    index.close(); index.close();
    throws(function() { index.add(4n, [1, 0, 0, 0]); }, Error, 'closed index rejects writes');
    ToolPkg.wasm.unload('usearch');
    return ['scalar-ABI', 'JS-imports', 'multi-value', 'memory-growth', 'indirect-calls', 'GC-structs', 'GC-array-fill', 'ToolPkg-WASM', 'USearch-memory-index'];
  }
  globalThis.__wasmSmokeResult = { status: 'pending' };
  Promise.resolve().then(function() { scalarsAndReferences(); return memoryPackage(); }).then(function(features) {
    globalThis.__wasmSmokeResult = { status: 'passed', checks: checks, features: features };
  }, function(error) {
    globalThis.__wasmSmokeResult = { status: 'failed', error: String(error.stack || error) };
  });
})();
