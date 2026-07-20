#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import shutil
import tarfile
import urllib.request
import zipfile
from dataclasses import dataclass
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parents[2]
FLUTTER_APP_DIR = ROOT_DIR / "apps" / "flutter" / "app"
APPLE_DIR = FLUTTER_APP_DIR / "apple"
CACHE_DIR = APPLE_DIR / "downloads" / "ios-runtime"
FRAMEWORKS_DIR = APPLE_DIR / "Frameworks"
PYTHON_RESOURCE_DIR = FLUTTER_APP_DIR / "ios" / "Runner" / "python"
NODE_RESOURCE_DIR = FLUTTER_APP_DIR / "ios" / "Runner" / "node"
PACKAGE_MANIFEST = Path(__file__).with_name("packages.json")
RESOURCE_DIR = Path(__file__).with_name("resources")
SCIENTIFIC_FRAMEWORK = (
    Path(__file__).with_name("native")
    / "output"
    / "OperitPythonScientific.xcframework"
)
TOYBOX_FRAMEWORK = (
    Path(__file__).with_name("toybox")
    / "output"
    / "OperitToybox.xcframework"
)
STAGING_DIR = APPLE_DIR / ".ios-runtime-staging"


@dataclass(frozen=True)
class RuntimeArchive:
    """Describes one verified external archive required by the embedded iOS runtime."""

    name: str
    url: str
    bytes: int
    sha256: str | None


NODE_ARCHIVE = RuntimeArchive(
    name="nodejs-mobile-v18.20.4-ios.zip",
    url=(
        "https://github.com/nodejs-mobile/nodejs-mobile/releases/download/v18.20.4/"
        "nodejs-mobile-v18.20.4-ios.zip"
    ),
    bytes=51_492_431,
    sha256=None,
)
PYTHON_ARCHIVE = RuntimeArchive(
    name="Python-3.13-iOS-support.b14.tar.gz",
    url=(
        "https://github.com/beeware/Python-Apple-support/releases/download/3.13-b14/"
        "Python-3.13-iOS-support.b14.tar.gz"
    ),
    bytes=32_387_292,
    sha256="8b5cb76ef8d8a2946052479358eeec9d54b4496cb60920e175ec1489b5cf7963",
)


def archive_path(archive: RuntimeArchive) -> Path:
    """Returns the persistent cache path of one runtime archive."""
    return CACHE_DIR / archive.name


