from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
import tarfile
import time
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
FLUTTER_APP_DIR = REPO_ROOT / "apps" / "flutter" / "app"
RELEASE_DIR = REPO_ROOT / "tools" / "release"
DIST_DIR = RELEASE_DIR / "dist"
ANDROID_DIR = FLUTTER_APP_DIR / "android"
ANDROID_LOCAL_PROPERTIES = ANDROID_DIR / "local.properties"
_fvm_sdk_prepared = False


def run(command: list[str | Path], cwd: Path = REPO_ROOT, env: dict[str, str] | None = None) -> None:
    """Run a command without opening child console windows on Windows.

    The child's stderr is captured and printed when the command fails, so build
    failures carry the actual compiler/tool error instead of a bare exit code.
    """
    print("+ " + " ".join(str(part) for part in command), flush=True)
    merged_env = os.environ.copy()
    if env:
        merged_env.update(env)
    subprocess_options: dict[str, object] = {}
    if os.name == "nt":
        startupinfo = subprocess.STARTUPINFO()
        startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        startupinfo.wShowWindow = subprocess.SW_HIDE
        subprocess_options["startupinfo"] = startupinfo
        subprocess_options["creationflags"] = subprocess.CREATE_NO_WINDOW
    completed = subprocess.run(
        [str(part) for part in command],
        cwd=cwd,
        env=merged_env,
        check=False,
        stderr=subprocess.PIPE,
        text=True,
        errors="replace",
        **subprocess_options,
    )
    if completed.returncode != 0:
        stderr_tail = (completed.stderr or "").strip()[-4000:]
        if stderr_tail:
            print(stderr_tail, file=sys.stderr, flush=True)
        raise RuntimeError(
            f"Command failed with exit code {completed.returncode}: "
            + " ".join(str(part) for part in command)
        )


def require_command(name: str) -> str:
    path = shutil.which(name)
    if path is None:
        raise RuntimeError(f"Required command is not available: {name}")
    return path


