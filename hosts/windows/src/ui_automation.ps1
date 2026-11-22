$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type -ReferencedAssemblies System.Windows.Forms, System.Drawing @'
using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.Windows.Forms;
public static class OperitUiInput {
    /// Gets the system window that currently owns foreground input.
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    private delegate bool EnumWindowCallback(IntPtr window, IntPtr parameter);
    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumWindowCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr window);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowText(IntPtr window, StringBuilder text, int length);
    [DllImport("user32.dll")] private static extern int GetWindowTextLength(IntPtr window);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr window, out RECT rect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr device, uint flags);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr GetProp(IntPtr window, string name);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern bool SetProp(IntPtr window, string name, IntPtr value);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr RemoveProp(IntPtr window, string name);
    [DllImport("user32.dll")] private static extern bool ScreenToClient(IntPtr window, ref POINT point);
    [DllImport("user32.dll")] private static extern bool ClientToScreen(IntPtr window, ref POINT point);
    [DllImport("user32.dll")] private static extern IntPtr GetWindow(IntPtr parent, uint command);
    [DllImport("user32.dll")] private static extern bool IsWindowEnabled(IntPtr window);
    [DllImport("user32.dll")] private static extern bool IsChild(IntPtr parent, IntPtr child);
    [DllImport("user32.dll")] private static extern bool PostMessage(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")] private static extern bool GetGUIThreadInfo(uint thread, ref GUIINFO info);
    [DllImport("user32.dll")] private static extern uint MapVirtualKey(uint code, uint type);
    [DllImport("user32.dll")] private static extern bool GetClientRect(IntPtr window, out RECT rect);
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x, y; }
    [StructLayout(LayoutKind.Sequential)] private struct GUIINFO {
        public uint size, flags; public IntPtr active, focus, capture, menu, move, caret; public RECT caretRect;
    }
    [StructLayout(LayoutKind.Sequential)] private struct RECT { public int left, top, right, bottom; }
    /// Enumerates native top-level windows without walking their accessibility descendants.
    public static IntPtr[] Windows() {
        var windows = new List<IntPtr>();
        if (!EnumWindows((window, parameter) => { if (IsWindowVisible(window) || GetProp(window, "Operit.Automation.Hosted") != IntPtr.Zero) windows.Add(window); return true; }, IntPtr.Zero))
            throw new InvalidOperationException("Windows could not enumerate desktop windows");
        return windows.ToArray();
    }
    /// Reads a native title without requiring the target accessibility provider to respond.
    public static string Title(IntPtr window) {
        var text = new StringBuilder(GetWindowTextLength(window) + 1);
        GetWindowText(window, text, text.Capacity);
        return text.ToString();
    }
    /// Reads bounds in physical desktop coordinates.
    public static string Bounds(IntPtr window) {
        RECT rect;
        if (!GetWindowRect(window, out rect)) throw new InvalidOperationException("Window bounds are unavailable");
        return String.Format("[{0},{1}][{2},{3}]", rect.left, rect.top, rect.right, rect.bottom);
    }
    /// Reads the exact frame used for local tree, input and screenshot coordinates.
    public static int[] Frame(IntPtr window) {
        RECT rect;
        if (!GetWindowRect(window, out rect)) throw new InvalidOperationException("Window bounds are unavailable");
        return new[] { rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top };
    }
    /// Locates the rendered preview content for indicators while keeping automation source-local.
    public static string VisualBounds(IntPtr window) {
        var preview = GetProp(window, "Operit.Automation.Hosted");
        if (preview == IntPtr.Zero) return Bounds(window);
        RECT client;
        var origin = new POINT();
        if (!GetClientRect(preview, out client) || !ClientToScreen(preview, ref origin))
            throw new InvalidOperationException("The selected window preview is unavailable");
        var frame = ClientSize(window);
        double scale = Math.Min((double)client.right / frame[2], (double)client.bottom / frame[3]);
        int width = Math.Max(1, (int)(frame[2] * scale)), height = Math.Max(1, (int)(frame[3] * scale));
        int left = origin.x + (client.right - width) / 2, top = origin.y + (client.bottom - height) / 2;
        return String.Format("[{0},{1}][{2},{3}]", left, top, left + width, top + height);
    }
    /// Reads the source client area so preview chrome never adds a second system title bar.
    public static int[] ClientSize(IntPtr window) {
        RECT rect;
        if (!GetClientRect(window, out rect)) throw new InvalidOperationException("The selected window client area is unavailable");
        return new[] { 0, 0, rect.right, rect.bottom };
    }
    /// Reads the last focused control in the target thread without adopting foreground input.
    public static IntPtr FocusedWindow(IntPtr window) {
        uint process;
        uint thread = GetWindowThreadProcessId(window, out process);
        var info = new GUIINFO { size = (uint)Marshal.SizeOf(typeof(GUIINFO)) };
        if (!GetGUIThreadInfo(thread, ref info)) throw new InvalidOperationException("Target input focus is unavailable");
        return info.focus != IntPtr.Zero && (info.focus == window || IsChild(window, info.focus)) ? info.focus : window;
    }
    private static IntPtr pointerTarget;
    /// Resolves the native child at a source-local point without hit-testing the user's desktop.
    public static IntPtr WindowTarget(IntPtr window, int x, int y) {
        var frame = Frame(window);
        if (x < 0 || y < 0 || x >= frame[2] || y >= frame[3]) throw new InvalidOperationException("Input coordinate is outside the selected window");
        var screen = new POINT { x = frame[0] + x, y = frame[1] + y };
        var target = window;
        while (true) {
            var point = screen;
            if (!ScreenToClient(target, ref point)) throw new InvalidOperationException("Target input coordinates are unavailable");
            RECT client;
            if (!GetClientRect(target, out client) || point.x < 0 || point.y < 0 || point.x >= client.right || point.y >= client.bottom)
                throw new InvalidOperationException("Use the preview's native title bar for window controls");
            var child = GetWindow(target, 5);
            while (child != IntPtr.Zero) {
                RECT rect;
                if (IsWindowVisible(child) && IsWindowEnabled(child) && GetWindowRect(child, out rect) &&
                    screen.x >= rect.left && screen.x < rect.right && screen.y >= rect.top && screen.y < rect.bottom) break;
                child = GetWindow(child, 2);
            }
            if (child == IntPtr.Zero) break;
            target = child;
        }
        return target;
    }
    /// Posts pointer events to the exact source HWND hierarchy without moving the user's cursor.
    public static void WindowMouse(IntPtr window, int x, int y, uint flags) {
        var frame = Frame(window);
        var screen = new POINT { x = frame[0] + x, y = frame[1] + y };
        var target = pointerTarget != IntPtr.Zero && IsWindow(pointerTarget) ? pointerTarget : WindowTarget(window, x, y);
        var local = screen;
        if (!ScreenToClient(target, ref local)) throw new InvalidOperationException("Target input coordinates are unavailable");
        if (local.x < short.MinValue || local.x > short.MaxValue || local.y < short.MinValue || local.y > short.MaxValue)
            throw new InvalidOperationException("Target input coordinates exceed Windows message limits");
        var position = new IntPtr((local.y & 0xffff) << 16 | (local.x & 0xffff));
        uint message = flags == 2 ? 0x0201u : flags == 4 ? 0x0202u : 0x0200u;
        if (flags == 2) pointerTarget = target;
        if (!PostMessage(target, message, new IntPtr(flags == 4 ? 0 : pointerTarget == IntPtr.Zero ? 0 : 1), position))
            throw new InvalidOperationException("Windows rejected input for the hosted source");
        if (flags == 4) pointerTarget = IntPtr.Zero;
    }
    /// Posts a named key to the source thread's focused control while leaving user focus unchanged.
    public static void WindowKey(IntPtr window, ushort key) {
        var target = FocusedWindow(window);
        int scan = (int)MapVirtualKey(key, 0) << 16;
        if (!PostMessage(target, 0x0100, new IntPtr(key), new IntPtr(scan | 1)) ||
            !PostMessage(target, 0x0101, new IntPtr(key), new IntPtr(unchecked((int)0xc0000001) | scan)))
            throw new InvalidOperationException("Windows rejected the hosted key event");
    }
    /// Positions the pointer using the calling thread's desktop coordinate space.
    [DllImport("user32.dll", SetLastError = true)] public static extern bool SetCursorPos(int x, int y);
    /// Selects physical per-monitor coordinates for the provider thread.
    [DllImport("user32.dll", SetLastError = true)] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    /// Submits an ordered input batch and returns the accepted event count.
    [DllImport("user32.dll", SetLastError = true)] private static extern uint SendInput(uint count, INPUT[] input, int size);
    [StructLayout(LayoutKind.Sequential)] private struct INPUT { public uint type; public INPUTDATA data; }
    [StructLayout(LayoutKind.Explicit)] private struct INPUTDATA {
        [FieldOffset(0)] public MOUSEINPUT mouse;
        [FieldOffset(0)] public KEYBDINPUT keyboard;
    }
    [StructLayout(LayoutKind.Sequential)] private struct MOUSEINPUT {
        public int dx, dy; public uint mouseData, flags, time; public UIntPtr extra;
    }
    [StructLayout(LayoutKind.Sequential)] private struct KEYBDINPUT {
        public ushort key, scan; public uint flags, time; public UIntPtr extra;
    }
    /// Sends input only when Windows accepts every event in the batch.
    private static void Send(INPUT[] events) {
        if (SendInput((uint)events.Length, events, Marshal.SizeOf(typeof(INPUT))) != events.Length)
            throw new InvalidOperationException("Windows rejected UI input; check target process privileges and the interactive desktop");
    }
    /// Sends a pointer button transition after the caller positions the cursor.
    public static void Mouse(uint flags) {
        Send(new[] { new INPUT { type = 0, data = new INPUTDATA { mouse = new MOUSEINPUT { flags = flags } } } });
    }
    /// Sends one named virtual key press and release in a single ordered batch.
    public static void Key(ushort key) {
        Send(new[] {
            new INPUT { type = 1, data = new INPUTDATA { keyboard = new KEYBDINPUT { key = key } } },
            new INPUT { type = 1, data = new INPUTDATA { keyboard = new KEYBDINPUT { key = key, flags = 2 } } }
        });
    }
}

