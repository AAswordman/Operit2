from __future__ import annotations

import os
import platform
import hashlib
import json
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
FLUTTER_APP_DIR = REPO_ROOT / "apps" / "flutter" / "app"
WEB_ACCESS_APP_DIR = REPO_ROOT / "apps" / "web_access"
WEB_ACCESS_SOURCE_DIR = WEB_ACCESS_APP_DIR / "web"
WEB_ACCESS_BUNDLE_DIR = WEB_ACCESS_APP_DIR / "build" / "bundle"
WEB_ACCESS_FLUTTER_STAGE_DIR = FLUTTER_APP_DIR / "web"
WEB_ACCESS_EMBEDDED_ASSETS_DIR = FLUTTER_APP_DIR / "assets" / "web_access"
WEB_ACCESS_ASSET_DECLARATION_PREFIX = "    - path: assets/web_access/"
WEB_ACCESS_VERSION_FILE = "web_access_version.json"
WEB_ACCESS_VERSION_SCHEMA = 1
WEB_ACCESS_REQUIRED_FILES = (
    "index.html",
    "main.dart.js",
    "operit_flutter_bridge.js",
    "operit_flutter_bridge_bg.wasm",
    "sql-wasm.js",
    "sql-wasm.wasm",
    WEB_ACCESS_VERSION_FILE,
)
RELEASE_DIR = REPO_ROOT / "tools" / "release"
DIST_DIR = RELEASE_DIR / "dist"
ANDROID_DIR = FLUTTER_APP_DIR / "android"
ANDROID_LOCAL_PROPERTIES = ANDROID_DIR / "local.properties"
OHOS_FVM_CACHE_DIR = REPO_ROOT / ".ci-tools" / "fvm-ohos"
OHOS_FLUTTER_REF = "8c403dd30158e63b8efccf988b129093820886c9"
OHOS_FLUTTER_GIT_URL = "https://gitcode.com/openharmony-sig/flutter_flutter.git"
_fvm_sdk_prepared = False
_ohos_fvm_sdk_prepared = False


def run(command: list[str | Path], cwd: Path = REPO_ROOT, env: dict[str, str] | None = None) -> None:
    print("+ " + " ".join(str(part) for part in command), flush=True)
    merged_env = os.environ.copy()
    if env:
        merged_env.update(env)
    subprocess.run([str(part) for part in command], cwd=cwd, env=merged_env, check=True)


def require_command(name: str) -> str:
    path = shutil.which(name)
    if path is None:
        raise RuntimeError(f"Required command is not available: {name}")
    return path


