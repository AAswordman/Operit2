#!/usr/bin/env python3
"""Reports the NumPy build interface from an iOS cross virtual environment."""

from __future__ import annotations

import sys
import sysconfig
from pathlib import Path


NUMPY_VERSION = "2.2.3"


def main() -> int:
    """Prints the version or include flags requested by Meson's NumPy config-tool lookup."""
    arguments = sys.argv[1:]
    if arguments == ["--version"]:
        print(NUMPY_VERSION)
        return 0
    if arguments == ["--cflags"]:
        include_directory = Path(sysconfig.get_path("platlib")) / "numpy" / "_core" / "include"
        if not include_directory.is_dir():
            raise RuntimeError(f"NumPy headers are missing from the cross environment: {include_directory}")
        print(f"-I{include_directory}")
        return 0
    raise RuntimeError("unsupported NumPy config-tool arguments: " + " ".join(arguments))


if __name__ == "__main__":
    raise SystemExit(main())
