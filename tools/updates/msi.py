"""Losslessly expose embedded CAB payloads to the package delta matcher on Windows."""
from __future__ import annotations

import json
from pathlib import Path
import struct
import subprocess
import tempfile

MAGIC = b"O2MSI1\0\0"
BUFFER = 1024 * 1024
LIMIT = 16 * 1024 ** 3


# Copies an exact bounded range without loading installer payloads into memory.
def copy(source, target, length: int) -> None:
    while length:
        block = source.read(min(length, BUFFER))
        if not block:
            raise ValueError("Truncated MSI data")
        target.write(block)
        length -= len(block)


# Reads the regular FAT streams while retaining their original physical sector positions.
class Compound:
    # Validates the compound header and gathers FAT and directory sectors.
    def __init__(self, path: Path):
        self.file = path.open("rb")
        self.size = path.stat().st_size
        header = self.read(0, 512)
        if header[:8] != bytes.fromhex("d0cf11e0a1b11ae1"):
            raise ValueError("Invalid MSI compound header")
        shift = struct.unpack_from("<H", header, 30)[0]
        if shift not in (9, 12):
            raise ValueError("Unsupported MSI sector size")
        self.sector = 1 << shift
        fats = list(struct.unpack_from("<109I", header, 76))
        next_sector, count = struct.unpack_from("<II", header, 68)
        if count > self.size // self.sector:
            raise ValueError("Invalid MSI DIFAT length")
        for _ in range(count):
            values = struct.unpack(f"<{self.sector // 4}I", self.read((next_sector + 1) * self.sector, self.sector))
            fats.extend(values[:-1])
            next_sector = values[-1]
        self.fat = []
        for sector in fats:
            if sector < 0xfffffffa:
                self.fat.extend(struct.unpack(f"<{self.sector // 4}I", self.read((sector + 1) * self.sector, self.sector)))
        self.directory = b"".join(self.read((sector + 1) * self.sector, self.sector)
                                  for sector in self.chain(struct.unpack_from("<I", header, 48)[0]))

    # Rejects invalid physical offsets before accessing the source package.
    def read(self, offset: int, length: int) -> bytes:
        if offset < 0 or length < 0 or offset + length > self.size:
            raise ValueError("MSI sector exceeds package")
        self.file.seek(offset)
        data = self.file.read(length)
        if len(data) != length:
            raise ValueError("Truncated MSI sector")
        return data

    # Rejects cyclic or out-of-range FAT chains.
    def chain(self, first: int) -> list[int]:
        result, seen = [], set()
        while first != 0xfffffffe:
            if first >= len(self.fat) or first in seen or first >= (self.size - 1) // self.sector:
                raise ValueError("Invalid MSI sector chain")
            seen.add(first)
            result.append(first)
            first = self.fat[first]
        return result

    # Maps a CAB stream range to coalesced physical ranges in the original MSI.
    def ranges(self, chain: list[int], offset: int, length: int) -> list[list[int]]:
        result = []
        while length:
            index, within = divmod(offset, self.sector)
            if index >= len(chain):
                raise ValueError("Truncated MSI stream")
            physical = (chain[index] + 1) * self.sector + within
            count = min(length, self.sector - within)
            if result and sum(result[-1]) == physical:
                result[-1][1] += count
            else:
                result.append([physical, count])
            offset += count
            length -= count
        return result

    # Reads only the small logical headers needed to discover compression folders.
    def stream(self, chain: list[int], offset: int, length: int) -> bytes:
        return b"".join(self.read(start, count) for start, count in self.ranges(chain, offset, length))

    # Discovers independent CAB compression folders, including WiX deduplicated payloads.
    def folders(self) -> list[dict]:
        folders = []
        for position in range(0, len(self.directory), 128):
            entry = self.directory[position:position + 128]
            size = struct.unpack_from("<Q", entry, 120)[0]
            if entry[66] != 2 or size < 4096:
                continue
            chain = self.chain(struct.unpack_from("<I", entry, 116)[0])
            if self.read((chain[0] + 1) * self.sector, 4) != b"MSCF":
                continue
            data = self.stream(chain, 0, 36)
            if data[24:26] != b"\x03\x01" or struct.unpack_from("<H", data, 30)[0] != 0:
                raise ValueError("Unsupported spanning or reserved CAB header")
            for index in range(struct.unpack_from("<H", data, 26)[0]):
                start, blocks, compression = struct.unpack("<IHH", self.stream(chain, 36 + index * 8, 8))
                offset, raw_size = start, 0
                if not blocks or not 15 <= compression >> 8 <= 21 or compression & 255 != 3:
                    raise ValueError("MSI delta requires LZX CAB folders")
                for _ in range(blocks):
                    compressed, raw = struct.unpack("<HH", self.stream(chain, offset + 4, 4))
                    if not compressed or not 0 < raw <= 32768:
                        raise ValueError("Invalid CAB data block")
                    offset += compressed + 8
                    raw_size += raw
                    if offset > size:
                        raise ValueError("CAB data exceeds stream")
                folders.append({"compression": compression, "rawSize": raw_size,
                                "ranges": self.ranges(chain, start, offset - start)})
        if not folders:
            raise ValueError("MSI contains no supported CAB payload")
        return folders


