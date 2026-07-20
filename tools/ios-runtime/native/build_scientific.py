#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import os
import platform
import plistlib
import shutil
import subprocess
import sys
import tarfile
import urllib.request
import zipfile
from dataclasses import dataclass
from pathlib import Path


TOOL_DIR = Path(__file__).resolve().parent
CACHE_DIR = TOOL_DIR / "cache"
SOURCE_DIR = TOOL_DIR / "sources"
BUILD_DIR = TOOL_DIR / "build"
OUTPUT_DIR = TOOL_DIR / "output"
WHEEL_DIR = BUILD_DIR / "wheels"
FRAMEWORK_NAME = "OperitPythonScientific"
FRAMEWORK_OUTPUT = OUTPUT_DIR / f"{FRAMEWORK_NAME}.xcframework"
PYTHON_VERSION = "3.13"
MINIMUM_IOS_VERSION = "13.0"


@dataclass(frozen=True)
class SourcePackage:
    """Describes one source distribution compiled into the embedded scientific runtime."""

    name: str
    version: str
    url: str
    sha256: str


NUMPY = SourcePackage(
    name="numpy",
    version="2.2.3",
    url=(
        "https://files.pythonhosted.org/packages/fb/90/8956572f5c4ae52201fdec7ba2044b2c"
        "882832dcec7d5d0922c9e9acf2de/numpy-2.2.3.tar.gz"
    ),
    sha256="dbdc15f0c81611925f382dfa97b3bd0bc2c1ce19d4fe50482cb0ddc12ba30020",
)
SCIPY = SourcePackage(
    name="scipy",
    version="1.15.2",
    url=(
        "https://files.pythonhosted.org/packages/b7/b9/31ba9cd990e626574baf93fbc1ac61cf"
        "9ed54faafd04c479117517661637/scipy-1.15.2.tar.gz"
    ),
    sha256="cd58a314d92838f7e6f755c8a2167ead4f27e1fd5c1251fd54289569ef3495ec",
)
IOS_BUILD_TAGS = (
    "cp313-ios_arm64_iphoneos",
    "cp313-ios_arm64_iphonesimulator",
    "cp313-ios_x86_64_iphonesimulator",
)


