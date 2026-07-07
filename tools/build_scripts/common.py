from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
FLUTTER_APP_DIR = REPO_ROOT / "apps" / "flutter" / "app"
RELEASE_DIR = REPO_ROOT / "tools" / "release"
DIST_DIR = RELEASE_DIR / "dist"
ANDROID_DIR = FLUTTER_APP_DIR / "android"
ANDROID_LOCAL_PROPERTIES = ANDROID_DIR / "local.properties"


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


def flutter_command() -> str:
    return require_command("flutter")


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
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [f"{key}={java_properties_value(value)}" for key, value in sorted(values.items())]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


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
        if target.exists() or target.is_symlink():
            target.unlink()
        target.symlink_to(python)
    run([str(target), "--version"])


def generate_dart_proxy_artifacts() -> None:
    manifest = REPO_ROOT / "core" / "crates" / "operit-core-proxy" / "Cargo.toml"
    run(["cargo", "clean", "--manifest-path", str(manifest), "-p", "operit-core-proxy"])
    run(["cargo", "check", "--manifest-path", str(manifest), "--quiet"])
    for path in [
        FLUTTER_APP_DIR / "lib" / "core" / "proxy" / "generated" / "CoreProxyClients.g.dart",
        FLUTTER_APP_DIR / "lib" / "core" / "proxy" / "generated" / "CoreProxyModels.g.dart",
    ]:
        if not path.is_file():
            raise RuntimeError(f"Expected generated Dart proxy artifact is missing: {path}")


def flutter_pub_get(enforce_lockfile: bool = False, env: dict[str, str] | None = None) -> None:
    command = [flutter_command(), "pub", "get"]
    if enforce_lockfile:
        command.append("--enforce-lockfile")
    run(command, cwd=FLUTTER_APP_DIR, env=env)


def build_env_with_typescript(version: str) -> dict[str, str]:
    env = os.environ.copy()
    typescript_bin = ensure_typescript(version)
    env["PATH"] = f"{typescript_bin}{os.pathsep}{env['PATH']}"
    return env
