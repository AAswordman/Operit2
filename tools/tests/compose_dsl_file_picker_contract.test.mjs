import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import { composeContextScript } from '../runtime/compose_scripts.mjs';

const contract = JSON.parse(readFileSync(new URL('../../core/crates/foundation/host-api/tests/fixtures/compose_file_picker.json', import.meta.url), 'utf8'));

/** Verifies the production SDK emits the payload consumed by Rust. */
for (const entry of contract.valid) {
  test('SDK file-picker protocol: ' + entry.name, async () => {
    const requests = [];
    const selected = { cancelled: false, files: [{ size: 3, contentBase64: 'AAH/' }] };
    const context = vm.createContext({
      /** Captures the structured native request after SDK serialization. */
      __operitNativeComposeFilePickerCommand(payload) { requests.push(JSON.parse(JSON.stringify(payload))); return selected; },
      /** Preserves asynchronous host delivery through the production SDK call. */
      async __operitInvokeHostAsync(command, args) { return command(...args); },
    });
    vm.runInContext(composeContextScript(), context);
    const runtime = context.OperitComposeDslRuntime.createContext(entry.request);
    assert.equal(await runtime.ctx.openFilePicker(entry.request.options), selected);
    assert.deepEqual(requests, [entry.request]);
  });
}