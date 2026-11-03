"""Run cache/JIT ELF fixtures assembled from encodings, without a guest compiler."""
import argparse
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest

parser = argparse.ArgumentParser()
parser.add_argument('--runtime', required=True)
OPTIONS, EXTRA = parser.parse_known_args()
BASE = 0x400000
CODE_OFFSET = 4096


class Program:
    """Resolve the few PC-relative forms used by these small ELF fixtures."""
    def __init__(self):
        self.data = bytearray()
        self.labels = {}
        self.fixups = []

    def word(self, word):
        self.data.extend(struct.pack('<I', word))

    def label(self, name):
        self.labels[name] = len(self.data)

    def relative(self, kind, label, register=0, addend=0):
        self.fixups.append((len(self.data), kind, label, register, addend))
        self.word(0)

    def finish(self):
        for offset, kind, label, register, addend in self.fixups:
            delta = self.labels[label] + addend - offset
            if kind == 'adr':
                assert -(1 << 20) <= delta < (1 << 20)
                word = 0x10000000 | ((delta & 3) << 29) | (((delta >> 2) & 0x7ffff) << 5) | register
            else:
                assert delta % 4 == 0
                immediate = delta // 4
                if kind == 'bl':
                    assert -(1 << 25) <= immediate < (1 << 25)
                    word = 0x94000000 | (immediate & 0x3ffffff)
                elif kind == 'bne':
                    assert -(1 << 18) <= immediate < (1 << 18)
                    word = 0x54000001 | ((immediate & 0x7ffff) << 5)
                elif kind == 'ldr32':
                    assert -(1 << 18) <= immediate < (1 << 18)
                    word = 0x18000000 | ((immediate & 0x7ffff) << 5) | register
                else:
                    raise AssertionError(kind)
            struct.pack_into('<I', self.data, offset, word)
        return bytes(self.data)

    def exit(self, code=0):
        self.word(0xd2800000 | (code << 5))  # MOV X0, #code.
        self.word(0xd2800ba8)  # MOV X8, #93 (exit).
        self.word(0xd4000001)  # SVC #0.


def elf(code):
    """One RWX load segment, allowing guest generated-code tests in a fresh VM."""
    size = CODE_OFFSET + len(code)
    header = struct.pack('<16sHHIQQQIHHHHHH', b'\x7fELF\x02\x01\x01' + b'\0' * 9,
                         2, 183, 1, BASE + CODE_OFFSET, 64, 0, 0, 64, 56, 1, 0, 0, 0)
    segment = struct.pack('<IIQQQQQQ', 1, 7, 0, BASE, BASE, size, size, 4096)
    return (header + segment).ljust(CODE_OFFSET, b'\0') + code


class CacheTests(unittest.TestCase):
    def run_program(self, program, expected=b''):
        with tempfile.TemporaryDirectory(prefix='arm64-cache-') as directory:
            root = Path(directory)
            (root / 'cache-test').write_bytes(elf(program.finish()))
            (root / 'cache-test').chmod(0o755)
            for extra in ([], ['--fixed-acceleration']):
                result = subprocess.run([OPTIONS.runtime, '--root', str(root), '--stats',
                                         *extra, '--', '/cache-test'],
                                        capture_output=True, timeout=10)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertEqual(result.stdout, expected)
                stats = json.loads(result.stderr.decode().splitlines()[-1])
                self.assertEqual(stats['unsupported_syscalls'], 0)

    def test_cache_registers_describe_guest_geometry(self):
        program = Program()
        program.relative('adr', 'result', 1)
        program.word(0xd53b0023)  # MRS X3, CTR_EL0.
        program.word(0xf9000023)  # STR X3, [X1].
        program.word(0xd53b00e3)  # MRS X3, DCZID_EL0.
        program.word(0xf9000423)  # STR X3, [X1, #8].
        program.word(0xd2800020)  # MOV X0, #1.
        program.word(0xd2800202)  # MOV X2, #16.
        program.word(0xd2800808)  # MOV X8, #64 (write).
        program.word(0xd4000001)
        program.exit()
        program.label('result')
        program.data.extend(b'\0' * 16)
        self.run_program(program, struct.pack('<QQ', 0x8004c004, 4))

    def test_clear_cache_then_execute_modified_code(self):
        program = Program()
        program.relative('adr', 'target', 20)
        program.relative('bl', 'target')
        program.word(0xf1001c1f)  # CMP X0, #7.
        program.relative('bne', 'failure')
        program.word(0xd53b0023)  # V8/compiler-rt cache geometry probe.
        program.relative('ldr32', 'replacement', 1)
        program.word(0xb9000281)  # STR W1, [X20].
        program.word(0xd50b7b34)  # DC CVAU, X20.
        program.word(0xd5033b9f)  # DSB ISH.
        program.word(0xd50b7534)  # IC IVAU, X20.
        program.word(0xd5033b9f)
        program.word(0xd5033fdf)  # ISB.
        program.relative('bl', 'target')
        program.word(0xf1002c1f)  # CMP X0, #11.
        program.relative('bne', 'failure')
        program.exit()
        program.label('failure')
        program.exit(42)
        program.label('target')
        program.word(0xd28000e0)  # MOV X0, #7.
        program.word(0xd65f03c0)  # RET.
        program.label('replacement')
        program.word(0xd2800160)  # MOV X0, #11.
        self.run_program(program)

    def test_dc_zva_zeros_only_its_aligned_block(self):
        program = Program()
        program.relative('adr', 'buffer', 0, 81)
        program.word(0xd50b7420)  # DC ZVA, X0 (unaligned operand).
        program.relative('adr', 'buffer', 1)
        program.word(0xd2800020)  # MOV X0, #1.
        program.word(0xd2801802)  # MOV X2, #192.
        program.word(0xd2800808)
        program.word(0xd4000001)
        program.exit()
        program.data.extend(b'\0' * (-len(program.data) % 64))
        program.label('buffer')
        program.data.extend(b'\xa5' * 192)
        self.run_program(program, b'\xa5' * 64 + b'\0' * 64 + b'\xa5' * 64)


if __name__ == '__main__':
    unittest.main(argv=[__file__, *EXTRA], verbosity=2)
