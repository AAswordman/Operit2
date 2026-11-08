#!/usr/bin/env python3
"""Safety checks for build-time profile superblock transformations."""
import importlib.util
from pathlib import Path
import unittest


path = Path(__file__).resolve().parents[1] / 'tools/generate_profile_superblocks.py'
spec = importlib.util.spec_from_file_location('profile_superblocks', path)
compiler = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compiler)


class SpecializationTests(unittest.TestCase):
    op = {'a': 320, 'b': 328, 'd': 336, 'immediate': '0x2a'}

    def test_field_guards_and_direct_access(self):
        body = ('    ldrh w12, [x28, #OP_A]\n    ldr x1, [x27, x12]\n'
                '    ldrh w12, [x28, #OP_B]\n    str x2, [x27, x12]\n    ADVANCE')
        output, guards = compiler.specialize(body, self.op)
        self.assertEqual(dict(guards), {'a': 320, 'b': 328})
        self.assertIn('ldr x1, [x27, #320]', output)
        self.assertIn('str x2, [x27, #328]', output)
        self.assertNotIn('OP_D', output)

    def test_stored_scratch_is_not_discarded(self):
        body = '    ldrh w12, [x28, #OP_A]\n    str x12, [x27, x12]\n    ADVANCE'
        output, guards = compiler.specialize(body, self.op)
        self.assertEqual(dict(guards), {'a': 320})
        self.assertIn('mov w12, #320\n    str x12, [x27, x12]', output)

    def test_scratch_live_and_branch_keep_assignment(self):
        for tail in ('    add x3, x12, x1', '    b label'):
            body = f'    ldrh w12, [x28, #OP_A]\n    ldr x1, [x27, x12]\n{tail}\n    ADVANCE'
            output, _ = compiler.specialize(body, self.op)
            self.assertIn('mov w12, #320', output)

    def test_immediate_and_live_scratch(self):
        body = '    ldr x12, [x28, #OP_IMMEDIATE]\n    add x3, x4, x12\n    ADVANCE'
        output, guards = compiler.specialize(body, self.op)
        self.assertEqual(dict(guards), {'immediate': 42})
        self.assertIn('add x3, x4, #42', output)
        for add in ('add x12, x4, x12', 'add x3, x12, x12'):
            output, _ = compiler.specialize(body.replace('add x3, x4, x12', add), self.op)
            self.assertIn('mov x12, #42', output)


class PromotionTests(unittest.TestCase):
    def promote(self, body, name='Add'):
        return compiler.promote_shadow([body], [name])

    def test_load_write_read_and_flush(self):
        body = ('    ldr x1, [x27, #320]\n    str x2, [x27, #320]\n'
                '    ldr w3, [x27, #320]\n    ADVANCE')
        result, promoted = self.promote(body)
        self.assertTrue(promoted)
        self.assertIn('ldr x14, [x27, #320]\n    mov x1, x14', result[0])
        self.assertIn('mov x14, x2', result[0])
        self.assertIn('mov w3, w14', result[0])
        self.assertTrue(result[0].endswith('str x14, [x27, #320]\n    ADVANCE'))

    def test_write_before_read_does_not_load(self):
        body = '    str x2, [x27, #320]\n    ldr x1, [x27, #320]\n    ADVANCE'
        result, promoted = self.promote(body)
        self.assertTrue(promoted)
        self.assertNotIn('ldr x14', result[0])

    def test_reject_unsafe_accesses_and_control_flow(self):
        safe = '    ldr x1, [x27, #320]\n    str x2, [x27, #320]\n'
        for extra in ('    bl helper\n', '    cbz x0, label\n', '    mov x1, x14\n',
                      '    ldr x3, [x27, x12]\n', '    ldr w3, [x27, #324]\n',
                      '    ldp x3, x4, [x27, #320]\n', '    add x3, x27, #320\n',
                      '    str w3, [x27, #320]\n'):
            with self.subTest(extra=extra):
                self.assertFalse(self.promote(safe + extra + '    ADVANCE')[1])
        self.assertFalse(self.promote(safe + '    ADVANCE', 'Load')[1])


if __name__ == '__main__':
    unittest.main()
