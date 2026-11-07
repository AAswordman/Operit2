import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
import test from 'node:test';

const source = readFileSync(new URL('../../apps/flutter/app/ios/Runner/AppleRuntimeChannel.swift', import.meta.url), 'utf8');

test('iOS runtime channel retains startup, storage and FFI dispatch', () => {
  for (const method of ['dataRescue', 'connectCoreFfi', 'localRuntimeStorageDefaults', 'restartApplication']) {
    assert.match(source, new RegExp(`case "${method}":`), `${method} is missing`);
  }
  assert.match(source, /command\?\["operation"\] as\? String == "launchMode"/);
  assert.match(source, /operit_flutter_bridge_data_rescue\(root, request\)/);
  assert.match(source, /operit_flutter_bridge_ffi_connect\(handle\)/);
});
