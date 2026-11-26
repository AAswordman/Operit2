"""Check the board's pinned MessagePack overlay with a desktop Rust toolchain."""
from pathlib import Path
import subprocess
import sys

EDITOR = Path(__file__).resolve().parent
ROOT = EDITOR.parents[3]


def main():
    # Cargo discovers the board's Xtensa defaults from the working directory,
    # even with a desktop manifest. Run outside that subtree for native tests.
    subprocess.run([sys.executable, "-X", "utf8", str(EDITOR / "prepare-msgpack.py")], cwd=ROOT, check=True)
    subprocess.run(["cargo", "test", "--locked", "--manifest-path",
                    str(EDITOR / "tests/msgpack-compat/Cargo.toml")], cwd=ROOT, check=True)
    subprocess.run(["cargo", "test", "--locked", "--manifest-path",
                    str(EDITOR.parent / ".embuild/rmp-serde-1.3.1/Cargo.toml"),
                    "--lib", "--test", "decode", "--test", "decode_derive",
                    "--test", "encode", "--test", "encode_derive"], cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
