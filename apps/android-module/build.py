#!/usr/bin/env python3
"""Build the Android module application into an installable module directory."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
import tempfile
import zipfile

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools/build_scripts"))
from common import FLUTTER_APP_DIR, flutter_command, flutter_pub_get, generate_dart_proxy_artifacts, require_command, run
from build_flutter_android import configure_android_flutter_sdk, ensure_android_signing
from release_artifacts import RELEASE_VERSION_PATTERN

APP_ROOT = Path(__file__).resolve().parent
PACKAGE = APP_ROOT / "package"
DEFAULT_OUTPUT = APP_ROOT / "build/module-arm64-v8a"
SYSTEM_APP = "system/app/Operit"
APK_ENTRY = "payload/operit-android.apk"
ROOTFS_ENTRY = "assets/android-runtime/arm64-v8a/rootfs.tar.gz.bin"
APP_MARKER = {"runtime": "external", "protocol": 1, "app": "android-module", "bootstrap": "app_process"}
APP_JNI = b"Java_app_operit_ModuleMain_runRuntime\0"


def parse_args() -> argparse.Namespace:
    """Accept one shared Android artifact and a directory destination for the module application."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apk", type=Path, help="Use an explicitly supplied compatible module APK instead of building it")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT, help="New module output directory; ZIP output is not accepted")
    parser.add_argument("--build-name")
    parser.add_argument("--build-number")
    parser.add_argument("--release-version", help="Stamp module.prop with the shared release version; requires --build-number")
    parser.add_argument("--enforce-lockfile", action="store_true")
    return parser.parse_args()


def elf_arm64(data: bytes, label: str) -> None:
    """Reject incompatible native libraries before module installation."""
    if len(data) < 20 or data[:4] != b"\x7fELF" or data[4:6] != bytes([2, 1]) or int.from_bytes(data[18:20], "little") != 183:
        raise ValueError(f"Not an ARM64 little-endian ELF: {label}")


def build_apk(build_name: str | None = None, build_number: str | None = None,
              enforce_lockfile: bool = False) -> Path:
    """Build the shared Dart UI and JNI library with the module application's Android entry point."""
    ensure_android_signing()
    flutter = flutter_command()
    configure_android_flutter_sdk(flutter)
    require_command("cargo")
    generate_dart_proxy_artifacts()
    flutter_pub_get(enforce_lockfile=enforce_lockfile)
    command = [flutter, "build", "apk", "--release", "--split-per-abi",
               "--target-platform", "android-arm64", "--android-project-arg=operitRuntime=module"]
    if build_name:
        command.extend(["--build-name", build_name])
    if build_number:
        command.extend(["--build-number", build_number])
    run(command, cwd=FLUTTER_APP_DIR)
    result = FLUTTER_APP_DIR / "build/app/outputs/flutter-apk/app-arm64-v8a-release.apk"
    if not result.is_file():
        raise FileNotFoundError(result)
    return result


def checksum_lines(checksums: dict[str, str]) -> bytes:
    """Emit exact relative paths in the checksum format supported by the module installer."""
    return "".join(f"{checksum}  {name}\n" for name, checksum in sorted(checksums.items())).encode()


