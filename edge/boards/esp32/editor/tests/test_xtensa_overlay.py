import importlib.util
import hashlib
import io
import os
import tarfile
import tempfile
from unittest.mock import patch
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

ring = load('prepare_ring', 'prepare-ring.py')
tokio = load('prepare_tokio', 'prepare-tokio.py')
abi = load('check_xtensa', 'check-xtensa-tokio.py')

def archive(crate, files):
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode='w:gz') as package:
        for name, text in files.items():
            data = text.encode('utf-8')
            entry = tarfile.TarInfo(f'{crate}/{name}')
            entry.size = len(data)
            package.addfile(entry, io.BytesIO(data))
    return output.getvalue()


class XtensaOverlayTests(unittest.TestCase):
    def test_tokio_prepares_a_clean_offline_cache_and_can_verify_it_again(self):
        align = '#[cfg_attr(\n    not(any(\n        target_arch = "x86_64",\n    )), repr(align(64)))]\n'
        files = {
            'src/runtime/metrics/worker.rs': '#[repr(align(128))]\nstruct WorkerMetrics;\n',
            'src/runtime/io/scheduled_io.rs': align + 'pub(crate) struct ScheduledIo;\n',
            'src/runtime/task/core.rs': align + 'pub(super) struct Cell<T: Future, S>;\n',
            'src/util/cacheline.rs': align + 'pub(crate) struct CachePadded<T>;\n',
            tokio.CHECK_PATH: 'impl CurrentThread {\n    pub(crate) fn new() {}\n}\n',
        }
        data = archive(tokio.CRATE, files)
        digest = hashlib.sha256(data).hexdigest()
        with tempfile.TemporaryDirectory(prefix='operit-tokio-overlay-test-') as directory:
            root = Path(directory).resolve()
            cargo = root / 'cargo'
            cached = cargo / 'registry/cache/fixture' / f'{tokio.CRATE}.crate'
            cached.parent.mkdir(parents=True)
            cached.write_bytes(data)
            destination = root / 'prepared'
            with patch.object(tokio, 'DEST', destination), patch.object(tokio, 'SHA256', digest), patch.dict(os.environ, {'CARGO_HOME': str(cargo)}):
                tokio.prepare()
                self.assertEqual((destination / tokio.CRATE / tokio.PATCH_MARKER).read_text().strip(), digest)
                tokio.prepare()

    def test_ring_refuses_a_modified_lto_inspection_boundary_in_its_cache(self):
        data = archive(ring.CRATE, {
            ring.RUST_PATH: '#[repr(C, align(64))]\nstruct Poly1305;\n',
            ring.C_PATH: 'struct poly1305_state_st { alignas(64) int x; };\n' + 'state % 64 == 0;\n' * 2,
            ring.AEAD_PATH: 'pub(super) fn seal() {}\npub(super) fn open() {}\n',
        })
        with tempfile.TemporaryDirectory(prefix='operit-ring-overlay-test-') as directory:
            destination = Path(directory).resolve()
            with patch.object(ring, 'DEST', destination), patch.object(ring, 'pinned_archive', return_value=data):
                ring.prepare()
                ring.prepare()
                boundary = destination / ring.CRATE / ring.AEAD_PATH
                boundary.write_text(boundary.read_text().replace('inline(never)', 'inline(always)'))
                with self.assertRaisesRegex(RuntimeError, 'differs from the pinned'):
                    ring.prepare()

    def test_patch_keeps_c_and_rust_layouts_and_assertions_consistent(self):
        rust = '#[repr(C, align(64))]\nstruct poly1305_state_st { r0: u32 }\n'
        c = 'struct poly1305_state_st { alignas(64) uint32_t r0; };\n' + 'state % 64 == 0;\n' * 2
        patched_rust, patched_c = ring.patch_sources(rust, c)
        self.assertIn('cfg_attr(target_arch = "xtensa", repr(align(16)))', patched_rust)
        self.assertIn('cfg_attr(not(target_arch = "xtensa"), repr(align(64)))', patched_rust)
        self.assertIn('#if defined(__XTENSA__)\n#define OPERIT_POLY1305_ALIGNMENT 16', patched_c)
        self.assertIn('#else\n#define OPERIT_POLY1305_ALIGNMENT 64', patched_c)
        self.assertEqual(patched_c.count('state % OPERIT_POLY1305_ALIGNMENT'), 2)
        self.assertIn('alignas(OPERIT_POLY1305_ALIGNMENT)', patched_c)

    def test_patch_refuses_changed_or_already_patched_layout(self):
        with self.assertRaises(RuntimeError):
            ring.patch_sources('#[repr(C, align(32))]', '')
        with self.assertRaises(RuntimeError):
            ring.patch_sources(ring.RUST_NEW, ring.C_DEFINE)

    def test_checker_rejects_actual_overaligned_seal_prologue(self):
        asm = 'entry a1, 0x2a0\nmov.n a8, 63\nand a8, a1, a8\nsub a8, a9, a8\nadd.n a1, a1, a8\nl32i a3, a1, 0x2a0\ncallx8 a8'
        with self.assertRaisesRegex(RuntimeError, 'Unsafe SP realignment'):
            abi.verify_prologue(asm, 'seal', {0})

    def test_checker_accepts_fixed_frames_and_rejects_wrong_offsets(self):
        asm = 'entry a1, 0x140\nl32i a2, a1, 0x140\nl32i a3, a1, 0x144\nl32i.n a4, a1, 328\ncallx8 a8'
        self.assertEqual(abi.verify_prologue(asm, 'new', {0, 4, 8}), 0x140)
        with self.assertRaisesRegex(RuntimeError, 'incoming stack argument'):
            abi.verify_prologue(asm, 'new', {0, 4, 12})

if __name__ == '__main__':
    unittest.main()
