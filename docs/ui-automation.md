# UI automation Host boundary

`Tools.UI.getPageInfo()` now reaches a registered executor and the owning
`UiAutomationHost`. The runtime and plugin SDK do not choose an OS backend.
Platform Host assembly installs that backend. `Tools.device(nodeId).UI` uses
the ordinary device tool transport and inspects the selected node.

Host input crosses the Rust capability boundary as the `UiAction` enum.
Every registered provider operates on the device's system accessibility surface,
including other applications. There is no Flutter semantics provider, application
input injection, or application-only UI channel. A host without a system provider
leaves the capability unregistered and tools report an unsupported-operation error.

The snapshot preserves the existing plugin format:

```ts
const page = await Tools.UI.getPageInfo();
// {packageName, activityName, uiElements, scope, truncated}
const root = UINode.fromPageInfo(page);
const button = root.findByText("Save");
if (button) await button.click();
```

`uiElements` contains `className`, `text`, `contentDesc`, `resourceId`, `bounds`,
`isClickable`, and recursive `children`. Missing attributes are `null`.
`bounds` uses `[left,top][right,bottom]`. The same coordinates must be used for
input on that provider: Windows and Android use desktop/screen pixels, macOS
uses desktop points, and AT-SPI uses desktop coordinates. `packageName` identifies
the foreground process/application; `activityName` identifies its active window.

| Owner | Provider | Reported scope | Requirements |
| --- | --- | --- | --- |
| Android Flutter | Legacy external provider over AIDL | `desktop` (active device surface) | Install the existing `com.ai.assistance.operit.provider` APK and enable its accessibility service |
| Windows | UI Automation Control View | `desktop` | An interactive desktop session and access to the target process |
| Linux | AT-SPI | `desktop` | `python3`, `python3-pyatspi`, an accessibility-enabled desktop session; input synthesis also depends on session/compositor permissions |
| macOS | System Events AX attributes and Quartz input | `desktop` | Accessibility and Automation permission for the host process |
| iOS | No system provider registered | Unsupported | An ordinary app's public APIs do not provide cross-app UI trees and input injection |
| OpenHarmony | No system provider registered | Unsupported | Needs a separately authorized system accessibility integration; the old application-only implementation has been removed |
| Web | No system provider registered | Unsupported | A browser page cannot control the device desktop; remote device routing can target another node with a system provider |

`desktop` is the only accepted scope. It means the provider can inspect the
system's active surface, including other applications. Permissions still apply:
protected Windows desktops, inaccessible applications, locked devices, and desktop
compositor restrictions are not bypassed. Unsupported operations and denied
permissions produce tool errors. This is not a claim of validated whole-device
support on every OS.

OpenHarmony's former public `getWindowRootElement` APIs are deprecated from API 12
with their capabilities removed; the newer accessibility extension authorization
is a system permission. See the [AccessibilityExtensionContext reference](https://github.com/openharmony/docs/blob/master/en/application-dev/reference/apis-accessibility-kit/js-apis-inner-application-accessibilityExtensionContext.md)
and [system permission definitions](https://github.com/openharmony/docs/blob/master/en/application-dev/security/AccessToken/permissions-for-system-apps.md).

Collection is bounded to 4,000 nodes and 40 descendant levels. Providers report
`truncated` when their traversal limit cuts off descendants. Consumers must not
interpret a truncated snapshot as proof that an element does not exist.

## Interaction contract

The registered tools are `get_page_info`, `tap`, `long_press`, `click_element`,
`set_input_text`, `press_key`, and `swipe`. Plugin methods use the existing
`Tools.UI` declarations and return typed `UIPageResultData` or
`UIActionResultData`. Common selectors implement exact or explicitly requested
partial matches, stable traversal indexes, and bounds validation. Input values
and match failures are validated before dispatching input. An empty text string
clears a field.

Desktop providers support the named keys `ENTER`, `TAB`, `ESCAPE`, `BACKSPACE`,
`DELETE`, `HOME`, `END`, `UP`, `DOWN`, `LEFT`, and `RIGHT`. The legacy Android
APK exposes global actions `BACK`, `HOME`, `RECENTS`, `NOTIFICATIONS`, and
`QUICK_SETTINGS`; `KEYCODE_BACK` and `KEYCODE_HOME` remain accepted spellings.
These backend limits live inside their providers.

Each platform provider serializes its operations so that concurrent gestures
cannot interleave their pointer presses and releases. The legacy Android APK
only acknowledges gesture dispatch. Its adapter waits the configured gesture
duration (50 ms for tap, 600 ms for long press, the requested duration for swipe)
before allowing another operation. It cannot report the completion/cancellation
callbacks that the existing APK does not expose.

Text snapshots include accessible control values as well as labels on desktop
providers. Addressed text input rejects ambiguous resource IDs. Windows input
uses physical screen coordinates with per-monitor DPI awareness and checks the
number of events accepted by `SendInput`.

This interface addresses the active surface. Explicit `display` selection is
rejected, so it cannot accidentally send input to the default screen when a
caller requested another display. The old SDK's high-level `runSubAgent`
declaration and the historical virtual-display tool descriptions are separate
from this implementation. No virtual-display backend or agent is registered by
this change, and `ui.virtual_display` remains false.

## Android APK isolation

The main APK contains the AIDL **client** and `AndroidUiAutomation` only. It does
not declare or implement an `AccessibilityService`. It binds the existing
`RemoteBinderService` by explicit component. The AIDL descriptor and transaction
order match Operit 1's provider; the external APK is not rebuilt or changed.
Android package visibility includes the provider package.

Connection establishment runs on the owner worker, times out after five seconds,
unbinds rejected/timed-out connections, invalidates a disconnected binder, and
unbinds on owner release. Late callbacks remain attached to their original
connection. XML-to-tree conversion and the legacy bounds-token translation used
by `setTextOnNode` live
inside the Android adapter. Neither is represented as an Android-specific branch
in common runtime or plugin code. Install/enable failures remain explicit errors.

## Validation

Common executor tests cover unavailable system providers, snapshot conversion,
indexed and partial selection, negative monitor coordinates, invalid input,
unsupported display targets and empty text. Generated JavaScript tests exercise
UI overloads and device routing.
A read-only Windows probe exercises actual foreground UIA collection.
Other OS backends require their respective OS/device sessions for integration
validation; successful Windows checks do not establish those results. Android
integration requires the external provider APK to be installed and enabled first.
