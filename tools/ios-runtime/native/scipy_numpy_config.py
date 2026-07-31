#!/usr/bin/env python3
"""Reports the NumPy build interface from an iOS cross virtual environment."""

from __future__ import annotations

import sys
import sysconfig
import subprocess
from pathlib import Path


NUMPY_VERSION = "2.2.3"


def numpy_include_directory() -> Path:
    """Returns the installed NumPy headers without importing target extension modules."""
    include_directory = Path(sysconfig.get_path("platlib")) / "numpy" / "_core" / "include"
    if not include_directory.is_dir():
        raise RuntimeError(f"NumPy headers are missing from the cross environment: {include_directory}")
    return include_directory


def write_cross_file(cross_file: Path) -> None:
    """Writes the Meson NumPy lookup and include directory for the active cross environment."""
    cross_file.parent.mkdir(parents=True, exist_ok=True)
    cross_file.write_text(
        "[binaries]\n"
        f"numpy-config = ['python', '{Path(__file__).resolve()}']\n"
        "[properties]\n"
        f"numpy-include-dir = '{numpy_include_directory()}'\n",
        encoding="utf-8",
    )


def prepare_cross_build(wheel_directory: Path, cross_file: Path) -> None:
    """Installs the matching target NumPy wheel and records its headers for SciPy Meson."""
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
    write_cross_file(cross_file)


def main() -> int:
    """Reports NumPy build settings or prepares the SciPy cross-build configuration."""
    arguments = sys.argv[1:]
    if len(arguments) == 3 and arguments[0] == "--prepare-cross-build":
        prepare_cross_build(Path(arguments[1]), Path(arguments[2]))
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
