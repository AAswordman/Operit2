import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const android = 'apps/flutter/app/android/app/src/main/kotlin/app/operit/';
const system = android + 'core/tools/system/';
const source = (path) => readFileSync(new URL(path, root), 'utf8');

function section(content, start, end) {
  const begin = content.indexOf(start);
  assert.notEqual(begin, -1);
  const finish = content.indexOf(end, begin + start.length);
  assert.notEqual(finish, -1);
  return content.slice(begin, finish);
}

test('Root authorization combines explicit consent with a fresh identity check', () => {
  const authorization = source(android + 'AndroidPrivilegeAuthorization.kt');
  const check = section(authorization, 'fun isRootAuthorized(', 'fun rootExecutionSettings(');
  assert.match(check, /getBoolean\(ROOT_AUTHORIZED_KEY, false\)/);
  assert.match(check, /if \(!hasRootConsent\(context\)\) return false/);
  assert.match(check, /AndroidRootShell\.checkAccess\(rootExecutionSettings\(context\), 10_000L\)\.granted/);
  const config = section(authorization, 'fun configureRootExecution(', '/** Consent');
  assert.match(config, /settings\.suArguments\(\)/);
  assert.match(config, /putBoolean\(ROOT_AUTHORIZED_KEY, false\)/);
});

test('onboarding probes Root off the main thread using the same execution settings', () => {
  const platform = source(android + 'AndroidPlatformChannel.kt');
  const snapshot = section(platform, 'private fun hostOnboardingPermissionSnapshot(', '/** Starts an Android onboarding');
  assert.match(snapshot, /runtimeHost\.runBackground/);
  assert.match(snapshot, /val snapshot = onboardingPermissionSnapshot\(\)/);
  assert.match(snapshot, /activity\.runOnUiThread \{ result\.success\(snapshot\) \}/);
  const verify = section(platform, 'private fun verifyRootAuthorization()', '/** Requests broad shared-storage');
  assert.match(verify, /AndroidRootShell\.checkAccess\(/);
  assert.match(verify, /AndroidPrivilegeAuthorization\.rootExecutionSettings\(activity\)/);
  assert.match(verify, /setRootAuthorized\(activity, status\.granted\)/);
  assert.doesNotMatch(verify, /RootExec|ProcessBuilder/);
});

test('Root requests support automatic, forced libsu, and custom exec settings', () => {
  const platform = source(android + 'AndroidPlatformChannel.kt');
  const request = section(platform, 'private fun requestRootAuthorization(', '/** Uses the same automatic');
  assert.match(request, /argument<String>\("rootExecutionMode"\)/);
  assert.match(request, /argument<String>\("suCommand"\)/);
  for (const [wire, mode] of [['auto', 'Auto'], ['libsu', 'ForceLibsu'], ['exec', 'ForceExec']]) {
    assert.match(request, new RegExp(`"${wire}" -> AndroidRootExecutionMode\\.${mode}`));
  }
});

test('all Root transports share the router; Shizuku retains an independent transport', () => {
  const executor = source(system + 'AndroidPrivilegedCommandExecutor.kt');
  assert.match(executor, /RootAuto,[\s\S]*?RootLibsu,[\s\S]*?RootExec -> AndroidRootShell\.execute\(target, command, timeoutMillis, rootSettings\)/);
  assert.match(executor, /Shizuku -> executeWithShizuku\(command, timeoutMillis\)/);
  assert.doesNotMatch(executor, /ProcessBuilder\("su"|Shell\.cmd/);
});

test('Root commands bind to an owned verified shell and close failed sessions', () => {
  const shell = source(system + 'AndroidRootShell.kt');
  assert.match(shell, /Shell\.Builder\.create\(\)[\s\S]*?setFlags\(Shell\.FLAG_MOUNT_MASTER\)[\s\S]*?setTimeout\(10\)/);
  assert.match(shell, /if \(!session\.isRoot\) \{\s*session\.close\(\)/);
  assert.match(shell, /session\.newJob\(\)\.add\(command\)/);
  assert.match(shell, /future\.get\(timeoutMillis, TimeUnit\.MILLISECONDS\)/);
  assert.match(shell, /shell\.getAndSet\(null\)\?\.close\(\)/);
  assert.doesNotMatch(shell, /Shell\.getShell\(|val result = Shell\.cmd\(/);
});

test('Root screenshot and owner commands honor the same selected Root configuration', () => {
  const owner = source(android + 'OwnerSystemCapabilityChannel.kt');
  assert.match(owner, /"root", "root_auto" -> AndroidPrivilegedCommandTarget\.RootAuto/);
  const commands = section(owner, 'private fun systemExecutePrivilegedCommand(', 'private fun ownerAudioPlay(');
  assert.match(commands, /rootSettings = AndroidPrivilegeAuthorization\.rootExecutionSettings\(activity\)/);
  assert.match(commands, /RootAuto,\s*AndroidPrivilegedCommandTarget\.RootLibsu,/);
  const screenshot = section(owner, 'private fun captureScreenshotWithRoot(', '/** Captures a PNG screen image through Shizuku');
  assert.match(screenshot, /target = AndroidPrivilegedCommandTarget\.RootAuto/);
  assert.match(screenshot, /rootSettings = AndroidPrivilegeAuthorization\.rootExecutionSettings\(activity\)/);
});

test('Root identity probing requires exit zero and exact UID zero, not root hints', () => {
  const router = source(system + 'AndroidRootCommandRouter.kt');
  assert.match(router, /result\.exitCode == 0 && result\.stdoutText\(\)\.trim\(\) == "0"/);
  assert.match(router, /version\.exitCode == 0/);
  assert.match(router, /text\.contains\("KernelSU", ignoreCase = true\)/);
  assert.match(router, /AndroidRootAccessStatus\(deviceRooted, null, diagnostics\)/);
  assert.match(router, /if \(error is InterruptedException\)/);
});

test('process collection drains both pipes and destroys processes on failed waits', () => {
  const process = source(system + 'AndroidCommandProcessRunner.kt');
  assert.match(process, /process\.outputStream\.close\(\)/);
  assert.match(process, /HostTaskScheduler\.executeServices\(arrayOf\(output, errors, exit\)\)/);
  assert.doesNotMatch(process, /Executors|\bThread\(/);
  assert.match(process, /stdout\.use \{ it\.readBytes\(\) \}/);
  assert.match(process, /stderr\.use \{ it\.readBytes\(\) \}/);
  assert.match(process, /catch \(error: Exception\) \{\s*runCatching \{ destroy\(\) \}/);
  assert.match(process, /stdout\.close\(\)/);
  assert.match(process, /stderr\.close\(\)/);
});
