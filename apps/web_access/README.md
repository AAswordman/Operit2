# Operit2 Web Access

This directory owns the Web Access frontend boundary.

- `web/` contains the tracked Flutter Web shell files for Web Access.
- `build/bundle/` contains the generated bundle consumed by the Flutter app and CLI packages.
- `tools/build_scripts/build_flutter_web_access.py` ensures the Flutter web entry link exists and writes the shared bundle.
- The current Dart entrypoint still lives in `apps/flutter/app`; consumers depend on the generated bundle here, not on Flutter's internal `build/web` directory.
- `apps/flutter/app/web` is a symlink to `apps/web_access/web`.
- `apps/flutter/app/build/web` is a symlink to `apps/web_access/build/bundle` during Flutter Web builds.
