import test from 'node:test';
import assert from 'node:assert/strict';
import {appPartitionBytes, firmwareCheckFromOutput, firmwareReportPrefix} from '../src/firmware-check.mts';

const passed = {status: 'passed', compilePassed: true, abiPassed: true, partitionPassed: true,
  imageBytes: 3997184, partitionBytes: 3997696, overflowBytes: 0, elfSha256: 'a'.repeat(64)};
const output = report => 'compiler output\n' + firmwareReportPrefix + JSON.stringify(report) + '\n';

test('Flash capacity comes from the board table, including unit suffixes', () => {
  for (const [size, expected] of [['0x3D0000', 3997696], ['3840K', 3932160], ['3M', 3145728]]) {
    assert.equal(appPartitionBytes(`# comment\nnvs,data,nvs,0x9000,0x6000,\nfactory,app,factory,0x10000,${size},`), expected);
  }
  for (const size of ['0', '-1', 'NaN', '3.5M', '0xgarbage']) {
    assert.throws(() => appPartitionBytes(`factory,app,factory,0x10000,${size},`));
  }
  assert.throws(() => appPartitionBytes('nvs,data,nvs,0x9000,0x6000,'));
});

test('a valid report requires compilation, ABI, image generation and successful process exit', () => {
  assert.deepEqual(firmwareCheckFromOutput(output(passed), 0), passed);
  for (const changes of [{compilePassed: false}, {abiPassed: false}, {partitionPassed: false},
    {imageBytes: 3998208, overflowBytes: 512}, {imageBytes: 0}, {imageBytes: 1.5}, {elfSha256: ''}]) {
    assert.equal(firmwareCheckFromOutput(output({...passed, ...changes}), 0).status, 'failed');
  }
  assert.equal(firmwareCheckFromOutput(output(passed), 1).status, 'failed');
  assert.equal(firmwareCheckFromOutput(output(passed), null).status, 'failed');
});

test('oversized and truncated results cannot reuse an older success', () => {
  const failure = {status: 'failed', stage: 'partition', compilePassed: true, abiPassed: true,
    imageBytes: 3998208, partitionBytes: 3997696, overflowBytes: 512, error: '固件超出应用分区 512 字节'};
  assert.deepEqual(firmwareCheckFromOutput(output(passed) + output(failure), 1), failure);
  for (const data of ['', 'an old image exists', firmwareReportPrefix + '{', firmwareReportPrefix + 'null']) {
    assert.equal(firmwareCheckFromOutput(data, 0).status, 'failed');
  }
});
