#!/bin/sh
# Keep the workaround local to Flutter assemble; never alter the selected SDK.
set -eu
export PATH="$PROJECT_DIR/toolchain:$PATH"
exec "$FLUTTER_ROOT/packages/flutter_tools/bin/macos_assemble.sh" "$@"
