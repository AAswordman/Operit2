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
`Tools.device(nodeId).UI`. `focus_window(window_id)` selects and retains the
target through its host. Windows, macOS and Linux/X11 open a live viewing panel without
bringing the original window in front of the user's work. A successful explicit
`get_page_info(window_id)` also retains it and opens its preview. An omitted snapshot ID keeps the current selection. Call
`focus_window()` / `Tools.UI.focusWindow()` without an ID to clear selection;
this changes automation scope and does not press Home or close any window.
A stale or invalid selected ID fails until the caller explicitly selects a
window again or clears selection. Clearing selection closes the desktop preview
before returning to the main screen. The original window stays visible and interactive.

`focus_window(close_windows=true)` / `Tools.UI.focusWindow(undefined, true)` also
releases all host-owned automation screens. Omit `window_id` for this request;
selecting a window and closing screens in the same request is rejected. Android
destroys its hosted virtual displays, tasks and floating surfaces, including
windows launched without a subsequent selection. The reusable window server stays
alive. Desktop hosts close their preview and release capture resources. Ordinary
system windows are not closed by this host-resource operation.

Selected trees, pointer input and `capture_screenshot` use the same window-local
coordinate space: the top-left of the screenshot is `(0,0)`. Window-list bounds
continue to locate the window preview on the physical desktop. Windows converts
physical UIA bounds and Win32 input relative to the native frame. macOS normalizes
Retina screenshot pixels to AX/Quartz points. Linux matches the exact process and
AT-SPI frame to an X11 drawable and captures its Composite surface, including when
covered by another window. Android
hosted windows capture the full native virtual display at its original size;
ordinary native windows use their bounds as the local origin and crop. Reading
the tree does not select a different active window implicitly.

After selection is cleared, screenshots capture the physical main surface
(the combined desktop on desktop hosts), tree bounds and input use its screen
coordinates, and snapshots inspect the active native window. `scope` is
`window` for a selected target and `desktop` for the main surface. The system
screenshot tool writes provider PNG bytes through the VFS; UI-enabled hosts use
their UI provider for screenshots as well as trees and input. Hosts without
system UI support keep their existing screen-capture capability. Capture errors
never replace a selected window with a desktop screenshot.

## Desktop viewing panels

Windows, macOS and Linux/X11 previews are rounded cards with light shadows and no system title bars,
caption buttons, drag bars or resize handles. Left-drag moves only the preview;
right-click opens actions to show the original window, enter fullscreen, dock or
restore the preview, choose its size, or close it. User clicks in the preview are
never mapped to the source. Automation still uses the selected source's full
window-local tree, screenshot and coordinates, independently of preview scaling
and the source title controls omitted from the viewing surface.

Windows renders the source client area through live DWM thumbnail rectangles.
DWM thumbnails ignore their destination HWND's region, so the helper applies the
rounded mask directly to the compositor rectangles, including subpixel edge
opacity. The same DPI-scaled outline defines the form and its separate transparent
shadow. Rectangles are rebuilt only when geometry changes; content stays live
without screenshot polling or frame readback. The source keeps its native size,
position, opacity and input styles. DWM continues rendering the preview when the
original is covered, and the user can still view and operate the original window.
UIPI still requires appropriate privileges. Hosted pointer/key events target the
source HWND hierarchy rather than moving the user's cursor; standard accessible
controls use their native invocation, selection or toggle patterns. Custom input
surfaces must support targeted native messages. A successful dispatch is followed
by a fresh snapshot to verify the application's actual response.

macOS 12.3+ uses a ScreenCaptureKit desktop-independent window stream. The source
stays visible at its original position and remains interactive. Screen Recording and Accessibility must be
authorized for the native host. The preview helper is built with the host rather
than compiled on the user's device.

Linux requires a native X11 session, an EWMH window manager, XComposite 0.2+,
GTK 3/PyGObject, Pycairo and pyatspi. The rounded RGBA panel also requires an active
compositing manager. It reads native pixels from a named window pixmap at 15 fps,
clips the image to rounded corners and paints a light alpha shadow. The source
is never hidden, moved, made transparent or made unclickable. Source resize
rebinds its pixmap without changing the application's layout. Standard control
clicks use their AT-SPI action. Other selected pointer and key events target the native source hierarchy with XSendEvent; custom apps
must accept synthetic X11 input. Editable text uses AT-SPI's EditableText API.
Wayland window previews and selected-window capture are explicitly unsupported;
the provider does not substitute a whole-desktop capture for a selected window.
On Debian/Ubuntu, the runtime packages are `python3-pyatspi`, `python3-gi`,
`python3-gi-cairo`, `python3-cairo`, `gir1.2-gtk-3.0`, `libx11-6`,
`libxcomposite1` and `at-spi2-core`. The desktop session supplies its own window
manager and compositor. `tools/tests/test_linux_window_preview.py` runs real GTK
fixtures under a private Xvfb/openbox/xcompmgr session; CI also checks occlusion,
source input, pixel alpha, EOF cleanup and source destruction without building the app.

