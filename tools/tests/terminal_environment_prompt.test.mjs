import assert from 'node:assert/strict';
import test from 'node:test';
import vm from 'node:vm';
import { loadSuperAdmin } from './fixtures/super_admin_runtime.mjs';

test('terminal context follows the current Host and selected profiles without exposing scripts or device identifiers', async () => {
  let platform = 'windows', calls = 0;
  const context = vm.createContext({ Tools: { System: { terminal: {
    async info() {
      calls++;
      return { platform, terminal: 'powershell', terminalType: 'powershell', defaultProfileId: 'ps7',
        profiles: [{ id: 'ps7', name: 'PowerShell 7', terminal: 'powershell', terminalType: 'powershell', available: true, executable: 'PRIVATE PATH' }] };
    },
  } } } });
  const { main } = loadSuperAdmin(context);
  const event = { eventPayload: { stage: 'before_send_to_model', promptFunctionType: 'CHAT',
    metadata: { workspacePath: '/operitapp/workspaces/project' },
    preparedHistory: [{ kind: 'SYSTEM', content: 'Host' }, { kind: 'USER', content: 'Task' }] } };
  const first = await main.onEnvironmentPrompt(event);
  assert.match(first.preparedHistory[1].content, /windows[\s\S]*PowerShell 7/);
  assert.doesNotMatch(first.preparedHistory[1].content, /PRIVATE PATH/);
  assert.equal(calls, 1);
  platform = 'linux';
  event.eventPayload.preparedHistory = first.preparedHistory;
  const second = await main.onEnvironmentPrompt(event);
  assert.equal(second.preparedHistory.length, 3);
  assert.match(second.preparedHistory[1].content, /linux/);
  assert.doesNotMatch(second.preparedHistory[1].content, /windows/);
  event.eventPayload.promptFunctionType = 'MEMORY';
  assert.equal(await main.onEnvironmentPrompt(event), null);
  assert.equal(calls, 2);
});
