"""Generate and verify portable, byte-exact Operit2 package deltas."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import mmap
from pathlib import Path
import re
import struct

MAGIC = b"O2PATCH1\0"
ANCHOR = re.compile(rb"[\x00-\x03]\x00")
MIN_CHUNK = 4096
MAX_CHUNK = 65536
BUFFER = 1024 * 1024


# Hashes a package with bounded memory.
def sha256(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


# Produces content-defined chunks that resynchronize after insertions.
def chunks(data: mmap.mmap):
    start = 0
    while start < len(data):
        end = min(start + MAX_CHUNK, len(data))
        anchor = ANCHOR.search(data, start + MIN_CHUNK, end)
        if anchor is not None:
            end = anchor.end()
        yield start, end
        start = end


# Encodes COPY and ADD instructions over the original signed package bytes.
def build(base: Path, target: Path, output: Path) -> dict:
    if base.stat().st_size == 0 or target.stat().st_size == 0:
        raise ValueError("Packages must not be empty")
    if output.resolve() in (base.resolve(), target.resolve()):
        raise ValueError("Patch output must differ from both input packages")
    base_hash, target_hash = sha256(base), sha256(target)
    output.parent.mkdir(parents=True, exist_ok=True)
    with base.open("rb") as old, target.open("rb") as new, \
            mmap.mmap(old.fileno(), 0, access=mmap.ACCESS_READ) as old_map, \
            mmap.mmap(new.fileno(), 0, access=mmap.ACCESS_READ) as new_map, \
            output.open("xb") as destination, \
            gzip.GzipFile(filename="", fileobj=destination, mode="wb", mtime=0) as patch:
        index = {}
        for start, end in chunks(old_map):
            index.setdefault(hashlib.sha256(old_map[start:end]).digest(), (start, end - start))
        patch.write(MAGIC)
        patch.write(struct.pack("<QQ", len(old_map), len(new_map)))
        patch.write(bytes.fromhex(base_hash) + bytes.fromhex(target_hash))
        for start, end in chunks(new_map):
            block = new_map[start:end]
            match = index.get(hashlib.sha256(block).digest())
            if match is None:
                patch.write(b"\x02" + struct.pack("<Q", len(block)) + block)
            else:
                offset, length = match
                patch.write(b"\x01" + struct.pack("<QQ", offset, length))
        patch.write(b"\x00")
    return {"baseSha256": base_hash, "baseSize": base.stat().st_size,
            "sha256": sha256(output), "size": output.stat().st_size,
            "targetSha256": target_hash, "targetSize": target.stat().st_size}


# Rejects truncated patch fields before decoding them.
def exact(source, size: int) -> bytes:
    data = source.read(size)
    if len(data) != size:
        raise ValueError("Truncated patch")
    return data


# Reconstructs into a new file and removes incomplete output on every failure.
def apply(base: Path, patch: Path, output: Path) -> None:
    if output.exists() or output.resolve() in (base.resolve(), patch.resolve()):
        raise ValueError("Output must be a new file")
    created = False
    try:
        with gzip.open(patch, "rb") as source, base.open("rb") as old:
            if exact(source, 9) != MAGIC:
                raise ValueError("Unsupported patch format")
            old_size, new_size = struct.unpack("<QQ", exact(source, 16))
            old_hash, new_hash = exact(source, 32).hex(), exact(source, 32).hex()
            if base.stat().st_size != old_size or sha256(base) != old_hash:
                raise ValueError("Base package does not match patch")
            with output.open("xb") as destination:
                created = True
                written = 0
                while True:
                    operation = exact(source, 1)[0]
                    if operation == 0:
                        break
                    if operation == 1:
                        offset, length = struct.unpack("<QQ", exact(source, 16))
                        if offset + length > old_size:
                            raise ValueError("COPY exceeds base package")
                        old.seek(offset)
                        reader = old
                    elif operation == 2:
                        length = struct.unpack("<Q", exact(source, 8))[0]
                        reader = source
                    else:
                        raise ValueError("Invalid patch instruction")
                    if length == 0 or written + length > new_size:
                        raise ValueError("Invalid output length")
                    remaining = length
                    while remaining:
                        block = exact(reader, min(remaining, BUFFER))
                        destination.write(block)
                        remaining -= len(block)
                    written += length
                if written != new_size or source.read(1):
                    raise ValueError("Invalid patch end")
            if sha256(output) != new_hash:
                raise ValueError("Reconstructed package SHA-256 mismatch")
    except BaseException:
        if created:
            output.unlink()
        raise


# Exposes generation and independent round-trip verification for any package format.
def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("build", "apply"):
        command = commands.add_parser(name)
        command.add_argument("base", type=Path)
        command.add_argument("input", type=Path)
        command.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.command == "build":
        print(json.dumps(build(args.base, args.input, args.output), indent=2))
    else:
        apply(args.base, args.input, args.output)


if __name__ == "__main__":
    main()