/// Renders the DWM source directly while leaving the original window visible and interactive.
public sealed class OperitWindowPreview : System.Windows.Forms.Form {
    private readonly IntPtr source;
    private OperitRoundedThumbnail thumbnail;
    private readonly System.Windows.Forms.Timer lifetime = new System.Windows.Forms.Timer();
    private readonly System.Threading.Tasks.Task<string> input;
    private bool ownsSource;
    private OperitPreviewShadow shadow;
    private readonly ContextMenuStrip actions = new ContextMenuStrip();
    private bool dragging, docked, openOriginal, maximizeOriginal;
    private Point dragStart, dragOrigin;
    private System.Drawing.Size expandedSize;
    /// Scales chrome in display-independent units rather than fixed physical pixels.
    public float DisplayScale { get { return OperitUiInput.GetDpiForWindow(Handle) / 96f; } }
    /// Shares one visible radius between the live thumbnail mask and its shadow silhouette.
    public float CornerRadius { get { return Math.Min(22f * DisplayScale, Math.Min(Width, Height) / 2f); } }
    protected override bool ShowWithoutActivation { get { return true; } }
    /// Keeps the preview visible without taking keyboard or pointer ownership away from the user.
    protected override System.Windows.Forms.CreateParams CreateParams {
        get { var value = base.CreateParams; value.ExStyle |= 0x08000080; return value; }
    }
    /// Owns the exact source and the parent pipe, with no full-desktop capture in the rendering path.
    public OperitWindowPreview(IntPtr window) {
        source = window;
        Text = "Operit · " + OperitUiInput.Title(window);
        TopMost = true; ShowInTaskbar = false; MaximizeBox = false; MinimizeBox = false;
        FormBorderStyle = System.Windows.Forms.FormBorderStyle.None;
        StartPosition = System.Windows.Forms.FormStartPosition.Manual;
        var bounds = System.Windows.Forms.Screen.PrimaryScreen.WorkingArea;
        SetPreviewWidth(320);
        Location = new System.Drawing.Point(bounds.Right - Width - 16, bounds.Top + 16);
        BackColor = System.Drawing.Color.FromArgb(24, 24, 24);
        actions.ShowImageMargin = false;
        actions.BackColor = Color.FromArgb(37, 42, 51); actions.ForeColor = Color.FromArgb(241, 243, 248);
        actions.Items.Add("Open original window", null, (sender, e) => { openOriginal = true; Close(); });
        actions.Items.Add("Fullscreen", null, (sender, e) => { openOriginal = true; maximizeOriginal = true; Close(); });
        var minimize = actions.Items.Add("Minimize preview", null, (sender, e) => {
            if (docked) { docked = false; ClientSize = expandedSize; }
            else { expandedSize = ClientSize; docked = true; ClientSize = new System.Drawing.Size(64, 64);
                var area = Screen.PrimaryScreen.WorkingArea; Location = new Point(area.Right - Width - 16, area.Top + 16); }
        });
        var sizes = new ToolStripMenuItem("Preview size");
        foreach (int width in new[] { 240, 320, 480 }) {
            int selectedWidth = width;
            sizes.DropDownItems.Add(width + " px", null, (sender, e) => { docked = false; SetPreviewWidth(selectedWidth); });
        }
        actions.Items.Add(sizes); actions.Items.Add(new ToolStripSeparator());
        actions.Items.Add("Close preview", null, (sender, e) => Close());
        actions.Opening += (sender, e) => { minimize.Text = docked ? "Restore preview" : "Minimize preview"; };
        ContextMenuStrip = actions;
        input = System.Threading.Tasks.Task.Factory.StartNew(() => Console.In.ReadLine());
        lifetime.Interval = 50;
        lifetime.Tick += OnLifetime;
    }
    /// Changes only preview geometry; the application's native content size remains stable.
    private void SetPreviewWidth(int width) {
        var client = OperitUiInput.ClientSize(source);
        if (client[2] <= 0 || client[3] <= 0) throw new InvalidOperationException("The source has no drawable client area");
        double scale = Math.Min((double)width / client[2], Screen.PrimaryScreen.WorkingArea.Height * 0.7 / client[3]);
        ClientSize = new System.Drawing.Size(Math.Max(64, (int)(client[2] * scale)), Math.Max(64, (int)(client[3] * scale)));
    }
    /// Registers the live thumbnail without changing source visibility, position or input styles.
    protected override void OnShown(EventArgs e) {
        base.OnShown(e);
        BeginInvoke(new Action(StartPreview));
    }
    /// Publishes readiness only after the live viewing panel is visible.
    private void StartPreview() {
        try {
            if (OperitUiInput.GetProp(source, "Operit.Automation.Hosted") != IntPtr.Zero)
                throw new InvalidOperationException("The source window already belongs to a preview");
            thumbnail = new OperitRoundedThumbnail(Handle, source);
            UpdateThumbnail();
            if (!OperitUiInput.SetProp(source, "Operit.Automation.Hosted", Handle))
                throw new InvalidOperationException("Windows denied preview ownership");
            ownsSource = true;
            if (!OperitUiInput.SetProp(Handle, "Operit.Automation.Preview", new IntPtr(1)))
                throw new InvalidOperationException("Windows denied preview identity");
            OperitUiInput.ShowWindow(Handle, 4);
            shadow = new OperitPreviewShadow(this);
            shadow.Show(this);
            shadow.RefreshShadow();
            lifetime.Start();
            Console.WriteLine("{\"ready\":true,\"windowId\":\"" + Handle.ToInt64() + "\",\"bounds\":\"" + OperitUiInput.Bounds(Handle) + "\"}"); Console.Out.Flush();
        } catch (Exception error) {
            string message = error.Message.Replace("\\", "\\\\").Replace("\"", "\\\"").Replace("\r", "\\r").Replace("\n", "\\n");
            Console.WriteLine("{\"ready\":false,\"error\":\"" + message + "\"}"); Console.Out.Flush();
            Console.Error.WriteLine(error); Close();
        }
    }
    /// Keeps source size and preview geometry synchronized without a screenshot polling loop.
    private void UpdateThumbnail() {
        if (thumbnail == null) return;
        var client = OperitUiInput.ClientSize(source);
        var size = new System.Drawing.Size(client[2], client[3]);
        if (size.Width <= 0 || size.Height <= 0) throw new InvalidOperationException("The source window has no drawable frame");
        double scale = Math.Min((double)ClientSize.Width / size.Width, (double)ClientSize.Height / size.Height);
        int width = Math.Max(1, (int)(size.Width * scale)), height = Math.Max(1, (int)(size.Height * scale));
        var destination = new Rectangle((ClientSize.Width - width) / 2, (ClientSize.Height - height) / 2, width, height);
        thumbnail.Update(destination, size, ClientSize, CornerRadius);
    }
    /// Closes after source destruction or parent death and refreshes only native thumbnail geometry.
    private void OnLifetime(object sender, EventArgs e) {
        if (input.IsCompleted || !OperitUiInput.IsWindow(source)) { Close(); return; }
        try { UpdateThumbnail(); } catch (Exception error) { Console.Error.WriteLine(error); Close(); }
    }
    /// Refits the direct DWM layer when the small preview is resized.
    protected override void OnResize(EventArgs e) {
        base.OnResize(e);
        ApplyClip();
        if (thumbnail != null) UpdateThumbnail();
        if (shadow != null) shadow.RefreshShadow();
    }
    /// Clips the form background and input region; DWM content has its own explicit rounded mask.
    private void ApplyClip() {
        if (Width > 0 && Height > 0) {
            using (var shape = OperitPreviewShadow.Rounded(new RectangleF(0, 0, Width, Height), CornerRadius)) {
                var previous = Region; Region = new Region(shape); if (previous != null) previous.Dispose();
            }
        }
    }
    /// Rebuilds both rounded surfaces when dragging crosses monitors with different DPI.
    protected override void WndProc(ref Message message) {
        base.WndProc(ref message);
        if (message.Msg == 0x02e0) { ApplyClip(); UpdateThumbnail(); if (shadow != null) shadow.RefreshShadow(); }
    }
    /// Moves the separate per-pixel shadow together with the borderless preview.
    protected override void OnLocationChanged(EventArgs e) {
        base.OnLocationChanged(e); if (shadow != null) shadow.Follow();
    }
    /// Starts a local drag without forwarding the user's click to the source application.
    protected override void OnMouseDown(MouseEventArgs e) {
        base.OnMouseDown(e);
        if (e.Button == MouseButtons.Left) { dragging = true; dragStart = MousePosition; dragOrigin = Location; Capture = true; }
    }
    /// Drags the preview using screen deltas without resizing or activating its source.
    protected override void OnMouseMove(MouseEventArgs e) {
        base.OnMouseMove(e);
        if (dragging) Location = new Point(dragOrigin.X + MousePosition.X - dragStart.X, dragOrigin.Y + MousePosition.Y - dragStart.Y);
    }
    /// Releases local drag capture after the left button is released.
    protected override void OnMouseUp(MouseEventArgs e) {
        base.OnMouseUp(e); if (e.Button == MouseButtons.Left) { dragging = false; Capture = false; }
    }
    /// Releases only preview resources when the owner disconnects or the user closes the preview.
    protected override void OnFormClosed(System.Windows.Forms.FormClosedEventArgs e) {
        lifetime.Stop(); lifetime.Dispose();
        if (shadow != null) { shadow.Close(); shadow.Dispose(); shadow = null; }
        actions.Dispose();
        if (ownsSource && OperitUiInput.IsWindow(source)) OperitUiInput.RemoveProp(source, "Operit.Automation.Hosted");
        if (thumbnail != null) { thumbnail.Dispose(); thumbnail = null; }
        if (openOriginal && OperitUiInput.IsWindow(source)) {
            OperitUiInput.ShowWindow(source, maximizeOriginal ? 3 : 9);
            OperitUiInput.SetForegroundWindow(source);
        }
        base.OnFormClosed(e);
    }
}

