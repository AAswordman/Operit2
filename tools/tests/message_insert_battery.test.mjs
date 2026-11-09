import assert from 'node:assert/strict';
import test from 'node:test';
import { loadModule } from '../../plugins/packages/buildin/character_cards/tests/runtime.mjs';

function plugin(info) {
  return loadModule('../../external/message_insert/src/shared.ts', {
    getLang: () => 'en-US',
    PluginConfig: { use: async (_, defaults) => ({ ...defaults, masterEnabled: true, injectTime: false, injectBattery: true }) },
    Tools: { System: { getDeviceInfo: async () => { if (info instanceof Error) throw info; return info; } } },
    console: { log() {}, info() {}, warn() {}, error() {} },
  });
}

test('battery-only injection contains the current device level and charging state', async () => {
  const output = await plugin({ batteryLevel: 73, batteryCharging: true }).buildExtraInfoAttachmentTags('hello');
  assert.equal(output.length, 1);
  assert.match(output[0], /Level: 73%/);
  assert.match(output[0], /Status: Charging/);
  assert.match(output[0], /\[Current Battery\]/);
});

test('empty and full batteries retain their actual states', async () => {
  for (const [level, charging, state] of [[0, false, 'Not charging'], [100, true, 'Fully charged']]) {
    const output = await plugin({ batteryLevel: level, batteryCharging: charging }).buildExtraInfoAttachmentTags('hello');
    assert.match(output[0], new RegExp(`Level: ${level}%`));
    assert.match(output[0], new RegExp(`Status: ${state}`));
  }
});

test('unavailable batteries and provider failures never inject a fabricated percentage', async () => {
  for (const value of [-1, NaN, 101]) {
    const output = await plugin({ batteryLevel: value, batteryCharging: false }).buildExtraInfoAttachmentTags('hello');
    assert.match(output[0], /unavailable/);
    assert.doesNotMatch(output[0], /Level:/);
  }
  const output = await plugin(new Error('device denied')).buildExtraInfoAttachmentTags('hello');
  assert.match(output[0], /device denied/);
});