def file_sha256(path: Path) -> str:
    """Calculates the SHA-256 digest of one local file."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run(command: list[str], *, cwd: Path | None = None, env: dict[str, str] | None = None) -> None:
    """Runs one required native scientific build command without suppressing failures."""
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, env=env, check=True)


def require_macos_xcode() -> None:
    """Requires the macOS Xcode SDK needed to build signed iOS native extension slices."""
    if platform.system() != "Darwin":
        raise RuntimeError("iOS NumPy/SciPy builds require macOS with Xcode")
    missing = [command for command in ["xcrun", "xcodebuild", "lipo"] if shutil.which(command) is None]
    if missing:
        raise RuntimeError("iOS NumPy/SciPy build tools are missing: " + ", ".join(missing))


def source_archive_path(package: SourcePackage) -> Path:
    """Returns the cache path used by one pinned scientific package source archive."""
    return CACHE_DIR / f"{package.name}-{package.version}.tar.gz"


def download_source(package: SourcePackage) -> Path:
    """Downloads and verifies one pinned NumPy or SciPy source distribution."""
    target = source_archive_path(package)
    if not target.is_file() or file_sha256(target) != package.sha256:
        target.parent.mkdir(parents=True, exist_ok=True)
        temporary = target.with_suffix(".part")
        temporary.unlink(missing_ok=True)
        with urllib.request.urlopen(package.url, timeout=120) as response:
            with temporary.open("wb") as output:
                shutil.copyfileobj(response, output, length=1024 * 1024)
        actual_sha256 = file_sha256(temporary)
        if actual_sha256 != package.sha256:
            raise RuntimeError(
                f"scientific source checksum is invalid: {package.name} "
                f"expected={package.sha256} actual={actual_sha256}"
            )
        temporary.replace(target)
    return target


def extract_source(package: SourcePackage) -> Path:
    """Extracts one verified scientific package source tree into a clean build directory."""
    target = SOURCE_DIR / f"{package.name}-{package.version}"
    shutil.rmtree(target, ignore_errors=True)
    target.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(download_source(package), "r:gz") as archive:
        roots = {member.name.split("/", 1)[0] for member in archive.getmembers() if member.name}
        expected_root = f"{package.name}-{package.version}"
        if roots != {expected_root}:
            raise RuntimeError(f"scientific source root is invalid: {package.name}")
        for member in archive.getmembers():
            destination = (SOURCE_DIR / member.name).resolve()
            if SOURCE_DIR.resolve() not in destination.parents and destination != SOURCE_DIR.resolve():
                raise RuntimeError(f"scientific source entry escapes build directory: {member.name}")
            if member.issym() or member.islnk() or (not member.isfile() and not member.isdir()):
                raise RuntimeError(f"scientific source contains unsupported member: {member.name}")
        archive.extractall(SOURCE_DIR, members=archive.getmembers())
    return target


def build_wheels(package: SourcePackage, source: Path) -> None:
    """Builds all device and simulator wheels for one scientific source distribution."""
    env = os.environ.copy()
    env["CIBW_BUILD"] = " ".join(IOS_BUILD_TAGS)
    env["CIBW_ARCHS_IOS"] = "arm64 x86_64"
    env["CIBW_ENVIRONMENT"] = (
        "NPY_BLAS_ORDER=accelerate "
        "NPY_LAPACK_ORDER=accelerate "
        f"IPHONEOS_DEPLOYMENT_TARGET={MINIMUM_IOS_VERSION}"
    )
    env["CIBW_TEST_SKIP"] = "*"
    run(
        [sys.executable, "-m", "cibuildwheel", "--platform", "ios", "--output-dir", str(WHEEL_DIR), str(source)],
        env=env,
    )


def wheel_path(package: SourcePackage, build_tag: str) -> Path:
    """Resolves one mandatory wheel selected by package and cibuildwheel iOS build tag."""
    candidates = sorted(WHEEL_DIR.glob(f"{package.name}-{package.version}-*{build_tag}*.whl"))
    if len(candidates) != 1:
        raise RuntimeError(
            f"expected one {package.name} wheel for {build_tag}, found {len(candidates)}"
        )
    return candidates[0]


def extract_wheels(packages: list[SourcePackage], build_tag: str, destination: Path) -> None:
    """Extracts the exact package wheels for one iOS architecture into site-packages."""
    destination.mkdir(parents=True, exist_ok=True)
    for package in packages:
        with zipfile.ZipFile(wheel_path(package, build_tag)) as wheel:
            for member in wheel.infolist():
                target = (destination / member.filename).resolve()
                if destination.resolve() not in target.parents and target != destination.resolve():
                    raise RuntimeError(f"scientific wheel entry escapes site-packages: {member.filename}")
            wheel.extractall(destination)


def framework_info() -> dict[str, object]:
    """Returns the canonical Info.plist contents for each embedded scientific framework slice."""
    return {
        "CFBundleDevelopmentRegion": "en",
        "CFBundleExecutable": FRAMEWORK_NAME,
        "CFBundleIdentifier": "com.ai.assistance.operit2.python-scientific",
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleName": FRAMEWORK_NAME,
        "CFBundlePackageType": "FMWK",
        "CFBundleShortVersionString": "1.0",
        "CFBundleVersion": "1",
        "MinimumOSVersion": MINIMUM_IOS_VERSION,
    }


def make_framework_binary(sdk: str, architectures: list[str], destination: Path) -> None:
    """Builds the dynamic marker binary that makes scientific resources a signed framework."""
    source = BUILD_DIR / "operit_python_scientific.c"
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text("int operit_python_scientific_marker(void) { return 1; }\n", encoding="utf-8")
    minimum_version_flag = (
        "-mios-simulator-version-min=" + MINIMUM_IOS_VERSION
        if sdk == "iphonesimulator"
        else "-miphoneos-version-min=" + MINIMUM_IOS_VERSION
    )
    architecture_arguments = [argument for architecture in architectures for argument in ["-arch", architecture]]
    run(
        [
            "xcrun",
            "--sdk",
            sdk,
            "clang",
            "-dynamiclib",
            *architecture_arguments,
            minimum_version_flag,
            "-install_name",
            f"@rpath/{FRAMEWORK_NAME}.framework/{FRAMEWORK_NAME}",
            str(source),
            "-o",
            str(destination),
        ]
    )


def create_framework_slice(
    name: str,
    sdk: str,
    architectures: list[str],
    site_packages: Path,
) -> Path:
    """Creates one resource-bearing framework slice containing architecture-matched extensions."""
    framework = BUILD_DIR / name / f"{FRAMEWORK_NAME}.framework"
    shutil.rmtree(framework.parent, ignore_errors=True)
    resources = framework / "Resources" / "python" / "site-packages"
    resources.parent.mkdir(parents=True)
    shutil.copytree(site_packages, resources)
    with (framework / "Info.plist").open("wb") as stream:
        plistlib.dump(framework_info(), stream)
    make_framework_binary(sdk, architectures, framework / FRAMEWORK_NAME)
    return framework


def merge_simulator_extensions(arm64_site_packages: Path, x86_site_packages: Path, destination: Path) -> None:
    """Creates universal simulator extension modules by lipo-merging every native Python module."""
    shutil.copytree(arm64_site_packages, destination)
    arm64_extensions = sorted(path for path in destination.rglob("*.so") if path.is_file())
    for arm64_extension in arm64_extensions:
        relative = arm64_extension.relative_to(destination)
        x86_extension = x86_site_packages / relative
        if not x86_extension.is_file():
            raise RuntimeError(f"x86_64 simulator extension is missing: {relative}")
        universal_extension = arm64_extension.with_suffix(".universal")
        run(["lipo", "-create", str(arm64_extension), str(x86_extension), "-output", str(universal_extension)])
        universal_extension.replace(arm64_extension)


def create_xcframework(packages: list[SourcePackage]) -> None:
    """Creates a device and universal-simulator XCFramework carrying NumPy and SciPy extensions."""
    device_site_packages = BUILD_DIR / "device-site-packages"
    arm64_simulator_site_packages = BUILD_DIR / "arm64-simulator-site-packages"
    x86_simulator_site_packages = BUILD_DIR / "x86-simulator-site-packages"
    universal_simulator_site_packages = BUILD_DIR / "universal-simulator-site-packages"
    for directory in [
        device_site_packages,
        arm64_simulator_site_packages,
        x86_simulator_site_packages,
        universal_simulator_site_packages,
    ]:
        shutil.rmtree(directory, ignore_errors=True)
    extract_wheels(packages, IOS_BUILD_TAGS[0], device_site_packages)
    extract_wheels(packages, IOS_BUILD_TAGS[1], arm64_simulator_site_packages)
    extract_wheels(packages, IOS_BUILD_TAGS[2], x86_simulator_site_packages)
    merge_simulator_extensions(
        arm64_simulator_site_packages,
        x86_simulator_site_packages,
        universal_simulator_site_packages,
    )
    device_framework = create_framework_slice(
        "device", "iphoneos", ["arm64"], device_site_packages
    )
    simulator_framework = create_framework_slice(
        "simulator", "iphonesimulator", ["arm64", "x86_64"], universal_simulator_site_packages
    )
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


def verify_output() -> None:
    """Verifies that both generated framework slices carry NumPy and SciPy package resources."""
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
        raise RuntimeError("scientific XCFramework files are missing: " + ", ".join(missing))


def main() -> int:
    """Builds the complete signed-framework payload used by embedded iOS NumPy and SciPy."""
    require_macos_xcode()
    run([sys.executable, "-m", "pip", "install", "cibuildwheel==2.22.0"])
    shutil.rmtree(WHEEL_DIR, ignore_errors=True)
    WHEEL_DIR.mkdir(parents=True)
    packages = [NUMPY, SCIPY]
    for package in packages:
        build_wheels(package, extract_source(package))
    create_xcframework(packages)
    verify_output()
    print(f"iOS scientific XCFramework: {FRAMEWORK_OUTPUT}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