def assemble(apk: Path, output: Path, *, release_version: str | None = None,
             build_number: str | None = None) -> dict:
    """Assemble one module folder without duplicating Core, native libraries or environment archives."""
    output = output.resolve()
    if output.suffix.lower() == ".zip":
        raise ValueError("The Android module app builds a directory, not a ZIP")
    if output.exists():
        raise FileExistsError(f"Module output already exists: {output}")
    if apk.resolve() == output or output in apk.resolve().parents:
        raise ValueError("Module output cannot contain its own APK input")
    if release_version is not None:
        if RELEASE_VERSION_PATTERN.fullmatch(release_version) is None:
            raise ValueError("Invalid module release version")
        if build_number is None or re.fullmatch(r"[1-9][0-9]*", build_number) is None:
            raise ValueError("A module release requires a positive --build-number")
    entries: dict[str, tuple[bytes, bool]] = {}
    installed: dict[str, str] = {}
    with zipfile.ZipFile(apk) as application:
        names = application.namelist()
        if len(names) != len(set(names)):
            raise ValueError("APK contains duplicate ZIP entries")
        marker = "assets/operit-android-module.json"
        if marker not in names or json.loads(application.read(marker)) != APP_MARKER:
            raise ValueError("APK is not the shared Android module build; rebuild with operitRuntime=module")
        dex_entries = [name for name in names if re.fullmatch(r"classes(?:\d+)?\.dex", name)]
        if not any(b"Lapp/operit/ModuleMain;" in application.read(name) for name in dex_entries):
            raise ValueError("APK is missing the module app_process entry point")
        native_names = sorted(name for name in names if name.startswith("lib/arm64-v8a/") and name.endswith(".so"))
        required = {"libbash.so", "liboperit_busybox.so", "liboperit_proot.so", "liboperit_loader.so",
                    "liboperit_local_llm.so", "liboperit_flutter_bridge.so", "libflutter.so", "libapp.so"}
        present = {Path(name).name for name in native_names}
        if not required <= present:
            raise ValueError(f"APK is missing module runtime libraries: {sorted(required - present)}")
        for name in native_names:
            library = Path(name).name
            if name != "lib/arm64-v8a/" + library or not re.fullmatch(r"lib[a-zA-Z0-9_.+\-]+\.so", library):
                raise ValueError(f"Invalid module library path: {name}")
            data = application.read(name)
            elf_arm64(data, name)
            if library == "liboperit_flutter_bridge.so" and APP_JNI not in data:
                raise ValueError("APK JNI library has no module entry point; rebuild the shared bridge")
            installed[f"{SYSTEM_APP}/lib/arm64/{library}"] = hashlib.sha256(data).hexdigest()
        rootfs = application.read(ROOTFS_ENTRY)
        rootfs_marker = application.read(ROOTFS_ENTRY + ".sha256")
        rootfs_sha = rootfs_marker.decode().split()[0]
        if hashlib.sha256(rootfs).hexdigest() != rootfs_sha:
            raise ValueError("APK rootfs checksum mismatch")
        installed["runtime/assets/rootfs.tar.gz"] = rootfs_sha
        installed["runtime/assets/rootfs.tar.gz.sha256"] = hashlib.sha256(rootfs_marker).hexdigest()
    apk_bytes = apk.read_bytes()
    installed[f"{SYSTEM_APP}/Operit.apk"] = hashlib.sha256(apk_bytes).hexdigest()
    entries[APK_ENTRY] = (apk_bytes, False)
    entries["payload/native-libraries.list"] = (("\n".join(sorted(present)) + "\n").encode(), False)
    entries["payload/installed.sha256"] = (checksum_lines(installed), False)
    for path in sorted(PACKAGE.iterdir()):
        if path.is_file():
            name = "bin/operit-host-ctl" if path.name == "operit-host-ctl" else path.name
            data = path.read_bytes().replace(b"\r\n", b"\n")
            if name == "module.prop" and release_version is not None:
                properties = dict(line.split("=", 1) for line in data.decode().splitlines() if "=" in line)
                properties.update(version=release_version, versionCode=build_number, name="Operit2 Android Module")
                data = "".join(f"{key}={value}\n" for key, value in properties.items()).encode()
            entries[name] = (data, path.suffix == ".sh" or path.name == "operit-host-ctl")
    manifest = {"format": 3, "app": "android-module", "moduleProtocol": 1, "abi": "arm64-v8a",
                "bootstrap": "app_process", "uiDeployment": "system-app", "systemApk": "/" + SYSTEM_APP + "/Operit.apk",
                "requiresMountProvider": True, "requiresReboot": True, "sharedCoreLibrary": "liboperit_flutter_bridge.so",
                "archiveFiles": {name: hashlib.sha256(data).hexdigest() for name, (data, _) in entries.items()},
                "installedFiles": installed}
    entries["deployment-manifest.json"] = ((json.dumps(manifest, ensure_ascii=False, indent=2) + "\n").encode(), False)
    entries["payload/archive.sha256"] = (checksum_lines({name: hashlib.sha256(data).hexdigest() for name, (data, _) in entries.items()}), False)
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="module-build-", dir=output.parent) as temporary:
        staged = Path(temporary) / "module"
        staged.mkdir()
        for name, (data, executable) in entries.items():
            destination = staged / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
            destination.chmod(0o755 if executable else 0o644)
        staged.rename(output)
    report = {"path": str(output), "bytes": sum(len(data) for data, _ in entries.values()), "files": len(entries),
              "app": "android-module", "bootstrap": "app_process", "abi": "arm64-v8a", "nativeLibraries": len(present),
              "apkBytes": len(apk_bytes), "apkSha256": hashlib.sha256(apk_bytes).hexdigest(),
              "separateCoreExecutable": False, "requiresMountProvider": True, "requiresReboot": True}
    output.with_name(output.name + ".verification.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


def main() -> int:
    """Build the module application and publish its folder as the canonical product artifact."""
    args = parse_args()
    if args.output.exists():
        raise FileExistsError(f"Module output already exists: {args.output}")
    if args.release_version is not None and args.build_number is None:
        raise ValueError("--release-version requires --build-number")
    apk = args.apk.resolve() if args.apk else build_apk(args.build_name, args.build_number, args.enforce_lockfile)
    print(json.dumps(assemble(apk, args.output, release_version=args.release_version,
                              build_number=args.build_number), ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