/// Masks the live compositor content explicitly because DWM thumbnails ignore the destination HWND region.
public sealed class OperitRoundedThumbnail : IDisposable {
    [StructLayout(LayoutKind.Sequential)] private struct RECT { public int left, top, right, bottom; }
    [StructLayout(LayoutKind.Sequential)] private struct PROPS {
        public uint flags; public RECT destination, source; public byte opacity;
        [MarshalAs(UnmanagedType.Bool)] public bool visible;
        [MarshalAs(UnmanagedType.Bool)] public bool clientOnly;
    }
    private sealed class Slice { public Rectangle bounds; public byte opacity; }
    [DllImport("dwmapi.dll")] private static extern int DwmRegisterThumbnail(IntPtr destination, IntPtr source, out IntPtr thumbnail);
    [DllImport("dwmapi.dll")] private static extern int DwmUpdateThumbnailProperties(IntPtr thumbnail, ref PROPS properties);
    [DllImport("dwmapi.dll")] private static extern int DwmUnregisterThumbnail(IntPtr thumbnail);
    private readonly IntPtr window, source;
    private readonly System.Collections.Generic.List<IntPtr> thumbnails = new System.Collections.Generic.List<IntPtr>();
    private Rectangle destination;
    private System.Drawing.Size sourceSize, canvas;
    private float radius;
    /// Binds every masked rectangle to the same live source surface.
    public OperitRoundedThumbnail(IntPtr window, IntPtr source) { this.window = window; this.source = source; }
    /// Preserves native failure details and leaves capture cleanup to the owning preview.
    private static void Check(int result) { if (result < 0) Marshal.ThrowExceptionForHR(result); }
    /// Measures edge coverage in the outer card coordinate space with sixteen subpixel samples.
    private static byte Coverage(int x, int y, System.Drawing.Size canvas, float radius) {
        if ((x >= radius && x + 1 <= canvas.Width - radius) || (y >= radius && y + 1 <= canvas.Height - radius)) return 255;
        int covered = 0;
        for (int sy = 0; sy < 4; sy++) for (int sx = 0; sx < 4; sx++) {
            double px = x + (sx + 0.5) / 4, py = y + (sy + 0.5) / 4;
            double dx = Math.Max(0, Math.Max(radius - px, px - (canvas.Width - radius)));
            double dy = Math.Max(0, Math.Max(radius - py, py - (canvas.Height - radius)));
            if (dx * dx + dy * dy <= radius * radius) covered++;
        }
        return (byte)((covered * 255 + 8) / 16);
    }
    /// Coalesces adjacent rows with identical coverage to keep the live compositor graph small.
    private static System.Collections.Generic.List<Slice> Mask(Rectangle destination, System.Drawing.Size canvas, float radius) {
        var slices = new System.Collections.Generic.List<Slice>();
        var previous = new System.Collections.Generic.Dictionary<Tuple<int, int, byte>, Slice>();
        for (int y = destination.Top; y < destination.Bottom; y++) {
            for (int left = destination.Left; left < destination.Right;) {
                byte opacity = Coverage(left, y, canvas, radius);
                int right = left + 1;
                while (right < destination.Right && Coverage(right, y, canvas, radius) == opacity) right++;
                if (opacity != 0) {
                    var key = Tuple.Create(left, right, opacity);
                    Slice slice;
                    if (previous.TryGetValue(key, out slice) && slice.bounds.Bottom == y) slice.bounds.Height++;
                    else {
                        slice = new Slice { bounds = new Rectangle(left, y, right - left, 1), opacity = opacity };
                        slices.Add(slice); previous[key] = slice;
                    }
                }
                left = right;
            }
        }
        return slices;
    }
    /// Rebuilds only when geometry changes; each DWM rectangle remains live without frame polling or readback.
    public void Update(Rectangle destination, System.Drawing.Size sourceSize, System.Drawing.Size canvas, float radius) {
        if (this.destination == destination && this.sourceSize == sourceSize && this.canvas == canvas && this.radius == radius) return;
        Dispose();
        foreach (var slice in Mask(destination, canvas, radius)) {
            var bounds = slice.bounds;
            var crop = new RECT {
                left = (bounds.Left - destination.Left) * sourceSize.Width / destination.Width,
                top = (bounds.Top - destination.Top) * sourceSize.Height / destination.Height,
                right = Math.Min(sourceSize.Width, (int)Math.Ceiling((double)(bounds.Right - destination.Left) * sourceSize.Width / destination.Width)),
                bottom = Math.Min(sourceSize.Height, (int)Math.Ceiling((double)(bounds.Bottom - destination.Top) * sourceSize.Height / destination.Height))
            };
            IntPtr thumbnail; Check(DwmRegisterThumbnail(window, source, out thumbnail)); thumbnails.Add(thumbnail);
            var properties = new PROPS { flags = 31, source = crop, opacity = slice.opacity, visible = true, clientOnly = true,
                destination = new RECT { left = bounds.Left, top = bounds.Top, right = bounds.Right, bottom = bounds.Bottom } };
            Check(DwmUpdateThumbnailProperties(thumbnail, ref properties));
        }
        this.destination = destination; this.sourceSize = sourceSize; this.canvas = canvas; this.radius = radius;
    }
    /// Releases every native relationship before the preview HWND is destroyed.
    public void Dispose() {
        foreach (var thumbnail in thumbnails) DwmUnregisterThumbnail(thumbnail);
        thumbnails.Clear(); destination = Rectangle.Empty;
    }
}

