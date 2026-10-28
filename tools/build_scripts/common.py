from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

from release_artifacts import release_asset_name
from cargo_build import cargo_build_environment


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
FLUTTER_APP_DIR = REPO_ROOT / "apps" / "flutter" / "app"
RELEASE_DIR = REPO_ROOT / "tools" / "release"
DIST_DIR = RELEASE_DIR / "dist"
ANDROID_DIR = FLUTTER_APP_DIR / "android"
ANDROID_LOCAL_PROPERTIES = ANDROID_DIR / "local.properties"
PLUGIN_PACKAGES_DIR = REPO_ROOT / "plugins" / "packages"
PLUGIN_SYNC_SCRIPT = REPO_ROOT / "plugins" / "tools" / "sync_plugin_packages.py"
PLUGIN_MANIFEST_FILENAMES = ("manifest.hjson", "manifest.json")
PLUGIN_SYNCABLE_SUFFIXES = (".js", ".toolpkg")
_fvm_sdk_prepared = False
_plugin_assets_prepared = False


def run(command: list[str | Path], cwd: Path = REPO_ROOT, env: dict[str, str] | None = None) -> None:
    """Run a command without opening child console windows on Windows.

    The child's stderr is captured and printed when the command fails, so build
    failures carry the actual compiler/tool error instead of a bare exit code.
    """
    print("+ " + " ".join(str(part) for part in command), flush=True)
    merged_env = os.environ.copy()
    if env:
        merged_env.update(env)
    if Path(command[0]).name.lower() in ("cargo", "cargo.exe"):
        merged_env = cargo_build_environment(command, merged_env, cwd=cwd)
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
        stderr_output = (completed.stderr or "").strip()
        if stderr_output:
            print(stderr_output, file=sys.stderr, flush=True)
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


# Creates a ZIP archive while preserving the complete versioned filename.
def compress_zip(source_dir: Path, destination: Path) -> None:
    if not source_dir.exists():
        raise RuntimeError(f"Expected build directory not found: {source_dir}")
    if destination.exists():
        destination.unlink()
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.make_archive(str(destination.with_suffix("")), "zip", source_dir)


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
    run([fvm, "flutter", "precache", "--web", "--ohos"], cwd=FLUTTER_APP_DIR)
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
    version = subprocess.run(["node", "--version"], check=True, capture_output=True, text=True).stdout.strip()
    if not version.startswith("v24."):
        raise RuntimeError(f"Node.js 24 is required for builds; found {version or 'unknown version'}")
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


# Returns the interpreter under the repository virtualenv used by build hooks.
def prepared_python_path() -> Path:
    if host_platform() == "windows":
        return REPO_ROOT / ".venv" / "Scripts" / "python.exe"
    return REPO_ROOT / ".venv" / "bin" / "python"


# Regenerates the gitignored ToolPkg assets that operit-runtime embeds at compile time.
def prepare_plugin_assets() -> None:
    global _plugin_assets_prepared
    if _plugin_assets_prepared:
        return
    prepare_python_command()
    run(
        [
            str(prepared_python_path()),
            str(PLUGIN_SYNC_SCRIPT),
            "--source",
            "runtime",
            "--no-hot-reload",
        ],
        cwd=REPO_ROOT,
    )
    _plugin_assets_prepared = True


# Mirrors sync_plugin_packages.py's plan so builds can assert the generated asset set.
def expected_plugin_asset_names() -> list[str]:
    names: list[str] = []
    for kind in ("buildin", "external"):
        source_dir = PLUGIN_PACKAGES_DIR / kind
        if not source_dir.is_dir():
            continue
        for child in sorted(source_dir.iterdir(), key=lambda path: path.name.lower()):
            if child.is_dir():
                if any((child / name).is_file() for name in PLUGIN_MANIFEST_FILENAMES):
                    names.append(f"{child.name}.toolpkg")
                continue
            lower_name = child.name.lower()
            if lower_name.endswith(".d.ts"):
                continue
            if lower_name.endswith(".ts"):
                names.append(f"{child.stem}.js")
            elif child.suffix.lower() in PLUGIN_SYNCABLE_SUFFIXES:
                names.append(child.name)
    return names


# Fails when a packaged binary omits any generated ToolPkg asset that build.rs must embed.
def verify_cli_plugin_assets(binary_path: Path) -> None:
    names = expected_plugin_asset_names()
    if not names:
        raise RuntimeError(
            f"No plugin package sources found under {PLUGIN_PACKAGES_DIR}; refusing to package a CLI without built-in plugins"
        )
    data = binary_path.read_bytes()
    missing = [name for name in names if name.encode("utf-8") not in data]
    if missing:
        raise RuntimeError(
            "Built CLI is missing embedded plugin assets: "
            + ", ".join(missing)
            + ". Run: python3 plugins/tools/sync_plugin_packages.py --source runtime --no-hot-reload"
        )


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
    run(command, cwd=FLUTTER_APP_DIR, env=env)


def flutter_pub_get(enforce_lockfile: bool = False, env: dict[str, str] | None = None) -> None:
    """Resolves app dependencies and generates native plugin registrants with FVM Flutter."""
    command = [require_command("fvm"), "flutter", "pub", "get"]
    if enforce_lockfile:
        command.append("--enforce-lockfile")
    run(command, cwd=FLUTTER_APP_DIR, env=env)


def build_env_with_typescript(version: str) -> dict[str, str]:
    env = os.environ.copy()
    typescript_bin = ensure_typescript(version)
    env["PATH"] = f"{typescript_bin}{os.pathsep}{env['PATH']}"
    return env
