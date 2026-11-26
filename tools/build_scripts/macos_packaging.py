"""Create the drag-to-Applications disk image for the unrestricted macOS app."""
from pathlib import Path
import shutil
import tempfile

from common import run
from macos_validation import verify_macos_dmg


def package_macos_dmg(app: Path, image: Path, build_type: str) -> Path:
    """Package the exact built bundle and validate the resulting read-only disk image."""
    image.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="operit-macos-dmg-stage-") as directory:
        staging = Path(directory)
        shutil.copytree(app, staging / app.name, symlinks=True)
        (staging / "Applications").symlink_to("/Applications", target_is_directory=True)
        run([
            "hdiutil", "create", "-volname", "Operit2", "-srcfolder", str(staging),
            "-format", "UDZO", "-ov", str(image.resolve()),
        ])
    if not image.is_file() or image.stat().st_size == 0:
        raise RuntimeError(f"macOS disk image was not produced: {image}")
    verify_macos_dmg(image, build_type)
    return image
