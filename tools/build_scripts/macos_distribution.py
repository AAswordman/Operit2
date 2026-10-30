"""Shared macOS distribution settings and checks against the actual signature."""
from __future__ import annotations

from contextlib import contextmanager
import os
from pathlib import Path
import plistlib
import subprocess
import tempfile
from collections.abc import Iterator

from common import FLUTTER_APP_DIR, REPO_ROOT

MACOS_TYPES = ("sandbox", "unsandbox")
RUNNER_DIR = FLUTTER_APP_DIR / "macos" / "Runner"
MACOS_WORK_DIR = REPO_ROOT / "tools" / "release" / "work" / "macos"


def entitlements_path(build_type: str) -> Path:
    if build_type not in MACOS_TYPES:
        raise ValueError(f"Unsupported macOS build type: {build_type}")
    return RUNNER_DIR / ("Release.entitlements" if build_type == "sandbox" else "UnsandboxRelease.entitlements")


def built_app_path(build_type: str) -> Path:
    entitlements_path(build_type)  # Validate before constructing any output path.
    return MACOS_WORK_DIR / build_type / "Operit2.app"


@contextmanager
def build_environment(build_type: str) -> Iterator[dict[str, str]]:
    """Override only our Runner setting; never apply app entitlements to Pods."""
    entitlements_path(build_type)
    with tempfile.TemporaryDirectory(prefix="operit-macos-") as directory:
        config = Path(directory) / "Distribution.xcconfig"
        lines = []
        existing = os.environ.get("XCODE_XCCONFIG_FILE")
        if existing:
            source = Path(existing).resolve(strict=True)
            if any(char in str(source) for char in ('"', '\n', '\r')):
                raise ValueError("XCODE_XCCONFIG_FILE contains unsupported characters")
            lines.append(f'#include "{source}"')
        lines.append(f"OPERIT_MACOS_TYPE = {build_type}")
        config.write_text("\n".join(lines) + "\n", encoding="utf-8")
        yield {"XCODE_XCCONFIG_FILE": str(config)}


def verify_app_type(app: Path, build_type: str) -> None:
    """Prevent mislabelled archives and signing a bundle for the wrong channel."""
    entitlements_path(build_type)
    result = subprocess.run(
        ["codesign", "-d", "--entitlements", ":-", str(app)],
        capture_output=True, check=True,
    )
    try:
        entitlements = plistlib.loads(result.stdout)
    except (plistlib.InvalidFileException, ValueError) as error:
        raise RuntimeError(f"Cannot read signed entitlements from {app}") from error
    actual = entitlements.get("com.apple.security.app-sandbox", False)
    expected = build_type == "sandbox"
    if not isinstance(actual, bool) or actual != expected:
        raise RuntimeError(
            f"macOS distribution mismatch: requested {build_type}, "
            f"but {app} has app-sandbox={actual!r}. Rebuild with --type {build_type}."
        )
