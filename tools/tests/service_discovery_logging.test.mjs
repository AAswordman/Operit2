import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const discovery = readFileSync(new URL(
  '../../hosts/common/operit-host-native-common/src/ServiceDiscovery.rs', import.meta.url,
), 'utf8');

/** Keeps routine mDNS responses and browser scans below the info log level. */
test('routine discovery diagnostics use verbose priority', () => {
  assert.match(discovery, /DaemonEvent::Error\(error\)[\s\S]*?6\s*\}\s*_ => 2,/);
  assert.match(discovery, /tryLogHostConsole\(2, "ServiceDiscovery", &search\)/);
  for (const event of ['resolved service=', 'scan complete service=']) {
    assert.ok(discovery.includes(event));
    const call = discovery.slice(0, discovery.indexOf(event));
    assert.match(call.slice(call.lastIndexOf('tryLogHostConsole(')), /^tryLogHostConsole\(\s*2,/);
  }
  assert.doesNotMatch(discovery, /tryLogHostConsole\(\s*4,/);
});

/** Preserves actionable discovery failures at error priority. */
test('discovery shutdown and cleanup errors remain visible', () => {
  for (const event of ['daemon shutdown failed:', 'stop browser ', 'unregister {}:']) {
    const call = discovery.slice(0, discovery.indexOf(event));
    assert.ok(discovery.includes(event));
    assert.match(call.slice(call.lastIndexOf('tryLogHostConsole(')), /^tryLogHostConsole\(\s*6,/);
  }
});