/// Draws a soft native per-pixel shadow without a title bar, resize border or extra input surface.
public sealed class OperitPreviewShadow : Form {
    [StructLayout(LayoutKind.Sequential)] private struct POINT { public int x, y; }
    [StructLayout(LayoutKind.Sequential)] private struct SIZE { public int width, height; }
    [StructLayout(LayoutKind.Sequential, Pack = 1)] private struct BLEND { public byte operation, flags, alpha, format; }
    [DllImport("user32.dll")] private static extern IntPtr GetDC(IntPtr window);
    [DllImport("user32.dll")] private static extern int ReleaseDC(IntPtr window, IntPtr device);
    [DllImport("gdi32.dll")] private static extern IntPtr CreateCompatibleDC(IntPtr device);
    [DllImport("gdi32.dll")] private static extern bool DeleteDC(IntPtr device);
    [DllImport("gdi32.dll")] private static extern IntPtr SelectObject(IntPtr device, IntPtr value);
    [DllImport("gdi32.dll")] private static extern bool DeleteObject(IntPtr value);
    [DllImport("user32.dll", SetLastError = true)] private static extern bool UpdateLayeredWindow(IntPtr window, IntPtr device,
        ref POINT position, ref SIZE size, IntPtr source, ref POINT origin, uint color, ref BLEND blend, uint flags);
    private readonly OperitWindowPreview card;
    private int padding;
    protected override bool ShowWithoutActivation { get { return true; } }
    /// Keeps decoration completely outside native input and task-switcher enumeration.
    protected override CreateParams CreateParams {
        get { var value = base.CreateParams; value.ExStyle |= 0x080800A0; return value; }
    }
    /// Attaches decoration to a single card and excludes it from automation targets.
    public OperitPreviewShadow(OperitWindowPreview owner) {
        card = owner; FormBorderStyle = FormBorderStyle.None; ShowInTaskbar = false; TopMost = true;
        StartPosition = FormStartPosition.Manual;
        OperitUiInput.SetProp(Handle, "Operit.Automation.Preview", new IntPtr(1));
    }
    /// Creates the same rounded card silhouette for native clipping and its soft shadow.
    public static GraphicsPath Rounded(RectangleF rect, float radius) {
        float diameter = Math.Min(radius * 2, Math.Min(rect.Width, rect.Height));
        var path = new GraphicsPath();
        path.AddArc(rect.Left, rect.Top, diameter, diameter, 180, 90);
        path.AddArc(rect.Right - diameter, rect.Top, diameter, diameter, 270, 90);
        path.AddArc(rect.Right - diameter, rect.Bottom - diameter, diameter, diameter, 0, 90);
        path.AddArc(rect.Left, rect.Bottom - diameter, diameter, diameter, 90, 90); path.CloseFigure(); return path;
    }
    /// Moves the cached shadow without allocating a new image on each pointer movement.
    public void Follow() { Location = new Point(card.Left - padding, card.Top - padding); }
    /// Uploads premultiplied decoration only after preview size changes.
    public void RefreshShadow() {
        padding = (int)Math.Ceiling(18 * card.DisplayScale);
        Follow(); Size = new System.Drawing.Size(card.Width + padding * 2, card.Height + padding * 2);
        using (var bitmap = new Bitmap(Width, Height, PixelFormat.Format32bppPArgb)) {
            using (var graphics = Graphics.FromImage(bitmap)) {
                graphics.SmoothingMode = SmoothingMode.AntiAlias;
                for (int step = 12; step >= 1; step--) {
                    float spread = step * card.DisplayScale;
                    using (var path = Rounded(new RectangleF(padding - spread, padding - spread + 2 * card.DisplayScale, card.Width + spread * 2, card.Height + spread * 2), card.CornerRadius + spread))
                    using (var brush = new SolidBrush(Color.FromArgb(2, 0, 0, 0))) graphics.FillPath(brush, path);
                }
                graphics.CompositingMode = CompositingMode.SourceCopy;
                using (var path = Rounded(new RectangleF(padding, padding, card.Width, card.Height), card.CornerRadius))
                using (var brush = new SolidBrush(Color.FromArgb(0, 0, 0, 0))) graphics.FillPath(brush, path);
            }
            var screen = GetDC(IntPtr.Zero); var memory = CreateCompatibleDC(screen);
            var native = bitmap.GetHbitmap(Color.FromArgb(0)); var previous = SelectObject(memory, native);
            try {
                var position = new POINT { x = Left, y = Top }; var size = new SIZE { width = Width, height = Height };
                var origin = new POINT(); var blend = new BLEND { alpha = 255, format = 1 };
                if (!UpdateLayeredWindow(Handle, screen, ref position, ref size, memory, ref origin, 0, ref blend, 2))
                    throw new InvalidOperationException("Windows could not render the preview shadow");
            } finally { SelectObject(memory, previous); DeleteObject(native); DeleteDC(memory); ReleaseDC(IntPtr.Zero, screen); }
        }
    }
}
'@
if ([OperitUiInput]::SetThreadDpiAwarenessContext([IntPtr](-4)) -eq [IntPtr]::Zero) {
    throw 'Windows could not enable physical desktop coordinates for UI automation'
}
$request = [Console]::In.ReadLine() | ConvertFrom-Json
$params = $request.parameters