# Runs built-in cabinet tools without showing a console or retaining their progress output.
def cabinet(command: list[str], work: Path) -> None:
    subprocess.run(command, cwd=work, check=True, stdout=subprocess.DEVNULL,
                   creationflags=subprocess.CREATE_NO_WINDOW)


# Validates all recipe ranges before reading or writing a reconstructed installer.
def ranges(recipe: dict) -> list[list[int]]:
    if not 0 < recipe["size"] <= LIMIT or not 0 < len(recipe["folders"]) <= 4096:
        raise ValueError("Invalid MSI recipe size")
    result = sorted(item for folder in recipe["folders"] for item in folder["ranges"])
    end = 0
    for offset, count in result:
        if offset < end or count <= 0 or offset + count > recipe["size"]:
            raise ValueError("Invalid MSI recipe ranges")
        end = offset + count
    if not 0 < sum(folder["rawSize"] for folder in recipe["folders"]) <= LIMIT:
        raise ValueError("Invalid MSI expanded size")
    return result


# Writes a synthetic one-file cabinet without changing its original compressed data blocks.
def unpack(source, folder: dict, work: Path) -> Path:
    length = sum(count for _, count in folder["ranges"])
    blocks = (folder["rawSize"] + 32767) // 32768
    with (work / "input.cab").open("wb") as output:
        output.write(struct.pack("<4sIIIII BBHHHHH", b"MSCF", 0, length + 68, 0, 44, 0, 3, 1, 1, 1, 0, 0, 0))
        output.write(struct.pack("<IHHIIHHHH", 68, blocks, folder["compression"], folder["rawSize"], 0, 0, 0, 0, 0))
        output.write(b"payload\0")
        for offset, count in folder["ranges"]:
            source.seek(offset)
            copy(source, output, count)
    payload = work / "payload"
    payload.unlink(missing_ok=True)
    cabinet(["expand.exe", "-r", "-F:payload", str(work / "input.cab"), str(work)], work)
    if payload.stat().st_size != folder["rawSize"]:
        raise ValueError("Expanded CAB size mismatch")
    return payload


# Removes compressed folders from the shell and appends their uncompressed byte streams.
def decode(source: Path, destination: Path, metadata: bytes | None = None) -> None:
    if metadata is None:
        compound = Compound(source)
        try:
            recipe = {"size": compound.size, "folders": compound.folders()}
            metadata = json.dumps(recipe, separators=(",", ":")).encode()
        finally:
            compound.file.close()
    if len(metadata) > 4 * BUFFER:
        raise ValueError("MSI recipe exceeds limit")
    recipe = json.loads(metadata)
    removed = ranges(recipe)
    if source.stat().st_size != recipe["size"]:
        raise ValueError("MSI recipe does not match original size")
    with source.open("rb") as original, tempfile.TemporaryDirectory(prefix="operit-cab-") as temporary, \
            destination.open("xb") as output:
        work = Path(temporary)
        output.write(MAGIC + struct.pack("<I", len(metadata)) + metadata)
        offset = 0
        for start, length in removed:
            original.seek(offset)
            copy(original, output, start - offset)
            offset = start + length
        original.seek(offset)
        copy(original, output, recipe["size"] - offset)
        for folder in recipe["folders"]:
            with unpack(original, folder, work).open("rb") as payload:
                copy(payload, output, folder["rawSize"])


# Recreates the original physical MSI bytes with the same Windows LZX cabinet encoder.
def encode(source: Path, destination: Path) -> None:
    with source.open("rb") as image, tempfile.TemporaryDirectory(prefix="operit-cab-") as temporary:
        if image.read(8) != MAGIC:
            raise ValueError("Invalid expanded MSI header")
        length, = struct.unpack("<I", image.read(4))
        if length > 4 * BUFFER:
            raise ValueError("MSI recipe exceeds limit")
        recipe = json.loads(image.read(length))
        removed, work = ranges(recipe), Path(temporary)
        expected = 12 + length + recipe["size"] - sum(count for _, count in removed) + sum(f["rawSize"] for f in recipe["folders"])
        if source.stat().st_size != expected:
            raise ValueError("Expanded MSI length mismatch")
        with destination.open("xb") as output:
            offset = 0
            for start, count in removed:
                output.seek(offset)
                copy(image, output, start - offset)
                offset = start + count
            output.seek(offset)
            copy(image, output, recipe["size"] - offset)
            output.truncate(recipe["size"])
            for folder in recipe["folders"]:
                with (work / "payload").open("wb") as payload:
                    copy(image, payload, folder["rawSize"])
                cabinet(["makecab.exe", "/D", "CompressionType=LZX", "/D", f"CompressionMemory={folder['compression'] >> 8}",
                         "/D", "FolderSizeThreshold=1000000000", "/D", "MaxDiskSize=0", "/D", "MaxCabinetSize=0",
                         str(work / "payload"), str(work / "output.cab")], work)
                with (work / "output.cab").open("rb") as cab:
                    cab.seek(36)
                    start, blocks, compression = struct.unpack("<IHH", cab.read(8))
                    if compression != folder["compression"] or blocks != (folder["rawSize"] + 32767) // 32768 \
                            or (work / "output.cab").stat().st_size - start != sum(count for _, count in folder["ranges"]):
                        raise ValueError("Windows CAB encoder does not reproduce target layout")
                    cab.seek(start)
                    for offset, count in folder["ranges"]:
                        output.seek(offset)
                        copy(cab, output, count)
