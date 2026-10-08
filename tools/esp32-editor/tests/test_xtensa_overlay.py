import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

ring = load('prepare_ring', 'prepare-ring.py')
abi = load('check_xtensa', 'check-xtensa-tokio.py')

class XtensaOverlayTests(unittest.TestCase):
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
