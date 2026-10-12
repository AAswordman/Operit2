#!/usr/bin/env python3
import argparse
import sys
from pathlib import Path

from common import (
    DIST_DIR,
    FLUTTER_APP_DIR,
    compress_zip,
    flutter_command,
    flutter_pub_get,
    generate_dart_proxy_artifacts,
    host_arch,
    prepare_python_command,
    require_command,
    run,
)


# Parses command-line options for the Windows Flutter build.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the Operit2 Windows Flutter app.")
    parser.add_argument("--build-name")
    parser.add_argument("--build-number")
    parser.add_argument("--archive-path", type=Path)
    parser.add_argument("--enforce-lockfile", action="store_true")
    parser.add_argument("--skip-proxy-generation", action="store_true")
    return parser.parse_args()


# Builds the Windows Flutter app and writes a release archive.
def main() -> int:
    args = parse_args()
    require_command("cargo")
    # The Flutter native-assets hook and the CMake sync_operit_plugins target
    # both invoke .venv/Scripts/python.exe.  Provision it before the build
    # so those steps do not fail on CI runners that have no .venv yet.
    prepare_python_command()
    if not args.skip_proxy_generation:
        generate_dart_proxy_artifacts()
    flutter_pub_get(enforce_lockfile=args.enforce_lockfile)
    command = [flutter_command(), "build", "windows", "--release", "--no-pub"]
    if args.build_name:
        command.extend(["--build-name", args.build_name])
    if args.build_number:
        command.extend(["--build-number", args.build_number])
    run(command, cwd=FLUTTER_APP_DIR)
    release_dir = FLUTTER_APP_DIR / "build" / "windows" / "x64" / "runner" / "Release"
    archive_path = args.archive_path or DIST_DIR / f"operit2-app-windows-{host_arch()}.zip"
    compress_zip(release_dir, archive_path)
    print(f"Windows archive: {archive_path}", flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
