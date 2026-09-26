# Operit2 Flutter app

This app is pinned to Flutter **3.41.9** (Dart **3.11.5**) through `.fvmrc`.

The SDK is installed side-by-side with the existing machine-wide Flutter SDK, so
other projects continue using their current version. The project helper resolves
an FVM SDK when present, or the side-by-side SDK installed at
`D:\ruanjiankaifa\flutterSDK\flutter-3.41.9`.

```powershell
# From apps/flutter/app
.\tool\flutter.ps1 pub get
.\tool\flutter.ps1 analyze
.\tool\flutter.ps1 run
```

To use another location without changing the global Flutter installation:

```powershell
$env:OPERIT2_FLUTTER_SDK = 'D:\path\to\flutter-3.41.9'
.\tool\flutter.ps1 --version
```

If the app is launched from an IDE, configure that IDE's Flutter SDK path to the
same `flutter-3.41.9` directory.
