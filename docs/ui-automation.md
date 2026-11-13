# System window UI automation

All automation tools use `UiAutomationHost`, rather than implementing platform
dispatch in the runtime or Flutter UI. The ordinary tool catalog, JavaScript SDK
and `Automatic_ui_base` expose the same workflow:

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

## Platform status

| Host | Provider | Availability |
| --- | --- | --- |
| Windows | UI Automation + Win32 window/input APIs | Native window discovery, focus and full exposed trees; requires the interactive desktop and appropriate target privileges. |
| Linux | AT-SPI / pyatspi | Native window discovery, focus and full exposed trees; requires the session accessibility service and pyatspi. |
| macOS | AX / System Events | Native window discovery, focus and full exposed trees; requires Accessibility and Automation authorization. |
| Android | `tools/android-accessibility` companion | Native interactive-window discovery, verified input focus and full exposed trees; install the companion and authorize it in system settings. |
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
