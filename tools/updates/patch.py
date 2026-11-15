"""Generate and verify portable, byte-exact Operit2 package deltas."""
from __future__ import annotations

import argparse
import bz2
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import shutil

from hdiff import executable
import msi

MAGIC = b"O2PATCH2\0"
BUFFER = 1024 * 1024


# Hashes a package with bounded memory.
def sha256(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


# Reads the signed-magnitude integers used by the Endsley BSDIFF stream.
def signed(source) -> int:
    value, = struct.unpack("<Q", exact(source, 8))
    return -(value & ((1 << 63) - 1)) if value >> 63 else value


# Converts mature matcher output to a bounded-memory, portable residual instruction stream.
def encode(delta: Path, output: Path, base_size: int, target_size: int,
           base_hash: str, target_hash: str) -> None:
    with delta.open("rb") as encoded:
        if exact(encoded, 16) != b"ENDSLEY/BSDIFF43" or signed(encoded) != target_size:
            raise ValueError("Invalid HDiffPatch output")
        with bz2.BZ2File(encoded) as source, output.open("xb") as destination, \
                gzip.GzipFile(filename="", fileobj=destination, mode="wb", mtime=0) as patch:
            patch.write(MAGIC)
            patch.write(struct.pack("<QQ", base_size, target_size))
            patch.write(bytes.fromhex(base_hash) + bytes.fromhex(target_hash))
            old_offset = written = 0
            for _ in range(4_000_000):
                if written == target_size:
                    break
                length, extra, seek = signed(source), signed(source), signed(source)
                if min(length, extra, old_offset) < 0 or old_offset + length > base_size \
                        or written + length + extra > target_size:
                    raise ValueError("Invalid HDiffPatch ranges")
                remaining = length
                while remaining:
                    block = exact(source, min(remaining, BUFFER))
                    if any(block):
                        patch.write(b"\x03" + struct.pack("<QQ", old_offset, len(block)) + block)
                    else:
                        patch.write(b"\x01" + struct.pack("<QQ", old_offset, len(block)))
                    old_offset += len(block)
                    remaining -= len(block)
                if extra:
                    patch.write(b"\x02" + struct.pack("<Q", extra))
                    remaining = extra
                    while remaining:
                        block = exact(source, min(remaining, BUFFER))
                        patch.write(block)
                        remaining -= len(block)
                old_offset += seek
                written += length + extra
            else:
                raise ValueError("Too many HDiffPatch instructions")
            if source.read(1):
                raise ValueError("Trailing HDiffPatch instructions")
            patch.write(b"\x00")


# Compares precise and streaming matches, publishing the smallest verified representation.
def build_binary(base: Path, target: Path, output: Path) -> dict:
    base_size, target_size = base.stat().st_size, target.stat().st_size
    if not base_size or not target_size:
        raise ValueError("Packages must not be empty")
    if output.resolve() in (base.resolve(), target.resolve()):
        raise ValueError("Patch output must differ from both input packages")
    if output.exists():
        raise FileExistsError(output)
    base_hash, target_hash = sha256(base), sha256(target)
    output.parent.mkdir(parents=True, exist_ok=True)
    generator = executable()
    profiles = [["-s-16"]]
    # The suffix index needs approximately oldSize * 5 + newSize bytes of builder memory.
    if base_size * 5 + target_size <= 2 * 1024 ** 3:
        profiles.append(["-m-0", "-block-0"])
    with tempfile.TemporaryDirectory(prefix="operit-hdiff-") as temporary:
        candidates = []
        for index, profile in enumerate(profiles):
            delta, candidate = [Path(temporary) / f"{index}.{suffix}" for suffix in ("diff", "gz")]
            subprocess.run([str(generator), *profile, "-p-1", "-BSD", "-SD",
                            str(base.resolve()), str(target.resolve()), str(delta)],
                           check=True, stdout=subprocess.DEVNULL)
            encode(delta, candidate, base_size, target_size, base_hash, target_hash)
            candidates.append(candidate)
        smallest = min(candidates, key=lambda path: path.stat().st_size)
        with smallest.open("rb") as source, output.open("xb") as destination:
            shutil.copyfileobj(source, destination, BUFFER)
    return {"baseSha256": base_hash, "baseSize": base.stat().st_size,
            "sha256": sha256(output), "size": output.stat().st_size,
            "targetSha256": target_hash, "targetSize": target.stat().st_size}


# Matches uncompressed MSI payloads and retains the exact original installer identities.
def build(base: Path, target: Path, output: Path) -> dict:
    if target.suffix != ".msi":
        return build_binary(base, target, output)
    if output.exists() or output.resolve() in (base.resolve(), target.resolve()):
        raise ValueError("Patch output must be a new file")
    with tempfile.TemporaryDirectory(prefix="operit-msi-") as temporary:
        old, new, delta = [Path(temporary) / name for name in ("old", "new", "delta")]
        msi.decode(base, old)
        msi.decode(target, new)
        build_binary(old, new, delta)
        output.parent.mkdir(parents=True, exist_ok=True)
        with gzip.open(delta, "rb") as instructions, output.open("xb") as destination, \
                gzip.GzipFile(filename="", fileobj=destination, mode="wb", mtime=0) as patch:
            if exact(instructions, 9) != MAGIC:
                raise ValueError("Invalid nested patch")
            patch.write(b"O2PATCH3\0" + struct.pack("<QQ", base.stat().st_size, target.stat().st_size))
            patch.write(bytes.fromhex(sha256(base)) + bytes.fromhex(sha256(target)))
            with old.open("rb") as expanded:
                if exact(expanded, 8) != msi.MAGIC:
                    raise ValueError("Invalid expanded MSI")
                length = exact(expanded, 4)
                patch.write(length + exact(expanded, struct.unpack("<I", length)[0]))
            shutil.copyfileobj(instructions, patch, BUFFER)
    return {"baseSha256": sha256(base), "baseSize": base.stat().st_size,
            "sha256": sha256(output), "size": output.stat().st_size,
            "targetSha256": sha256(target), "targetSize": target.stat().st_size}


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
            magic = exact(source, 9)
            if magic not in (b"O2PATCH1\0", MAGIC, b"O2PATCH3\0"):
                raise ValueError("Unsupported patch format")
            old_size, new_size = struct.unpack("<QQ", exact(source, 16))
            old_hash, new_hash = exact(source, 32).hex(), exact(source, 32).hex()
            if base.stat().st_size != old_size or sha256(base) != old_hash:
                raise ValueError("Base package does not match patch")
            if magic == b"O2PATCH3\0":
                length, = struct.unpack("<I", exact(source, 4))
                if length > 4 * BUFFER:
                    raise ValueError("MSI recipe exceeds limit")
                metadata = exact(source, length)
                with tempfile.TemporaryDirectory(prefix="operit-msi-") as temporary:
                    old, new, restored = [Path(temporary) / name for name in ("old", "new", "restored")]
                    msi.decode(base, old, metadata)
                    with old.open("rb") as expanded:
                        apply_stream(expanded, source, new, True)
                    msi.encode(new, restored)
                    with output.open("xb") as destination, restored.open("rb") as installer:
                        created = True
                        shutil.copyfileobj(installer, destination, BUFFER)
            else:
                apply_stream(old, source, output, magic == MAGIC, (old_size, new_size, old_hash, new_hash))
                created = True
            if output.stat().st_size != new_size or sha256(output) != new_hash:
                raise ValueError("Reconstructed package SHA-256 mismatch")
    except BaseException:
        if created:
            output.unlink(missing_ok=True)
        raise


# Applies bounded residual instructions to original or expanded package bytes.
def apply_stream(old, source, output: Path, residuals: bool, header=None) -> None:
    if header is None:
        old_size, new_size = struct.unpack("<QQ", exact(source, 16))
        old_hash, new_hash = exact(source, 32).hex(), exact(source, 32).hex()
    else:
        old_size, new_size, old_hash, new_hash = header
    old.seek(0, 2)
    if old.tell() != old_size:
        raise ValueError("Expanded base size mismatch")
    old.seek(0)
    if hashlib.file_digest(old, "sha256").hexdigest() != old_hash:
        raise ValueError("Expanded base SHA-256 mismatch")
    created = False
    try:
        with output.open("xb") as destination:
            created = True
            written = 0
            while True:
                operation = exact(source, 1)[0]
                if operation == 0:
                    break
                if operation == 1 or (operation == 3 and residuals):
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
                    if operation == 3:
                        residual = exact(source, len(block))
                        block = bytes((old + diff) & 255 for old, diff in zip(block, residual))
                    destination.write(block)
                    remaining -= len(block)
                written += length
            if written != new_size or source.read(1):
                raise ValueError("Invalid patch end")
        if sha256(output) != new_hash:
            raise ValueError("Reconstructed package SHA-256 mismatch")
    except BaseException:
        if created:
            output.unlink(missing_ok=True)
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
