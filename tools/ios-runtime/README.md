# iOS Native Runtime

This directory owns every resource and build step for the iOS embedded Toybox, Node, and CPython runtimes. The Runner target embeds Toybox, Node Mobile, BeeWare CPython, bundled pure packages, and the native NumPy/SciPy framework. It does not use V86, a web runtime, a server relay, or an iOS subprocess.

Run the scientific build on macOS with Xcode before an iOS application build:

```bash
python3 tools/ios-runtime/native/build_scientific.py
python3 tools/ios-runtime/toybox/build.py
python3 tools/ios-runtime/prepare.py
```

`prepare.py` requires `native/output/OperitPythonScientific.xcframework` and `toybox/output/OperitToybox.xcframework`. It stops the build when either framework is not present.

The app exposes three explicit workspace terminal types:

- `toybox`: Direct Toybox applet terminal with the common Unix command suite selected in `toybox/toybox.config`.
- `python`: CPython REPL with pure packages, NumPy, and SciPy.
- `node`: Node Mobile REPL with the packages declared in `packages.json`.

Toybox 0.8.12 is distributed under its 0BSD license. The package manifest and Toybox source manifest are pinned by URL, version, and SHA-256. Archives remain in `apps/flutter/app/apple/downloads/ios-runtime`, allowing repeated builds to reuse verified downloads.
