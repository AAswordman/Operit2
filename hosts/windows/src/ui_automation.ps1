$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Collections.Generic;
public static class OperitUiInput {
    /// Gets the system window that currently owns foreground input.
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    private delegate bool EnumWindowCallback(IntPtr window, IntPtr parameter);
    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumWindowCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr window);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowText(IntPtr window, StringBuilder text, int length);
    [DllImport("user32.dll")] private static extern int GetWindowTextLength(IntPtr window);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr window, out RECT rect);
    [StructLayout(LayoutKind.Sequential)] private struct RECT { public int left, top, right, bottom; }
    /// Enumerates native top-level windows without walking their accessibility descendants.
    public static IntPtr[] Windows() {
        var windows = new List<IntPtr>();
        if (!EnumWindows((window, parameter) => { if (IsWindowVisible(window)) windows.Add(window); return true; }, IntPtr.Zero))
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
'@
if ([OperitUiInput]::SetThreadDpiAwarenessContext([IntPtr](-4)) -eq [IntPtr]::Zero) {
    throw 'Windows could not enable physical desktop coordinates for UI automation'
}
$request = [Console]::In.ReadToEnd() | ConvertFrom-Json
$params = $request.parameters

# Describes a native window without collecting its accessibility subtree.
function Read-Window([IntPtr]$handle) {
    [uint32]$processId = 0
    $null = [OperitUiInput]::GetWindowThreadProcessId($handle, [ref]$processId)
    $process = Get-Process -Id $processId
    return @{
        windowId = $handle.ToInt64().ToString(); title = [OperitUiInput]::Title($handle)
        packageName = $process.ProcessName; bounds = [OperitUiInput]::Bounds($handle)
        isActive = $handle -eq [OperitUiInput]::GetForegroundWindow()
        isFocused = $handle -eq [OperitUiInput]::GetForegroundWindow()
        isMinimized = [OperitUiInput]::IsIconic($handle)
    }
}

if ($request.operation -eq 'list_windows') {
    $windows = @([OperitUiInput]::Windows() | ForEach-Object { Read-Window $_ })
    ConvertTo-Json -InputObject $windows -Depth 5 -Compress
    exit
}
$handle = if ($null -ne $params.window_id) { [IntPtr]([long]::Parse([string]$params.window_id)) } else { [OperitUiInput]::GetForegroundWindow() }
if ($handle -eq [IntPtr]::Zero -or ![OperitUiInput]::IsWindow($handle)) { throw 'The selected window no longer exists' }
if ($request.operation -eq 'focus_window') {
    if ([OperitUiInput]::IsIconic($handle)) { $null = [OperitUiInput]::ShowWindow($handle, 9) }
    $null = [OperitUiInput]::SetForegroundWindow($handle)
    if ([OperitUiInput]::GetForegroundWindow() -ne $handle) { throw 'Windows denied foreground activation of the selected window' }
    '{}'
    exit
}
$root = [System.Windows.Automation.AutomationElement]::FromHandle($handle)
if ($null -eq $root) { throw 'The selected window has no accessible root' }

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
        $bounds = '[{0},{1}][{2},{3}]' -f [int]$rect.Left,[int]$rect.Top,[int]$rect.Right,[int]$rect.Bottom
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
    if (![OperitUiInput]::SetCursorPos($x, $y)) { throw 'SetCursorPos failed' }
    if ($flags -ne 1) { [OperitUiInput]::Mouse($flags) }
}

switch ($request.operation) {
    'get_page_info' {
        $node = Read-UiNode $root
        $process = Get-Process -Id $root.Current.ProcessId
        $page = @{ windowId = $handle.ToInt64().ToString(); packageName = $process.ProcessName; activityName = $root.Current.Name; scope = 'desktop'; truncated = $false } | ConvertTo-Json -Compress
        $page.Substring(0, $page.Length - 1) + ',"uiElements":' + $node + '}'
        exit
    }
    'tap' { Send-Mouse $params.x $params.y 2; Send-Mouse $params.x $params.y 4 }
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
        } else { $element = [System.Windows.Automation.AutomationElement]::FocusedElement }
        if ($null -eq $element) { throw 'No matching editable element' }
        $pattern = $element.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
        $pattern.SetValue([string]$params.text)
    }
    'press_key' {
        $keys = @{ ENTER = 0x0D; TAB = 0x09; ESCAPE = 0x1B; BACKSPACE = 0x08; DELETE = 0x2E; HOME = 0x24; END = 0x23; UP = 0x26; DOWN = 0x28; LEFT = 0x25; RIGHT = 0x27 }
        $key = $keys[[string]$params.key_code]
        if ($null -eq $key) { throw 'Unsupported key; use ENTER, TAB, ESCAPE, BACKSPACE, DELETE, HOME, END, UP, DOWN, LEFT, RIGHT' }
        [OperitUiInput]::Key($key)
    }
    default { throw "Unknown UI operation: $($request.operation)" }
}
'{}'
