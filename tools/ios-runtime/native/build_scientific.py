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
class SourceSubmodule:
    """Describes one pinned Git submodule required by a scientific source tree."""

    name: str
    version: str
    url: str
    sha256: str
    archive_root: str
    destination: str


@dataclass(frozen=True)
class SourcePackage:
    """Describes one source distribution compiled into the embedded scientific runtime."""

    name: str
    version: str
    url: str
    sha256: str
    archive_root: str
    submodules: tuple[SourceSubmodule, ...] = ()


NUMPY = SourcePackage(
    name="numpy",
    version="2.2.3",
    url=(
        "https://files.pythonhosted.org/packages/fb/90/8956572f5c4ae52201fdec7ba2044b2c"
        "882832dcec7d5d0922c9e9acf2de/numpy-2.2.3.tar.gz"
    ),
    sha256="dbdc15f0c81611925f382dfa97b3bd0bc2c1ce19d4fe50482cb0ddc12ba30020",
    archive_root="numpy-2.2.3",
)
SCIPY = SourcePackage(
    name="scipy",
    version="2.0.0.dev0",
    url="https://codeload.github.com/scipy/scipy/tar.gz/bc48810bef70f93b67d107f5263f267affc0fbe2",
    sha256="0ce3e85ff03bbbaac6a3f5597737f0b8dc2f0ffe79670a1d9c60d0fb3c122775",
    archive_root="scipy-bc48810bef70f93b67d107f5263f267affc0fbe2",
    submodules=(
        SourceSubmodule(
            name="array-api-compat",
            version="e1d4eed1389f1d93318ec855730488db48475320",
            url=(
                "https://codeload.github.com/data-apis/array-api-compat/tar.gz/"
                "e1d4eed1389f1d93318ec855730488db48475320"
            ),
            sha256="c1b86317a38b9b52759fdf8bb1ccf2d08e683e2e5374d13a048d03c3329d884e",
            archive_root="array-api-compat-e1d4eed1389f1d93318ec855730488db48475320",
            destination="subprojects/array_api_compat",
        ),
        SourceSubmodule(
            name="array-api-extra",
            version="842457a8e22f9bad8b11ef547073d1bed2a2a8a5",
            url=(
                "https://codeload.github.com/data-apis/array-api-extra/tar.gz/"
                "842457a8e22f9bad8b11ef547073d1bed2a2a8a5"
            ),
            sha256="49b11aec2af72e7dacfbed8b53cbc9a1cd83fcf2cd4ccc1540a99059dcca5b06",
            archive_root="array-api-extra-842457a8e22f9bad8b11ef547073d1bed2a2a8a5",
            destination="subprojects/array_api_extra",
        ),
        SourceSubmodule(
            name="boost-math",
            version="c114cf4a6ae958ce87ed6db4d55c1db4ec0a9c74",
            url=(
                "https://codeload.github.com/boostorg/math/tar.gz/"
                "c114cf4a6ae958ce87ed6db4d55c1db4ec0a9c74"
            ),
            sha256="807688f4197e00112fb8f3ba2223eb71de6a427f528b985f5ffe1f284b83f304",
            archive_root="math-c114cf4a6ae958ce87ed6db4d55c1db4ec0a9c74",
            destination="subprojects/boost_math/math",
        ),
        SourceSubmodule(
            name="cobyqa",
            version="69b7177b9febb9178a5f7a9d61200407f1f77d25",
            url="https://codeload.github.com/cobyqa/cobyqa/tar.gz/69b7177b9febb9178a5f7a9d61200407f1f77d25",
            sha256="d2e5bf006bbfe670387645dae3264b588ea3f30edfff6bee4d315aa12364a463",
            archive_root="cobyqa-69b7177b9febb9178a5f7a9d61200407f1f77d25",
            destination="subprojects/cobyqa",
        ),
        SourceSubmodule(
            name="highs",
            version="4f96ee8f40b2d3a46e00d6137d5eb31044680776",
            url="https://codeload.github.com/scipy/HiGHs/tar.gz/4f96ee8f40b2d3a46e00d6137d5eb31044680776",
            sha256="562e3bfc1932d3280e2334e44a818cecf5b1ef8c8d11ef80e75f260b60ad633e",
            archive_root="HiGHS-4f96ee8f40b2d3a46e00d6137d5eb31044680776",
            destination="subprojects/highs",
        ),
        SourceSubmodule(
            name="unuran",
            version="7e2344e93eb688acc430393c12228419fa130b9b",
            url="https://codeload.github.com/scipy/unuran/tar.gz/7e2344e93eb688acc430393c12228419fa130b9b",
            sha256="af5395b88018c8a29c0ec6e4b576129a6209e8581334052e8f1192949a477a38",
            archive_root="unuran-7e2344e93eb688acc430393c12228419fa130b9b",
            destination="subprojects/unuran",
        ),
        SourceSubmodule(
            name="xsf",
            version="69b66c3d76b5006ae9a26901b3a0d11cbbf4d664",
            url="https://codeload.github.com/scipy/xsf/tar.gz/69b66c3d76b5006ae9a26901b3a0d11cbbf4d664",
            sha256="036fee30e3d5123bf8d643338a6d73709274593905dc8bb92eb3156480c684c1",
            archive_root="xsf-69b66c3d76b5006ae9a26901b3a0d11cbbf4d664",
            destination="subprojects/xsf",
        ),
    ),
)
IOS_BUILD_TAGS = (
    "cp313-ios_arm64_iphoneos",
    "cp313-ios_arm64_iphonesimulator",
    "cp313-ios_x86_64_iphonesimulator",
)
CIBW_CONFIG_SETTINGS_BY_PACKAGE = {
    "numpy": (
        "setup-args=-Duse-ilp64=false "
        "setup-args=-Dallow-noblas=false "
        "setup-args=-Dblas=accelerate "
        "setup-args=-Dlapack=accelerate "
        "build-dir=build"
    ),
    "scipy": (
        "setup-args=-Dblas=accelerate "
        "setup-args=-Dlapack=accelerate "
        "build-dir=build"
    ),
}


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


