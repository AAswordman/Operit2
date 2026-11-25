import test from 'node:test';
import assert from 'node:assert/strict';
import {firmwareStatusLabel, simulatorConnectionLabel, simulatorStatusLabel, simulatorViewState} from '../web/device-state.ts';

test('simulator status distinguishes pairing, approval and chat readiness', () => {
  assert.equal(simulatorConnectionLabel(null), '等待配对');
  assert.equal(simulatorConnectionLabel({paired: true}), '已配对，等待 Core 对话');
  assert.equal(simulatorConnectionLabel({pairingCode: '123456', paired: true}), '等待在 Core 输入配对码');
  assert.equal(simulatorConnectionLabel({paired: true, spaceJoinPrompt: '审批申请'}), '等待设备空间审批');
  assert.equal(simulatorConnectionLabel({paired: true, chat: {connected: true}}), 'Core 对话已就绪');
  assert.equal(simulatorConnectionLabel({spaceJoinPrompt: '新的审批申请', chat: {connected: true}}), '等待设备空间审批');
  assert.equal(simulatorConnectionLabel({pairingCode: '123456', chat: {connected: true}}), '等待在 Core 输入配对码');
});

test('starting or stopped process does not display an old connected session', () => {
  const device = {paired: true, chat: {connected: true, chatId: 'old-chat'},
    scene: {active: true, revision: 10}, spaceJoinPrompt: 'old-request', pairingCode: '123456'};
  assert.equal(simulatorStatusLabel({running: true, ready: false, device}), '正在启动模拟设备…');
  assert.equal(simulatorStatusLabel({running: false, ready: false, device}), '模拟设备已停止');
  const view = simulatorViewState({ready: false, device});
  assert.equal(view.connected, false);
  assert.equal(view.paired, false);
  assert.equal(view.chat, undefined);
  assert.equal(view.scene, undefined);
  assert.equal(view.spaceJoinPrompt, '');
  assert.equal(view.pairingCode, '');
});

test('active device projection preserves approval busy state and reports its real error', () => {
  const device = {paired: true, spaceJoinPrompt: '审批申请', spaceJoinBusy: true,
    spaceJoinRequestId: 'current-request', spaceJoinAssignmentVersion: 7, error: 'Core 不可达'};
  const view = simulatorViewState({ready: true, device});
  assert.equal(view.spaceJoinBusy, true);
  assert.equal(view.spaceJoinRequestId, 'current-request');
  assert.equal(view.spaceJoinAssignmentVersion, 7);
  assert.equal(simulatorStatusLabel({ready: true, running: true, device}), '等待设备空间审批 · Core 不可达');
});

test('firmware checks cannot be confused with desktop simulation', () => {
  assert.match(firmwareStatusLabel({status: 'not-checked'}), /未验证固件/);
  assert.match(firmwareStatusLabel({status: 'checking'}), /真实固件/);
  assert.match(firmwareStatusLabel({status: 'failed', imageBytes: 3998208, partitionBytes: 3997696, overflowBytes: 512}), /超出 512 字节/);
  assert.match(simulatorStatusLabel({running: true, firmwareCheck: {status: 'checking'}}), /校验 ESP32 固件/);
  assert.match(simulatorStatusLabel({ready: false, firmwareCheck: {status: 'failed', error: '镜像超出分区'}}), /镜像超出分区/);
  assert.match(firmwareStatusLabel({status: 'passed', imageBytes: 1000, partitionBytes: 2000}), /1000 \/ 2000/);
});
