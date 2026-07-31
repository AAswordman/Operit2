#!/usr/bin/env python3
"""Reports the NumPy build interface from an iOS cross virtual environment."""

from __future__ import annotations

import os
import re
import sys
import sysconfig
import subprocess
import zipfile
from pathlib import Path


NUMPY_VERSION = "2.2.3"
PYTHRAN_VERSION = "0.18.1"


def numpy_include_directory() -> Path:
    """Returns the installed NumPy headers without importing target extension modules."""
    include_directory = Path(sysconfig.get_path("platlib")) / "numpy" / "_core" / "include"
    if not include_directory.is_dir():
        raise RuntimeError(f"NumPy headers are missing from the cross environment: {include_directory}")
    return include_directory


def pythran_include_directory() -> Path:
    """Returns the installed Pythran headers without importing its NumPy-dependent config."""
    include_directory = Path(sysconfig.get_path("platlib")) / "pythran"
    if not include_directory.is_dir():
        raise RuntimeError(f"Pythran headers are missing from the cross environment: {include_directory}")
    return include_directory


def target_wheel_platform_tag() -> str:
    """Returns the exact iOS wheel platform tag for the active cross environment."""
    deployment_target = os.environ.get("IPHONEOS_DEPLOYMENT_TARGET")
    if deployment_target is None:
        raise RuntimeError("IPHONEOS_DEPLOYMENT_TARGET is required for the SciPy cross build")
    platform_match = re.fullmatch(
        r"ios-\d+\.\d+-(arm64|x86_64)-(iphoneos|iphonesimulator)",
        sysconfig.get_platform(),
    )
    if platform_match is None:
        raise RuntimeError(f"unsupported iOS cross platform: {sysconfig.get_platform()}")
    architecture, environment = platform_match.groups()
    normalized_target = deployment_target.replace(".", "_")
    return f"ios_{normalized_target}_{architecture}_{environment}"


def install_numpy_wheel(wheel_directory: Path) -> None:
    """Installs the exact target NumPy wheel without host platform tag validation."""
    python_tag = f"cp{sys.version_info.major}{sys.version_info.minor}"
    wheel_name = f"numpy-{NUMPY_VERSION}-{python_tag}-{python_tag}-{target_wheel_platform_tag()}.whl"
    wheel = wheel_directory / wheel_name
    if not wheel.is_file():
        raise RuntimeError(f"target NumPy wheel is missing: {wheel}")
    destination = Path(sysconfig.get_path("platlib"))
    destination.mkdir(parents=True, exist_ok=True)
    resolved_destination = destination.resolve()
    with zipfile.ZipFile(wheel) as archive:
        for member in archive.infolist():
            target = (destination / member.filename).resolve()
            if target != resolved_destination and resolved_destination not in target.parents:
                raise RuntimeError(f"target NumPy wheel entry escapes site-packages: {member.filename}")
        archive.extractall(destination)


def write_cross_file(cross_file: Path, host_f2py: Path, host_pythran: Path) -> None:
    """Writes Meson NumPy and Pythran settings for the active cross environment."""
    if not host_f2py.is_file():
        raise RuntimeError(f"host F2Py executable is missing: {host_f2py}")
    if not host_pythran.is_file():
        raise RuntimeError(f"host Pythran executable is missing: {host_pythran}")
    cross_file.parent.mkdir(parents=True, exist_ok=True)
    cross_file.write_text(
        "[binaries]\n"
        f"numpy-config = ['python', '{Path(__file__).resolve()}']\n"
        f"f2py = '{host_f2py}'\n"
        "[properties]\n"
        f"pythran-program = '{host_pythran}'\n"
        f"numpy-include-dir = '{numpy_include_directory()}'\n"
        f"pythran-include-dir = '{pythran_include_directory()}'\n",
        encoding="utf-8",
    )


def prepare_cross_build(
    wheel_directory: Path,
    cross_file: Path,
    host_f2py: Path,
    host_pythran: Path,
) -> None:
    """Installs target headers and records the host code generators for SciPy Meson."""
    install_numpy_wheel(wheel_directory)
    subprocess.run(
        [
            sys.executable,
            "-m",
            "pip",
            "install",
            "--no-deps",
            f"pythran=={PYTHRAN_VERSION}",
        ],
        check=True,
    )
    write_cross_file(cross_file, host_f2py, host_pythran)


def main() -> int:
    """Reports NumPy build settings or prepares the SciPy cross-build configuration."""
    arguments = sys.argv[1:]
    if len(arguments) == 5 and arguments[0] == "--prepare-cross-build":
        prepare_cross_build(
            Path(arguments[1]),
            Path(arguments[2]),
            Path(arguments[3]),
            Path(arguments[4]),
        )
        return 0
    if arguments == ["--version"]:
        print(NUMPY_VERSION)
        return 0
    if arguments == ["--cflags"]:
        print(f"-I{numpy_include_directory()}")
        return 0
    raise RuntimeError("unsupported NumPy config-tool arguments: " + " ".join(arguments))


if __name__ == "__main__":
    raise SystemExit(main())