Native stdin lifetime owns each preview: clearing selection, switching targets or
destroying the host closes its pipe and waits for capture resource cleanup. Source
destruction closes the preview. A preview closed through its context menu leaves
the selection requiring an explicit re-selection or clear before further input;
it never silently targets a desktop app. Window-list bounds locate the visible
preview for the operation indicator, while tree/screenshot/input remain local to
the original source.

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

Summaries label native `isVisible=false` nodes as `invisible` and disabled nodes
as `disabled`; the complete tree still includes them. Element clicks reject
these states and rectangles reduced to a one-pixel viewport edge. Scroll the
target into view and take a fresh snapshot before clicking. A successful input
result confirms dispatch, and the next snapshot verifies the application changed.

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
user. This tool sends an explicit `closeWindows` flag through main-runtime IPC to
the UI Host's `close_windows` action, clears selection, and waits for native screen
cleanup before dismissing the indicator. It is safe to repeat. Host cleanup errors
remain failures and can be retried. If other chats are still operating, their
screens and indicator remain until the last active operation completes. A
subsequent UI tool starts a new operation and displays the overlay
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
window for the calling chat. A snapshot without an ID preserves this selection;
clearing selection restores the full-scope indicator. Subsequent input tools retain
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
| Windows | UI Automation + Win32 + DWM | Native discovery, local trees/input, isolated window capture and live rounded viewing panels; requires the interactive desktop and appropriate target privileges. |
| Linux/X11 | AT-SPI / XComposite / GTK 3 | Local trees, selected native capture and targeted input; rounded live preview with the original window visible and interactive. Requires pyatspi, PyGObject, Pycairo, libX11, libXcomposite and a compositing EWMH window manager. Wayland capture/previews are unsupported. |
| macOS | AX / System Events / WindowServer / ScreenCaptureKit | Local trees/input, native-window screenshots normalized to AX points and live rounded viewing panels on macOS 12.3+; requires Accessibility, Automation and Screen Recording authorization. |
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

On Android 11+, an already-authorized Shizuku or Root Host launches applications
through the existing `start_app`/`app_launch` interface into independently rendered
native windows. The main APK embeds a separate uninstalled DEX payload and pushes
it to `/data/local/tmp`; that process owns the floating SurfaceView and virtual
display. Display output goes straight to the Surface, without image capture or
video encoding/decoding. Root and Shizuku share the existing shell-identity
boundary, including Root dropping to UID/GID 2000. The accessibility companion
continues to supply trees and gestures through its existing pipe.

The owner passes its existing native window-session Binder to the tmp process,
because Android 15 requires a registered app process to open that session. Native
chrome uses the owner's existing overlay permission; Views, touch handling and
Surface rendering execute in the shell process. Android security screens may hide
application overlays according to system policy.

Use the same `list_windows`, `focus_window`, snapshots and input tools on these
windows. Opaque window IDs survive minimization and restore; closed IDs fail.
Selecting a hosted window and sending input operate on its original virtual
display, including while its preview is minimized. Selection, screenshots and
repeated app launches preserve the user's preview size and minimized state.
Window-list bounds describe the physical floating content area for the indicator;
tree bounds and automation coordinates refer to the virtual screen. Only the
Android Host translates window IDs and coordinates. No Android-specific tool,
transport selector or virtual-display field is added to shared contracts. See
[native Android windows](../tools/android-ui-server/README.md) for deployment,
lifecycle and device verification.

`app_launch` opening one of these windows is normal operation. Selecting the
window makes screenshots capture its native display, excluding the scaled
preview and window controls on the phone. Screenshot pixels, tree bounds and
input therefore share the same coordinates. Clear selection to capture the
physical primary screen again. The automation indicator passes through input.
The native Fullscreen action
moves existing application tasks to the primary display and closes the old
window identity; refresh `list_windows` before further automation.
