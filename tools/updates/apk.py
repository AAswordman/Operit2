"""Expose changed DEFLATE entries while preserving every original signed APK byte."""
from __future__ import annotations

import json
from pathlib import Path
import struct
import zipfile
import zlib

MAGIC = b"O2APK1\0\0"
BUFFER = 1024 * 1024
LIMIT = 16 * 1024 ** 3


# Copies exactly one bounded range without retaining a complete APK in memory.
def copy(source, target, length: int) -> None:
    while length:
        block = source.read(min(length, BUFFER))
        if not block:
            raise ValueError("Truncated APK image")
        target.write(block)
        length -= len(block)


# Rejects overlapping entries, oversized images and unrecognized compression parameters.
def validate(recipe: dict) -> None:
    if set(recipe) != {"size", "entries"} or not 0 < recipe["size"] <= LIMIT \
            or len(recipe["entries"]) > 4096:
        raise ValueError("Invalid APK recipe")
    end = total = 0
    for entry in recipe["entries"]:
        if set(entry) != {"offset", "compressed", "size", "level"} \
                or entry["offset"] < end or entry["compressed"] <= 0 \
                or not 0 < entry["size"] <= LIMIT or entry["level"] != 6:
            raise ValueError("Invalid APK entry recipe")
        end = entry["offset"] + entry["compressed"]
        total += entry["size"]
        if end > recipe["size"] or total > LIMIT:
            raise ValueError("APK entry exceeds package bounds")


# Locates and verifies the exact standard-zlib stream emitted by the Android packager.
def entry_recipe(archive: zipfile.ZipFile, source, entry: zipfile.ZipInfo) -> dict:
    source.seek(entry.header_offset)
    header = source.read(30)
    if len(header) != 30 or header[:4] != b"PK\x03\x04" \
            or struct.unpack_from("<H", header, 8)[0] != 8 or entry.flag_bits & 1:
        raise ValueError("Unsupported APK local entry")
    name_size, extra_size = struct.unpack_from("<HH", header, 26)
    offset = entry.header_offset + 30 + name_size + extra_size
    source.seek(offset)
    compressor = zlib.compressobj(6, zlib.DEFLATED, -15)
    with archive.open(entry) as payload:
        while block := payload.read(BUFFER):
            compressed = compressor.compress(block)
            if source.read(len(compressed)) != compressed:
                raise ValueError(f"APK entry is not reproducible with zlib level 6: {entry.filename}")
        compressed = compressor.flush()
        if source.read(len(compressed)) != compressed or source.tell() != offset + entry.compress_size:
            raise ValueError(f"APK DEFLATE layout mismatch: {entry.filename}")
    return {"offset": offset, "compressed": entry.compress_size, "size": entry.file_size, "level": 6}


# Expands only changed large entries; unchanged compressed streams remain in the signed shell.
def recipes(base: Path, target: Path) -> tuple[bytes, bytes]:
    with zipfile.ZipFile(base) as old, zipfile.ZipFile(target) as new:
        old_entries = {entry.filename: entry for entry in old.infolist()}
        new_entries = {entry.filename: entry for entry in new.infolist()}
        if len(old_entries) != len(old.infolist()) or len(new_entries) != len(new.infolist()):
            raise ValueError("APK contains duplicate entry names")
        changed = {name for name in old_entries.keys() | new_entries.keys()
                   if name not in old_entries or name not in new_entries
                   or (old_entries[name].CRC, old_entries[name].file_size)
                   != (new_entries[name].CRC, new_entries[name].file_size)}
        results = []
        for path, archive in ((base, old), (target, new)):
            with path.open("rb") as source:
                entries = [entry_recipe(archive, source, entry)
                           for entry in sorted(archive.infolist(), key=lambda entry: entry.header_offset)
                           if entry.filename in changed and entry.compress_type == 8
                           and entry.file_size >= 65536]
            recipe = {"size": path.stat().st_size, "entries": entries}
            validate(recipe)
            results.append(json.dumps(recipe, separators=(",", ":")).encode())
        return tuple(results)


# Replaces compressed ranges with their raw payloads in an authenticated synthetic image.
def decode(source: Path, destination: Path, metadata: bytes) -> None:
    if len(metadata) > 4 * BUFFER:
        raise ValueError("APK recipe exceeds limit")
    recipe = json.loads(metadata)
    validate(recipe)
    if source.stat().st_size != recipe["size"]:
        raise ValueError("APK recipe size mismatch")
    with source.open("rb") as original, destination.open("xb") as output:
        output.write(MAGIC + struct.pack("<I", len(metadata)) + metadata)
        offset = 0
        for entry in recipe["entries"]:
            original.seek(offset)
            copy(original, output, entry["offset"] - offset)
            offset = entry["offset"] + entry["compressed"]
        original.seek(offset)
        copy(original, output, recipe["size"] - offset)
        for entry in recipe["entries"]:
            original.seek(entry["offset"])
            inflater = zlib.decompressobj(-15)
            remaining, written = entry["compressed"], 0
            while remaining:
                block = original.read(min(remaining, BUFFER))
                if not block:
                    raise ValueError("Truncated APK DEFLATE stream")
                remaining -= len(block)
                while block:
                    payload = inflater.decompress(block, BUFFER)
                    block = inflater.unconsumed_tail
                    written += len(payload)
                    if written > entry["size"]:
                        raise ValueError("Expanded APK entry exceeds recipe")
                    output.write(payload)
            if not inflater.eof or inflater.unused_data or written != entry["size"]:
                raise ValueError("Expanded APK entry mismatch")


# Restores raw DEFLATE streams, local headers, alignment, central directory and signature bytes.
def encode(source: Path, destination: Path) -> None:
    with source.open("rb") as image, destination.open("xb+") as output:
        if image.read(8) != MAGIC:
            raise ValueError("Invalid expanded APK header")
        length, = struct.unpack("<I", image.read(4))
        if length > 4 * BUFFER:
            raise ValueError("APK recipe exceeds limit")
        recipe = json.loads(image.read(length))
        validate(recipe)
        expected = 12 + length + recipe["size"] + sum(e["size"] - e["compressed"] for e in recipe["entries"])
        if source.stat().st_size != expected:
            raise ValueError("Expanded APK length mismatch")
        offset = 0
        for entry in recipe["entries"]:
            output.seek(offset)
            copy(image, output, entry["offset"] - offset)
            offset = entry["offset"] + entry["compressed"]
        output.seek(offset)
        copy(image, output, recipe["size"] - offset)
        output.truncate(recipe["size"])
        for entry in recipe["entries"]:
            output.seek(entry["offset"])
            compressor = zlib.compressobj(entry["level"], zlib.DEFLATED, -15)
            remaining = entry["size"]
            while remaining:
                payload = image.read(min(remaining, BUFFER))
                if not payload:
                    raise ValueError("Truncated expanded APK payload")
                remaining -= len(payload)
                output.write(compressor.compress(payload))
            output.write(compressor.flush())
            if output.tell() != entry["offset"] + entry["compressed"]:
                raise ValueError("APK encoder does not reproduce target layout")
