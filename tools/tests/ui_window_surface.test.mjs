import assert from 'node:assert/strict';
import { execFileSync, spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import vm from 'node:vm';

const scriptPath = fileURLToPath(new URL('../../hosts/windows/src/ui_automation.ps1', import.meta.url));

/** Invokes the actual Windows provider without sending pointer or keyboard input. */
function windows(operation, parameters = {}) {
  return JSON.parse(execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-STA', '-File', scriptPath], {
    input: JSON.stringify({ operation, parameters }), encoding: 'utf8', windowsHide: true,
    timeout: 75000, maxBuffer: 48 * 1024 * 1024, stdio: ['pipe', 'pipe', 'pipe'],
  }));
}

/** Reads dimensions from the actual provider PNG rather than trusting metadata. */
function pngSize(result) {
  const png = Buffer.from(result.pngBase64, 'base64');
  assert.equal(png.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
  return [png.readUInt32BE(16), png.readUInt32BE(20)];
}

/** Waits for a native source or preview to publish its readiness without closing its lifetime pipe. */
function ready(child) {
  return new Promise((resolve, reject) => {
    let output = '', errors = '';
    /** Removes this response's listeners after success or failure. */
    function cleanup() {
      clearTimeout(timeout); child.stderr.off('data', errorData); child.stdout.off('data', response); child.off('exit', exited);
    }
    /** Collects native diagnostics while waiting for one response. */
    function errorData(data) { errors += data; }
    /** Rejects a helper that exits before reporting its result. */
    function exited(code) { cleanup(); reject(new Error(`Native helper exited (${code}): ${errors}`)); }
    /** Resolves one complete structured result. */
    function response(data) {
      output += data;
      if (!output.includes('\n')) return;
      cleanup();
      try { resolve(JSON.parse(output.split('\n')[0])); } catch (error) { reject(error); }
    }
    const timeout = setTimeout(() => { cleanup(); reject(new Error(`Native preview response timed out: ${errors}`)); }, 15000);
    child.stderr.on('data', errorData); child.stdout.on('data', response); child.once('exit', exited);
  });
}

/** Observes native styles and actual desktop pixels rather than trusting preview metadata. */
function nativeState(windowId, x = 0, y = 0, captureBounds, points = []) {
  const code = `
Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class PreviewState {
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll", EntryPoint="GetWindowLongPtrW")] public static extern IntPtr Style(IntPtr window, int index);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
 [DllImport("user32.dll")] public static extern bool GetLayeredWindowAttributes(IntPtr window, out uint color, out byte alpha, out uint flags);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT frame);
 [StructLayout(LayoutKind.Sequential)] public struct RECT { public int left,top,right,bottom; }
}
'@
$null=[PreviewState]::SetThreadDpiAwarenessContext([IntPtr](-4))
[uint32]$color=0; [byte]$alpha=255; [uint32]$flags=0
$null=[PreviewState]::GetLayeredWindowAttributes([IntPtr]${windowId},[ref]$color,[ref]$alpha,[ref]$flags)
$frame=[PreviewState+RECT]::new(); $null=[PreviewState]::GetWindowRect([IntPtr]${windowId},[ref]$frame)
$bitmap=[Drawing.Bitmap]::new(1,1); $graphics=[Drawing.Graphics]::FromImage($bitmap)
try { $graphics.CopyFromScreen(${x},${y},0,0,$bitmap.Size); $pixel=$bitmap.GetPixel(0,0)
$samples=@(foreach ($point in @(${points.map(([px, py]) => `@{x=${px};y=${py}}`).join(',')})) {
 $graphics.CopyFromScreen($point.x,$point.y,0,0,$bitmap.Size); $sample=$bitmap.GetPixel(0,0)
 @{r=$sample.R;g=$sample.G;b=$sample.B}
})
@{style=[PreviewState]::Style([IntPtr]${windowId},-20).ToInt64(); normalStyle=[PreviewState]::Style([IntPtr]${windowId},-16).ToInt64(); dpi=[PreviewState]::GetDpiForWindow([IntPtr]${windowId}); alpha=$alpha; visible=[PreviewState]::IsWindowVisible([IntPtr]${windowId}); frame=@($frame.left,$frame.top,$frame.right,$frame.bottom); foreground=[PreviewState]::GetForegroundWindow().ToInt64(); pixel=@($pixel.R,$pixel.G,$pixel.B); samples=$samples} | ConvertTo-Json -Compress
} finally { $graphics.Dispose(); $bitmap.Dispose() }
${captureBounds ? `
if ($env:OPERIT_UI_PREVIEW_SNAPSHOT) {
 $gutter=[int][Math]::Ceiling(32*[PreviewState]::GetDpiForWindow([IntPtr]${windowId})/96)
 $capture=[Drawing.Rectangle]::FromLTRB((${captureBounds[0]}-$gutter),(${captureBounds[1]}-$gutter),(${captureBounds[2]}+$gutter),(${captureBounds[3]}+$gutter))
 $capture.Intersect([Windows.Forms.SystemInformation]::VirtualScreen)
 $shot=[Drawing.Bitmap]::new($capture.Width,$capture.Height); $surface=[Drawing.Graphics]::FromImage($shot)
 try { $surface.CopyFromScreen($capture.X,$capture.Y,0,0,$shot.Size); $shot.Save($env:OPERIT_UI_PREVIEW_SNAPSHOT,[Drawing.Imaging.ImageFormat]::Png) }
 finally { $surface.Dispose(); $shot.Dispose() }
}` : ''}
`;
  return JSON.parse(execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', code], {encoding: 'utf8', windowsHide: true}));
}

/** Releases a helper through the same EOF path used on owner destruction. */
async function close(child) {
  if (child.exitCode !== null) return;
  child.stdin.end();
  await once(child, 'exit');
}

test('Windows rounds an occluded live source and sends local input without foreground activation', { skip: process.platform !== 'win32', timeout: 40000 }, async () => {
  const sourceScript = `
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms, System.Drawing @'
using System;
using System.Windows.Forms;
using System.Drawing;
using System.Runtime.InteropServices;
public sealed class OperitTestBackdrop : Form {
    protected override bool ShowWithoutActivation { get { return true; } }
    /// Keeps the fixture behind the preview without taking user focus.
    protected override CreateParams CreateParams {
        get { var value=base.CreateParams; value.ExStyle|=0x08000080; return value; }
    }
}
public sealed class OperitTestSource : Form {
    [DllImport("user32.dll")] private static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] private static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] private static extern IntPtr CreateWindowEx(int exStyle, string type, string title, int style, int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr data);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] private static extern bool SetWindowText(IntPtr window, string text);
    [StructLayout(LayoutKind.Sequential)] private struct RECT {public int left, top, right, bottom;}
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr window, out RECT frame);
    private IntPtr button;
    private int count;
    private Form backdrop;
    protected override bool ShowWithoutActivation { get { return true; } }
    /// Prevents fixture startup from changing the user's foreground window.
    protected override CreateParams CreateParams {
        get { var value=base.CreateParams; value.ExStyle|=0x08000080; return value; }
    }
    /// Reports input receipt together with foreground ownership at the exact dispatch point.
    protected override void WndProc(ref Message message) {
        if (message.Msg == 0x0111 && (message.WParam.ToInt64() & 65535) == 42) {
            count++; SetWindowText(button, "Count " + count);
            BackColor = count % 2 == 0 ? Color.Magenta : Color.Lime;
            Console.WriteLine("{\\"count\\":" + count + ",\\"foreground\\":" + GetForegroundWindow().ToInt64() + "}"); Console.Out.Flush();
        }
        base.WndProc(ref message);
    }
    /// Places a neutral fixture behind all preview corners so tinted compositor leakage is measurable.
    protected override void OnShown(EventArgs e) {
        base.OnShown(e);
        BeginInvoke(new Action(() => {
            ShowWindow(Handle, 4);
            var area=Screen.PrimaryScreen.WorkingArea;
            backdrop=new OperitTestBackdrop { Text="Operit preview test backdrop", FormBorderStyle=FormBorderStyle.None,
                ShowInTaskbar=false, TopMost=true, StartPosition=FormStartPosition.Manual, BackColor=Color.FromArgb(160,160,160),
                Bounds=new Rectangle(area.Right-560,area.Top,560,Math.Min(600,area.Height)) };
            backdrop.Show(); ShowWindow(backdrop.Handle,4);
            button = CreateWindowEx(0, "BUTTON", "Count 0", unchecked((int)0x50000000), 60, 80, 140, 40, Handle, new IntPtr(42), IntPtr.Zero, IntPtr.Zero);
            RECT frame, child; GetWindowRect(Handle, out frame); GetWindowRect(button, out child);
            Console.WriteLine("{\\"ready\\":true,\\"windowId\\":\\"" + Handle.ToInt64() + "\\",\\"x\\":" + ((child.left+child.right)/2-frame.left) + ",\\"y\\":" + ((child.top+child.bottom)/2-frame.top) + "}"); Console.Out.Flush();
        }));
    }
    /// Removes every fixture window when its owner pipe is closed.
    protected override void OnFormClosed(FormClosedEventArgs e) {
        if (backdrop != null) backdrop.Dispose(); base.OnFormClosed(e);
    }
    /// Runs the native source under the same parent-pipe lifetime contract as production.
    public static void Run() {
        SetThreadDpiAwarenessContext(new IntPtr(-4));
        using (var form = new OperitTestSource()) using (var timer = new Timer()) {
            form.Text = "Operit synthetic window preview test";
            var area=Screen.PrimaryScreen.WorkingArea;
            form.StartPosition = FormStartPosition.Manual; form.Location = new Point(area.Right-540,area.Top+100);
            form.ClientSize = new Size(320,240); form.AutoScaleMode = AutoScaleMode.None; form.BackColor = Color.Magenta;
            var input = System.Threading.Tasks.Task.Factory.StartNew(() => Console.In.ReadLine());
            timer.Interval=50; timer.Tick += (s,e) => { if (input.IsCompleted) form.Close(); }; timer.Start();
            Application.Run(form);
        }
    }
}
'@
[OperitTestSource]::Run()
`;
  const launch = code => spawn('powershell.exe', ['-NoProfile', '-NonInteractive', '-STA', '-Command', code], {windowsHide: true, stdio: ['pipe','pipe','pipe']});
  const source = launch(sourceScript);
  let preview;
  try {
    const identity = await ready(source);
    const original = nativeState(identity.windowId);
    const baseline = windows('get_page_info', {window_id: identity.windowId});
    const sourceBounds = baseline.uiElements.bounds.match(/-?\d+/g).map(Number);
    assert.deepEqual(pngSize(windows('capture_screenshot', {window_id: identity.windowId})), [sourceBounds[2], sourceBounds[3]]);
    const desktopSize = pngSize(windows('capture_screenshot'));
    assert.ok(desktopSize[0] > 0 && desktopSize[1] > 0);
    assert.throws(() => windows('capture_screenshot', {window_id: '0'}), /no longer exists/);
    preview = spawn('powershell.exe', ['-NoProfile', '-NonInteractive', '-STA', '-File', scriptPath], {windowsHide: true, stdio: ['pipe','pipe','pipe']});
    const waiting = ready(preview);
    preview.stdin.write(JSON.stringify({operation: 'preview_window', parameters: {window_id: identity.windowId}}) + '\n');
    const hosted = await waiting;
    assert.equal(hosted.ready, true, hosted.error);
    const list = windows('list_windows');
    assert.ok(list.some(window => window.windowId === identity.windowId));
    assert.ok(!list.some(window => window.windowId === hosted.windowId));
    const visual = list.find(window => window.windowId === identity.windowId).bounds.match(/-?\d+/g).map(Number);
    const previewState = nativeState(hosted.windowId, visual[0], visual[1]);
    assert.equal(previewState.normalStyle & 0x00c40000, 0, 'The preview must have no system caption or native resize frame');
    const inset = Math.round(22 * previewState.dpi / 96 * 0.18);
    const corners = [[visual[0]+inset,visual[1]+inset], [visual[2]-inset-1,visual[1]+inset],
      [visual[0]+inset,visual[3]-inset-1], [visual[2]-inset-1,visual[3]-inset-1]];
    /** Rejects dark source-colored rectangles as well as fully opaque corner leakage. */
    function checkCorners(state) {
      assert.equal(state.samples.length, 4);
      for (const {r,g,b} of state.samples) {
        assert.ok(Math.max(r,g,b)-Math.min(r,g,b) <= 3,
          `A rounded corner must contain only the neutral backdrop and shadow, received ${r},${g},${b}`);
        assert.ok(r >= 125 && r < 160, 'The corner shadow must remain subtle over the neutral backdrop');
      }
    }
    const point = [visual[0] + Math.floor((visual[2] - visual[0]) / 10), visual[1] + Math.floor((visual[3] - visual[1]) / 3)];
    const before = nativeState(identity.windowId, ...point, visual, corners);
    assert.equal(before.style, original.style, 'The original source must retain its interactive window styles');
    assert.equal(before.alpha, original.alpha);
    assert.equal(before.visible, true);
    assert.deepEqual(before.frame, original.frame, 'The source must remain at its original location and size');
    assert.deepEqual(before.pixel, [255,0,255], 'The preview must show the real source, not a black DWM surface');
    checkCorners(before);
    const page = windows('get_page_info', {window_id: identity.windowId, hosted_window: true});
    assert.equal(page.uiElements.bounds, baseline.uiElements.bounds);
    assert.equal(page.uiElements.properties.isOffscreen, baseline.uiElements.properties.isOffscreen);
    assert.equal(page.uiElements.children.length, baseline.uiElements.children.length);
    windows('tap', {window_id: identity.windowId, hosted_window: true, x: identity.x, y: identity.y});
    const clicked = await ready(source);
    assert.equal(clicked.count, 1);
    assert.notEqual(String(clicked.foreground), identity.windowId, 'Hosted input must not activate the source');
    assert.notEqual(String(clicked.foreground), hosted.windowId, 'Hosted input must not activate the preview');
    const after = nativeState(identity.windowId, ...point, visual, corners);
    assert.deepEqual(after.pixel, [0,255,0], 'The preview must remain live while the visible source is occluded');
    assert.equal(after.style, original.style);
    assert.equal(after.alpha, original.alpha);
    assert.deepEqual(after.frame, original.frame);
    checkCorners(after);
    assert.deepEqual(pngSize(windows('capture_screenshot', {window_id: identity.windowId, hosted_window: true})), pngSize(windows('capture_screenshot', {window_id: identity.windowId})));
    await close(preview);
    const closed = nativeState(identity.windowId);
    assert.equal(closed.style, original.style, 'Closing the preview must leave the source styles intact');
    assert.equal(closed.visible, true);
    assert.deepEqual(closed.frame, original.frame);
  } finally {
    if (preview) await close(preview);
    await close(source);
  }
});

test('macOS excludes its viewing panel and reports preview bounds without changing source identity', () => {
  /** Creates a System Events window backed by explicit AX geometry. */
  function window(position, size) {
    const attrs = {AXPosition: position, AXSize: size, AXMinimized: false};
    return {attributes: {byName: key => ({exists: () => key in attrs, value: () => attrs[key]})}, focused: () => false, name: () => 'Source'};
  }
  const source = {unixId: () => 42, windows: () => [window([4000,80],[640,480])], bundleIdentifier: () => 'app.source', frontmost: () => false};
  const preview = {unixId: () => 99, windows: () => [window([900,20],[320,240])]};
  const context = vm.createContext({ObjC: {import() {}, unwrap: value => value}, Ref: () => [],
    Application: () => ({applicationProcesses: () => [source, preview]}), $: Object.assign(value => value, {
      AXUIElementCreateApplication: pid => pid,
      AXUIElementCopyAttributeValue: (_pid, _key, result) => { result[0] = [{hash: 7}]; return 0; }, CFHash: value => value.hash,
    })});
  vm.runInContext(readFileSync(new URL('../../hosts/macos/src/ui_automation.js', import.meta.url), 'utf8'), context);
  context.previewProcessId = 99; context.previewSourceId = '42:7';
  const entries = context.windows();
  assert.equal(entries.length, 1);
  assert.equal(entries[0].id, '42:7');
  assert.equal(context.describeWindow(entries[0]).bounds, '[900,20][1220,260]');
});

test('macOS aligns AX trees and Quartz input with the selected frame', () => {
  const events = [];
  const dollar = Object.assign(value => value, {
    CGEventCreateMouseEvent: (_, kind, point) => ({kind, point}),
    CGEventPost: (_, event) => events.push(event), CFRelease() {},
  });
  const context = vm.createContext({ObjC: {import() {}}, Application: () => ({}), $: dollar});
  vm.runInContext(readFileSync(new URL('../../hosts/macos/src/ui_automation.js', import.meta.url), 'utf8'), context);
  /** Builds native AX nodes independently of the provider's local conversion. */
  function node(attributes, children = []) {
    return {attributes: {byName: key => ({exists: () => key in attributes, value: () => attributes[key]})},
      uiElements: () => children, actions: {byName: () => ({exists: () => false})}};
  }
  context.origin = [300, -80];
  const tree = context.readNode(node({AXPosition: [300, -80], AXSize: [640, 480]}, [
    node({AXRole: 'AXButton', AXTitle: 'Save', AXPosition: [320, -50], AXSize: [80, 20]}),
  ]));
  assert.equal(tree.bounds, '[0,0][640,480]');
  assert.equal(tree.children[0].bounds, '[20,30][100,50]');
  context.mouse(1, 60, 40);
  assert.deepEqual(JSON.parse(JSON.stringify(events)), [{kind: 1, point: {x: 360, y: -40}}]);
});

test('macOS captures by exact process and frame without guessing titles', () => {
  const frame = [300, 80, 640, 480];
  let native = [{kCGWindowOwnerPID: 42, kCGWindowLayer: 0, kCGWindowNumber: 9,
    kCGWindowBounds: {X: 300, Y: 80, Width: 640, Height: 480}}];
  const context = vm.createContext({ObjC: {import() {}, deepUnwrap: value => value}, Application: () => ({}),
    $: {CGWindowListCopyWindowInfo: () => native}});
  vm.runInContext(readFileSync(new URL('../../hosts/macos/src/ui_automation.js', import.meta.url), 'utf8'), context);
  const entry = {process: {unixId: () => 42}};
  assert.equal(context.captureWindowId(entry, frame), 9);
  native = [];
  assert.throws(() => context.captureWindowId(entry, frame), /exact selected/);
});
