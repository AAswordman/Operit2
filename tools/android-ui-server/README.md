# Android native window payload

This module is an **uninstalled app_process payload**. Its APK is a DEX archive,
embedded as `companions/operit-ui-server.jar` by the main Android build. It has
no installed services, activities or providers. Do not install this APK.

The Android owner reuses `AndroidPrivilegedCommandExecutor`, including saved Root
execution settings and the existing Shizuku UserService. It uploads the archive
in bounded chunks directly into a unique `/data/local/tmp/operit-ui-<nonce>`
directory. No external-storage staging or storage permission is needed. The
archive is read-only before app_process opens it. Root deployments transfer
directory ownership to UID/GID 2000. The launcher selects app_process32/64 to
match the main application's native library ABI.

Shizuku command children already use the terminal Host's shell identity boundary.
For Root, the payload loads the main application's existing native bridge and
calls that same identity implementation and re-execs app_process before
initializing Android or Binder, replacing all threads of the Root-started VM:
shell supplementary groups, GID 2000, UID 2000 and the existing SELinux identity
handling. The server verifies its final UID/GID. It does not invoke another su
implementation or depend on the installed accessibility companion to bootstrap.

`ShellContext` initializes ActivityThread, configuration, application metadata and
instrumentation for the uninstalled process. The bootstrap sets the native process
name and matching application/bind metadata before creating Views, so framework
and accessibility integrations can read a valid process identity. Calls use the shell package and UID.
The ActivityManager shell entry points launch apps and hand off Binder without
presenting an unregistered application-thread token to system_server.

Android 15 requires a registered application process when opening an IWindowSession.
The existing Android owner supplies its window-session Binder during attach; the
payload owns every View, Surface, input receiver and rendering operation. Window
app-op attribution uses the verified owner's package and existing overlay permission.
Display creation, application launch and input injection still use shell UID 2000.
The native chrome has no autofill or content-capture service: those features belong
to the hosted application's own process and do not apply to the window controls.
The delegated session gives each window's native input channel directly to
`WindowInputReceiver` in the tmp process. ViewRoot owns drawing and layout, while
the receiver dispatches native touch, scroll and key events to the same View tree
and acknowledges each event. Pointer batches follow Choreographer frames. A close
action finishes its current input event before releasing the channel. This avoids
application-only ViewRoot gesture integrations that require a process registered
with ActivityManager. It uses Android's input-channel contract, with no vendor
fields or replacement gesture implementations. The small framework declarations
are compile-only and are not packaged into the DEX payload.

The platform's zygote shared-library preload initializes rendering libraries,
including the running ROM's own extensions. The actual window service supplies
animator scale without requiring zygote application shared memory. Full-display
bounds and public maximum-window metrics determine bars and cutouts; no hidden
InsetsState calculation signature is assumed.
The shell bootstrap has a local `BlockedPrivateApi` lint suppression because its
zygote preload runs in an uninstalled `app_process` payload as shell, outside
installed applications' hidden-API enforcement. Release lint remains enabled.

System fonts are initialized before any View is measured. Android 15's standalone
shell process initializes the font map from the platform's legacy font configuration,
whose access is permitted by shell SELinux policy.

The nonce-bound, package-targeted Binder handoff goes to a temporary receiver in
the Android owner. It requires the sender's DUMP permission. The endpoint checks
the launching application's UID and attaches its Binder owner token. Owner death
or explicit release destroys every window/display, quits the process and removes
the deployment files. An unattached server exits after 15 seconds. A lost server
never receives old sessions through a replacement connection.

Each application has its own `DisplayWindow` and opaque UUID window identity.
Repeated launches of an app use its existing display. The server owns the handles,
SurfaceView, VirtualDisplay and input injection. VirtualDisplay writes directly
to the SurfaceView's Surface. Binder carries control and geometry only: no frame
buffers, screenshots, encoders, decoders or video streams. Drag the top handle
to move the rounded card, or tap its slim, light grip to open Minimize, Fullscreen,
Back and Close. Fullscreen moves the display's existing task stacks onto the
physical primary display before closing the floating window, preserving the
current task instead of starting a new instance. Both lower corners resize the
content proportionally. Resizing can continue below the normal 184dp width;
Up to 32dp of further inward resize triggers docking at the top right. Restoring that
preview returns to the normal minimum size. Geometry updates are coalesced once
per display frame, and the buffer-space corner radius is corrected after the
SurfaceView's own pre-draw update in the same draw transaction. The virtual display uses a 2:3
resolution fitted to the device's pixel dimensions. Minimize animates the live Surface
into a preview below the system bars near the top-right edge. Tapping that
preview restores the previous bounds, with the same display and window ID. MotionEvent copies
retain all pointers, history and axes and transform coordinates into that exact
display. Android's IME policy is local to the virtual display.

