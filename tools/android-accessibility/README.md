# Operit2 Android accessibility companion

This standalone APK owns system window discovery, complete accessibility snapshots
and device input. The shared AIDL lives in `contract/src/main/aidl` and is consumed
directly by this APK and `apps/flutter/app/android/app`. The Operit 1 provider is
not used by this transport.

## Build and install

Normal Flutter Android builds include this project automatically and embed its
signed APK at `assets/companions/operit-accessibility.apk`. Debug/profile builds
embed the debug companion; release builds share the main application's release
certificate. The companion has its own version in `version.properties`; both
its APK and the main app's required provider version read this single source.
Increase the provider version when distributing a provider update. The main
app compares the installed provider version with this requirement, independently
of its own version code or ABI-specific APK version codes.
This integration runs for `fvm flutter run`, `fvm flutter build apk --debug`,
profile and release builds. Debug uses the main application's actual debug
signing configuration and does not require release signing credentials.

In Operit2's Android authorization list, click **UI 自动化 → 授予**. Operit2 installs the embedded
provider when it is absent or older than the required provider version, requesting
unknown-source authorization when necessary, then opens accessibility settings.
A current or newer provider goes directly to accessibility settings. Enable
**Operit2 Accessibility** there. Status is satisfied only when the provider meets
the required version and its service is enabled. Only the provider's own setup
screen requests an exact-text risk acknowledgment when the user clicks enable.
Confirmation is saved in the provider's private preferences and survives app
restarts, service reauthorization and APK updates. Canceling does not save consent.
No download or separate manual APK distribution is needed for normal main APKs.

For standalone development, build and install only the companion as follows.

Use JDK 17 or newer and an Android SDK with API 35. Run from the repository root:

```powershell
$env:ANDROID_HOME = 'C:\path\to\Android\Sdk'
.\apps\flutter\app\android\gradlew.bat -p tools/android-accessibility assembleDebug
adb install -r tools/android-accessibility/build/outputs/apk/debug/OperitAccessibility-debug.apk
```

The standalone Gradle project reuses the repository's Gradle wrapper. It does not
require Flutter or build the main application. Android 8.0/API 26 is the minimum.

Open **Operit2 Accessibility** and enable its service in Android accessibility
settings. Install a main application containing the new adapter as well.
Both APKs must use the same signing certificate: the exported Binder service
requires `app.operit.accessibility.permission.AUTOMATE_UI`, a signature permission.
Default debug builds use the standard Android debug keystore. For release builds,
pass `-PoperitSigningProperties=/absolute/path/to/signing.properties`. That file
must contain `RELEASE_STORE_FILE`, `RELEASE_STORE_PASSWORD`, `RELEASE_KEY_ALIAS` and
`RELEASE_KEY_PASSWORD`, using the main application's release certificate. Keep it
outside version control. Without those properties, a release APK is unsigned.

## Protocol version 1

`getProtocolVersion()` returns `1`. `execute(requestJson)` returns a
`ParcelFileDescriptor`; read it to EOF as UTF-8 JSON:

```json
{"operation":"get_page_info","parameters":{"window_id":"123"}}
```

Success is `{"success":true,"data":{...}}`; failure is
`{"success":false,"error":"..."}`. Trees travel through the pipe, so their
size is not constrained by Binder's transaction buffer. The response retains
all exposed descendants and native state properties. Password contents are
excluded. Android 11+ window discovery includes all displays.

Supported operations are `list_windows`, `focus_window`, `get_page_info`, `tap`,
`long_press`, `swipe`, `set_input_text` and `press_key`. `list_windows` returns
`{"windows":[...]}`. Explicit window reads do not change input focus. Input
operations use the active window and screen coordinates, so focus the target
before sending input. Gesture results wait for Android's completion callback.
Focus is verified against Android's input-focused window; a denied request
returns an error.

Window IDs refer to Android's current interactive-window session. Refresh the
list after windows close or change. A virtualized view can only expose the
descendants its application has materialized through accessibility.
