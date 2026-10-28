import test from 'node:test';
import assert from 'node:assert/strict';
import {copySimulatorBinary} from '../src/api/simulator-api.mts';

const failure = code => Object.assign(new Error(code), {code});

test('simulator restart retries a transient executable lock and then succeeds', async () => {
  let attempts = 0;
  await copySimulatorBinary('build.exe', 'runtime.exe', async (source, destination) => {
    assert.equal(source, 'build.exe');
    assert.equal(destination, 'runtime.exe');
    if (++attempts < 3) throw failure('EBUSY');
  });
  assert.equal(attempts, 3);
});

test('simulator copy propagates non-sharing errors without retrying', async () => {
  let attempts = 0;
  const error = failure('ENOENT');
  await assert.rejects(copySimulatorBinary('missing.exe', 'runtime.exe', async () => {
    attempts += 1; throw error;
  }), value => value === error);
  assert.equal(attempts, 1);
});

test('persistent executable locks stop retrying and preserve the actual error', async () => {
  let attempts = 0;
  const error = failure('EPERM');
  await assert.rejects(copySimulatorBinary('build.exe', 'runtime.exe', async () => {
    attempts += 1; throw error;
  }), value => value === error);
  assert.equal(attempts, 10);
});
