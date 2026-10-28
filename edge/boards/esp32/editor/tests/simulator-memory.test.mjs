import {simulatorBuildArguments, simulatorExecutable} from '../src/simulator-build.mts';
import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const root = fileURLToPath(new URL('../../../../../', import.meta.url));
const executable = simulatorExecutable;

/** Runs probes in isolated real processes because a guard-page fault is fatal. */
test('simulator enforces the fixed heap and each configured task stack', {timeout: 180000}, () => {
  const python = fileURLToPath(new URL(process.platform === 'win32' ? '../../../../../.venv/Scripts/python.exe' : '../../../../../.venv/bin/python', import.meta.url));
  const prepare = spawnSync(python, ['-X', 'utf8', 'edge/boards/esp32/editor/prepare-simulator.py'],
    {cwd:root, encoding:'utf8', windowsHide:true, timeout:90000});
  assert.equal(prepare.status, 0, prepare.stderr);
  const build = spawnSync('cargo', simulatorBuildArguments, {cwd: root, encoding:'utf8', windowsHide:true, timeout:150000});
  assert.equal(build.status, 0, build.stderr?.slice(-6000));
  for (const mode of ['--probe-heap-fit', '--probe-heap-limit', '--probe-heap-fragmentation']) {
    const result = spawnSync(executable, [mode], {cwd:root, windowsHide:true, timeout:10000});
    assert.equal(result.error, undefined);
    assert.equal(result.status, 0, mode + ': ' + result.stderr);
  }
  for (const worker of ['main', 'async', 'blocking']) {
    const fit = spawnSync(executable, ['--probe-stack-fit', worker], {cwd:root, windowsHide:true, timeout:10000});
    assert.equal(fit.error, undefined);
    assert.equal(fit.status, 0, worker + ' within budget: ' + fit.stderr);
    const overflow = spawnSync(executable, ['--probe-stack-limit', worker], {cwd:root, windowsHide:true, timeout:10000});
    assert.equal(overflow.error, undefined);
    if (process.platform === 'win32') assert.equal(overflow.status, 0xc0000005, worker);
    else assert(['SIGSEGV', 'SIGBUS'].includes(overflow.signal), worker + ': ' + JSON.stringify(overflow));
  }
});
