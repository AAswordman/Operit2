import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { randomUUID } from 'node:crypto';

assert.equal(process.platform, 'win32', 'This acceptance test requires the real Windows terminal Host');
const binary = resolve(process.argv[2] ?? 'apps/cli/target/debug/operit2.exe');
const profile = mkdtempSync(join(tmpdir(), 'operit-terminal-cli-'));
const config = join(profile, 'config');
mkdirSync(config);
const identity = randomUUID();
writeFileSync(join(config, 'storage.json'), JSON.stringify({
  runtimeRoot: join(profile, 'runtime'), workspaceRoot: join(profile, 'workspace'),
  activeIdentityId: identity, identities: [{ id: identity, name: 'Terminal acceptance', createdAt: 1 }],
}));
const env = { ...process.env, OPERIT_CLI_CONFIG_DIR: config };
const sessions = new Set(), checks = [];

/** Invokes the actual CLI against its isolated persistent daemon and retains diagnostic output. */
function invoke(args) {
  const result = spawnSync(binary, args, { env, encoding: 'utf8', windowsHide: true, timeout: 180000 });
  if (result.error || result.status !== 0) throw new Error(`${args.slice(0, 4).join(' ')}: ${result.error ?? result.status}\n${result.stdout}\n${result.stderr}`);
  return result.stdout;
}

/** Decodes the CLI's structured command result. */
function cli(...args) {
  return JSON.parse(invoke([...args, '--json']));
}

/** Reads the concrete result of a real built-in tool invocation. */
function tool(name, params) {
  const result = cli('cli', 'tool', 'exec', name, JSON.stringify(params));
  assert.equal(result.success, true, JSON.stringify(result));
  return result.result;
}

/** Starts a named command through the platform-independent managed terminal entry point. */
function run(sessionName, terminalType, command, waitMs = 20000) {
  const receipt = tool('terminal_observe', { observation: { op: 'run', sessionName, terminalType, command, waitMs } }).result;
  sessions.add(receipt.sessionId);
  return receipt;
}

/** Waits for the original command instead of sending another command into its busy terminal. */
function wait(sessionId, waitMs = 20000) {
  return tool('terminal_observe', { observation: { op: 'wait', sessionId, waitMs } }).result;
}

/** Records an acceptance check alongside the actual receipts for later inspection. */
function pass(name, details) {
  checks.push({ name, details });
  writeFileSync(join(profile, 'report.json'), JSON.stringify({ binary, profile, checks }, null, 2));
  console.log('PASS', name);
}