def reset_dir(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


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


WINDOWS_ENGINE_ARTIFACT_VERSION = "3.41.10-ohos-0.0.2-beta.operit.1"
WINDOWS_ENGINE_ARTIFACT_BASE_URL = (
    "https://github.com/AAswordman/flutter-ohos/releases/download/"
    + WINDOWS_ENGINE_ARTIFACT_VERSION
)
WINDOWS_ENGINE_BINARY_DIRS = {
    "windows-x64": "windows-x64-debug.zip",
    "windows-x64-profile": "windows-x64-profile.zip",
    "windows-x64-release": "windows-x64-release.zip",
}


def _download_windows_engine_artifact(url: str, archive_path: Path) -> None:
    """Downloads one engine artifact with backoff retries."""
    import time
    import urllib.request

    for attempt in range(1, 5):
        try:
            print(f"Downloading {archive_path.name} (attempt {attempt}) ...", flush=True)
            urllib.request.urlretrieve(url, archive_path)
            return
        except Exception as error:  # noqa: BLE001 - network errors are unpredictable
            if attempt == 4:
                raise
            print(
                f"Download attempt {attempt} failed: {error}. Retrying in 15s...",
                flush=True,
            )
            time.sleep(15)


def _precache_windows_engine_artifacts(sdk_root: Path) -> None:
    """Downloads the flutter-ohos native composition Windows engine artifacts.

    The flutter-ohos WindowsEngineArtifacts is an EngineCachedArtifact whose
    location is bin/cache/artifacts/engine (see flutter_cache.dart and
    cache.dart). It uses the 'windows-sdk' stamp name, and EngineCachedArtifact
    also compares version against cache.engineRevision (read from
    bin/cache/engine.stamp). Populating bin/cache/artifacts/engine and writing
    both stamps here (after fvm install and precache) prevents flutter build
    from re-downloading the artifacts during the native-assets phase.
    """
    import zipfile

    cache_dir = sdk_root / "bin" / "cache" / "artifacts" / "engine"
    cache_dir.mkdir(parents=True, exist_ok=True)
    for directory, archive_name in WINDOWS_ENGINE_BINARY_DIRS.items():
        dest = cache_dir / directory
        if dest.exists() and any(dest.iterdir()):
            print(f"Windows engine artifact already cached: {directory}", flush=True)
            continue
        url = f"{WINDOWS_ENGINE_ARTIFACT_BASE_URL}/{archive_name}"
        archive_path = cache_dir / archive_name
        _download_windows_engine_artifact(url, archive_path)
        dest.mkdir(parents=True, exist_ok=True)
        print(f"Extracting {archive_name} to {dest} ...", flush=True)
        with zipfile.ZipFile(archive_path) as zf:
            zf.extractall(dest)
        archive_path.unlink()
    cache_root = sdk_root / "bin" / "cache"
    stamp_file = cache_root / "windows-sdk.stamp"
    stamp_file.write_text(WINDOWS_ENGINE_ARTIFACT_VERSION, encoding="ascii")
    engine_stamp_file = cache_root / "engine.stamp"
    engine_stamp_file.write_text(WINDOWS_ENGINE_ARTIFACT_VERSION, encoding="ascii")
    print(
        f"Wrote windows-sdk stamp and engine stamp: {WINDOWS_ENGINE_ARTIFACT_VERSION}",
        flush=True,
    )


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
    run([fvm, "flutter", "precache", "--web", "--ohos", "--windows"], cwd=FLUTTER_APP_DIR)
    _precache_windows_engine_artifacts(FLUTTER_APP_DIR / ".fvm" / "flutter_sdk")
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


def node_package_command(name: str) -> str:
    command = f"{name}.cmd" if host_platform() == "windows" else name
    return require_command(command)


def node_bin_command(root: Path, name: str) -> Path:
    suffix = ".cmd" if host_platform() == "windows" else ""
    return root / "node_modules" / ".bin" / f"{name}{suffix}"


def unescape_java_properties_value(value: str) -> str:
    """Decodes escaped characters in a Java properties value."""
    return value.replace("\\\\", "\\").replace("\\:", ":").replace("\\=", "=").replace("\\n", "\n")


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
        values[key.strip()] = unescape_java_properties_value(value.strip())
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
    manifest = REPO_ROOT / "core" / "crates" / "proxy" / "local" / "Cargo.toml"
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
    run_with_retries(command, cwd=FLUTTER_APP_DIR, env=env)


def flutter_pub_get(enforce_lockfile: bool = False, env: dict[str, str] | None = None) -> None:
    """Resolves app dependencies and generates native plugin registrants with FVM Flutter."""
    command = [flutter_command(), "pub", "get"]
    if enforce_lockfile:
        command.append("--enforce-lockfile")
    run_with_retries(command, cwd=FLUTTER_APP_DIR, env=env)


def run_with_retries(
    command: list[str | Path],
    cwd: Path = REPO_ROOT,
    env: dict[str, str] | None = None,
    attempts: int = 4,
) -> None:
    """Runs a network-facing command with backoff retries.

    Git-hosted pub dependencies (e.g. gitee mirrors) reset connections from
    some CI regions; a bare failure wastes a full runner provisioning cycle,
    so retry locally before surfacing the error.
    """
    delay = 15
    for attempt in range(1, attempts + 1):
        try:
            run(command, cwd=cwd, env=env)
            return
        except RuntimeError:
            if attempt == attempts:
                raise
            print(
                f"attempt {attempt}/{attempts} failed, retrying in {delay}s...",
                flush=True,
            )
            time.sleep(delay)
            delay *= 2


def build_env_with_typescript(version: str) -> dict[str, str]:
    env = os.environ.copy()
    typescript_bin = ensure_typescript(version)
    env["PATH"] = f"{typescript_bin}{os.pathsep}{env['PATH']}"
    return env


WORKFLOW_PACKAGE_DIR = (
    REPO_ROOT / "plugins" / "packages" / "buildin" / "workflow"
)


def ensure_workflow_pnpm_deps(env: dict[str, str] | None = None) -> None:
    """Installs built-in workflow plugin dependencies with Corepack pnpm.

    Plugin sync (tsc + pack:toolpkg) assumes node_modules is present, which
    is only true after a manual install on dev machines. Fresh CI runners
    must install first, honoring the pnpm pin in package.json.
    """
    if not (WORKFLOW_PACKAGE_DIR / "package.json").is_file():
        raise RuntimeError(
            f"Workflow package.json not found: {WORKFLOW_PACKAGE_DIR}"
        )
    if not (WORKFLOW_PACKAGE_DIR / "pnpm-lock.yaml").is_file():
        raise RuntimeError(
            f"Workflow pnpm lockfile not found: {WORKFLOW_PACKAGE_DIR}"
        )
    corepack = shutil.which("corepack")
    if corepack is None:
        raise RuntimeError(
            "Corepack is required to install workflow dependencies "
            "(run `corepack enable` first)"
        )
    run_with_retries(
        [corepack, "pnpm", "install", "--frozen-lockfile"],
        cwd=WORKFLOW_PACKAGE_DIR,
        env=env,
        attempts=3,
    )