## Existing public interfaces

With existing Shizuku or Root authorization, Android `start_app` / the UI package's
`app_launch` starts the app in this native window. With ordinary app authorization,
the existing Android activity launch remains its separate capability. A failed
privileged launch is an error. Automation never prompts for permissions or tries
another transport after a failed command.

Privileged launch requires the existing UI automation companion connection before
creating a window, then confirms the app's actual accessibility window appears on
the requested display. Activity launch acceptance alone is not reported as
success; display rejection/redirection and unavailable accessibility are errors.

`list_windows` exposes native windows through the existing shared schema.
`focus_window(window_id)` selects the hosted application's original virtual display
without restoring or raising its preview. Minimizing or resizing the preview does
not disable trees, screenshots, gestures or keys on that display. Repeated app
launches preserve the preview's current size and minimized state. Only explicit
human interaction with the preview restores it. `get_page_info`
continues to use the signed accessibility companion and stream its full tree
through its existing pipe. Subsequent input follows the selected window. The
Android Host alone translates the opaque session ID to an internal display ID;
the common Rust/Dart/JS contracts have no virtual-window operation or transport
parameter. Closed IDs fail. Minimized sessions remain discoverable and can be
restored with the same window ID.

Completion passes the shared `close_windows` flag to the Android Host. It clears
selection and releases every hosted display and floating surface, including a
launched screen that was never selected. The native server's protocol version 2
adds `close_windows`; it closes its current windows synchronously and remains
available for the next launch. This cleanup does not require a live accessibility
companion connection. `complete_ui_operation` waits for cleanup after the last
active chat finishes, and propagates native failures so cleanup can be retried.

Window metadata bounds describe the physical floating content area for the
existing indicator. Tree bounds and automation coordinates belong to the native
virtual display, independent of the floating window's scale. The server scales
human touch input; accessibility gestures target the same display explicitly.
This payload requires Android 11/API 30 or newer for multi-display accessibility
and input routing. Native focus/flags and IME setup use the Android release's API.
Android 12+ enforces `ADD_TRUSTED_DISPLAY`; AOSP grants this to shell from Android
13. Android 12 devices need their firmware to grant that capability to shell.
Denial of trusted displays, system windows, launch or input is reported as an
error, including on older/vendor firmware; the process does not retain Root to
bypass the shell permission boundary.

## Device verification

Build the normal main application, grant UI automation in the existing onboarding
flow, and authorize either Shizuku or Root. Test each authorization separately.
On Root, verify the server process UID/GID are 2000. Start two apps using the
existing launch tool and select each from `list_windows`. Read trees, tap, type,
swipe and send Back; verify the physical display and the other session receive
no input. Drag/resize and exercise multi-touch and local IME in each window.
Continue resizing inward past the minimum from each corner and verify top-right
docking, stable opposite-corner positioning and restoration. Tap Fullscreen and
verify the same task and page appear on display 0 before its old window ID closes.
Minimize, select through `focus_window` and continue trees, screenshots, gestures
and keys while the preview stays minimized; relaunch the same app and verify the
preview still stays minimized. Restore by tapping the preview, then close and
verify its old ID fails. Stop an app through the existing tool. Release/kill the Android owner and
verify its server, tasks and tmp directory disappear. Inspect the packaged archive
for `classes.dex` and run on Android 11 and a current Android release.

Repository contract checks validate packaging and identity/interface boundaries;
they do not replace these device checks.

`tools/tests/run_android_window_smoke.py --sdk <Android SDK>` builds a small
debug-only test owner, signed with the normal Android debug keystore. Install the
matching debug accessibility companion, enable it and grant the test owner's overlay
permission. `--start-server` pushes the actual production payload to tmp and starts
it as the ADB shell. `--verify` exercises the production Binder and accessibility
protocols, captures screenshots and saves evidence under
`tools/release/work/android-window-smoke/`. The fixture currently targets an Android
15 emulator with Google Clock and Files, a 1080x2400 display and density 420. This
checks shell-owned native windows separately from the main application's deployment
and Root/Shizuku consent. Android security screens can hide application overlays,
including overlays presenting a sensitive activity on another display.
