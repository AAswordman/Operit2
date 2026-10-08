import test from 'node:test';
import assert from 'node:assert/strict';
import {performDeviceAction} from '../web/device-actions.ts';

function fixture() {
  const diagnostics = [];
  return {diagnostics, ui: {ccall(name, result, types, values) {
    assert.equal(name, 'operit_ui_action_error');
    assert.equal(result, null);
    assert.deepEqual(types, ['string']);
    diagnostics.push(values[0]);
  }}};
}

for (const action of ['edge_space_approve', 'edge_space_reject']) {
  test(`${action} uses exactly the displayed request and assignment version`, async () => {
    const {ui, diagnostics} = fixture();
    const review = {requestId: 'displayed-A', assignmentVersion: 3};
    let received;
    await performDeviceAction(ui, action, async request => {
      // A later state poll must not change the already captured action payload.
      review.requestId = 'new-B'; review.assignmentVersion = 4;
      received = request;
    }, review);
    assert.deepEqual(received, {action, requestId: 'displayed-A', assignmentVersion: 3});
    assert.deepEqual(diagnostics, ['']);
  });
  test(`${action} rejects missing or invalid displayed identities without invoking RPC`, async () => {
    for (const review of [undefined, {}, {requestId: ' '}, {requestId: 'A', assignmentVersion: -1},
      {requestId: 'A', assignmentVersion: 1.5}, {requestId: 'A', assignmentVersion: Number.MAX_SAFE_INTEGER + 1}]) {
      const {ui, diagnostics} = fixture();
      await assert.rejects(performDeviceAction(ui, action, async () => assert.fail('must not retarget a review'), review), /请刷新/);
      assert.equal(diagnostics[0], '');
      assert.match(diagnostics[1], /请刷新/);
    }
  });
}

test('failed action displays its real error and a successful retry clears the old overlay', async () => {
  const {ui, diagnostics} = fixture();
  const error = new Error('Join request is no longer awaiting this decision');
  const review = {requestId: 'cancelled-A', assignmentVersion: 0};
  await assert.rejects(performDeviceAction(ui, 'edge_space_approve', async () => { throw error; }, review), value => value === error);
  await performDeviceAction(ui, 'edge_space_leave', async request => assert.deepEqual(request, {action: 'edge_space_leave'}));
  assert.deepEqual(diagnostics, ['', error.message, '']);
});
