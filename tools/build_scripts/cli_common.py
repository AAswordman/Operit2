from __future__ import annotations

import os
import platform
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

from common import (
    DIST_DIR,
    REPO_ROOT,
    copy_required_file,
    compress_tar_gz,
    compress_zip,
    host_platform,
    require_command,
    reset_dir,
    run,
)


CLI_MANIFEST = REPO_ROOT / "apps" / "cli" / "Cargo.toml"
CLI_WORK_DIR = REPO_ROOT / "tools" / "release" / "work"
CLI_RELEASE_ARCHES = ("x86_64", "aarch64")
CLI_RUST_TARGETS = {
    ("windows", "x86_64"): "x86_64-pc-windows-msvc",
    ("windows", "aarch64"): "aarch64-pc-windows-msvc",
    ("linux", "x86_64"): "x86_64-unknown-linux-musl",
    ("linux", "aarch64"): "aarch64-unknown-linux-musl",
    ("macos", "x86_64"): "x86_64-apple-darwin",
    ("macos", "aarch64"): "aarch64-apple-darwin",
}


@dataclass(frozen=True)
class CliBuildTarget:
    platform: str
    arch: str
    rust_target: str


def cli_target(target_platform: str, arch: str) -> CliBuildTarget:
    return CliBuildTarget(
        platform=target_platform,
        arch=arch,
        rust_target=CLI_RUST_TARGETS[(target_platform, arch)],
    )


def cli_targets_for_platform(target_platform: str) -> tuple[CliBuildTarget, ...]:
    return tuple(cli_target(target_platform, arch) for arch in CLI_RELEASE_ARCHES)


def host_arch() -> str:
    machine = platform.machine().lower()
    if machine in ("amd64", "x86_64"):
        return "x86_64"
    if machine in ("arm64", "aarch64"):
        return "aarch64"
    raise RuntimeError(f"Unsupported host architecture: {machine}")


def cli_binary_name(target_platform: str) -> str:
    return "operit2.exe" if target_platform == "windows" else "operit2"


def cli_archive_extension(target_platform: str) -> str:
    return "zip" if target_platform == "windows" else "tar.gz"


def cli_package_dir(target: CliBuildTarget) -> Path:
    return CLI_WORK_DIR / f"cli-{target.platform}-{target.arch}"


def cli_package_path(target: CliBuildTarget) -> Path:
    return DIST_DIR / f"operit2-cli-{target.platform}-{target.arch}.{cli_archive_extension(target.platform)}"


def cli_target_binary_path(target: CliBuildTarget) -> Path:
    return (
        REPO_ROOT
        / "apps"
        / "cli"
        / "target"
        / target.rust_target
        / "release"
        / cli_binary_name(target.platform)
    )