console.log('Isolated profile:', profile);
const daemon = spawn(binary, ['daemon', 'run', '--no-listen'], { env, windowsHide: true, stdio: 'ignore' });
try {
  let ready = false;
  for (let attempt = 0; attempt < 60; attempt++) {
    const status = cli('daemon', 'status');
    if (status.running && status.status.state === 'ready') { ready = true; break; }
    await delay(250);
  }
  assert.equal(ready, true, 'The isolated CLI daemon must be ready before terminal acceptance');
  const first = run('powershell-normal', 'powershell', "$env:OPERIT_TERMINAL_TEST = 'kept'; Write-Output 'NORMAL_中文'");
  assert.equal(first.timedOut, false);
  assert.equal(first.exitCode, 0);
  assert.match(first.output, /NORMAL_中文/);
  const reused = run('powershell-normal', 'powershell', 'Write-Output $env:OPERIT_TERMINAL_TEST');
  assert.equal(reused.sessionId, first.sessionId);
  assert.match(reused.output, /kept/);
  pass('PowerShell normal completion and session context', { first, reused });

  const started = Date.now();
  const background = tool('execute_in_terminal_session', {
    session_id: first.sessionId,
    command: "Write-Output 'BACKGROUND_START'; Start-Sleep -Seconds 25; Write-Output 'BACKGROUND_END'; cmd /c exit 7",
  });
  const elapsedMs = Date.now() - started;
  assert.equal(background.timedOut, true);
  assert.ok(elapsedMs >= 19000 && elapsedMs < 25000, `Expected a 20s foreground wait, got ${elapsedMs}ms`);
  assert.equal(tool('get_terminal_session_screen', { session_id: background.sessionId }).commandRunning, true);
  const independent = tool('execute_in_terminal_session', { session_id: background.sessionId, command: "Write-Output 'INDEPENDENT'" });
  sessions.add(independent.sessionId);
  assert.notEqual(independent.sessionId, background.sessionId);
  assert.equal(independent.timedOut, false);
  assert.match(independent.output, /INDEPENDENT/);
  const completed = wait(background.sessionId);
  assert.equal(completed.timedOut, false);
  assert.equal(completed.exitCode, 7);
  assert.match(completed.output, /BACKGROUND_END/);
  pass('Default 20s wait, automatic session rotation and original exit code', { elapsedMs, background, independent, completed });

  const opened = tool('terminal_observe', { observation: { op: 'open', request: {
    sessionId: independent.sessionId, command: "Write-Output 'STREAM_START'; Start-Sleep -Seconds 3; Write-Output 'STREAM_END'", timeoutMs: 1000,
  } } });
  let streamed;
  do {
    const next = tool('terminal_observe', { observation: { op: 'next', handle: opened.handle } });
    assert.ok(next.event);
    if (next.event.type === 'completed') streamed = next.event.result;
  } while (streamed === undefined);
  assert.equal(streamed.timedOut, true);
  sessions.add(streamed.sessionId);
  tool('terminal_observe', { observation: { op: 'close', handle: opened.handle } });
  assert.equal(tool('get_terminal_session_screen', { session_id: streamed.sessionId }).commandRunning, true);
  const streamCompleted = wait(streamed.sessionId);
  assert.match(streamCompleted.output, /STREAM_END/);
  assert.equal(streamCompleted.timedOut, false);
  pass('Streaming wait expiry and iterator detach preserve execution', { streamed, streamCompleted });

  const bash = run('bash-normal', 'bash', "export OPERIT_TERMINAL_TEST=kept; printf 'BASH_NORMAL\\n'");
  assert.equal(bash.timedOut, false);
  assert.equal(bash.exitCode, 0);
  assert.match(bash.output, /BASH_NORMAL/);
  const bashReused = run('bash-normal', 'bash', "printf '%s\\n' \"$OPERIT_TERMINAL_TEST\"");
  assert.equal(bashReused.sessionId, bash.sessionId);
  assert.match(bashReused.output, /kept/);
  const bashBackground = run('bash-normal', 'bash', "printf 'BASH_START\\n'; sleep 3; printf 'BASH_END\\n'; (exit 9)", 1000);
  assert.equal(bashBackground.timedOut, true);
  const bashNext = run('bash-normal', 'bash', "printf 'BASH_INDEPENDENT\\n'");
  assert.notEqual(bashNext.sessionId, bashBackground.sessionId);
  assert.match(bashNext.output, /BASH_INDEPENDENT/);
  const bashCompleted = wait(bashBackground.sessionId);
  assert.equal(bashCompleted.exitCode, 9);
  assert.match(bashCompleted.output, /BASH_END/);
  pass('Git Bash normal completion, context, rotation and background completion', { bash, bashReused, bashBackground, bashNext, bashCompleted });

  const closing = run('powershell-close', 'powershell', 'Start-Sleep -Seconds 60', 0);
  assert.equal(closing.timedOut, true);
  assert.equal(tool('close_terminal_session', { session_id: closing.sessionId }).success, true);
  sessions.delete(closing.sessionId);
  pass('Explicit close terminates a running background session', closing);
} finally {
  for (const session_id of sessions) {
    try { tool('close_terminal_session', { session_id }); }
    catch (error) { console.error('Session cleanup:', String(error)); }
  }
  invoke(['daemon', 'stop']);
  daemon.unref();
  console.log('Evidence:', join(profile, 'report.json'));
}
