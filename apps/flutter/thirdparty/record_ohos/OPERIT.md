# Operit maintenance notes

This package is based on
`bruce3x/flutter_record_ohos@2c0c5f9e000ea78ff97e9c86a71f021a8f33dfc1`.
Its OpenHarmony implementation is derived from the Apache-2.0 sources in that
repository and retains the upstream package license.

Operit maintains these changes:

- PCM16 microphone streaming through `AudioCapturer`.
- Compatibility with `record_platform_interface` 1.6.0.
- Permission checks honor the interface's `request` argument.
- Foreground recording is independent from OpenHarmony background tasks.

The Flutter application depends directly on this plugin alongside the hosted
`record` package. The shared Dart API continues to come from upstream
`record`; this package only supplies the OpenHarmony native implementation.
