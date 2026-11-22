import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const android = 'apps/flutter/app/android/app/src/main/kotlin/app/operit/';
const payload = 'tools/android-ui-server/';

/** Reads each side of an Android build, Binder or JNI contract. */
function source(path) { return readFileSync(new URL(path, root), 'utf8'); }

test('the independent DEX payload is built and embedded without installed Android components', () => {
  const settings = source('apps/flutter/app/android/settings.gradle.kts');
  const gradle = source('apps/flutter/app/android/app/build.gradle.kts');
  const module = source(payload + 'build.gradle.kts');
  assert.match(settings, /project\(":windowServer"\)\.projectDir = file\("\.\.\/\.\.\/\.\.\/\.\.\/tools\/android-ui-server"\)/);
  assert.match(gradle, /"path" to ":windowServer"/);
  assert.match(gradle, /rename \{ "operit-ui-server\.jar" \}/);
  assert.match(module, /SingleArtifact\.APK/);
  assert.match(module, /isMinifyEnabled = false/);
  assert.doesNotMatch(source(payload + 'src/main/AndroidManifest.xml'), /<(?:service|provider|activity|receiver)\b/);
  const client = source(android + 'AndroidUiServer.kt');
  assert.match(client, /assets\.open\("companions\/operit-ui-server\.jar"\)/);
  assert.match(client, /\/data\/local\/tmp\/operit-ui-/);
  assert.match(client, /chmod 444/);
  assert.match(client, /Process\.is64Bit\(\)/);
  assert.doesNotMatch(client, /sourceDir|AccessibilityService|ContentResolver|su 2000|requestPermission\(/);
});

test('Root and Shizuku use the existing Host shell identity boundary before the payload starts Binder', () => {
  const client = source(android + 'AndroidUiServer.kt');
  assert.match(client, /AndroidPrivilegedCommandExecutor\.execute\(context, target/);
  assert.match(client, /rootExecutionSettings\(context\)/);
  assert.match(client, /chown 2000:2000/);
  const main = source(payload + 'src/main/java/app/operit/uiserver/UiServerMain.java');
  assert.ok(main.indexOf('restartAsShell(new String[]') < main.indexOf('Looper.prepare();'));
  assert.match(main, /Process\.myUid\(\) != 2000 \|\| Os\.getgid\(\) != 2000/);
  const jni = source('apps/flutter/native/operit-flutter-bridge/src/AndroidJni.rs');
  assert.match(jni, /Java_app_operit_uiserver_UiServerMain_restartAsShell/);
  assert.match(jni, /operit_host_android_native::androidExecShellProcess\(&decoded\)/);
  const terminal = source('hosts/android/src/terminal.rs');
  const entry = terminal.slice(terminal.indexOf('pub fn androidExecShellProcess('), terminal.indexOf('fn forkPtyExecve('));
  assert.match(entry, /dropRootToShellIdentity\(\)/);
  assert.ok(entry.indexOf('dropRootToShellIdentity();') < entry.indexOf('libc::execv('));
  assert.match(source('hosts/android/src/lib.rs'), /pub use terminal::\{androidExecShellProcess, androidPrivilegedPtyOperation\}/);
});

test('the tmp process owns direct Surfaces and preserves full native event streams', () => {
  const directory = new URL(payload + 'src/main/java/app/operit/uiserver/', root);
  const java = readdirSync(directory).filter(name => name.endsWith('.java'))
    .map(name => readFileSync(new URL(name, directory), 'utf8')).join('\n');
  assert.doesNotMatch(java, /MediaCodec|ImageReader|Bitmap|VideoSink|Base64/);
  assert.match(java, /createVirtualDisplay\([\s\S]*?holder\.getSurface\(\), flags\)/);
  assert.match(java, /MotionEvent\.obtain\(original\)/);
  assert.match(java, /event\.transform\(matrix\)/);
  assert.match(java, /setDisplay\.invoke\(event, displayId\)/);
  assert.match(java, /ACTION_DOWN, KeyEvent\.ACTION_UP/);
  assert.match(java, /setDisplayImePolicy/);
  assert.match(java, /setShouldShowIme/);
});

test('nonce, caller identity and Binder death bind all windows to one live owner', () => {
  const client = source(android + 'AndroidUiServer.kt');
  const main = source(payload + 'src/main/java/app/operit/uiserver/UiServerMain.java');
  assert.match(client, /intent\.getStringExtra\("nonce"\) != nonce/);
  assert.match(client, /Manifest\.permission\.DUMP/);
  assert.match(client, /endpoint\.attach\(owner, windowSession\.asBinder\(\)\)/);
  assert.match(main, /Binder\.getCallingUid\(\) != ownerUid/);
  assert.match(main, /token\.linkToDeath\(death, 0\)/);
  assert.match(main, /DeathRecipient death = \(\) -> main\.post\(this::shutdown\)/);
  assert.match(main, /15000/);
  assert.match(main, /window\.close\(\)/);
});

test('the standalone renderer reuses the registered owner window session on Android 15', () => {
  const context = source(payload + 'src/main/java/app/operit/uiserver/ShellContext.java');
  const window = source(payload + 'src/main/java/app/operit/uiserver/DisplayWindow.java');
  const client = source(android + 'AndroidUiServer.kt');
  assert.match(context, /ownerInfo\.uid != ownerUid/);
  assert.match(context, /"sWindowSession", standalone/);
  assert.match(context, /getSystemPreinstalledFontConfigFromLegacyXml/);
  assert.match(window, /layout\.packageName = viewContext\.getPackageName\(\)/);
  assert.match(client, /Settings\.canDrawOverlays\(context\)/);
  assert.doesNotMatch(window, /TYPE_SYSTEM_DIALOG/);
});

test('standalone input uses native window channels without application gesture hooks or vendor fields', () => {
  const context = source(payload + 'src/main/java/app/operit/uiserver/ShellContext.java');
  const receiver = source(payload + 'src/main/java/app/operit/uiserver/WindowInputReceiver.java');
  const window = source(payload + 'src/main/java/app/operit/uiserver/DisplayWindow.java');
  assert.match(context, /INPUT_FEATURE_NO_INPUT_CHANNEL/);
  assert.match(context, /parameters\[index\] == InputChannel\.class/);
  assert.match(context, /method\.invoke\(session, arguments\)/);
  assert.match(receiver, /extends InputEventReceiver/);
  assert.match(receiver, /root\.dispatchTouchEvent\(motion\)/);
  assert.match(receiver, /finally \{\s*finishInputEvent\(event, handled\)/);
  assert.match(receiver, /consumeBatchedInputEvents\(frameTimeNanos\)/);
  assert.match(window, /receiver\.dispose\(\)/);
  assert.match(source(payload + 'build.gradle.kts'), /compileOnly\(files\(frameworkStubJar/);
  assert.doesNotMatch(context + receiver, /Oplus|oplus|OnePlus|mInputEventReceiver|mInputChannel|mViewRootImplExt/);
});

test('standalone rendering initializes platform libraries and uses public display metrics', () => {
  const context = source(payload + 'src/main/java/app/operit/uiserver/ShellContext.java');
  const main = source(payload + 'src/main/java/app/operit/uiserver/UiServerMain.java');
  assert.ok(main.indexOf('"setArgV0", String.class') < main.indexOf('Looper.prepare();'));
  assert.match(context, /set\(bindClass, bind, "processName", PROCESS_NAME\)/);
  assert.match(context, /ZygoteInit.*getDeclaredMethod\("preloadSharedLibraries"\)/);
  assert.match(context, /getMaximumWindowMetrics\(\)/);
  assert.match(context, /getInsetsIgnoringVisibility\(WindowInsets\.Type\.systemBars\(\) \| WindowInsets\.Type\.displayCutout\(\)\)/);
  assert.doesNotMatch(context, /calculateInsets|ApplicationSharedMemory/);
});

test('ordinary launch and window selection share the existing public abstraction', () => {
  assert.match(source(android + 'AndroidPackageOperations.kt'), /uiAutomation\.launch\(component\.flattenToString\(\)\)/);
  const host = source(android + 'AndroidUiAutomation.kt');
  assert.match(host, /uiServer\.launch\(component\)/);
  assert.match(host, /it\.getInt\("displayId"\) == window\.getInt\("displayId"\)/);
  assert.match(host, /virtual\.getString\("windowId"\)/);
  assert.match(host, /parameters\.put\("display_id", displayId\)/);
  assert.match(host, /result\.put\("windowId", targetId\)/);
  assert.match(host, /overlay\.remove\("displayId"\)/);
  const service = source('tools/android-accessibility/src/main/kotlin/app/operit/accessibility/UiAccessibilityService.kt');
  assert.match(service, /it\.isActive && displayIdOf\(it\) == displayId/);
  assert.match(service, /builder\.setDisplayId\(displayId\)/);
  for (const path of [
    'core/crates/foundation/host-contract/src/UiAutomation.rs',
    'core/crates/plugin/sdk/src/js_sdk/ui.rs',
    'plugins/packages/buildin/automatic_ui_base/src/tools.ts',
  ]) {
    assert.doesNotMatch(source(path), /[Oo]penVirtualWindow|open_virtual_window|close_virtual_window|UiVirtualWindowTransport|is_virtual|isVirtual/);
  }
});
