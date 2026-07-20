#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import platform
import plistlib
import re
import shutil
import subprocess
import tarfile
import urllib.request
from pathlib import Path


TOOL_DIR = Path(__file__).resolve().parent
RUNTIME_DIR = TOOL_DIR.parent
ROOT_DIR = RUNTIME_DIR.parents[1]
FLUTTER_APP_DIR = ROOT_DIR / "apps" / "flutter" / "app"
CACHE_DIR = FLUTTER_APP_DIR / "apple" / "downloads" / "ios-runtime" / "toybox"
BUILD_DIR = TOOL_DIR / "build"
OUTPUT_DIR = TOOL_DIR / "output"
MANIFEST_PATH = TOOL_DIR / "manifest.json"
CONFIG_PATH = TOOL_DIR / "toybox.config"
RUNNER_PATH = TOOL_DIR / "operit_toybox_runner.c"
PUBLIC_HEADER_PATH = TOOL_DIR / "operit_toybox.h"
STREAMS_HEADER_PATH = TOOL_DIR / "operit_toybox_streams.h"
FRAMEWORK_NAME = "OperitToybox"
FRAMEWORK_OUTPUT = OUTPUT_DIR / f"{FRAMEWORK_NAME}.xcframework"
MINIMUM_IOS_VERSION = "13.0"


def run(command: list[str], *, cwd: Path | None = None, env: dict[str, str] | None = None) -> None:
    """Runs one required Toybox build command and preserves its failure output."""
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, env=env, check=True)