# Describes a native window without collecting its accessibility subtree.
function Read-Window([IntPtr]$handle) {
    [uint32]$processId = 0
    $null = [OperitUiInput]::GetWindowThreadProcessId($handle, [ref]$processId)
    $process = Get-Process -Id $processId
    return @{
        windowId = $handle.ToInt64().ToString(); title = [OperitUiInput]::Title($handle)
        packageName = $process.ProcessName; bounds = [OperitUiInput]::VisualBounds($handle)
        isActive = $handle -eq [OperitUiInput]::GetForegroundWindow()
        isFocused = $handle -eq [OperitUiInput]::GetForegroundWindow()
        isMinimized = [OperitUiInput]::IsIconic($handle)
    }
}

if ($request.operation -eq 'list_windows') {
    $windows = @([OperitUiInput]::Windows() | Where-Object { [OperitUiInput]::GetProp($_, 'Operit.Automation.Preview') -eq [IntPtr]::Zero } | ForEach-Object { Read-Window $_ })
    ConvertTo-Json -InputObject $windows -Depth 5 -Compress
    exit
}
$selected = $null -ne $params.window_id
$desktop = [System.Windows.Forms.SystemInformation]::VirtualScreen
if ($request.operation -eq 'capture_screenshot' -and !$selected) {
    $bitmap = [System.Drawing.Bitmap]::new($desktop.Width, $desktop.Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $stream = [System.IO.MemoryStream]::new()
    try {
        $graphics.CopyFromScreen($desktop.Left, $desktop.Top, 0, 0, $desktop.Size)
        $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
        @{ pngBase64 = [Convert]::ToBase64String($stream.ToArray()) } | ConvertTo-Json -Compress
    } finally { $stream.Dispose(); $graphics.Dispose(); $bitmap.Dispose() }
    exit
}
$handle = if ($null -ne $params.window_id) { [IntPtr]([long]::Parse([string]$params.window_id)) } else { [OperitUiInput]::GetForegroundWindow() }
if ($handle -eq [IntPtr]::Zero -or ![OperitUiInput]::IsWindow($handle)) { throw 'The selected window no longer exists' }
if ($request.operation -eq 'preview_window') {
    [Windows.Forms.Application]::Run([OperitWindowPreview]::new($handle))
    exit
}
if ($request.operation -eq 'focus_window') {
    if ($params.hosted_window) {
        if ([OperitUiInput]::IsIconic($handle)) { $null = [OperitUiInput]::ShowWindow($handle, 4) }
        '{}'
        exit
    }
    if ([OperitUiInput]::IsIconic($handle)) { $null = [OperitUiInput]::ShowWindow($handle, 9) }
    $null = [OperitUiInput]::SetForegroundWindow($handle)
    if ([OperitUiInput]::GetForegroundWindow() -ne $handle) { throw 'Windows denied foreground activation of the selected window' }
    '{}'
    exit
}
if ($selected -and [OperitUiInput]::IsIconic($handle)) { throw 'Focus the minimized window before inspecting or operating it' }
$frame = [OperitUiInput]::Frame($handle)
$originX = if ($selected) { $frame[0] } else { $desktop.Left }
$originY = if ($selected) { $frame[1] } else { $desktop.Top }
if ($request.operation -eq 'capture_screenshot') {
    if ($frame[2] -le 0 -or $frame[3] -le 0) { throw 'The selected window has no drawable surface' }
    $bitmap = [System.Drawing.Bitmap]::new($frame[2], $frame[3])
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $stream = [System.IO.MemoryStream]::new()
    try {
        $device = $graphics.GetHdc()
        try {
            if (![OperitUiInput]::PrintWindow($handle, $device, 2)) { throw 'Windows could not capture the selected window' }
        } finally { $graphics.ReleaseHdc($device) }
        $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
        @{ pngBase64 = [Convert]::ToBase64String($stream.ToArray()) } | ConvertTo-Json -Compress
    } finally { $stream.Dispose(); $graphics.Dispose(); $bitmap.Dispose() }
    exit
}
$root = [System.Windows.Automation.AutomationElement]::FromHandle($handle)
if ($null -eq $root) { throw 'The selected window has no accessible root' }
if ($selected -and !$params.hosted_window -and $request.operation -ne 'get_page_info' -and [OperitUiInput]::GetForegroundWindow() -ne $handle) { throw 'Focus the selected window before sending input' }

# Converts one UIA element without flattening away its accessibility hierarchy.
function Read-UiNode($element) {
    $current = $element.Current
    $rect = $current.BoundingRectangle
    $children = [Collections.Generic.List[string]]::new()
    $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
    $child = $walker.GetFirstChild($element)
    while ($null -ne $child) {
        $children.Add((Read-UiNode $child))
        $child = $walker.GetNextSibling($child)
    }
    $bounds = $null
    if (!$rect.IsEmpty -and $rect.Width -gt 0 -and $rect.Height -gt 0) {
        $bounds = '[{0},{1}][{2},{3}]' -f ([int]$rect.Left - $originX),([int]$rect.Top - $originY),([int]$rect.Right - $originX),([int]$rect.Bottom - $originY)
    }
    $text = $null
    if (!$current.IsPassword) {
        $text = $current.Name
        $valuePattern = $null
        if ($element.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$valuePattern)) {
            $value = $valuePattern.Current.Value
            if ($value.Length -gt 0 -and $value -ne $text) { $text = (@($text, $value) | Where-Object { $_.Length -gt 0 }) -join "`n" }
        }
        $textPattern = $null
        if ($element.TryGetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern, [ref]$textPattern)) {
            $documentText = $textPattern.DocumentRange.GetText(-1)
            if ($documentText.Length -gt 0 -and $documentText -ne $text) {
                $text = (@($text, $documentText) | Where-Object { $_.Length -gt 0 }) -join "`n"
            }
        }
    }
    $node = @{
        className = $current.ClassName; text = $text; contentDesc = $current.HelpText
        resourceId = $current.AutomationId; bounds = $bounds
        isClickable = $current.IsEnabled -and ($element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsInvokePatternAvailableProperty) -or $element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsSelectionItemPatternAvailableProperty) -or $element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsTogglePatternAvailableProperty) -or $element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsExpandCollapsePatternAvailableProperty))
        properties = @{
            role = $current.ControlType.ProgrammaticName; isEnabled = $current.IsEnabled
            isFocused = $current.HasKeyboardFocus; isFocusable = $current.IsKeyboardFocusable
            isOffscreen = $current.IsOffscreen; isPassword = $current.IsPassword
            runtimeId = ($element.GetRuntimeId() -join '.')
        }
    } | ConvertTo-Json -Depth 5 -Compress
    # Serialize each node independently so PowerShell's JSON depth limit cannot prune a deep tree.
    return $node.Substring(0, $node.Length - 1) + ',"children":[' + ($children -join ',') + ']}'
}

