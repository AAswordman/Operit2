#!/usr/bin/env python3
import argparse
import sys
from pathlib import Path

from common import (
    DIST_DIR,
    FLUTTER_APP_DIR,
    compress_tar_gz,
    flutter_command,
    dart_pub_get,
    host_arch,
    prepare_web_access_embedded_assets,
    run,
)


# Parses command-line options for the Linux Flutter build.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the Operit2 Linux Flutter app.")
    parser.add_argument("--build-name")
    parser.add_argument("--build-number")
    parser.add_argument("--archive-path", type=Path)
    parser.add_argument("--enforce-lockfile", action="store_true")
    return parser.parse_args()


# Builds the Linux Flutter app and writes a release archive.
def main() -> int:
    args = parse_args()
    prepare_web_access_embedded_assets()
    dart_pub_get(enforce_lockfile=args.enforce_lockfile)
    command = [flutter_command(), "build", "linux", "--release", "--no-pub"]
    if args.build_name:
        command.extend(["--build-name", args.build_name])
    if args.build_number:
        command.extend(["--build-number", args.build_number])
    run(command, cwd=FLUTTER_APP_DIR)
    bundle = FLUTTER_APP_DIR / "build" / "linux" / "x64" / "release" / "bundle"
    archive_path = args.archive_path or DIST_DIR / f"operit2-app-linux-{host_arch()}.tar.gz"
    compress_tar_gz(bundle, archive_path)
    print(f"Linux archive: {archive_path}", flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
