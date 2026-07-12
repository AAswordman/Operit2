#!/usr/bin/env python3
import argparse
import sys
from enum import Enum
from pathlib import Path

from common import (
    DIST_DIR,
    FLUTTER_APP_DIR,
    copy_required_file,
    dart_pub_get,
    flutter_command,
    prepare_web_access_embedded_assets,
    run,
)


OHOS_PROJECT_DIR = FLUTTER_APP_DIR / "ohos"
OHOS_HAP_PATH = OHOS_PROJECT_DIR / "entry" / "build" / "default" / "outputs" / "default" / "entry-default-signed.hap"


class ValueEnum(str, Enum):
    # Returns the raw enum value for command-line interpolation.
    def __str__(self):
        return self.value


class OhosTargetPlatform(ValueEnum):
    ARM64 = "ohos-arm64"


# Parses command-line options for the OpenHarmony Flutter build.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the Operit2 OpenHarmony Flutter app.")
    parser.add_argument(
        "--output",
        type=Path,
        default=DIST_DIR / "operit2-app-ohos-arm64.hap",
    )
    parser.add_argument("--build-name")
    parser.add_argument("--build-number")
    parser.add_argument("--enforce-lockfile", action="store_true")
    return parser.parse_args()


# Builds the signed OpenHarmony HAP through the FVM-selected Flutter SDK.
def main() -> int:
    args = parse_args()
    prepare_web_access_embedded_assets()
    if not OHOS_PROJECT_DIR.is_dir():
        raise RuntimeError(
            "OpenHarmony Flutter project not found at "
            f"{OHOS_PROJECT_DIR}. Generate it with the OpenHarmony Flutter SDK."
        )
    flutter = flutter_command()
    dart_pub_get(enforce_lockfile=args.enforce_lockfile)
    run(
        (
            [
                flutter,
                "build",
                "hap",
                "--release",
                "--no-pub",
                "--target-platform",
                OhosTargetPlatform.ARM64.value,
            ]
            + (["--build-name", args.build_name] if args.build_name else [])
            + (["--build-number", args.build_number] if args.build_number else [])
        ),
        cwd=FLUTTER_APP_DIR,
    )
    copy_required_file(OHOS_HAP_PATH, args.output)
    print(f"OpenHarmony HAP: {args.output}", flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