# Sends an absolute-position mouse button transition in desktop pixel coordinates.
function Send-Mouse([int]$x, [int]$y, [uint32]$flags) {
    if ($params.hosted_window) { [OperitUiInput]::WindowMouse($handle, $x, $y, $flags); return }
    if ($selected -and ($x -lt 0 -or $y -lt 0 -or $x -ge $frame[2] -or $y -ge $frame[3])) { throw 'Input coordinate is outside the selected window' }
    if (![OperitUiInput]::SetCursorPos(($x + $originX), ($y + $originY))) { throw 'SetCursorPos failed' }
    if ($flags -ne 1) { [OperitUiInput]::Mouse($flags) }
}

# Uses native semantic activation for standard controls and pointer delivery for pointer surfaces.
function Invoke-WindowTap([int]$x, [int]$y) {
    $target = [OperitUiInput]::WindowTarget($handle, $x, $y)
    $element = [System.Windows.Automation.AutomationElement]::FromHandle($target)
    switch ($element.Current.ControlType.ProgrammaticName) {
        'ControlType.Button' { $element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke(); return }
        'ControlType.CheckBox' { $element.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle(); return }
        'ControlType.RadioButton' { $element.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Select(); return }
        'ControlType.ListItem' { $element.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Select(); return }
        'ControlType.TabItem' { $element.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Select(); return }
        default { Send-Mouse $x $y 2; Send-Mouse $x $y 4 }
    }
}

