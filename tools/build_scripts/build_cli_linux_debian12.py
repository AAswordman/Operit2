#!/usr/bin/env python3
"""Build the Linux CLI inside a Debian 12 container.

Binaries linked in Debian 12 keep their glibc symbol references within the
2.36 baseline, so they run on distributions that reject binaries built on
newer hosts (`GLIBC_2.xx not found`). Packaging and embedded plugin assets
match the regular Linux CLI build; the musl release build stays available
through build_cli_linux.py.
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPTS_DIR.parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

from cli_common import host_arch, verify_cli_plugin_assets, write_cli_installer_files
from common import (
    DIST_DIR,
    compress_tar_gz,
    copy_required_file,
    release_asset_name,
    reset_dir,
)

CONTAINER_DIR = SCRIPTS_DIR / "containers" / "debian12"
IMAGE_TAG = "operit2-build:debian12"
DEB12_WORK_DIR = REPO_ROOT / "tools" / "release" / "work" / "debian12"
RUST_TARGETS = {
    "x86_64": "x86_64-unknown-linux-gnu",
    "aarch64": "aarch64-unknown-linux-gnu",
}
DEBIAN_GLIBC_VERSION = (2, 36)
# Forwarded into the container when present on the host: proxies for the
# default endpoints, the rustup mirrors for restricted networks, and the
# release-pinned toolchain version.
PASSTHROUGH_ENV_VARS = (
    "RUST_TOOLCHAIN_VERSION",
    "RUSTUP_DIST_SERVER",
    "RUSTUP_UPDATE_ROOT",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "no_proxy",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--arches", default="host", choices=["host", "all", "x86_64", "aarch64"])
    parser.add_argument("--runtime", default="auto", choices=["auto", "docker", "podman"])
    parser.add_argument("--base-image", default=None, help="Base image for the initial container build (default: docker.io/library/debian:12)")
    parser.add_argument("--apt-mirror", default=None, help="Scheme-less Debian mirror base used while building the image, e.g. mirrors.ustc.edu.cn (requires --rebuild-image to apply later)")
    parser.add_argument("--env", action="append", default=[], metavar="KEY=VALUE", help="Additional environment variable forwarded into the container")
    parser.add_argument("--rebuild-image", action="store_true", help="Rebuild the container image even when it already exists")
    parser.add_argument("--shell", action="store_true", help="Open an interactive shell in the container instead of building")
    return parser.parse_args()


def selected_arches(arch_mode: str) -> list[str]:
    if arch_mode == "host":
        return [host_arch()]
    if arch_mode == "all":
        return list(RUST_TARGETS)
    return [arch_mode]


def select_runtime(explicit: str) -> str:
    if explicit != "auto":
        if shutil.which(explicit) is None:
            raise RuntimeError(f"Container runtime not found: {explicit}")
        return explicit
    for name in ("podman", "docker"):
        if shutil.which(name) is not None:
            return name
    raise RuntimeError("Neither podman nor docker was found; install podman for rootless builds")


# Returns extra environment for every runtime subprocess. Rootless podman
# fails on btrfs home filesystems when a distro drop-in pins the overlay
# driver; the user storage configuration (btrfs on such machines) is the
# working fallback.
def runtime_extra_env(runtime: str) -> dict[str, str]:
    if runtime != "podman":
        return {}
    probe = subprocess.run([runtime, "info"], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    if probe.returncode == 0 and "overwritten by graph driver" not in (probe.stderr or ""):
        return {}
    custom_home = os.environ.get("XDG_CONFIG_HOME")
    config_home = Path(custom_home) if custom_home else Path.home() / ".config"
    candidate = config_home / "containers" / "storage.conf"
    if candidate.is_file():
        trial = {**os.environ, "CONTAINERS_STORAGE_CONF": str(candidate)}
        retry = subprocess.run([runtime, "info"], env=trial, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if retry.returncode == 0:
            print(f"podman's default storage configuration is unusable; using CONTAINERS_STORAGE_CONF={candidate}", flush=True)
            return {"CONTAINERS_STORAGE_CONF": str(candidate)}
    raise RuntimeError(
        f"Container storage is not configured: '{runtime} info' failed. On btrfs filesystems set"
        ' driver = "btrfs" in ~/.config/containers/storage.conf, then export'
        " CONTAINERS_STORAGE_CONF=~/.config/containers/storage.conf"
    )


def image_exists(runtime: str, tag: str, extra_env: dict[str, str]) -> bool:
    command = subprocess.run([runtime, "image", "inspect", tag], env={**os.environ, **extra_env}, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return command.returncode == 0


# Provisions the build image without `podman build`: buildah mounts the build
# context through kernel overlayfs, which kernels without the overlay module
# cannot service. A scratch container plus `commit` has no such dependency.
def build_image(runtime: str, args: argparse.Namespace, extra_env: dict[str, str]) -> None:
    base_image = args.base_image or "docker.io/library/debian:12"
    env = {**os.environ, **extra_env}
    print(f"Pulling base image {base_image}", flush=True)
    subprocess.run([runtime, "pull", base_image], check=True, env=env)
    setup_container = "operit2-build-debian12-setup"
    subprocess.run([runtime, "rm", "-f", setup_container], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    run_command = [
        runtime, "run", "--name", setup_container, "--network", "host",
        "-v", f"{REPO_ROOT}:/work:ro",
    ]
    if args.apt_mirror:
        run_command += ["-e", f"DEBIAN_MIRROR={args.apt_mirror}"]
    for name in PASSTHROUGH_ENV_VARS:
        value = os.environ.get(name)
        if value:
            run_command += ["-e", f"{name}={value}"]
    run_command += [base_image, "bash", "/work/tools/build_scripts/containers/debian12/image-setup.sh"]
    print("Provisioning image contents", flush=True)
    subprocess.run(run_command, check=True, env=env)
    subprocess.run([
        runtime, "commit",
        "--change", 'ENV PATH="/opt/operit-cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"',
        "--change", "ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0",
        "--change", "ENV RUSTUP_HOME=/opt/operit-rustup",
        "--change", "ENV CARGO_HOME=/opt/operit-cargo",
        "--change", "WORKDIR /work",
        setup_container, IMAGE_TAG,
    ], check=True, env=env)
    subprocess.run([runtime, "rm", "-f", setup_container], check=True, env=env)


# Runs one container invocation with the shared repository and cache mounts.
def run_in_container(runtime: str, args: argparse.Namespace, command: list[str], extra_env: dict[str, str]) -> None:
    DEB12_WORK_DIR.joinpath("rustup").mkdir(parents=True, exist_ok=True)
    DEB12_WORK_DIR.joinpath("cargo-home").mkdir(parents=True, exist_ok=True)
    run_command = [runtime, "run", "--rm", "--network", "host"]
    if runtime == "podman":
        run_command += ["--userns=keep-id"]
    else:
        run_command += ["--user", f"{os.getuid()}:{os.getgid()}"]
    run_command += [
        "-v", f"{REPO_ROOT}:/work",
        "-v", f"{DEB12_WORK_DIR / 'rustup'}:/opt/operit-rustup",
        "-v", f"{DEB12_WORK_DIR / 'cargo-home'}:/opt/operit-cargo",
        "-e", "HOME=/opt/operit-cargo/home",
        "-e", f"CARGO_TARGET_DIR=/work/{DEB12_WORK_DIR.relative_to(REPO_ROOT).as_posix()}/target",
    ]
    for name in PASSTHROUGH_ENV_VARS:
        value = os.environ.get(name)
        if value:
            run_command += ["-e", f"{name}={value}"]
    for entry in args.env:
        if "=" not in entry:
            raise RuntimeError(f"--env expects KEY=VALUE, got: {entry}")
        run_command += ["-e", entry]
    run_command += [IMAGE_TAG, *command]
    print(f"Building CLI in Debian 12 container: {' '.join(run_command[:2])}", flush=True)
    subprocess.run(run_command, check=True, env={**os.environ, **extra_env})


# Returns the highest GLIBC symbol version referenced in objdump output.
def highest_glibc_version(text: str) -> tuple[int, ...] | None:
    versions = [tuple(int(part) for part in found.split(".")) for found in re.findall(r"GLIBC_([0-9.]+)", text)]
    if not versions:
        return None
    return max(versions)


# Fails when the binary references glibc symbols newer than Debian 12 ships.
def verify_glibc_version(binary_path: Path) -> str:
    if shutil.which("objdump") is None:
        raise RuntimeError("objdump is required to verify the packaged glibc target")
    result = subprocess.run(["objdump", "-T", str(binary_path)], text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode != 0:
        raise RuntimeError(f"objdump failed on {binary_path}: {result.stderr.strip()}")
    highest = highest_glibc_version(result.stdout)
    if highest is None:
        raise RuntimeError(f"No glibc symbol versions found in {binary_path}")
    if highest > DEBIAN_GLIBC_VERSION:
        formatted = ".".join(str(part) for part in highest)
        raise RuntimeError(f"{binary_path} references GLIBC_{formatted}, newer than the Debian 12 baseline")
    return ".".join(str(part) for part in highest)


def package_target(arch: str) -> Path:
    rust_target = RUST_TARGETS[arch]
    binary_path = DEB12_WORK_DIR / "target" / rust_target / "release" / "operit2"
    if not binary_path.is_file():
        raise RuntimeError(f"Expected container build output not found: {binary_path}")
    verify_cli_plugin_assets(binary_path)
    glibc_version = verify_glibc_version(binary_path)

    package_dir = DEB12_WORK_DIR / "package" / f"cli-linux-{arch}"
    reset_dir(package_dir)
    copy_required_file(binary_path, package_dir / "operit2")
    write_cli_installer_files(package_dir, "linux")
    os.chmod(package_dir / "operit2", 0o755)
    for name in ("install.sh", "uninstall.sh"):
        os.chmod(package_dir / name, 0o755)
    package_path = DIST_DIR / release_asset_name("cli", "linux", arch, "tar.gz")
    compress_tar_gz(package_dir, package_path)
    print(f"CLI binary: {binary_path} (glibc <= {glibc_version})", flush=True)
    print(f"CLI archive: {package_path}", flush=True)
    return package_path


def main() -> int:
    args = parse_args()
    runtime = select_runtime(args.runtime)
    extra_env = runtime_extra_env(runtime)
    if args.rebuild_image:
        subprocess.run([runtime, "rmi", IMAGE_TAG], env={**os.environ, **extra_env}, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if args.rebuild_image or not image_exists(runtime, IMAGE_TAG, extra_env):
        build_image(runtime, args, extra_env)
    if args.shell:
        run_in_container(runtime, args, ["bash"], extra_env)
        return 0
    for arch in selected_arches(args.arches):
        run_in_container(runtime, args, ["bash", "/work/tools/build_scripts/containers/debian12/container-build.sh", arch], extra_env)
        package_target(arch)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