def file_sha256(path: Path) -> str:
    """Calculates the SHA-256 digest of one local file."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url: str, target: Path, expected_sha256: str) -> None:
    """Downloads one pinned package archive and verifies its SHA-256 digest."""
    target.parent.mkdir(parents=True, exist_ok=True)
    temporary = target.with_suffix(target.suffix + ".part")
    temporary.unlink(missing_ok=True)
    with urllib.request.urlopen(url, timeout=120) as response:
        with temporary.open("wb") as output:
            shutil.copyfileobj(response, output, length=1024 * 1024)
    actual_sha256 = file_sha256(temporary)
    if actual_sha256 != expected_sha256:
        raise RuntimeError(
            f"iOS runtime download checksum is invalid: {target.name} "
            f"expected={expected_sha256} actual={actual_sha256}"
        )
    temporary.replace(target)


def ensure_runtime_archive(archive: RuntimeArchive) -> Path:
    """Downloads and validates one official native runtime archive in the persistent cache."""
    target = archive_path(archive)
    if not target.is_file() or target.stat().st_size != archive.bytes:
        CACHE_DIR.mkdir(parents=True, exist_ok=True)
        temporary = target.with_suffix(target.suffix + ".part")
        temporary.unlink(missing_ok=True)
        with urllib.request.urlopen(archive.url, timeout=120) as response:
            with temporary.open("wb") as output:
                shutil.copyfileobj(response, output, length=1024 * 1024)
        if temporary.stat().st_size != archive.bytes:
            raise RuntimeError(f"iOS runtime download size is invalid: {archive.name}")
        if archive.sha256 is not None and file_sha256(temporary) != archive.sha256:
            raise RuntimeError(f"iOS runtime download checksum is invalid: {archive.name}")
        temporary.replace(target)
    if target.stat().st_size != archive.bytes:
        raise RuntimeError(f"iOS runtime archive size is invalid: {target}")
    if archive.sha256 is not None and file_sha256(target) != archive.sha256:
        raise RuntimeError(f"iOS runtime archive checksum is invalid: {target}")
    return target


def require_safe_archive_path(root: Path, member_name: str) -> None:
    """Rejects archive members that resolve outside the designated staging directory."""
    destination = (root / member_name).resolve()
    resolved_root = root.resolve()
    if destination != resolved_root and resolved_root not in destination.parents:
        raise RuntimeError(f"iOS runtime archive entry escapes staging: {member_name}")


def copy_directory(source: Path, target: Path) -> None:
    """Replaces one generated application directory with verified staged contents."""
    if not source.is_dir():
        raise RuntimeError(f"iOS runtime component is missing: {source}")
    shutil.rmtree(target, ignore_errors=True)
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(source, target)


def load_manifest() -> dict[str, object]:
    """Loads the complete pinned iOS package manifest."""
    with PACKAGE_MANIFEST.open(encoding="utf-8") as stream:
        manifest = json.load(stream)
    if not isinstance(manifest, dict):
        raise RuntimeError("iOS package manifest must contain an object")
    return manifest


def package_archive_path(package: dict[str, object]) -> Path:
    """Returns the cached filename used by one package manifest entry."""
    name = str(package["name"])
    version = str(package["version"])
    suffix = Path(str(package["url"])).suffix
    return CACHE_DIR / "packages" / f"{name}-{version}{suffix}"


def ensure_package_archive(package: dict[str, object]) -> Path:
    """Downloads one manifest package archive and checks its pinned content digest."""
    target = package_archive_path(package)
    expected_sha256 = str(package["sha256"])
    if not target.is_file() or file_sha256(target) != expected_sha256:
        download(str(package["url"]), target, expected_sha256)
    return target


def extract_node_packages(manifest: dict[str, object], destination: Path) -> None:
    """Extracts all bundled JavaScript packages into the Node module search directory."""
    packages = manifest.get("node")
    if not isinstance(packages, list):
        raise RuntimeError("iOS package manifest node package list is invalid")
    modules = destination / "node_modules"
    modules.mkdir(parents=True, exist_ok=True)
    for entry in packages:
        if not isinstance(entry, dict):
            raise RuntimeError("iOS node package manifest entry is invalid")
        archive = ensure_package_archive(entry)
        package_name = str(entry["name"])
        package_root = modules / package_name
        with tarfile.open(archive, "r:gz") as bundle:
            for member in bundle.getmembers():
                if not member.name.startswith("package/"):
                    raise RuntimeError(
                        f"iOS Node package archive has invalid root: {archive.name}"
                    )
                require_safe_archive_path(package_root, member.name.removeprefix("package/"))
                if member.issym() or member.islnk():
                    raise RuntimeError(
                        f"iOS Node package archive contains a link: {archive.name}"
                    )
            package_root.mkdir(parents=True, exist_ok=True)
            for member in bundle.getmembers():
                relative_name = member.name.removeprefix("package/")
                if not relative_name:
                    continue
                member.name = relative_name
                bundle.extract(member, package_root)


def extract_python_packages(manifest: dict[str, object], destination: Path) -> None:
    """Extracts all bundled pure-Python wheels into the embedded CPython site-packages path."""
    packages = manifest.get("python")
    if not isinstance(packages, list):
        raise RuntimeError("iOS package manifest Python package list is invalid")
    for entry in packages:
        if not isinstance(entry, dict):
            raise RuntimeError("iOS Python package manifest entry is invalid")
        archive = ensure_package_archive(entry)
        with zipfile.ZipFile(archive) as wheel:
            for member in wheel.infolist():
                require_safe_archive_path(destination, member.filename)
            wheel.extractall(destination)


def extract_native_runtimes(destination: Path) -> tuple[Path, Path]:
    """Extracts official Node Mobile and CPython iOS runtime artifacts into staging."""
    node_staging = destination / "node"
    python_staging = destination / "python"
    node_staging.mkdir(parents=True)
    python_staging.mkdir(parents=True)
    with zipfile.ZipFile(ensure_runtime_archive(NODE_ARCHIVE)) as archive:
        for member in archive.infolist():
            require_safe_archive_path(node_staging, member.filename)
            if not member.is_dir() and not member.filename.startswith("NodeMobile.xcframework/"):
                raise RuntimeError(f"unexpected Node Mobile archive entry: {member.filename}")
        archive.extractall(node_staging)
    with tarfile.open(ensure_runtime_archive(PYTHON_ARCHIVE), "r:gz") as archive:
        members = archive.getmembers()
        for member in members:
            require_safe_archive_path(python_staging, member.name)
            if member.issym() or member.islnk() or (not member.isfile() and not member.isdir()):
                raise RuntimeError(f"unsupported Python iOS archive member: {member.name}")
        archive.extractall(python_staging, members=members)
    return node_staging, python_staging


def stage_runtime_resources(manifest: dict[str, object]) -> None:
    """Stages frameworks, interpreter resources, bundled packages, and terminal services for Runner."""
    shutil.rmtree(STAGING_DIR, ignore_errors=True)
    STAGING_DIR.mkdir(parents=True)
    node_staging, python_staging = extract_native_runtimes(STAGING_DIR)
    python_resource_staging = STAGING_DIR / "python-resource"
    node_resource_staging = STAGING_DIR / "node-resource"
    shutil.copytree(python_staging / "python", python_resource_staging)
    node_resource_staging.mkdir()
    site_packages = python_resource_staging / "lib" / "python3.13" / "site-packages"
    site_packages.mkdir(parents=True)
    extract_python_packages(manifest, site_packages)
    extract_node_packages(manifest, node_resource_staging)
    shutil.copy2(RESOURCE_DIR / "node_terminal_service.js", node_resource_staging / "operit_node_terminal_service.js")
    (node_resource_staging / "runtime-entry.js").write_text("\"use strict\";\n", encoding="utf-8")
    shutil.copy2(RESOURCE_DIR / "python_terminal_runtime.py", python_resource_staging / "operit_terminal_runtime.py")
    copy_directory(node_staging / "NodeMobile.xcframework", FRAMEWORKS_DIR / "NodeMobile.xcframework")
    copy_directory(python_staging / "Python.xcframework", FRAMEWORKS_DIR / "Python.xcframework")
    copy_directory(SCIENTIFIC_FRAMEWORK, FRAMEWORKS_DIR / "OperitPythonScientific.xcframework")
    copy_directory(TOYBOX_FRAMEWORK, FRAMEWORKS_DIR / "OperitToybox.xcframework")
    copy_directory(python_resource_staging, PYTHON_RESOURCE_DIR)
    copy_directory(node_resource_staging, NODE_RESOURCE_DIR)
    shutil.rmtree(STAGING_DIR)


def verify_scientific_runtime() -> None:
    """Requires the native NumPy and SciPy framework produced for both iOS runtime slices."""
    required = [
        SCIENTIFIC_FRAMEWORK / "Info.plist",
        SCIENTIFIC_FRAMEWORK / "ios-arm64" / "OperitPythonScientific.framework" / "OperitPythonScientific",
        SCIENTIFIC_FRAMEWORK
        / "ios-arm64_x86_64-simulator"
        / "OperitPythonScientific.framework"
        / "OperitPythonScientific",
    ]
    missing = [str(path) for path in required if not path.is_file()]
    if missing:
        raise RuntimeError(
            "iOS native NumPy/SciPy framework is required before packaging: " + ", ".join(missing)
        )


def verify_toybox_runtime() -> None:
    """Requires the no-fork Toybox framework built for device and simulator slices."""
    required = [
        TOYBOX_FRAMEWORK / "Info.plist",
        TOYBOX_FRAMEWORK / "ios-arm64" / "OperitToybox.framework" / "OperitToybox",
        TOYBOX_FRAMEWORK
        / "ios-arm64_x86_64-simulator"
        / "OperitToybox.framework"
        / "OperitToybox",
    ]
    missing = [str(path) for path in required if not path.is_file()]
    if missing:
        raise RuntimeError(
            "iOS Toybox framework is required before packaging: " + ", ".join(missing)
        )


def verify_staged_runtime() -> None:
    """Verifies each native runtime and package entry used by the iOS Runner target."""
    required = [
        FRAMEWORKS_DIR / "NodeMobile.xcframework" / "Info.plist",
        FRAMEWORKS_DIR / "Python.xcframework" / "Info.plist",
        FRAMEWORKS_DIR / "OperitPythonScientific.xcframework" / "Info.plist",
        FRAMEWORKS_DIR / "OperitToybox.xcframework" / "Info.plist",
        PYTHON_RESOURCE_DIR / "lib" / "python3.13" / "os.py",
        PYTHON_RESOURCE_DIR / "lib" / "python3.13" / "site-packages" / "requests" / "__init__.py",
        NODE_RESOURCE_DIR / "operit_node_terminal_service.js",
        NODE_RESOURCE_DIR / "node_modules" / "lodash" / "lodash.js",
    ]
    missing = [str(path) for path in required if not path.is_file()]
    if missing:
        raise RuntimeError("iOS staged runtime files are missing: " + ", ".join(missing))


def prepare_ios_runtime() -> None:
    """Prepares the complete native Node, Python, Toybox, NumPy, and SciPy iOS runtime package."""
    verify_scientific_runtime()
    verify_toybox_runtime()
    stage_runtime_resources(load_manifest())
    verify_staged_runtime()


def main() -> int:
    """Runs the iOS runtime preparation command."""
    prepare_ios_runtime()
    print(f"iOS native runtime staged: {FRAMEWORKS_DIR}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
