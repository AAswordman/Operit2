#!/usr/bin/env python3
import argparse
import sys
import zipfile
from pathlib import Path

from common import (
    ANDROID_LOCAL_PROPERTIES,
    DIST_DIR,
    FLUTTER_APP_DIR,
    RELEASE_DIR,
    copy_required_file,
    flutter_command,
    flutter_pub_get,
    generate_dart_proxy_artifacts,
    read_properties,
    require_command,
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


# Configures Android Gradle to use the project Flutter SDK selected by FVM.
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
    parser.add_argument("--build-name")
    parser.add_argument("--build-number")
    parser.add_argument("--enforce-lockfile", action="store_true")
    parser.add_argument("--skip-proxy-generation", action="store_true")
    parser.add_argument("--skip-signing", action="store_true")
    parser.add_argument("--dist-dir", type=Path, default=DIST_DIR)
    return parser.parse_args()


# Rejects release APKs whose terminal assets do not match the requested ABI.
def verify_android_runtime_assets(apk_path: Path, abi: str) -> None:
    prefix = "assets/android-runtime/"
    with zipfile.ZipFile(apk_path) as archive:
        asset_names = {
            entry.filename for entry in archive.infolist()
            if not entry.is_dir() and entry.filename.startswith(prefix)
        }
    packaged_abis = {name[len(prefix):].split("/", 1)[0] for name in asset_names}
    if packaged_abis != {abi}:
        raise RuntimeError(
            f"Android APK runtime asset ABIs must be exactly {abi}: "
            f"{sorted(packaged_abis)} ({apk_path})"
        )
    required_assets = {
        f"{prefix}{abi}/rootfs.tar.gz.bin",
        f"{prefix}{abi}/rootfs.tar.gz.bin.sha256",
    }
    missing_assets = required_assets - asset_names
    if missing_assets:
        raise RuntimeError(
            f"Android APK is missing required runtime assets: {sorted(missing_assets)} ({apk_path})"
        )


# Builds and verifies the ABI-specific Android release before publishing it.
def main() -> int:
    args = parse_args()
    if not args.skip_signing:
        ensure_android_signing()
    flutter = flutter_command()
    configure_android_flutter_sdk(flutter)
    require_command("cargo")
    if not args.skip_proxy_generation:
        generate_dart_proxy_artifacts()
    flutter_pub_get(enforce_lockfile=args.enforce_lockfile)
    # Omit `--no-pub` on purpose: this build rewrites GeneratedPluginRegistrant.java
    # for release mode, dropping dev-only plugins (integration_test) that
    # `flutter pub get` leaves in. Gradle excludes dev-dependency modules from release
    # builds, so a registrant written by pub get alone cannot resolve those classes.
    command = [
        flutter,
        "build",
        "apk",
        "--release",
        "--split-per-abi",
        "--target-platform",
        "android-arm64",
    ]
    if args.build_name:
        command.extend(["--build-name", args.build_name])
    if args.build_number:
        command.extend(["--build-number", args.build_number])
    run(command, cwd=FLUTTER_APP_DIR)

    apk_dir = FLUTTER_APP_DIR / "build" / "app" / "outputs" / "flutter-apk"
    apk_path = apk_dir / "app-arm64-v8a-release.apk"
    verify_android_runtime_assets(apk_path, "arm64-v8a")
    copy_required_file(
        apk_path,
        args.dist_dir / "operit2-app-android-arm64-v8a.apk",
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
