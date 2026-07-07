#!/usr/bin/env python3
import argparse
import sys
from pathlib import Path

from common import (
    ANDROID_LOCAL_PROPERTIES,
    DIST_DIR,
    FLUTTER_APP_DIR,
    RELEASE_DIR,
    copy_required_file,
    flutter_command,
    flutter_pub_get,
    read_properties,
    run,
    write_properties,
)


def ensure_android_signing() -> None:
    signing_properties = RELEASE_DIR / "secrets" / "android-signing.properties"
    if not signing_properties.exists():
        raise RuntimeError(f"Android signing properties not found: {signing_properties}")

    signing = read_properties(signing_properties)
    local = read_properties(ANDROID_LOCAL_PROPERTIES)
    for key in (
        "RELEASE_STORE_FILE",
        "RELEASE_STORE_PASSWORD",
        "RELEASE_KEY_ALIAS",
        "RELEASE_KEY_PASSWORD",
    ):
        if key not in signing:
            raise RuntimeError(f"Android signing property missing from {signing_properties}: {key}")
        local[key] = signing[key]
    write_properties(ANDROID_LOCAL_PROPERTIES, local)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the Operit2 Android Flutter app.")
    parser.add_argument("--build-name", required=True)
    parser.add_argument("--build-number", required=True)
    parser.add_argument("--enforce-lockfile", action="store_true")
    parser.add_argument("--skip-signing", action="store_true")
    parser.add_argument("--dist-dir", type=Path, default=DIST_DIR)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.skip_signing:
        ensure_android_signing()
    flutter_pub_get(enforce_lockfile=args.enforce_lockfile)
    run(
        [
            flutter_command(),
            "build",
            "apk",
            "--release",
            "--split-per-abi",
            "--build-name",
            args.build_name,
            "--build-number",
            args.build_number,
        ],
        cwd=FLUTTER_APP_DIR,
    )

    apk_dir = FLUTTER_APP_DIR / "build" / "app" / "outputs" / "flutter-apk"
    outputs = {
        "arm64-v8a": "app-arm64-v8a-release.apk",
        "armeabi-v7a": "app-armeabi-v7a-release.apk",
        "x86_64": "app-x86_64-release.apk",
    }
    for abi, filename in outputs.items():
        copy_required_file(apk_dir / filename, args.dist_dir / f"operit2-app-android-{abi}.apk")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
