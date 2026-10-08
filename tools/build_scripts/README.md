# Operit2 build scripts

Local builds and GitHub Actions use the same Python entry points. Scripts are
named by their product, platform, or operation; there are no separate
`_action.py` / `_local.py` platform wrappers.

Build outputs and transient logs belong under `tools/release/work` and
`tools/release/dist`. Private credentials belong under `tools/release/secrets`.
These ignored directories are not used to store reusable build scripts.

## Entry points

- `build_flutter_<platform>.py`: Flutter builds for Android, iOS, Linux, macOS,
  OpenHarmony, and Windows.
- `build_cli_<platform>.py`: CLI builds for Linux, macOS, and Windows.
- `build_cli_current.py`: CLI build for the current host platform.
- `build_local.py`: existing current-host App/CLI/ESP32 dispatcher.
- `build_apple_release.py`: Apple release asset dispatcher.
- `build_esp32.py`: ESP32 firmware builder.
- `check_build_environment.py`: shared local/CI environment checker.
- `common.py`, `cli_common.py`, and `appstore_connect_common.py`: shared helpers.

## Apple release helpers

```bash
# Build Flutter artifacts
python3 tools/build_scripts/build_flutter_ios.py --build-name 2.0.0 --build-number 15
python3 tools/build_scripts/build_flutter_macos.py --build-name 2.0.0 --build-number 15

# Sign the macOS app and create a .pkg
python3 tools/build_scripts/sign_macos.py

# Validate or upload an .ipa/.pkg
python3 tools/build_scripts/validate_appstore.py path/to/App.ipa
python3 tools/build_scripts/upload_appstore.py path/to/App.ipa
```

The App Store Connect scripts read the ignored file
`tools/release/secrets/appstoreconnect/appstoreconnect.env`. No credentials are
stored in tracked build scripts.

## Local CLI development and macOS Keychain authorization

Use the local runner to build and restore the CLI's configured signing identity
before running it:

```bash
python3 tools/build_scripts/run_cli.py -- cli character list --json
python3 tools/build_scripts/run_cli.py --release -- cli chat list --json
# Sign/run an existing binary without rebuilding:
python3 tools/build_scripts/run_cli.py --no-build -- cli character list --json
```

On macOS, choose an existing real signing identity via
`OPERIT_MACOS_SIGNING_IDENTITY`, `run_cli.py --identity`, or the Git-ignored
`tools/build_scripts/cli_macos_signing.local` file:

```json
{"identity": "<certificate SHA-1 or certificate name>"}
```

Both the local runner and macOS CLI packaging use the fixed identifier
`com.operit.cli`. Direct `cargo build` does not run this post-build signing step
and can replace a trusted artifact with a new ad-hoc-signed binary. Re-run the
local runner after such a build. Approve the new signed CLI's Keychain access
with **Always Allow** only after checking the requesting program.

The helper does not export certificates/private keys, store passwords, change
Keychain ACLs, or disable encrypted preferences. An explicitly configured
signing failure stops the build rather than falling back to ad-hoc signing.
Local signing is not public-release notarization; public macOS distribution
still needs an appropriate distribution identity and its release workflow.

The role-card CLI acceptance test uses a disposable profile. Its optional AI
fixture runs on loopback and binds every functional model (including title
and memory generation) to that endpoint:

```bash
python3 tools/tests/character_cards_cli_smoke.py --mock-ai
```