switch ($request.operation) {
    'get_page_info' {
        $node = Read-UiNode $root
        $process = Get-Process -Id $root.Current.ProcessId
        $page = @{ windowId = $handle.ToInt64().ToString(); packageName = $process.ProcessName; activityName = $root.Current.Name; scope = $(if ($selected) { 'window' } else { 'desktop' }); truncated = $false } | ConvertTo-Json -Compress
        $page.Substring(0, $page.Length - 1) + ',"uiElements":' + $node + '}'
        exit
    }
    'tap' {
        if ($params.hosted_window) { Invoke-WindowTap $params.x $params.y }
        else { Send-Mouse $params.x $params.y 2; Send-Mouse $params.x $params.y 4 }
    }
    'long_press' { Send-Mouse $params.x $params.y 2; try { Start-Sleep -Milliseconds 600 } finally { Send-Mouse $params.x $params.y 4 } }
    'swipe' {
        Send-Mouse $params.start_x $params.start_y 2
        try {
            for ($step = 1; $step -le 30; $step++) {
                Send-Mouse ($params.start_x + ($params.end_x - $params.start_x) * $step / 30) ($params.start_y + ($params.end_y - $params.start_y) * $step / 30) 1
                Start-Sleep -Milliseconds ([int]($params.duration / 30))
            }
        } finally { Send-Mouse $params.end_x $params.end_y 4 }
    }
    'set_input_text' {
        if ($null -ne $params.resource_id) {
            $condition = [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::AutomationIdProperty, [string]$params.resource_id)
            $matches = $root.FindAll([System.Windows.Automation.TreeScope]::Subtree, $condition)
            if ($matches.Count -ne 1) { throw 'Select one unique input resource ID' }
            $element = $matches.Item(0)
        } elseif ($params.hosted_window) { $element = [System.Windows.Automation.AutomationElement]::FromHandle([OperitUiInput]::FocusedWindow($handle)) }
        else { $element = [System.Windows.Automation.AutomationElement]::FocusedElement }
        if ($null -eq $element) { throw 'No matching editable element' }
        $pattern = $element.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
        $pattern.SetValue([string]$params.text)
    }
    'press_key' {
        $keys = @{ ENTER = 0x0D; TAB = 0x09; ESCAPE = 0x1B; BACKSPACE = 0x08; DELETE = 0x2E; HOME = 0x24; END = 0x23; UP = 0x26; DOWN = 0x28; LEFT = 0x25; RIGHT = 0x27 }
        $key = $keys[[string]$params.key_code]
        if ($null -eq $key) { throw 'Unsupported key; use ENTER, TAB, ESCAPE, BACKSPACE, DELETE, HOME, END, UP, DOWN, LEFT, RIGHT' }
        if ($params.hosted_window) { [OperitUiInput]::WindowKey($handle, $key) }
        else { [OperitUiInput]::Key($key) }
    }
    default { throw "Unknown UI operation: $($request.operation)" }
}
'{}'
