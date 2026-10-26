#!/usr/bin/env python3
"""Prepare a checksum-pinned ESP32-only ring overlay, never mutate Cargo's cache.

The Xtensa LLVM windowed backend misaddresses incoming stack arguments after
realigning SP beyond the 16-byte ABI. Poly1305's scalar fallback uses 64-byte
padding, propagating this to ChaCha20-Poly1305 seal/open. Keep its Rust/C layouts
consistent at 16 bytes on Xtensa only. Keep seal/open named and non-inlined
on Xtensa for final ELF inspection under LTO. No arithmetic, key or protocol
changes.
"""
from pathlib import Path
import hashlib
import io
import os
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
VERSION = "0.17.14"
SHA256 = "a4689e6c2294d81e88dc6261c768b63bc4fcdb852be6d1352498b114f61383b7"
CRATE = f"ring-{VERSION}"
DEST = ROOT / "apps/esp32/.embuild"
MARKER = "operit-xtensa-poly1305-align-v2"
AEAD_PATH = "src/aead/chacha20_poly1305/mod.rs"
RUST_PATH = "src/aead/poly1305/ffi_fallback.rs"
C_PATH = "crypto/poly1305/poly1305.c"
RUST_OLD = "#[repr(C, align(64))]"
RUST_NEW = '\n'.join([
    '#[repr(C)]',
    '#[cfg_attr(target_arch = "xtensa", repr(align(16)))]',
    '#[cfg_attr(not(target_arch = "xtensa"), repr(align(64)))]',
])
C_DEFINE = '\n'.join([
    '// Scalar fallback only: match the Rust layout and the Xtensa stack ABI.',
    '#if defined(__XTENSA__)',
    '#define OPERIT_POLY1305_ALIGNMENT 16',
    '#else',
    '#define OPERIT_POLY1305_ALIGNMENT 64',
    '#endif',
    '',
])


def patch_sources(rust, c):
    if rust.count(RUST_OLD) != 1 or c.count('alignas(64)') != 1 or c.count('state % 64') != 2:
        raise RuntimeError("Unexpected pinned ring layout; manual review required")
    rust = rust.replace(RUST_OLD, RUST_NEW)
    c = c.replace('struct poly1305_state_st {', C_DEFINE + 'struct poly1305_state_st {', 1)
    c = c.replace('alignas(64)', 'alignas(OPERIT_POLY1305_ALIGNMENT)')
    c = c.replace('state % 64', 'state % OPERIT_POLY1305_ALIGNMENT')
    return rust, c


def patch_check_boundaries(text):
    for method in ("seal", "open"):
        needle = f"pub(super) fn {method}("
        export = f"operit_xtensa_ring_chacha20_poly1305_{method}"
        if text.count(needle) != 1 or export in text:
            raise RuntimeError("Unexpected pinned ring AEAD boundary; manual review required")
        text = text.replace(needle, '\n'.join([
            '#[cfg_attr(target_arch = "xtensa", inline(never))]',
            f'#[cfg_attr(target_arch = "xtensa", export_name = "{export}")]',
            needle,
        ]), 1)
    return text


def pinned_archive():
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    cached = list((cargo_home / "registry/cache").glob(f"*/{CRATE}.crate"))
    if cached:
        data = cached[0].read_bytes()
    else:
        with urllib.request.urlopen(f"https://static.crates.io/crates/ring/{CRATE}.crate", timeout=60) as response:
            data = response.read()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise RuntimeError("ring archive checksum mismatch; refusing to patch")
    return data


def expected_sources(data):
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        rust = archive.extractfile(f"{CRATE}/{RUST_PATH}").read().decode("utf-8")
        c = archive.extractfile(f"{CRATE}/{C_PATH}").read().decode("utf-8")
        aead = archive.extractfile(f"{CRATE}/{AEAD_PATH}").read().decode("utf-8")
    return (*patch_sources(rust, c), patch_check_boundaries(aead))


def prepare():
    data = pinned_archive()
    rust, c, aead = expected_sources(data)
    crate = DEST / CRATE
    marker = crate / MARKER
    if not marker.exists():
        DEST.mkdir(parents=True, exist_ok=True)
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
            for member in archive.getmembers():
                target = (DEST / member.name).resolve()
                if not target.is_relative_to(crate.resolve()) or not (member.isfile() or member.isdir()):
                    raise RuntimeError(f"Unsafe archive entry: {member.name}")
            archive.extractall(DEST, filter="data")
        (crate / RUST_PATH).write_text(rust, encoding="utf-8")
        (crate / C_PATH).write_text(c, encoding="utf-8")
        (crate / AEAD_PATH).write_text(aead, encoding="utf-8")
        marker.write_text(SHA256 + "\n", encoding="utf-8")
    # Fail closed on an edited/incomplete overlay, including mismatched C/Rust.
    if marker.read_text(encoding="utf-8").strip() != SHA256 or \
            (crate / RUST_PATH).read_text(encoding="utf-8") != rust or \
            (crate / C_PATH).read_text(encoding="utf-8") != c or \
            (crate / AEAD_PATH).read_text(encoding="utf-8") != aead:
        raise RuntimeError("ring overlay differs from the pinned Xtensa patch")
    print(f"ESP32 ring {VERSION}: verified pinned source, matching Xtensa Rust/C alignment and ABI inspection boundaries")


if __name__ == "__main__":
    prepare()
