"""Derive watch atlases and studio animations from the Task Status Cat originals."""

import argparse
import hashlib
import io
import json
from pathlib import Path

from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parents[3]
SOURCE = ROOT / "plugins/packages/external/task_status_pet/resources"
ANDROID = Path(__file__).resolve().parent / "android/app/src/main/assets/task-cat"
STUDIO = ROOT / "plugins/packages/external/edge_watch_pet/resources"
MOODS = ("working", "thinking", "approval", "completed", "idle", "error")


def publish(path: Path, data: bytes, check: bool) -> None:
    """Write derived data or require it to match the checked-in artifact exactly."""
    if check:
        if not path.is_file() or path.read_bytes() != data:
            raise ValueError(f"Stale pet asset: {path.relative_to(ROOT)}")
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def synchronize(check: bool) -> None:
    """Preserve every source frame and duration on the original 90 by 88 pixel grid."""
    for mood in MOODS:
        source = SOURCE / f"{mood}.webp"
        data = source.read_bytes()
        publish(STUDIO / source.name, data, check)
        with Image.open(source) as animation:
            atlas = Image.new("RGBA", (90 * 8, 88 * ((animation.n_frames + 7) // 8)))
            durations = []
            for index in range(animation.n_frames):
                animation.seek(index)
                frame = animation.convert("RGBA")
                small = frame.resize((90, 88), Image.Resampling.NEAREST)
                restored = small.resize(frame.size, Image.Resampling.NEAREST)
                if ImageChops.difference(frame, restored).getbbox(alpha_only=False):
                    raise ValueError(f"{source.name} frame {index} is not on the pixel grid")
                atlas.paste(small, ((index % 8) * 90, (index // 8) * 88))
                durations.append(animation.info["duration"])
            output = io.BytesIO()
            atlas.save(output, format="PNG", optimize=True)
            publish(ANDROID / f"{mood}.png", output.getvalue(), check)
            manifest = {"width": 90, "height": 88, "columns": 8, "durations": durations,
                        "sourceSha256": hashlib.sha256(data).hexdigest()}
            publish(ANDROID / f"{mood}.json", (json.dumps(manifest, separators=(",", ":")) + "\n").encode(), check)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    synchronize(parser.parse_args().check)
