$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class OperitUiInput {
    /// Gets the system window that currently owns foreground input.
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
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
$root = [System.Windows.Automation.AutomationElement]::FromHandle([OperitUiInput]::GetForegroundWindow())
if ($null -eq $root) { throw 'There is no accessible foreground window' }
$script:count = 0
$script:truncated = $false

# Converts one UIA element without flattening away its accessibility hierarchy.
function Read-UiNode($element, [int]$depth) {
    $script:count++
    $current = $element.Current
    $rect = $current.BoundingRectangle
    $children = [Collections.Generic.List[object]]::new()
    $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
    $child = $walker.GetFirstChild($element)
    if ($depth -ge 40 -or $script:count -ge 4000) { $script:truncated = $script:truncated -or $null -ne $child }
    else {
        while ($null -ne $child) {
            if ($script:count -ge 4000) { $script:truncated = $true; break }
            $children.Add((Read-UiNode $child ($depth + 1)))
            $child = $walker.GetNextSibling($child)
        }
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
    }
    return @{
        className = $current.ClassName; text = $text; contentDesc = $current.HelpText
        resourceId = $current.AutomationId; bounds = $bounds
        isClickable = $current.IsEnabled -and ($element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsInvokePatternAvailableProperty) -or $element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsSelectionItemPatternAvailableProperty) -or $element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsTogglePatternAvailableProperty) -or $element.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsExpandCollapsePatternAvailableProperty))
        children = $children.ToArray()
    }
}

# Sends an absolute-position mouse button transition in desktop pixel coordinates.
function Send-Mouse([int]$x, [int]$y, [uint32]$flags) {
    if (![OperitUiInput]::SetCursorPos($x, $y)) { throw 'SetCursorPos failed' }
    if ($flags -ne 1) { [OperitUiInput]::Mouse($flags) }
}

switch ($request.operation) {
    'get_page_info' {
        $node = Read-UiNode $root 0
        $process = Get-Process -Id $root.Current.ProcessId
        @{ packageName = $process.ProcessName; activityName = $root.Current.Name; uiElements = $node; scope = 'desktop'; truncated = $script:truncated } | ConvertTo-Json -Depth 100 -Compress
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
