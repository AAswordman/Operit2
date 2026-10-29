import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const android = 'apps/flutter/app/android/app/src/main/kotlin/app/operit/';

/** Reads the production Rust and Android JNI sources for cross-language ABI checks. */
function source(path) { return readFileSync(new URL(path, root), 'utf8'); }

test('Rust owns priority selection and only the non-proot shell reads privileges', () => {
  const policy = source('hosts/android/src/terminal_privilege.rs');
  assert.match(policy, /Some\(arguments\) => Ok\(ShellTransport::Root\(arguments\)\)/);
  assert.match(policy, /None if self.shizuku_authorized => Ok\(ShellTransport::Shizuku\)/);
  assert.match(policy, /None => Ok\(ShellTransport::App\)/);
  const terminal = source('hosts/android/src/terminal.rs');
  const bash = terminal.slice(terminal.indexOf('fn buildAndroidBashPtyCommand('), terminal.indexOf('fn buildAndroidShellPtyCommand('));
  assert.doesNotMatch(bash, /privileges\(\)|Shizuku|suArguments/);
  assert.match(terminal, /fn buildAndroidShellPtyCommand[\s\S]*?terminal_privilege::privileges\(\)\?\.select\(\)\?/);
});

test('native bridge exposes the same terminal method and keeps Shizuku service names', () => {
  const rust = source('hosts/android/src/terminal_privilege.rs');
  assert.match(rust, /"terminalPrivilegeOperation"/);
  assert.match(source(android + 'AndroidRuntimeHost.kt'), /fun terminalPrivilegeOperation\(request: String\): String = terminalPrivileges.execute\(request\)/);
  assert.match(source(android + 'OperitRuntimeNative.kt'), /external fun privilegedPtyOperation\(request: String\): String/);
  assert.match(source('apps/flutter/native/operit-flutter-bridge/src/AndroidJni.rs'), /Java_app_operit_OperitRuntimeNative_privilegedPtyOperation/);
  assert.match(source('apps/flutter/app/android/app/proguard-rules.pro'), /-keep class app\.operit\.AndroidTerminalUserService/);
});

test('Shizuku owns the PTY and transports ordinary pipes with the same process lifecycle', () => {
  const client = source(android + 'AndroidTerminalPrivileges.kt');
  const service = source(android + 'AndroidTerminalUserService.kt');
  assert.match(client, /hasRootConsent\(context\)/);
  assert.doesNotMatch(client, /requestPermission\(/);
  assert.match(client, /"readFd", reader\.detachFd\(\)/);
  assert.match(client, /"writeFd", writer\.detachFd\(\)/);
  assert.match(client, /RemoteSession\(binder, response\.getInt\("pid"\)\)/);
  assert.match(client, /session\?\.binder \?: bindService\(\)/);
  assert.match(service, /ParcelFileDescriptor\.adoptFd/);
  assert.match(service, /ParcelFileDescriptor\.createPipe\(\)/);
  assert.match(service, /HostTaskScheduler\.executeServices/);
  assert.doesNotMatch(service, /writeTypedObject\(master/);
  assert.match(service, /token\.linkToDeath/);
  assert.match(service, /getCallingUid\(\) == ownerUid/);
  assert.match(service, /"poll", "resize", "close"/);
  assert.doesNotMatch(service, /newProcess\(/);
  const terminal = source('hosts/android/src/terminal.rs');
  assert.match(terminal, /AndroidPtyProcess::Shizuku/);
  assert.match(terminal, /resizeAndroidPtyProcess\(session.pid, session.masterFd, rows, cols\)/);
  assert.match(terminal, /"readFd"/);
  assert.match(terminal, /"writeFd"/);
  assert.match(terminal, /Android PTY reached EOF/);
});
