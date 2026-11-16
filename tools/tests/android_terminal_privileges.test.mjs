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

test('a root-started Shizuku drops PTY children to the shell identity before exec', () => {
  const terminal = source('hosts/android/src/terminal.rs');
  const privileged = terminal.slice(terminal.indexOf('pub fn androidPrivilegedPtyOperation('));
  assert.match(privileged, /dropRootToShell: true/);
  const drop = terminal.slice(terminal.indexOf('unsafe fn dropRootToShellIdentity('), terminal.indexOf('fn forkPtyExecve('));
  const groups = drop.indexOf('libc::setgroups(');
  const gid = drop.indexOf('libc::setgid(SHELL_GID)');
  const uid = drop.indexOf('libc::setuid(SHELL_UID)');
  const setcon = drop.indexOf('/proc/self/attr/current');
  assert.ok(groups >= 0 && gid > groups && uid > gid && setcon > uid, 'drop order must be groups, gid, uid, then setcon');
  assert.match(drop, /u:r:shell:s0/);
  assert.match(drop, /libc::getuid\(\) != SHELL_UID \|\| libc::getgid\(\) != SHELL_GID/);
  assert.match(drop, /libc::_exit\(126\)/);
  const fork = terminal.slice(terminal.indexOf('fn forkPtyExecve('), terminal.indexOf('fn operitTermios('));
  assert.match(fork, /command\.dropRootToShell && libc::geteuid\(\) == 0/);
  assert.ok(fork.indexOf('dropRootToShellIdentity()') < fork.indexOf('libc::execve('), 'identity drop must happen before execve');
  const appBuilders = terminal.slice(terminal.indexOf('fn buildAndroidBashPtyCommand('), terminal.indexOf('fn buildAndroidShellPtyCommand('))
    + terminal.slice(terminal.indexOf('fn buildAndroidShellPtyCommand('), terminal.indexOf('fn quoteAndroidShellArgument('));
  assert.doesNotMatch(appBuilders, /dropRootToShell: true/, 'app-side and Root transports keep their explicitly authorized identity');
});
