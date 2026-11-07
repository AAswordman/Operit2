#!/usr/bin/env python3
"""Run the Android module application's own directory builder."""
from pathlib import Path
import runpy


def main() -> None:
    """Delegate application packaging to its product directory without owning a second build recipe."""
    runpy.run_path(str(Path(__file__).resolve().parents[2] / "apps/android-module/build.py"), run_name="__main__")


if __name__ == "__main__":
    main()
