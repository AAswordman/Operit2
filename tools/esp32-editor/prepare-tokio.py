#!/usr/bin/env python3
"""Prepare the pinned ESP32-only Tokio overlay without changing Cargo's cache.

The esp LLVM 21 windowed backend realigns SP then addresses incoming stack
arguments relative to the shifted SP. Tokio's 128-byte WorkerMetrics and
64-byte CachePadded trigger this. On Xtensa only, cap these performance-only
padding alignments at the 16-byte ABI alignment. Keep the constructor named
and non-inlined on Xtensa for the final ELF check even with LTO enabled.
No scheduler/protocol changes.
"""
from pathlib import Path
import hashlib
import io
import os
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
VERSION = "1.53.1"
SHA256 = "202caea871b69668250d242070849eb495be178ed697a3e98aebce5bc81a0bed"
DEST = ROOT / "apps/esp32/.embuild"
CRATE = f"tokio-{VERSION}"
PATCH_MARKER = "operit-xtensa-align-v2"
CHECK_PATH = "src/runtime/scheduler/current_thread/mod.rs"
CHECK_EXPORT = "operit_xtensa_tokio_current_thread_new"


def patch_check_boundary(text):
    needle = "impl CurrentThread {\n    pub(crate) fn new("
    if text.count(needle) != 1 or CHECK_EXPORT in text:
        raise RuntimeError("Unexpected pinned Tokio constructor; manual review required")
    return text.replace(needle, '\n'.join([
        "impl CurrentThread {",
        '    #[cfg_attr(target_arch = "xtensa", inline(never))]',
        f'    #[cfg_attr(target_arch = "xtensa", export_name = "{CHECK_EXPORT}")]',
        "    pub(crate) fn new(",
    ]), 1)


def prepare():
    marker = DEST / CRATE / PATCH_MARKER
    if marker.exists():
        verify()
        return
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    archives = list((cargo_home / "registry/cache").glob(f"*/{CRATE}.crate"))
    if archives:
        data = archives[0].read_bytes()
    else:
        with urllib.request.urlopen(f"https://static.crates.io/crates/tokio/{CRATE}.crate", timeout=60) as response:
            data = response.read()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise RuntimeError("Tokio archive checksum mismatch; refusing to patch")
    DEST.mkdir(parents=True, exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        # The pinned archive is still treated as untrusted input.
        for member in archive.getmembers():
            target = (DEST / member.name).resolve()
            if not target.is_relative_to((DEST / CRATE).resolve()) or not (member.isfile() or member.isdir()):
                raise RuntimeError(f"Unsafe archive entry: {member.name}")
        archive.extractall(DEST, filter="data")
    worker = DEST / CRATE / "src/runtime/metrics/worker.rs"
    text = worker.read_text(encoding="utf-8")
    assert text.count("#[repr(align(128))]") == 1
    text = text.replace("#[repr(align(128))]", '#[cfg_attr(not(target_arch = "xtensa"), repr(align(128)))]\n#[cfg_attr(target_arch = "xtensa", repr(align(16)))]')
    worker.write_text(text, encoding="utf-8")
    for relative, struct_marker in [
        ("src/runtime/io/scheduled_io.rs", "pub(crate) struct ScheduledIo"),
        ("src/runtime/task/core.rs", "pub(super) struct Cell<T: Future, S>"),
    ]:
        path = DEST / CRATE / relative
        text = path.read_text(encoding="utf-8")
        fallback = text.rfind("not(any(")
        if 'target_arch = "xtensa"' not in text[fallback:]:
            text = text[:fallback] + text[fallback:].replace(
                '        target_arch = "x86_64",',
                '        target_arch = "xtensa",\n        target_arch = "x86_64",', 1)
        if '#[cfg_attr(target_arch = "xtensa", repr(align(16)))]' not in text:
            text = text.replace(struct_marker, '#[cfg_attr(target_arch = "xtensa", repr(align(16)))]\n' + struct_marker, 1)
        path.write_text(text, encoding="utf-8")

    cache = DEST / CRATE / "src/util/cacheline.rs"
    text = cache.read_text(encoding="utf-8")
    needle = '    not(any(\n        target_arch = "x86_64",'
    assert text.count(needle) == 1
    text = text.replace(needle, '    not(any(\n        target_arch = "xtensa",\n        target_arch = "x86_64",')
    text = text.replace("pub(crate) struct CachePadded<T>", '#[cfg_attr(target_arch = "xtensa", repr(align(16)))]\npub(crate) struct CachePadded<T>')
    cache.write_text(text, encoding="utf-8")
    constructor = DEST / CRATE / CHECK_PATH
    constructor.write_text(patch_check_boundary(constructor.read_text(encoding="utf-8")), encoding="utf-8")
    marker.write_text(SHA256 + "\n", encoding="utf-8")
    verify()


def verify():
    for name in ["runtime/metrics/worker.rs", "util/cacheline.rs", "runtime/io/scheduled_io.rs", "runtime/task/core.rs"]:
        text = (DEST / CRATE / "src" / name).read_text(encoding="utf-8")
        if '#[cfg_attr(target_arch = "xtensa", repr(align(16)))]' not in text:
            raise RuntimeError(f"Missing Xtensa alignment workaround in {name}")
    constructor = (DEST / CRATE / CHECK_PATH).read_text(encoding="utf-8")
    if f'export_name = "{CHECK_EXPORT}"' not in constructor or '#[cfg_attr(target_arch = "xtensa", inline(never))]' not in constructor:
        raise RuntimeError("Missing Tokio ABI inspection boundary")
    print(f"ESP32 Tokio {VERSION}: verified pinned source, Xtensa padding and ABI inspection boundary")


if __name__ == "__main__":
    prepare()
