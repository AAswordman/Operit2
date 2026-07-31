#!/usr/bin/env python3
"""Reports the NumPy build interface from an iOS cross virtual environment."""

from __future__ import annotations

import sys
import sysconfig
import subprocess
from pathlib import Path


NUMPY_VERSION = "2.2.3"
PYTHRAN_VERSION = "0.17.0"


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
    subprocess.run(
        [
            sys.executable,
            "-m",
            "pip",
            "install",
            "--no-index",
            f"--find-links={wheel_directory}",
            f"numpy=={NUMPY_VERSION}",
        ],
        check=True,
    )
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
