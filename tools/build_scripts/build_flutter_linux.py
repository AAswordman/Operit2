#!/usr/bin/env python3
import argparse
import sys
from pathlib import Path

from common import (
    DIST_DIR,
    FLUTTER_APP_DIR,
    compress_tar_gz,
    flutter_command,
    flutter_pub_get,
    host_arch,
    require_web_access_bundle,
    run,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the Operit2 Linux Flutter app.")
    parser.add_argument("--build-name", required=True)
    parser.add_argument("--build-number", required=True)
    parser.add_argument("--archive-path", type=Path)
    parser.add_argument("--enforce-lockfile", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    require_web_access_bundle()
    flutter_pub_get(enforce_lockfile=args.enforce_lockfile)
    run(
        [
            flutter_command(),
            "build",
            "linux",
            "--release",
            "--build-name",
            args.build_name,
            "--build-number",
            args.build_number,
        ],
        cwd=FLUTTER_APP_DIR,
    )
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