def source_archive_path(source: SourcePackage | SourceSubmodule) -> Path:
    """Returns the cache path used by one pinned scientific source or source submodule archive."""
    return CACHE_DIR / f"{source.name}-{source.version}.tar.gz"


def download_source(source: SourcePackage | SourceSubmodule) -> Path:
    """Downloads and verifies one pinned scientific source or source submodule archive."""
    target = source_archive_path(source)
    if not target.is_file() or file_sha256(target) != source.sha256:
        target.parent.mkdir(parents=True, exist_ok=True)
        temporary = target.with_suffix(".part")
        temporary.unlink(missing_ok=True)
        with urllib.request.urlopen(source.url, timeout=120) as response:
            with temporary.open("wb") as output:
                shutil.copyfileobj(response, output, length=1024 * 1024)
        actual_sha256 = file_sha256(temporary)
        if actual_sha256 != source.sha256:
            raise RuntimeError(
                f"scientific source checksum is invalid: {source.name} "
                f"expected={source.sha256} actual={actual_sha256}"
            )
        temporary.replace(target)
    return target


def extract_archive(source: SourcePackage | SourceSubmodule, destination: Path) -> Path:
    """Extracts a verified archive while requiring every entry to remain within its declared root."""
    target = destination / source.archive_root
    shutil.rmtree(target, ignore_errors=True)
    target.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(download_source(source), "r:gz") as archive:
        members = archive.getmembers()
        roots = {member.name.split("/", 1)[0] for member in archive.getmembers() if member.name}
        expected_root = source.archive_root
        if roots != {expected_root}:
            raise RuntimeError(f"scientific source root is invalid: {source.name}")
        root_member = next(
            (member for member in members if member.name.rstrip("/") == expected_root),
            None,
        )
        if root_member is None or not root_member.isdir():
            raise RuntimeError(f"scientific source root is not a directory: {source.name}")
        resolved_destination = destination.resolve()
        resolved_target = target.resolve()
        for member in members:
            member_destination = (destination / member.name).resolve()
            if (
                resolved_destination not in member_destination.parents
                and member_destination != resolved_destination
            ):
                raise RuntimeError(f"scientific source entry escapes build directory: {member.name}")
            if member.islnk() or (not member.isfile() and not member.isdir() and not member.issym()):
                raise RuntimeError(f"scientific source contains unsupported member: {member.name}")
            if member.issym():
                link_destination = (member_destination.parent / member.linkname).resolve()
                if (
                    resolved_target not in link_destination.parents
                    and link_destination != resolved_target
                ):
                    raise RuntimeError(
                        f"scientific source symbolic link escapes source tree: {member.name}"
                    )
        archive.extractall(destination, members=members, filter="data")
    return target


