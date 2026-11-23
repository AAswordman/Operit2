# Embedded Operit shell service

This is an embedded, owner-only Shizuku fork. The Android Host uses the existing
Shizuku Binder API 13 and UserService protocol. It does not install a second manager
APK, register another global authorization manager, or authorize third-party apps.

Vendored upstream sources:

- RikkaApps/Shizuku `b844bc491f1790c72328e1a8e5b2349f8978f0ea`, Apache-2.0:
  server, common, UserService starter, ADB protocol/pairing and native pairing;
  original manager UI resources, app bar, pairing tutorial and RemoteInput service.
- RikkaApps/Shizuku-API `a27f6e4151ba7b39965ca47edb2bf0aeed7102e5`, MIT:
  Binder server protocol and UserService transport from server-shared.

Licenses ship in Android assets. Modified upstream files carry an Operit notice.
Stellar (MPL-2.0) and ShizukuX (Apache-2.0) were reviewed for lifecycle behavior;
their code is not incorporated. Brevent is not incorporated.

Operit changes: explicit private backend selection; shell identity established
before ART starts, including Root activation; detached native supervisor; separate
Android 17 user enumeration; owner-only Binder publication;
in-app activation and saved ADB credentials. Reboot recovery reactivates a service;
it cannot preserve a process across power loss or bypass Android's ADB policy.

Root activation leaves the application's memory cgroup, sets supplementary groups,
GID/UID 2000 and the shell SELinux context before starting ART. Identity or cgroup
transition failures reject activation. The supervisor closes inherited descriptors,
detaches its session, restarts at most five crashed children and respects a normal
exit or owner APK replacement. It never changes another Shizuku installation.

This directory is an Android library consumed only by `:app`; it has no application,
independent manager manifest, publishing plugin or separate APK build. The app's
management UI and lifecycle components are grouped under `app.operit.shizuku`.
Standalone-manager app lists, third-party authorization/configuration storage,
legacy client protocol handling and the unused rish CLI/native library are removed.
Operit's existing UserService/JNI terminal transport executes commands.

The native Host setup screen supports Root, Android 11+ local ADB pairing/connection,
computer ADB activation and direct external authorization cards. Recovery is enabled
as part of embedded activation, with its behavior described in the startup cards;
there is no toolbar menu or separate recovery switch. Explicit stop also cancels recovery.
Recovery uses boot/package-replacement broadcasts, Binder death and a JobScheduler job with
network constraints for ADB and exponential retry scheduling. No permanent ADB TCP
port, global process-killer setting or third-party watchdog code is included.

`upstream/manager-ui/res` retains the original card and guide XML. Unused manager
resources and strings have been removed. The original Material theme
generator configuration is in `ui-theme.gradle`. The Host UI opens inside Operit;
wireless pairing discovers the pairing port and accepts the six-digit code directly
in the notification while Android's pairing dialog remains visible. There is no
second manager APK or a manual port/code form in the home screen.

See [integration and device checks](../../docs/android-embedded-shizuku.md).
