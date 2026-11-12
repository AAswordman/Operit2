#!/usr/bin/env python3
"""Build or check the standalone Android Edge product with an API 21 NDK."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys

PRODUCT = Path(__file__).resolve().parent
REPO = PRODUCT.parents[2]
TARGETS = {
    "armeabi-v7a": ("armv7-linux-androideabi", "armv7a-linux-androideabi"),
    "arm64-v8a": ("aarch64-linux-android", "aarch64-linux-android"),
}


def main():
    """Select the explicit SDK, then check or build both old-phone native ABIs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sdk-dir", type=Path, required=True)
    parser.add_argument("--ndk-version", default="27.3.13750724")
    parser.add_argument("--check", action="store_true", help="Check Rust targets without linking or producing an APK")
    parser.add_argument("--apk", action="store_true", help="Also assemble a debug APK using the repository Gradle wrapper")
    args = parser.parse_args()
    if args.check and args.apk:
        parser.error("--check and --apk cannot be combined")
    sdk = args.sdk_dir.resolve(strict=True)
    ndk = sdk / "ndk" / args.ndk_version
    platform = {"win32": "windows-x86_64", "linux": "linux-x86_64", "darwin": "darwin-x86_64"}[sys.platform]
    tools = ndk / "toolchains" / "llvm" / "prebuilt" / platform / "bin"
    suffix = ".cmd" if os.name == "nt" else ""
    env = os.environ.copy()
    env["ANDROID_HOME"] = str(sdk)
    env["RUSTFLAGS"] = "-Awarnings -C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,common-page-size=16384"
    output = PRODUCT / "target"
    env["CARGO_TARGET_DIR"] = str(output)
    for abi, (target, clang_target) in TARGETS.items():
        compiler = tools / f"{clang_target}21-clang{suffix}"
        if not compiler.is_file():
            parser.error(f"NDK API 21 compiler is missing: {compiler}")
        env[f"CARGO_TARGET_{target.upper().replace('-', '_')}_LINKER"] = str(compiler)
        env[f"CC_{target.replace('-', '_')}"] = str(compiler)
        env[f"AR_{target.replace('-', '_')}"] = str(tools / ("llvm-ar.exe" if os.name == "nt" else "llvm-ar"))
        command = ["cargo", "check" if args.check else "build", "--locked", "--manifest-path", str(PRODUCT / "Cargo.toml"), "--target", target, "--lib"]
        if not args.check:
            command.append("--release")
        subprocess.run(command, env=env, cwd=REPO, check=True)
        if not args.check:
            destination = PRODUCT / "android/app/src/main/jniLibs" / abi
            destination.mkdir(parents=True, exist_ok=True)
            shutil.copy2(output / target / "release/liboperit_edge_phone.so", destination)
    if args.apk:
        wrapper = REPO / "apps/flutter/app/android" / ("gradlew.bat" if os.name == "nt" else "gradlew")
        subprocess.run([str(wrapper), "-p", str(PRODUCT / "android"), ":app:assembleDebug"], env=env, cwd=PRODUCT, check=True)


if __name__ == "__main__":
    main()