def extract_submodule(submodule: SourceSubmodule, source: Path) -> None:
    """Places one verified Git submodule archive at the exact path required by its parent source."""
    resolved_source = source.resolve()
    target = (source / submodule.destination).resolve()
    if resolved_source not in target.parents:
        raise RuntimeError(f"scientific source submodule path escapes source tree: {submodule.name}")
    if target.exists():
        if not target.is_dir():
            raise RuntimeError(f"scientific source submodule path is not a directory: {submodule.destination}")
        if any(target.iterdir()):
            raise RuntimeError(f"scientific source submodule path is not empty: {submodule.destination}")
        target.rmdir()
    staging = BUILD_DIR / "submodule-sources"
    extracted = extract_archive(submodule, staging)
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(extracted), str(target))


def extract_source(package: SourcePackage) -> Path:
    """Extracts one package source and all mandatory Git submodules into the scientific build tree."""
    target = extract_archive(package, SOURCE_DIR)
    for submodule in package.submodules:
        extract_submodule(submodule, target)
    return target


def build_wheels(package: SourcePackage, source: Path, build_python: Path) -> None:
    """Builds all device and simulator wheels for one scientific source distribution."""
    env = os.environ.copy()
    env["CIBW_BUILD"] = " ".join(IOS_BUILD_TAGS)
    env["CIBW_ARCHS_IOS"] = "arm64_iphoneos arm64_iphonesimulator x86_64_iphonesimulator"
    env["CIBW_BEFORE_BUILD"] = ""
    env["CIBW_XBUILD_TOOLS"] = "cmake ninja"
    env["CIBW_CONFIG_SETTINGS"] = CIBW_CONFIG_SETTINGS_BY_PACKAGE[package.name]
    env["CIBW_ENVIRONMENT"] = (
        "NPY_BLAS_ORDER=accelerate "
        "NPY_LAPACK_ORDER=accelerate "
        f"IPHONEOS_DEPLOYMENT_TARGET={MINIMUM_IOS_VERSION}"
    )
    env["CIBW_TEST_SKIP"] = "*"
    run(
        [
            str(build_python),
            "-m",
            "cibuildwheel",
            "--platform",
            "ios",
            "--output-dir",
            str(WHEEL_DIR),
            str(source),
        ],
        env=env,
    )


def prepare_cibuildwheel_python() -> Path:
    """Creates the local virtual environment used exclusively for cibuildwheel tooling."""
    virtual_environment = BUILD_DIR / "cibuildwheel-venv"
    run([sys.executable, "-m", "venv", str(virtual_environment)])
    python = virtual_environment / "bin" / "python"
    if not python.is_file():
        raise RuntimeError(f"cibuildwheel virtual environment is missing Python: {python}")
    run([str(python), "-m", "pip", "install", "cibuildwheel==4.1.1"])
    return python


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
    build_python = prepare_cibuildwheel_python()
    shutil.rmtree(WHEEL_DIR, ignore_errors=True)
    WHEEL_DIR.mkdir(parents=True)
    packages = [NUMPY, SCIPY]
    for package in packages:
        build_wheels(package, extract_source(package), build_python)
    create_xcframework(packages)
    verify_output()
    print(f"iOS scientific XCFramework: {FRAMEWORK_OUTPUT}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