def reset_dir(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


# Reads and validates one Web Access bundle version manifest.
def read_web_access_version_manifest(path: Path) -> dict[str, object] | None:
    if not path.is_file():
        return None
    manifest = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(manifest, dict):
        raise RuntimeError(f"Web Access version manifest must be an object: {path}")
    schema_version = manifest.get("schemaVersion")
    version = manifest.get("version")
    content_hash = manifest.get("contentHash")
    file_count = manifest.get("fileCount")
    byte_size = manifest.get("byteSize")
    if schema_version != WEB_ACCESS_VERSION_SCHEMA:
        raise RuntimeError(f"Unexpected Web Access manifest schema in {path}: {schema_version}")
    if not isinstance(version, int) or version < 1:
        raise RuntimeError(f"Invalid Web Access version in {path}: {version}")
    if not isinstance(content_hash, str) or not content_hash:
        raise RuntimeError(f"Invalid Web Access content hash in {path}")
    if not isinstance(file_count, int) or file_count < 1:
        raise RuntimeError(f"Invalid Web Access file count in {path}: {file_count}")
    if not isinstance(byte_size, int) or byte_size < 1:
        raise RuntimeError(f"Invalid Web Access byte size in {path}: {byte_size}")
    return manifest


# Computes a stable content hash for every generated Web Access bundle file.
def compute_web_access_bundle_digest(bundle_dir: Path) -> tuple[str, int, int]:
    if not bundle_dir.is_dir():
        raise RuntimeError(f"Web Access bundle directory not found: {bundle_dir}")
    digest = hashlib.sha256()
    file_count = 0
    byte_size = 0
    for path in sorted(candidate for candidate in bundle_dir.rglob("*") if candidate.is_file()):
        relative_path = path.relative_to(bundle_dir).as_posix()
        if relative_path == WEB_ACCESS_VERSION_FILE:
            continue
        data = path.read_bytes()
        digest.update(relative_path.encode("utf-8"))
        digest.update(b"\0")
        digest.update(len(data).to_bytes(8, "big"))
        digest.update(data)
        file_count += 1
        byte_size += len(data)
    if file_count == 0:
        raise RuntimeError(f"Web Access bundle contains no files: {bundle_dir}")
    return digest.hexdigest(), file_count, byte_size


# Finds the last generated Web Access version manifest from build-owned outputs.
def read_previous_web_access_version_manifest() -> dict[str, object] | None:
    embedded_manifest = read_web_access_version_manifest(
        WEB_ACCESS_EMBEDDED_ASSETS_DIR / WEB_ACCESS_VERSION_FILE
    )
    if embedded_manifest is not None:
        return embedded_manifest
    return read_web_access_version_manifest(WEB_ACCESS_BUNDLE_DIR / WEB_ACCESS_VERSION_FILE)


# Writes the version manifest consumed by Flutter and CLI Web Access launchers.
def write_web_access_version_manifest() -> dict[str, object]:
    content_hash, file_count, byte_size = compute_web_access_bundle_digest(WEB_ACCESS_BUNDLE_DIR)
    previous_manifest = read_previous_web_access_version_manifest()
    version = 1
    if previous_manifest is not None:
        previous_version = previous_manifest["version"]
        previous_hash = previous_manifest["contentHash"]
        if previous_hash == content_hash:
            version = int(previous_version)
        else:
            version = int(previous_version) + 1
    manifest: dict[str, object] = {
        "schemaVersion": WEB_ACCESS_VERSION_SCHEMA,
        "version": version,
        "contentHash": content_hash,
        "fileCount": file_count,
        "byteSize": byte_size,
    }
    with (WEB_ACCESS_BUNDLE_DIR / WEB_ACCESS_VERSION_FILE).open(
        "w", encoding="utf-8", newline="\n"
    ) as output:
        output.write(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    return manifest


# Copies one generated directory tree into another build-owned directory.
def sync_dir(source: Path, destination: Path) -> None:
    if not source.is_dir():
        raise RuntimeError(f"Expected source directory not found: {source}")
    if destination.is_symlink():
        destination.unlink()
    elif destination.exists():
        shutil.rmtree(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(source, destination)


# Ensures Flutter's web directory points to the Web Access source shell.
def stage_web_access_source() -> None:
    if not WEB_ACCESS_SOURCE_DIR.is_dir():
        raise RuntimeError(f"Web Access source directory not found: {WEB_ACCESS_SOURCE_DIR}")
    expected_target = WEB_ACCESS_SOURCE_DIR.resolve()
    if WEB_ACCESS_FLUTTER_STAGE_DIR.exists() or WEB_ACCESS_FLUTTER_STAGE_DIR.is_symlink():
        if not WEB_ACCESS_FLUTTER_STAGE_DIR.is_symlink():
            raise RuntimeError(
                "Flutter web staging path must be a directory symlink: "
                f"{WEB_ACCESS_FLUTTER_STAGE_DIR}"
            )
        actual_target = WEB_ACCESS_FLUTTER_STAGE_DIR.resolve()
        if actual_target == expected_target:
            return
        WEB_ACCESS_FLUTTER_STAGE_DIR.unlink()
    WEB_ACCESS_FLUTTER_STAGE_DIR.parent.mkdir(parents=True, exist_ok=True)
    WEB_ACCESS_FLUTTER_STAGE_DIR.symlink_to(expected_target, target_is_directory=True)


# Verifies the shared Web Access bundle has been built.
def require_web_access_bundle() -> None:
    missing = [
        name for name in WEB_ACCESS_REQUIRED_FILES if not (WEB_ACCESS_BUNDLE_DIR / name).is_file()
    ]
    if missing:
        raise RuntimeError(
            "Web Access bundle is incomplete at "
            f"{WEB_ACCESS_BUNDLE_DIR}; missing: {', '.join(missing)}. "
            "Run tools/build_scripts/build_flutter_web_access.py before building this product."
        )


# Synchronizes the Web Access bundle and validates every pubspec asset directory.
def prepare_web_access_embedded_assets() -> None:
    require_web_access_bundle()
    sync_dir(WEB_ACCESS_BUNDLE_DIR, WEB_ACCESS_EMBEDDED_ASSETS_DIR)

    pubspec = FLUTTER_APP_DIR / "pubspec.yaml"
    declared_paths = []
    for line in pubspec.read_text(encoding="utf-8").splitlines():
        if line.startswith(WEB_ACCESS_ASSET_DECLARATION_PREFIX):
            relative_path = line.removeprefix("    - path: ").rstrip("/")
            declared_paths.append(FLUTTER_APP_DIR / Path(relative_path))
    if not declared_paths:
        raise RuntimeError(f"No Web Access asset directories are declared in {pubspec}")

    missing = [path for path in declared_paths if not path.is_dir()]
    if missing:
        raise RuntimeError(
            "Embedded Web Access asset directories declared in pubspec.yaml are missing: "
            + ", ".join(str(path) for path in missing)
        )


def copy_required_file(source: Path, destination: Path) -> None:
    if not source.exists():
        raise RuntimeError(f"Expected build output not found: {source}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def compress_zip(source_dir: Path, destination: Path) -> None:
    if not source_dir.exists():
        raise RuntimeError(f"Expected build directory not found: {source_dir}")
    if destination.exists():
        destination.unlink()
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.make_archive(str(destination.with_suffix("")), "zip", source_dir)
    produced = destination.with_suffix("").with_suffix(".zip")
    if produced != destination:
        produced.replace(destination)


def compress_tar_gz(source_dir: Path, destination: Path) -> None:
    if not source_dir.exists():
        raise RuntimeError(f"Expected build directory not found: {source_dir}")
    if destination.exists():
        destination.unlink()
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(destination, "w:gz") as archive:
        for child in sorted(source_dir.iterdir(), key=lambda path: path.name):
            archive.add(child, arcname=child.name)


def host_platform() -> str:
    name = platform.system().lower()
    if name == "darwin":
        return "macos"
    if name == "windows":
        return "windows"
    if name == "linux":
        return "linux"
    raise RuntimeError(f"Unsupported host platform: {name}")


def host_arch() -> str:
    machine = platform.machine().lower()
    if machine in ("amd64", "x86_64"):
        return "x86_64"
    if machine in ("arm64", "aarch64"):
        return "aarch64"
    raise RuntimeError(f"Unsupported host architecture: {machine}")


def prepare_fvm_flutter_sdk() -> None:
    """Installs and selects the Flutter SDK pinned by the app FVM configuration."""
    global _fvm_sdk_prepared
    if _fvm_sdk_prepared:
        return
    fvm = require_command("fvm")
    run([fvm, "install", "--skip-pub-get"], cwd=FLUTTER_APP_DIR)
    executable_name = "flutter.bat" if host_platform() == "windows" else "flutter"
    executable = FLUTTER_APP_DIR / ".fvm" / "flutter_sdk" / "bin" / executable_name
    if not executable.is_file():
        raise RuntimeError(f"FVM SDK command not found: {executable}")
    run([str(executable), "precache", "--web"], cwd=FLUTTER_APP_DIR)
    _fvm_sdk_prepared = True


def fvm_sdk_command(name: str) -> str:
    """Returns one executable from the prepared project Flutter SDK."""
    prepare_fvm_flutter_sdk()
    executable_name = f"{name}.bat" if host_platform() == "windows" else name
    executable = FLUTTER_APP_DIR / ".fvm" / "flutter_sdk" / "bin" / executable_name
    if not executable.is_file():
        raise RuntimeError(f"FVM SDK command not found: {executable}")
    return str(executable)


def flutter_command() -> str:
    """Returns Flutter from the SDK pinned by the app FVM configuration."""
    return fvm_sdk_command("flutter")


def dart_command() -> str:
    """Returns Dart from the SDK pinned by the app FVM configuration."""
    return fvm_sdk_command("dart")


def ohos_fvm_env(env: dict[str, str]) -> dict[str, str]:
    """Builds the isolated FVM environment for OpenHarmony Flutter commands."""
    configured = env.copy()
    configured["FLUTTER_GIT_URL"] = OHOS_FLUTTER_GIT_URL
    configured["FVM_CACHE_PATH"] = str(OHOS_FVM_CACHE_DIR)
    return configured


def prepare_ohos_fvm_flutter_sdk(env: dict[str, str]) -> None:
    """Installs and precaches the fixed OpenHarmony Flutter SDK through FVM."""
    global _ohos_fvm_sdk_prepared
    if _ohos_fvm_sdk_prepared:
        return
    fvm = require_command("fvm")
    run(
        [fvm, "spawn", OHOS_FLUTTER_REF, "precache", "--ohos"],
        cwd=FLUTTER_APP_DIR,
        env=ohos_fvm_env(env),
    )
    executable_name = "flutter.bat" if host_platform() == "windows" else "flutter"
    executable = ohos_fvm_flutter_sdk_dir() / "bin" / executable_name
    if not executable.is_file():
        raise RuntimeError(f"OpenHarmony FVM SDK command not found: {executable}")
    _ohos_fvm_sdk_prepared = True


def ohos_fvm_flutter_sdk_dir() -> Path:
    """Returns the isolated FVM cache directory for the OpenHarmony Flutter SDK."""
    return OHOS_FVM_CACHE_DIR / "versions" / OHOS_FLUTTER_REF


def run_ohos_fvm_flutter(args: list[str], env: dict[str, str]) -> None:
    """Runs one OpenHarmony Flutter command through its dedicated FVM SDK."""
    prepare_ohos_fvm_flutter_sdk(env)
    fvm = require_command("fvm")
    run(
        [fvm, "spawn", OHOS_FLUTTER_REF, *args],
        cwd=FLUTTER_APP_DIR,
        env=ohos_fvm_env(env),
    )


def node_package_command(name: str) -> str:
    command = f"{name}.cmd" if host_platform() == "windows" else name
    return require_command(command)


def node_bin_command(root: Path, name: str) -> Path:
    suffix = ".cmd" if host_platform() == "windows" else ""
    return root / "node_modules" / ".bin" / f"{name}{suffix}"


def read_properties(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    if not path.exists():
        return values
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        key, separator, value = stripped.partition("=")
        if not separator:
            continue
        values[key.strip()] = value.strip()
    return values


def write_properties(path: Path, values: dict[str, str]) -> None:
    """Writes sorted Java properties with LF line endings."""
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [f"{key}={java_properties_value(value)}" for key, value in sorted(values.items())]
    with path.open("w", encoding="utf-8", newline="\n") as output:
        output.write("\n".join(lines) + "\n")


def java_properties_value(value: str) -> str:
    return str(value).replace("\\", "\\\\").replace("\n", "\\n")


def ensure_node_and_npm() -> None:
    run(["node", "--version"])
    run([node_package_command("npm"), "--version"])


def ensure_typescript(version: str) -> Path:
    root = REPO_ROOT / ".ci-tools" / "typescript"
    tsc = node_bin_command(root, "tsc")
    if not tsc.exists():
        npm = node_package_command("npm")
        run(
            [
                npm,
                "install",
                "--prefix",
                str(root),
                "--no-audit",
                "--no-fund",
                f"typescript@{version}",
            ]
        )
    run([str(tsc), "--version"])
    return tsc.parent


# Ensures Terser is installed at the requested version.
def ensure_terser(version: str) -> Path:
    root = REPO_ROOT / ".ci-tools" / "terser"
    terser = node_bin_command(root, "terser")
    if not terser.exists():
        npm = node_package_command("npm")
        run(
            [
                npm,
                "install",
                "--prefix",
                str(root),
                "--no-audit",
                "--no-fund",
                f"terser@{version}",
            ]
        )
    run([str(terser), "--version"])
    return terser.parent


def prepare_python_command() -> None:
    python = Path(sys.executable)
    if not python.is_file():
        raise RuntimeError(f"Current Python executable is not available: {python}")
    if host_platform() == "windows":
        target = REPO_ROOT / ".venv" / "Scripts" / "python.exe"
        target.parent.mkdir(parents=True, exist_ok=True)
        if python.resolve() != target.resolve():
            shutil.copy2(python, target)
    else:
        target = REPO_ROOT / ".venv" / "bin" / "python"
        target.parent.mkdir(parents=True, exist_ok=True)
        if python.absolute() == target.absolute():
            run([str(python), "--version"])
            return
        if target.exists() or target.is_symlink():
            target.unlink()
        target.symlink_to(python)
    run([str(target), "--version"])


def generate_dart_proxy_artifacts() -> None:
    manifest = REPO_ROOT / "core" / "crates" / "operit-core-proxy" / "Cargo.toml"
    run(["cargo", "check", "--manifest-path", str(manifest), "--quiet"])
    for path in [
        FLUTTER_APP_DIR / "lib" / "core" / "proxy" / "generated" / "CoreProxyClients.g.dart",
        FLUTTER_APP_DIR / "lib" / "core" / "proxy" / "generated" / "CoreProxyModels.g.dart",
    ]:
        if not path.is_file():
            raise RuntimeError(f"Expected generated Dart proxy artifact is missing: {path}")


def dart_pub_get(enforce_lockfile: bool = False, env: dict[str, str] | None = None) -> None:
    """Resolves app dependencies with the Dart SDK selected by FVM."""
    command = [dart_command(), "pub", "get"]
    if enforce_lockfile:
        command.append("--enforce-lockfile")
    run(command, cwd=FLUTTER_APP_DIR, env=env)


def flutter_pub_get(enforce_lockfile: bool = False, env: dict[str, str] | None = None) -> None:
    """Resolves app dependencies and generates native plugin registrants with FVM Flutter."""
    command = [flutter_command(), "pub", "get"]
    if enforce_lockfile:
        command.append("--enforce-lockfile")
    run(command, cwd=FLUTTER_APP_DIR, env=env)


def build_env_with_typescript(version: str) -> dict[str, str]:
    env = os.environ.copy()
    typescript_bin = ensure_typescript(version)
    env["PATH"] = f"{typescript_bin}{os.pathsep}{env['PATH']}"
    return env
