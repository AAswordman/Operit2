"""Convert the attributed VP9-alpha source clips to transparent animated WebP.

Requires Pillow plus ffmpeg with libvpx-vp9, or Pillow plus imageio-ffmpeg.
Runs entirely offline against resources/source; verifies source SHA-256 first.
All clips use the same crop and scale. No synthetic motion or frame transforms.
"""
import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "resources/source"
OUTPUT = ROOT / "resources"
# VP9 alpha compression leaves alpha=1 on the background. Remove only near-zero
# alpha before finding a common crop; otherwise the entire video looks opaque
# enough to retain its wide 16:9 canvas and can intercept desktop input.
ALPHA_FLOOR = 4


def ffmpeg_path(explicit):
    if explicit:
        return explicit
    binary = shutil.which("ffmpeg")
    if binary:
        return binary
    try:
        import imageio_ffmpeg
        return imageio_ffmpeg.get_ffmpeg_exe()
    except ImportError as error:
        raise SystemExit("Install ffmpeg with libvpx-vp9, or imageio-ffmpeg") from error


def convert(ffmpeg):
    provenance = json.loads((SOURCE / "provenance.json").read_text())
    fps = provenance["fps"]
    size = tuple(provenance["canvas"])
    report = {"canvas": size, "fps": fps, "alpha_floor": ALPHA_FLOOR, "actions": {}}
    with tempfile.TemporaryDirectory(prefix="dafeiyu-frames-") as work:
        decoded = {}
        bounds = None
        for name, action in provenance["actions"].items():
            source = SOURCE / (name + ".webm")
            if hashlib.sha256(source.read_bytes()).hexdigest() != action["sha256"]:
                raise ValueError(f"Source checksum mismatch: {source}")
            frames_dir = Path(work) / name
            frames_dir.mkdir()
            # The native VP9 decoder drops alpha. Explicitly use libvpx-vp9.
            subprocess.run([ffmpeg, "-v", "error", "-c:v", "libvpx-vp9",
                            "-i", str(source), "-vf", f"fps={fps}",
                            str(frames_dir / "%04d.png")], check=True)
            files = sorted(frames_dir.glob("*.png"))
            if len(files) < 2:
                raise ValueError(f"Not an animation: {name}")
            decoded[name] = files
            for file in files:
                with Image.open(file) as frame:
                    box = frame.getchannel("A").point(
                        lambda a: 255 if a > ALPHA_FLOOR else 0).getbbox()
                if box:
                    bounds = box if bounds is None else (
                        min(bounds[0], box[0]), min(bounds[1], box[1]),
                        max(bounds[2], box[2]), max(bounds[3], box[3]))
        if bounds is None:
            raise ValueError("No visible pixels in source clips")
        # A single spatial transform for every frame of every action prevents
        # the character from jumping in size or position when actions change.
        ratio = min((size[0] - 16) / (bounds[2] - bounds[0]),
                    (size[1] - 16) / (bounds[3] - bounds[1]))
        scaled = (round((bounds[2] - bounds[0]) * ratio),
                  round((bounds[3] - bounds[1]) * ratio))
        position = ((size[0] - scaled[0]) // 2, (size[1] - scaled[1]) // 2)
        report["source_crop"] = bounds
        for name, files in decoded.items():
            frames = []
            durations = [round((i + 1) * 1000 / fps) - round(i * 1000 / fps)
                         for i in range(len(files))]
            for file in files:
                with Image.open(file) as image:
                    frame = image.convert("RGBA")
                frame.putalpha(frame.getchannel("A").point(
                    lambda a: 0 if a <= ALPHA_FLOOR else a))
                frame = frame.crop(bounds).resize(scaled, Image.Resampling.LANCZOS)
                canvas = Image.new("RGBA", size)
                canvas.paste(frame, position)
                frames.append(canvas)
            frames[0].save(OUTPUT / (name + ".webp"), save_all=True,
                           append_images=frames[1:], duration=durations,
                           loop=0 if provenance["actions"][name]["loop"] else 1,
                           lossless=False, quality=88, method=4, minimize_size=True)
            if name == "idle":
                frames[0].save(OUTPUT / "still.png")
            with Image.open(OUTPUT / (name + ".webp")) as encoded:
                duration_ms = 0
                for index in range(encoded.n_frames):
                    encoded.seek(index)
                    encoded.load()
                    duration_ms += encoded.info["duration"]
                # WebP may coalesce identical adjacent source frames; it must
                # preserve their accumulated display time.
                if duration_ms != sum(durations):
                    raise ValueError(f"Encoded duration changed: {name}")
                report["actions"][name] = {"source_frames": len(frames),
                                           "frames": encoded.n_frames,
                                           "duration_ms": duration_ms,
                                           "loop": provenance["actions"][name]["loop"]}
            print(name, report["actions"][name], (OUTPUT / (name + ".webp")).stat().st_size,
                  flush=True)
    (OUTPUT / "animation-info.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ffmpeg", help="Path to ffmpeg with libvpx-vp9 support")
    convert(ffmpeg_path(parser.parse_args().ffmpeg))
