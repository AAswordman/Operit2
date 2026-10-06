#!/usr/bin/env python3
"""Reject known bad Xtensa Tokio/AEAD prologues before flashing.
This is machine-code verification, NOT a replacement for hardware pairing tests.
"""
from pathlib import Path
import argparse
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def check(elf: Path):
    bins = list((ROOT / 'apps/esp32/.embuild/espressif/tools').glob('xtensa-esp-elf/*/xtensa-esp-elf/bin'))
    if not bins:
        raise RuntimeError('ESP Xtensa binutils not found')
    suffix = '.exe' if __import__('os').name == 'nt' else ''
    def run(tool, *args):
        return subprocess.check_output([str(bins[-1] / ('xtensa-esp32-elf-' + tool + suffix)), *map(str, args)], text=True, encoding='utf-8')
    symbols = run('nm', '-C', elf).splitlines()
    required = [
        ('<tokio::runtime::scheduler::current_thread::CurrentThread>::new', {0, 4, 8}),
        ('ring::aead::chacha20_poly1305::seal', {0}),
        # open receives its Overlapping descriptor by pointer; all live arguments
        # fit in registers. Its over-aligned frame must still be rejected.
        ('ring::aead::chacha20_poly1305::open', set()),
    ]
    # Xtensa property tables can label LLVM code as data; remove only from a
    # disposable inspection copy. The build/flashed ELF remains unchanged.
    with tempfile.TemporaryDirectory(prefix='operit-elf-check-') as directory:
        copy = Path(directory) / 'inspect.elf'
        run('objcopy', '--remove-section=.xt.prop', '--remove-section=.xt.lit', elf, copy)
        for name, argument_offsets in required:
            symbol = next((line for line in symbols if line.endswith(name)), None)
            if not symbol:
                raise RuntimeError(f'{name} symbol missing; manual disassembly review required')
            address = int(symbol.split()[0], 16)
            asm = run('objdump', '-d', '-C', f'--start-address={address}', f'--stop-address={address + 1536}', copy)
            # Do not accidentally inspect the adjacent function's prologue.
            asm = re.split(r'\n[0-9a-f]+ <', asm, maxsplit=2)[1]
            frame = verify_prologue(asm, name, argument_offsets)
            print(f'PASS: {name} at 0x{address:x}: fixed 0x{frame:x}-byte frame, no SP realignment, incoming stack arguments verified' if argument_offsets else f'PASS: {name} at 0x{address:x}: fixed 0x{frame:x}-byte frame, no SP realignment')


def verify_prologue(asm, name, argument_offsets):
    prologue = re.split(r'\bcall(?:x)?(?:4|8|12)?\b', asm, maxsplit=1)[0]
    if re.search(r'\b(?:add(?:i)?(?:\.n)?|sub|mov(?:\.n)?|movsp|and)\s+a1\s*,', prologue):
        raise RuntimeError(f'Unsafe SP realignment before stack argument reads in {name}\n' + asm)
    entry = re.search(r'\bentry\s+a1,\s*(0x[0-9a-f]+|[0-9]+)', prologue)
    if not entry:
        raise RuntimeError(f'Cannot verify {name} stack frame\n' + asm)
    frame = int(entry[1], 0)
    offsets = {int(offset, 0) for offset in re.findall(r'\bl32i(?:\.n)?\s+a\d+,\s*a1,\s*(0x[0-9a-f]+|[0-9]+)', asm)}
    if not {frame + offset for offset in argument_offsets}.issubset(offsets):
        raise RuntimeError(f'Cannot verify incoming stack argument offsets in {name}\n' + asm)
    return frame



if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('elf', type=Path)
    args = parser.parse_args()
    check(args.elf.resolve())
