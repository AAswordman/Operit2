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
    prepare_web_access_embedded_assets,
    run,
    write_properties,
)


def ensure_android_signing() -> None:
    signing_properties = RELEASE_DIR / "secrets" / "android-signing.properties"
    if not signing_properties.exists():
        raise RuntimeError(f"Android signing properties not found: {signing_properties}")

    signing = read_properties(signing_properties)
    local = read_properties(ANDROID_LOCAL_PROPERTIES)
    local["RELEASE_STORE_FILE"] = str(android_release_store_file(signing, signing_properties))
    for key in (
        "RELEASE_STORE_PASSWORD",
        "RELEASE_KEY_ALIAS",
        "RELEASE_KEY_PASSWORD",
    ):
        if key not in signing:
            raise RuntimeError(f"Android signing property missing from {signing_properties}: {key}")
        local[key] = signing[key]
    write_properties(ANDROID_LOCAL_PROPERTIES, local)


# Configures Android Gradle to use the official Flutter SDK selected by FVM.
def configure_android_flutter_sdk(flutter: str) -> None:
    flutter_sdk = Path(flutter).parent.parent
    local = read_properties(ANDROID_LOCAL_PROPERTIES)
    local["flutter.sdk"] = str(flutter_sdk)
    write_properties(ANDROID_LOCAL_PROPERTIES, local)


# Reads the Android release keystore path from signing properties.
def android_release_store_file(signing: dict[str, str], signing_properties: Path) -> Path:
    key = "RELEASE_STORE_FILE"
    value = signing.get(key)
    if not value:
        raise RuntimeError(f"Android signing property missing from {signing_properties}: {key}")
    store_file = Path(value)
    if not store_file.is_absolute():
        store_file = signing_properties.parent / store_file
    if not store_file.is_file():
        raise RuntimeError(f"Android release keystore not found: {store_file}")
    return store_file


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
    prepare_web_access_embedded_assets()
    if not args.skip_signing:
        ensure_android_signing()
    flutter = flutter_command()
    configure_android_flutter_sdk(flutter)
    flutter_pub_get(enforce_lockfile=args.enforce_lockfile)
    run(
        [
            flutter,
            "build",
            "apk",
            "--release",
            "--no-pub",
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
