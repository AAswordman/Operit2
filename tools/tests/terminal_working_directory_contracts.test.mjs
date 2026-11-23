import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);

/** Reads an implementation for cross-language terminal boundary checks. */
function source(path) {
  return readFileSync(new URL(path, root), 'utf8');
}

/** Extracts one required method span without matching unrelated call sites. */
function section(content, start, end) {
  const begin = content.indexOf(start);
  assert.notEqual(begin, -1, `Missing method marker: ${start}`);
  const finish = content.indexOf(end, begin + start.length);
  assert.notEqual(finish, -1, `Missing method terminator: ${end}`);
  return content.slice(begin, finish);
}

/** Requires the selected host to dispatch its directory before any PTY can start. */
test('manual PTY creation resolves the selected terminal namespace first', () => {
  const runtime = source('core/crates/runtime/application/src/services/RuntimeTerminalService.rs');
  const create = section(runtime, 'pub fn startTerminalPty(', 'pub fn terminalPtyOutput(');
  assert.match(create, /\.resolveWorkingDirectory\(/);
  assert.ok(create.indexOf('.resolveWorkingDirectory(') < create.indexOf('.startPtySession('));
  assert.doesNotMatch(create, /resolve_terminal_working_dir\(&self\.context, &workingDir\)/);
  const api = source('core/crates/foundation/host-contract/src/lib.rs');
  assert.match(api, /fn resolveWorkingDirectory\([\s\S]*?\) -> HostResult<String>;/);
});

/** Keeps Linux guest locators out of native Linux mount validation on mobile hosts. */
test('Linux guest hosts own guest working directory resolution', () => {
  for (const path of [
    'hosts/android/src/terminal.rs',
    'hosts/ios/src/terminal.rs',
    'hosts/ohos/src/terminal.rs',
  ]) {
    const host = source(path);
    const resolve = section(host, 'fn resolveWorkingDirectory(', 'fn terminalInfo(');
    assert.match(resolve, /resolveLinuxGuestDirectory\(/);
    assert.match(resolve, /resolveHostDirectory\(workingDir\)/);
  }
  const guest = source('core/crates/foundation/host-contract/src/TerminalWorkingDirectory.rs');
  const resolve = section(guest, 'pub fn resolveLinuxGuestDirectory(', 'pub fn resolveSessionDirectory(');
  assert.match(resolve, /strip_prefix\("\/virtual\/linux"\)/);
  assert.match(resolve, /strip_prefix\("\/mnt\/linux"\)/);
  assert.doesNotMatch(resolve, /target_os|target_arch|\.contains\(/);
});

/** Preserves host-owned resolution through the shared session-publication wrapper. */
test('shared terminal publisher delegates directory resolution to its exact host', () => {
  const wrapper = source('core/crates/runtime/application/src/services/PublishingTerminalHost.rs');
  const resolve = section(wrapper, 'fn resolveWorkingDirectory(', 'fn terminalInfo(');
  assert.match(resolve, /self\.inner\s*\.resolveWorkingDirectory\(/);
});

test('macOS ARM64 uses the real workspace resolver instead of guest home', () => {
  const macos = source('hosts/apple/src/tools/terminal/terminal_macos.rs');
  const resolve = section(macos, 'fn resolveWorkingDirectory(', 'fn terminalInfo(');
  assert.match(resolve, /self\.arm64\.resolveWorkingDirectory\([\s\S]*?workingDir,\s*resolveHostDirectory,/);
  assert.doesNotMatch(resolve, /\|_\|\s*Ok\("\/root"/);
});

/** Declares browser terminal isolation in the host instead of platform branches in Dart. */
test('manual Flutter terminals use uniform VFS locators', () => {
  const panel = source('apps/flutter/app/lib/ui/features/chat/components/workspace/WorkspacePanel.dart');
  const directory = section(panel, 'String _manualTerminalWorkingDirectory(', 'String _nextManualTerminalSessionName(');
  assert.match(directory, /widget\.fileDirectory\?\.trim\(\) \?\? widget\.workspacePath\?\.trim\(\)/);
  assert.match(directory, /return directory;/);
  assert.doesNotMatch(directory, /v86|Platform|kIsWeb|operitRootPath/);
  const web = source('hosts/web/src/tools/terminal/mod.rs');
  const resolve = section(web, 'fn resolveWorkingDirectory(', 'fn terminalInfo(');
  assert.match(resolve, /requireLinuxVmTerminalType\(terminal, terminalType\)\?/);
  assert.match(resolve, /resolveHostDirectory\(path\)/);
  assert.match(resolve, /\/mnt\/operit-host\//);
});

/** Presents launch failures locally and keeps tools on their structured error path. */
test('manual launch errors are not promoted to chat errors', () => {
  const launcher = source('apps/flutter/app/lib/ui/features/chat/components/workspace/terminal/WorkspaceTerminalLaunch.dart');
  assert.match(launcher, /await launch\(\)/);
  assert.match(launcher, /catch \(error, stackTrace\)/);
  assert.match(launcher, /await showDialog<void>/);
  assert.doesNotMatch(launcher, /_errorMessage|InputProcessingState|startPtySession|startTerminalPty/);
  const tools = source('core/crates/tool/services/src/tools/defaultTool/standard/StandardTerminalTools.rs');
  const create = section(tools, 'pub fn createOrGetSession(', 'pub fn executeHiddenCommand(');
  assert.match(create, /Err\(error\) => toolError\(/);
});

/** Retains non-shell startup output in the returned error rather than only device logs. */
test('Android startup errors return the real process output', () => {
  const android = source('hosts/android/src/terminal.rs');
  assert.match(android, /waitForTerminalPrompt\([\s\S]*?androidPtyStartupError,/);
  const error = section(android, 'fn androidPtyStartupError(', 'struct AndroidPtyPromptMarker');
  assert.match(error, /stripPtyPromptMarkers\(data\)/);
  assert.match(error, /renderTerminalText/);
  assert.match(error, /Startup output:/);
});

/** Document mounts must reach the exact terminal host through every launch entry point. */
test('document-backed terminal directories reach Android instead of nativePath rejection', () => {
  const runtime = source('core/crates/runtime/application/src/services/RuntimeTerminalService.rs');
  assert.match(runtime, /filesystem\.terminalDirectory\(terminalHost, &path\)/);
  const tools = source('core/crates/tool/services/src/tools/defaultTool/standard/StandardTerminalTools.rs');
  assert.equal((tools.match(/\.terminalDirectory\(host, path\)/g) ?? []).length, 1);
  assert.match(tools, /self\.resolveFileDirectory\(host, &terminal, terminalType, &workspace\)/);
  const mapper = source('core/crates/foundation/vfs/src/PathMapper.rs');
  assert.match(mapper, /host\.resolveNativeWorkingDirectory\(&self\.physicalPath\)/);
  const wrapper = source('core/crates/runtime/application/src/services/PublishingTerminalHost.rs');
  assert.match(wrapper, /self\.inner\.resolveNativeWorkingDirectory\(path\)/);
  const host = source('hosts/android/src/terminal.rs');
  assert.match(host, /resolveAndroidNativeWorkingDirectory\(path, &crate::document_filesystem::call\)/);
  const backend = source('apps/flutter/app/android/app/src/main/kotlin/app/operit/AndroidDocumentFileSystem.kt');
  assert.match(backend, /"resolveNativeDirectory" -> node\(request\.getString\("path"\)\)\.nativeDirectory\(\)/);
  const launcher = source('hosts/android/src/runtime_common.rs');
  assert.match(launcher, /command\.arg\("-b"\)\.arg\("\/storage"\)/);
});

/** All Android Linux PTYs must preserve selected storage roots as individual argv entries. */
test('built-in and configured Android terminals share the literal proot launcher', () => {
  const host = source('hosts/android/src/terminal.rs');
  const builtin = section(host, 'fn buildAndroidBashPtyCommand(', 'fn buildAndroidShellPtyCommand(');
  const configured = section(host, 'fn buildAndroidProfileCommand(', 'fn buildAndroidProotPtyCommand(');
  assert.match(builtin, /buildAndroidProotPtyCommand\(&rootfs, workingDir, &\[\], &\["--login", "-i"\]\)/);
  assert.match(configured, /buildAndroidProotPtyCommand\(&rootfs, workingDir, &mounts, &\["--noprofile", "--norc", "-i"\]\)/);
  const launch = section(host, 'fn buildAndroidProotPtyCommand(', 'fn buildAndroidPtyCommand(');
  assert.match(launch, /buildAndroidProotCommandWithRootfs\(/);
  assert.match(launch, /command\.get_args\(\)/);
  assert.doesNotMatch(host, /common\.sh|start_shell/);
  const assets = source('apps/flutter/app/android/app/src/main/kotlin/app/operit/AndroidRuntimeAssets.kt');
  assert.doesNotMatch(assets, /PROOT_BIND_ARGS|probe_and_append_bind_arg|writeCommonScript/);
  const launcher = source('hosts/android/src/runtime_common.rs');
  for (const key of ['INTERNAL_ROOT', 'RUNTIME_ROOT', 'WORKSPACE_ROOT']) {
    assert.ok(launcher.includes(`requiredAndroidRuntimePath("OPERIT_ANDROID_${key}")?`));
  }
  for (const name of ['internalRoot', 'runtimeRoot', 'workspaceRoot']) {
    assert.ok(launcher.includes(`command.arg("-b").arg(bindSamePath(&${name}))`));
  }
  assert.match(launcher, /command\.arg\("-w"\)\.arg\(&workingDir\)/);
  assert.match(launcher, /ensureRootfsAbsolutePath\(rootfsDir, Path::new\(&mount\.target\)\)\?/);
});

/** Host workspaces must enter a profile's guest mount rather than its inaccessible source. */
test('Android maps host workspaces through configured mounts before launch', () => {
  const host = source('hosts/android/src/terminal.rs');
  const resolve = section(host, 'fn resolveWorkingDirectory(', 'fn terminalInfo(');
  assert.match(resolve, /let directory = resolveHostDirectory\(path\)\?/);
  assert.match(resolve, /androidMountedWorkingDirectory\(&directory, &mounts\)/);
  const map = section(host, 'fn androidMountedWorkingDirectory(', 'fn androidProfileMounts(');
  assert.match(map, /strip_prefix\(&mount\.source\)/);
  assert.match(map, /max_by_key/);
  assert.doesNotMatch(map, /\.contains\(/);
});

/** Shells receive the resolved directory and never need an application VFS mount. */
test('application VFS aliases resolve before the terminal receives its working directory', () => {
  const mapper = source('core/crates/foundation/vfs/src/PathMapper.rs');
  const regression = section(mapper, 'fn customStorageAliasesResolveToNativePlatformDirectories(', 'fn nativeStorageIngressRejectsUnsupportedPathModels(');
  for (const path of ['/operitapp/data/config', '/operitapp/outputs/chat', '/operitapp/workspaces/project']) {
    assert.ok(regression.includes(`"${path}"`));
  }
  assert.match(regression, /mapper\.resolve\(&platformPath\)\.unwrap\(\)\.nativePath\(\)\.unwrap\(\), directory/);
  const runtime = source('core/crates/runtime/application/src/services/RuntimeTerminalService.rs');
  const create = section(runtime, 'fn start_terminal_pty(', 'pub fn terminalPtyOutput(');
  assert.match(create, /\.startPtySession\([\s\S]*?&resolvedWorkingDir,/);
  const launcher = source('hosts/android/src/runtime_common.rs');
  assert.doesNotMatch(launcher, /operitapp/);
});