def file_sha256(path: Path) -> str:
    """Calculates the SHA-256 digest of one local archive."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_source_manifest() -> dict[str, str]:
    """Loads the exact Toybox release metadata used by the iOS build."""
    with MANIFEST_PATH.open(encoding="utf-8") as stream:
        manifest = json.load(stream)
    source = manifest.get("source")
    if not isinstance(source, dict):
        raise RuntimeError("Toybox manifest source entry is invalid")
    required = ["name", "version", "url", "sha256"]
    if any(not isinstance(source.get(key), str) or not source[key] for key in required):
        raise RuntimeError("Toybox manifest source fields are invalid")
    return {key: source[key] for key in required}


def source_archive_path(source: dict[str, str]) -> Path:
    """Returns the stable local cache path for the pinned Toybox source archive."""
    return CACHE_DIR / f"{source['name']}-{source['version']}.tar.gz"


def download_source(source: dict[str, str]) -> Path:
    """Downloads and checksum-verifies the pinned Toybox source archive."""
    target = source_archive_path(source)
    expected_sha256 = source["sha256"].lower()
    if target.is_file() and file_sha256(target) == expected_sha256:
        return target
    target.parent.mkdir(parents=True, exist_ok=True)
    temporary = target.with_suffix(".part")
    temporary.unlink(missing_ok=True)
    with urllib.request.urlopen(source["url"], timeout=120) as response:
        with temporary.open("wb") as output:
            shutil.copyfileobj(response, output, length=1024 * 1024)
    actual_sha256 = file_sha256(temporary)
    if actual_sha256 != expected_sha256:
        raise RuntimeError(
            f"Toybox source checksum is invalid: expected={expected_sha256} actual={actual_sha256}"
        )
    temporary.replace(target)
    return target


def extract_source(source: dict[str, str], destination: Path) -> Path:
    """Extracts the verified Toybox source into one clean architecture build directory."""
    shutil.rmtree(destination, ignore_errors=True)
    destination.mkdir(parents=True)
    expected_root = f"{source['name']}-{source['version']}"
    with tarfile.open(download_source(source), "r:gz") as archive:
        members = archive.getmembers()
        roots = {member.name.split("/", 1)[0] for member in members if member.name}
        if roots != {expected_root}:
            raise RuntimeError("Toybox source archive root is invalid")
        root = destination.resolve()
        for member in members:
            extracted = (destination / member.name).resolve()
            if extracted != root and root not in extracted.parents:
                raise RuntimeError(f"Toybox source entry escapes build directory: {member.name}")
            if member.issym() or member.islnk() or (not member.isfile() and not member.isdir()):
                raise RuntimeError(f"Toybox source archive member is unsupported: {member.name}")
        archive.extractall(destination, members=members)
    return destination / expected_root


def require_macos_xcode() -> None:
    """Requires the macOS Xcode tools that compile iOS framework slices."""
    if platform.system() != "Darwin":
        raise RuntimeError("iOS Toybox builds require macOS with Xcode")
    missing = [tool for tool in ["make", "xcodebuild", "xcrun", "libtool", "lipo"] if shutil.which(tool) is None]
    if missing:
        raise RuntimeError("iOS Toybox build tools are missing: " + ", ".join(missing))


def set_toybox_config(source: Path) -> None:
    """Applies the committed no-fork Toybox command configuration to one source tree."""
    requested: dict[str, str] = {}
    for raw_line in CONFIG_PATH.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line:
            continue
        match = re.fullmatch(r"CONFIG_([A-Z0-9_]+)=([yn])", line)
        if not match:
            raise RuntimeError(f"Toybox configuration entry is invalid: {line}")
        requested[match.group(1)] = match.group(2)
    config_path = source / ".config"
    current = config_path.read_text(encoding="utf-8")
    for symbol, value in requested.items():
        enabled = f"CONFIG_{symbol}=y"
        disabled = f"# CONFIG_{symbol} is not set"
        replacement = enabled if value == "y" else disabled
        current, count = re.subn(
            rf"^(?:CONFIG_{re.escape(symbol)}=.[^\n]*|# CONFIG_{re.escape(symbol)} is not set)$",
            replacement,
            current,
            flags=re.MULTILINE,
        )
        if count != 1:
            raise RuntimeError(f"Toybox configuration symbol is unavailable: {symbol}")
    config_path.write_text(current, encoding="utf-8")


def patch_toybox_sources(source: Path) -> None:
    """Installs the stream indirection and compiles the Toybox wrapper with its source objects."""
    shutil.copy2(RUNNER_PATH, source / RUNNER_PATH.name)
    shutil.copy2(PUBLIC_HEADER_PATH, source / PUBLIC_HEADER_PATH.name)
    shutil.copy2(STREAMS_HEADER_PATH, source / STREAMS_HEADER_PATH.name)
    toys_header = source / "toys.h"
    header_source = toys_header.read_text(encoding="utf-8")
    header_anchor = '#include "lib/toyflags.h"\n'
    if header_source.count(header_anchor) != 1:
        raise RuntimeError("Toybox stream injection anchor is invalid")
    toys_header.write_text(
        header_source.replace(header_anchor, header_anchor + '#include "operit_toybox_streams.h"\n'),
        encoding="utf-8",
    )
    make_script = source / "scripts" / "make.sh"
    make_source = make_script.read_text(encoding="utf-8")
    make_anchor = "for i in lib/*.c click $TOYFILES\n"
    if make_source.count(make_anchor) != 1:
        raise RuntimeError("Toybox wrapper compilation anchor is invalid")
    make_script.write_text(
        make_source.replace(make_anchor, "for i in lib/*.c operit_toybox_runner.c click $TOYFILES\n"),
        encoding="utf-8",
    )


def compile_embedded_main(source: Path, compiler: str, flags: list[str]) -> None:
    """Recompiles Toybox dispatch code without exporting an application main symbol."""
    main_source = source / "main.c"
    main_text = main_source.read_text(encoding="utf-8")
    main_anchor = "int main(int argc, char *argv[])\n"
    if main_text.count(main_anchor) != 1:
        raise RuntimeError("Toybox main entry-point anchor is invalid")
    main_source.write_text(
        main_text.replace(main_anchor, "int operit_toybox_embedded_main(int argc, char *argv[])\n"),
        encoding="utf-8",
    )
    run(
        [
            compiler,
            *flags,
            "-I",
            str(source),
            "-c",
            str(main_source),
            "-o",
            str(source / "generated" / "obj" / "main.o"),
        ],
        cwd=source,
    )


def sdk_path(sdk: str) -> str:
    """Resolves the SDK root used for one Xcode compilation target."""
    return subprocess.check_output(["xcrun", "--sdk", sdk, "--show-sdk-path"], text=True).strip()


def clang_path(sdk: str) -> str:
    """Resolves the target C compiler path from the active Xcode installation."""
    return subprocess.check_output(["xcrun", "--sdk", sdk, "--find", "clang"], text=True).strip()


def target_compile_flags(sdk: str, architectures: list[str]) -> list[str]:
    """Builds the common C compiler flags for one device or simulator framework slice."""
    version_flag = (
        f"-mios-simulator-version-min={MINIMUM_IOS_VERSION}"
        if sdk == "iphonesimulator"
        else f"-miphoneos-version-min={MINIMUM_IOS_VERSION}"
    )
    architecture_flags = [flag for architecture in architectures for flag in ["-arch", architecture]]
    return [
        "-O2",
        "-fPIC",
        "-isysroot",
        sdk_path(sdk),
        version_flag,
        *architecture_flags,
    ]


def framework_info() -> dict[str, object]:
    """Returns the metadata embedded in every static OperitToybox framework slice."""
    return {
        "CFBundleDevelopmentRegion": "en",
        "CFBundleExecutable": FRAMEWORK_NAME,
        "CFBundleIdentifier": "app.operit.runtime.toybox",
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleName": FRAMEWORK_NAME,
        "CFBundlePackageType": "FMWK",
        "CFBundleShortVersionString": "0.8.12",
        "CFBundleVersion": "1",
        "MinimumOSVersion": MINIMUM_IOS_VERSION,
    }


def create_framework_slice(source: Path, sdk: str, architectures: list[str], name: str) -> Path:
    """Compiles one static framework slice containing Toybox and the in-process bridge."""
    flags = target_compile_flags(sdk, architectures)
    environment = {
        "CC": clang_path(sdk),
        "CFLAGS": " ".join(flags),
        "HOSTCC": clang_path("macosx"),
        "NOSTRIP": "1",
        "OPTIMIZE": "",
    }
    run(["make", "allnoconfig"], cwd=source, env=environment)
    set_toybox_config(source)
    run(["make", "silentoldconfig"], cwd=source, env=environment)
    patch_toybox_sources(source)
    run(["make"], cwd=source, env=environment)
    compile_embedded_main(source, clang_path(sdk), flags)
    objects = sorted((source / "generated" / "obj").glob("*.o"))
    if not objects:
        raise RuntimeError("Toybox build did not produce object files")
    framework = BUILD_DIR / name / f"{FRAMEWORK_NAME}.framework"
    shutil.rmtree(framework.parent, ignore_errors=True)
    headers = framework / "Headers"
    modules = framework / "Modules"
    headers.mkdir(parents=True)
    modules.mkdir()
    shutil.copy2(PUBLIC_HEADER_PATH, headers / PUBLIC_HEADER_PATH.name)
    (modules / "module.modulemap").write_text(
        f'framework module {FRAMEWORK_NAME} {{\n  header "{PUBLIC_HEADER_PATH.name}"\n  export *\n}}\n',
        encoding="utf-8",
    )
    with (framework / "Info.plist").open("wb") as stream:
        plistlib.dump(framework_info(), stream)
    run(
        ["libtool", "-static", "-o", str(framework / FRAMEWORK_NAME), *map(str, objects)],
        cwd=source,
        env=environment,
    )
    return framework


def verify_framework_slice(framework: Path, expected_architectures: list[str]) -> None:
    """Validates that one static framework contains every requested architecture."""
    binary = framework / FRAMEWORK_NAME
    if not binary.is_file():
        raise RuntimeError(f"Toybox framework binary is missing: {binary}")
    details = subprocess.check_output(["lipo", "-archs", str(binary)], text=True).split()
    if sorted(details) != sorted(expected_architectures):
        raise RuntimeError(
            f"Toybox framework architectures are invalid: expected={expected_architectures} actual={details}"
        )


def create_xcframework(source: dict[str, str]) -> None:
    """Builds the device and universal-simulator OperitToybox XCFramework."""
    device_framework = create_framework_slice(
        extract_source(source, BUILD_DIR / "iphoneos-source"), "iphoneos", ["arm64"], "iphoneos"
    )
    simulator_framework = create_framework_slice(
        extract_source(source, BUILD_DIR / "iphonesimulator-source"),
        "iphonesimulator",
        ["arm64", "x86_64"],
        "iphonesimulator",
    )
    verify_framework_slice(device_framework, ["arm64"])
    verify_framework_slice(simulator_framework, ["arm64", "x86_64"])
    shutil.rmtree(FRAMEWORK_OUTPUT, ignore_errors=True)
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    run(
        [
            "xcodebuild",
            "-create-xcframework",
            "-framework",
            str(device_framework),
            "-framework",
            str(simulator_framework),
            "-output",
            str(FRAMEWORK_OUTPUT),
        ]
    )


def verify_xcframework() -> None:
    """Verifies that the completed XCFramework has device and simulator framework slices."""
    required = [
        FRAMEWORK_OUTPUT / "Info.plist",
        FRAMEWORK_OUTPUT / "ios-arm64" / f"{FRAMEWORK_NAME}.framework" / FRAMEWORK_NAME,
        FRAMEWORK_OUTPUT
        / "ios-arm64_x86_64-simulator"
        / f"{FRAMEWORK_NAME}.framework"
        / FRAMEWORK_NAME,
    ]
    missing = [str(path) for path in required if not path.is_file()]
    if missing:
        raise RuntimeError("Toybox XCFramework output is incomplete: " + ", ".join(missing))


def main() -> int:
    """Builds the complete embedded no-fork Toybox runtime for iOS."""
    require_macos_xcode()
    create_xcframework(load_source_manifest())
    verify_xcframework()
    print(f"iOS Toybox XCFramework: {FRAMEWORK_OUTPUT}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
