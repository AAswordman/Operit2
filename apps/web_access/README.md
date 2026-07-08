# Operit2 Web Access

This directory owns the Web Access frontend boundary.

- `web/` contains the tracked Flutter Web shell files for Web Access.
- `build/bundle/` contains the generated bundle consumed by the Flutter app and CLI packages.
- `tools/build_scripts/build_flutter_web_access.py` stages `web/` into the Flutter package and writes the shared bundle.
- The current Dart entrypoint still lives in `apps/flutter/app`; consumers depend on the generated bundle here, not on Flutter's internal `build/web` directory.
