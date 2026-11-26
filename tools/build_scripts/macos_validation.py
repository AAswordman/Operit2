"""Check the extracted macOS release bundle for signature and immediate launch failures."""
from __future__ import annotations

from pathlib import Path
import plistlib
import subprocess
import tempfile

from common import run
from macos_distribution import verify_app_type


def verify_app_launch(app: Path, startup_seconds: float = 15.0) -> None:
    """Require the packaged executable to stay alive throughout the startup interval."""
    if startup_seconds <= 0:
        raise ValueError("The macOS startup interval must be positive")
    info = plistlib.loads((app / "Contents" / "Info.plist").read_bytes())
    name = info.get("CFBundleExecutable")
    if not isinstance(name, str) or not name or Path(name).name != name:
        raise RuntimeError(f"Invalid CFBundleExecutable in {app}")
    executable = app / "Contents" / "MacOS" / name
    if not executable.is_file():
        raise RuntimeError(f"macOS executable is missing: {executable}")
    with tempfile.TemporaryFile() as log:
        process = subprocess.Popen(
            [str(executable)], cwd=app.parent, stdin=subprocess.DEVNULL,
            stdout=log, stderr=subprocess.STDOUT,
        )
        try:
            try:
                status = process.wait(timeout=startup_seconds)
            except subprocess.TimeoutExpired:
                print(f"macOS launch check: {app.name} survived {startup_seconds:g}s", flush=True)
                return
            log.seek(0, 2)
            log.seek(max(0, log.tell() - 16384))
            output = log.read().decode("utf-8", errors="replace")
            raise RuntimeError(
                f"macOS launch check failed: {app.name} exited with status {status} "
                f"within {startup_seconds:g}s.\n{output}\n"
                "Inspect ~/Library/Logs/DiagnosticReports for the macOS crash report."
            )
        finally:
            if process.poll() is None:
                process.kill()
            process.wait(timeout=10)


def verify_macos_archive(archive: Path, build_type: str) -> None:
    """Extract the final ZIP and check its actual signature and startup survival."""
    with tempfile.TemporaryDirectory(prefix="operit-macos-launch-") as directory:
        extracted = Path(directory)
        run(["ditto", "-x", "-k", str(archive.resolve()), str(extracted)])
        app = extracted / "Operit2.app"
        run(["codesign", "--verify", "--deep", "--strict", str(app)])
        verify_app_type(app, build_type)
        verify_app_launch(app)


def verify_macos_dmg(image: Path, build_type: str) -> None:
    """Verify and mount the final disk image, then check the app distributed inside it."""
    run(["hdiutil", "verify", str(image.resolve())])
    with tempfile.TemporaryDirectory(prefix="operit-macos-dmg-check-") as directory:
        mount = Path(directory) / "mounted"
        mount.mkdir()
        run(["hdiutil", "attach", "-readonly", "-nobrowse", "-mountpoint", str(mount), str(image.resolve())])
        try:
            app = mount / "Operit2.app"
            run(["codesign", "--verify", "--deep", "--strict", str(app)])
            verify_app_type(app, build_type)
            verify_app_launch(app)
        finally:
            run(["hdiutil", "detach", str(mount)])
