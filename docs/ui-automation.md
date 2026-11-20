# System window UI automation

All automation tools use `UiAutomationHost` for platform dispatch. The ordinary
tool catalog, JavaScript SDK and `Automatic_ui_base` expose the same workflow:

```typescript
const {windows} = await Tools.UI.listWindows();
const target = windows.find(window => window.windowId === selectedWindowId);
if (!target) throw new Error('Refresh the system window list');
await Tools.UI.focusWindow(target.windowId);
const page = await Tools.UI.getPageInfo(target.windowId);
```

The tool names are `list_windows`, `focus_window(window_id)` and
`get_page_info(window_id?)`. The SDK also supports the same methods through
`Tools.device(nodeId).UI`. An omitted ID explicitly requests the active window;
a stale or invalid explicit ID fails and never selects a different window.
Reading a selected window does not activate it. Focus the window before using
input operations, then read a fresh tree to verify the result.

Windows have an opaque `windowId`, title, application identifier, bounds,
active/focused state and minimized state. IDs belong to the native session and
must be refreshed when a window closes. Snapshots carry their `windowId`,
`scope`, `truncated` and a nested `uiElements` tree. Nodes retain text, identifiers,
bounds, descriptions and native roles/states in `properties`. Programmatic SDK
results retain this full tree for selectors and targeted inspection. Model-facing
tool output uses `summary`: compact text bounded to 16,000 characters, with empty
containers flattened, long text shortened, key selectors/states preserved and
explicit shown/omitted node counts. `Automatic_ui_base` returns this summary and
window metadata instead of serializing `uiElements` into model context.

Providers traverse the whole exposed window subtree, without the previous
4000-node or 40-level pruning. Windows uses UIA Raw View and reads ValuePattern
and TextPattern content. Linux uses AT-SPI; macOS uses AX. The Android companion
includes non-important views, queries interactive windows across displays on
Android 11+ and streams full responses through a pipe. Password contents remain
excluded. Ordinary tool-message output limits still apply. Native provider failures and
transport resource errors are reported explicitly, never as complete partial
snapshots. Accessibility can only reveal what each application's provider
exposes; virtualized or custom-drawn content may not have materialized nodes.

## Plugin and operation overlay

Enable **UI Automation** (`com.operit.ui_automation`) in plugin management. The
built-in ToolPkg is disabled by default. Its tool subpackage retains the
`Automatic_ui_base` namespace for existing calls and permissions.

The plugin owns the status orb, the transparent animated edge glow and the
display lifecycle. There is no separate automation page or toolbox entry.
Packaged automation tools await the main runtime's overlay reconciliation before
executing device operations. Native UI tools also mark their chat through the
tool lifecycle hook. The indicator stays across successive tool calls and model
steps. After completing or abandoning UI operations, the AI must call
`Automatic_ui_base:complete_ui_operation` (no parameters) before replying to the
user. This tool releases the calling chat's operation and waits for window
cleanup; it is safe to repeat. If other chats are still operating, their indicator
remains. A subsequent UI tool starts a new operation and displays the overlay
again. Turn completion, failure, cancellation and application termination also
clean up automatically. Disabling the plugin closes its surfaces. The orb and outline share one
click-through window that leaves input and focus on the underlying application.
The edge combines flowing aurora colors, a wide inward bloom, a slow breathing
intensity and two moving perimeter glints. Its center remains transparent.
Flutter paints animation frames locally without plugin IPC and respects reduced
motion. Geometry uses the host's full display/view bounds and logical pixels;
display scaling, resolution changes and device rotation do not stretch a fixed
size image. A square perimeter fills all four corners of rectangular screens
and windows. Android fill surfaces avoid system insets and keyboard resizing;
macOS fill surfaces include the menu bar and Dock, and Windows re-fits after DPI
changes. Operating-system-owned bars and protected surfaces may still appear
above Android application overlays.

`focus_window(window_id)` and `get_page_info(window_id)` select the indicated
window for the calling chat. A snapshot without an ID also binds the packaged
tools' indicator to the actual returned window. Subsequent input tools retain
that target. Its indicator follows native window bounds, hides while the target
is minimized, closed or temporarily unavailable, and reappears when valid again.
Selected targets never fall back to another window or the full display. A shared,
non-overlapping refresh loop runs only while selected system windows are active.
Chats selecting the same window share an indicator; different targets retain
separate indicators. Until a target is selected, operations use the full scope.

For a native macOS fill-window smoke check, run from `apps/flutter/app`:
`flutter run -d macos -t tool/floating_windows_smoke.dart --no-pub --dart-define=OPERIT_FLOATING_SMOKE_FILL=true`.
It compares the window with the display's full logical size, reports the Flutter
viewport, exercises hide/show and confirms child-engine removal after closing.
Use `OPERIT_FLOATING_SMOKE_REGION=true` instead of the fill define to check native
screen-coordinate positioning, movement, resizing and Retina viewport sizing.

| Window scope | When and where the indicator appears |
| --- | --- |
| System (Windows, macOS, Linux/X11 with composition, Android with permission) | Marks the selected native window; covers the primary display until a target is selected. Switching applications keeps it visible. System surfaces and system security restrictions still apply. |
| Application (iOS, OpenHarmony, Web, Linux/Wayland) | Covers the Operit view while Operit is active. This scope is limited to the app's own view. |

On Android, grant the overlay permission through the application's existing
onboarding permission flow or Android system settings. Automation never opens a
permission prompt by itself. Unsupported scopes or missing overlay permission
do not open an indicator. The overlay uses the shared
[floating-window API](../plugins/docs/FLOATING_WINDOWS.md). Window presentation
and the device's accessibility provider are separate host capabilities.

## Platform status

| Host | Provider | Availability |
| --- | --- | --- |
| Windows | UI Automation + Win32 window/input APIs | Native window discovery, focus and full exposed trees; requires the interactive desktop and appropriate target privileges. |
| Linux | AT-SPI / pyatspi | Native window discovery, focus and full exposed trees; requires the session accessibility service and pyatspi. |
| macOS | AX / System Events | Native window discovery, focus and full exposed trees; requires Accessibility and Automation authorization. |
| Android | `tools/android-accessibility` companion | Native interactive-window discovery, verified input focus and full exposed trees; the main APK embeds the signed companion. Use UI automation's grant action to install it and open system accessibility settings. |
| OpenHarmony | No system accessibility Host registered | Modern window enumeration and element APIs require system API access and `ohos.permission.ACCESSIBILITY_EXTENSION_ABILITY`; a normally signed application cannot call them. The earlier public window/root methods were removed from API 12. A system-authorized extension and transport are still required. |
| iOS | No system accessibility Host registered | Ordinary application APIs do not expose other applications' native window trees or input control. A separately authorized external test/automation service is required for device-wide operation. |
| Web | No system accessibility Host registered | Browser sandbox APIs cannot enumerate/control other applications' native windows. Device-wide automation must execute on a connected native host. |

Unsupported hosts report an explicit unsupported-provider error. An app-internal
widget tree is not registered as a device-wide provider.

The OpenHarmony restrictions are described in the official
[public context API](https://github.com/openharmony/docs/blob/master/en/application-dev/reference/apis-accessibility-kit/js-apis-inner-application-accessibilityExtensionContext.md)
and [system context API](https://github.com/openharmony/docs/blob/master/en/application-dev/reference/apis-accessibility-kit/js-apis-inner-application-accessibilityExtensionContext-sys.md).

See [Android companion](../tools/android-accessibility/README.md) for its build,
authorization, signing and AIDL protocol.