def write_text_file(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8", newline="\n")


def write_windows_cli_installer_files(package_dir: Path) -> None:
    write_text_file(
        package_dir / "install.bat",
        r'''@echo off
setlocal

set "SOURCE=%~dp0operit2.exe"

if not exist "%SOURCE%" (
  echo operit2.exe not found next to install.bat 1>&2
  exit /b 1
)

"%SOURCE%" cli install --source "%SOURCE%"
exit /b %ERRORLEVEL%
''',
    )
    write_text_file(
        package_dir / "uninstall.bat",
        r'''@echo off
setlocal

set "SOURCE=%~dp0operit2.exe"

if not exist "%SOURCE%" (
  echo operit2.exe not found next to uninstall.bat 1>&2
  exit /b 1
)

"%SOURCE%" cli uninstall
exit /b %ERRORLEVEL%
''',
    )
    write_text_file(
        package_dir / "README.txt",
        """Operit2 CLI for Windows

Install:
  install.bat

Uninstall:
  uninstall.bat

Command after install:
  operit
  operit2
""",
    )


def write_unix_cli_installer_files(package_dir: Path) -> None:
    write_text_file(
        package_dir / "install.sh",
        """#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
source_file="$script_dir/operit2"

test -f "$source_file"
chmod +x "$source_file"
"$source_file" cli install --source "$source_file"
""",
    )
    write_text_file(
        package_dir / "uninstall.sh",
        """#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
source_file="$script_dir/operit2"

test -f "$source_file"
chmod +x "$source_file"
"$source_file" cli uninstall
""",
    )
    write_text_file(
        package_dir / "README.txt",
        """Operit2 CLI for Linux/macOS

Install:
  chmod +x install.sh
  ./install.sh

Uninstall:
  chmod +x uninstall.sh
  ./uninstall.sh

Command after install:
  operit
  operit2
""",
    )


def write_cli_installer_files(package_dir: Path, target_platform: str) -> None:
    if target_platform == "windows":
        write_windows_cli_installer_files(package_dir)
    else:
        write_unix_cli_installer_files(package_dir)


def vs_installation_path() -> Path | None:
    vswhere = (
        Path(os.environ.get("ProgramFiles(x86)", "C:\\Program Files (x86)"))
        / "Microsoft Visual Studio"
        / "Installer"
        / "vswhere.exe"
    )
    if not vswhere.exists():
        return None
    try:
        path = run_capture([str(vswhere), "-latest", "-property", "installationPath"])
        return Path(path) if path else None
    except (subprocess.CalledProcessError, OSError):
        return None


def run_capture(command: list[str]) -> str:
    return subprocess.run(
        command,
        cwd=REPO_ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    ).stdout.strip()


def vs_dev_env(vcvars_path: Path, arch: str) -> dict[str, str]:
    tmp = tempfile.NamedTemporaryFile(mode="w", suffix=".bat", delete=False, dir=Path(__file__).resolve().parent)
    try:
        tmp.write(f'@call "{vcvars_path}" {arch}\n')
        tmp.write("@set\n")
        tmp.close()
        result = subprocess.run(["cmd.exe", "/c", tmp.name], capture_output=True, text=True)
    finally:
        os.unlink(tmp.name)

    if result.returncode != 0:
        raise RuntimeError(f"vcvarsall.bat failed for {arch}: {result.stderr.strip()}")

    env = {}
    for line in result.stdout.splitlines():
        if "=" in line:
            key, _, value = line.partition("=")
            env[key] = value
    canonical_env = {}
    for key in ("INCLUDE", "LIB", "PATH"):
        actual_key = next((name for name in env.keys() if name.upper() == key), None)
        if actual_key is None:
            raise RuntimeError(f"vcvarsall.bat did not produce {key} for {arch}")
        canonical_env[key] = env[actual_key]
    return canonical_env


def windows_aarch64_env() -> dict[str, str]:
    build_env = {**os.environ}
    vs_path = vs_installation_path()
    if not vs_path:
        raise RuntimeError("Visual Studio installation not found for Windows aarch64 CLI build")
    vcvars = vs_path / "VC" / "Auxiliary" / "Build" / "vcvarsall.bat"
    if not vcvars.exists():
        raise RuntimeError(f"vcvarsall.bat not found: {vcvars}")
    build_env.update(vs_dev_env(vcvars, "x64_arm64"))
    llvm_bin = Path("C:/Program Files/LLVM/bin")
    clang = llvm_bin / "clang.exe"
    if not clang.exists():
        raise RuntimeError(f"LLVM clang not found for Windows aarch64 CLI build: {clang}")
    build_env["PATH"] = f"{llvm_bin};{build_env.get('PATH', '')}"
    return build_env


def build_cli_target(target: CliBuildTarget, use_default_target: bool = False) -> Path:
    require_command("cargo")
    binary_name = cli_binary_name(target.platform)
    package_dir = cli_package_dir(target)
    package_path = cli_package_path(target)

    if use_default_target:
        run(["cargo", "build", "--release", "--manifest-path", CLI_MANIFEST])
        binary_source = REPO_ROOT / "apps" / "cli" / "target" / "release" / binary_name
    else:
        build_env = {**os.environ}
        if target.platform == "windows" and target.arch == "aarch64":
            build_env = windows_aarch64_env()
        run(
            ["cargo", "build", "--release", "--target", target.rust_target, "--manifest-path", CLI_MANIFEST],
            env=build_env,
        )
        binary_source = cli_target_binary_path(target)

    reset_dir(package_dir)
    copy_required_file(binary_source, package_dir / binary_name)
    write_cli_installer_files(package_dir, target.platform)

    if target.platform == "windows":
        compress_zip(package_dir, package_path)
    else:
        os.chmod(package_dir / binary_name, 0o755)
        os.chmod(package_dir / "install.sh", 0o755)
        os.chmod(package_dir / "uninstall.sh", 0o755)
        compress_tar_gz(package_dir, package_path)

    print(f"CLI archive: {package_path}", flush=True)
    return package_path


def build_cli_platform(target_platform: str, arch_mode: str = "host") -> None:
    if arch_mode == "host":
        current_platform = host_platform()
        if target_platform != current_platform:
            raise RuntimeError(
                f"{target_platform} CLI host build requires a {target_platform} host; current host is {current_platform}"
            )
        build_cli_target(cli_target(target_platform, host_arch()), use_default_target=True)
        return

    if arch_mode == "all":
        for target in cli_targets_for_platform(target_platform):
            build_cli_target(target, use_default_target=False)
        return

    build_cli_target(cli_target(target_platform, arch_mode), use_default_target=False)


def parse_cli_arch_mode(value: str) -> str:
    if value in ("host", "all", "x86_64", "aarch64"):
        return value
    raise RuntimeError(f"Unsupported CLI arch mode: {value}")
