"""Generate realtime scheduling ELF fixtures without invoking a guest compiler."""
import argparse
from pathlib import Path
import struct
import subprocess
import tempfile

BASE = 0x400000
CODE_OFFSET = 4096
PEER_OFFSET = 512
BACKGROUND_OFFSET = 768
DATA_OFFSET = 1024
SUCCESS = 'realtime FIFO/RR single/quad-core guest dispatch passed'
WATCHDOG_SUCCESS = 'realtime RTTIME per-thread watchdog and signal pipeline passed'


class Program:
    """Encode the small ADR/conditional-branch subset used by the fixtures."""

    def __init__(self):
        self.data = bytearray()
        self.labels = {}
        self.fixups = []

    def word(self, value):
        self.data.extend(struct.pack('<I', value))

    def label(self, name):
        if name in self.labels:
            raise ValueError(f'duplicate label: {name}')
        self.labels[name] = len(self.data)

    def relative(self, kind, label, register=0):
        self.fixups.append((len(self.data), kind, label, register))
        self.word(0)

    def pad_to(self, offset):
        if len(self.data) > offset or offset % 4:
            raise ValueError(f'fixture overlaps its next section at {offset}')
        self.data.extend(b'\0' * (offset - len(self.data)))

    def exit(self, code):
        self.word(0xd2800000 | (code << 5))  # MOV X0, #code.
        self.word(0xd2800ba8)  # MOV X8, #93 (exit).
        self.word(0xd4000001)  # SVC #0.

    def finish(self):
        for offset, kind, label, register in self.fixups:
            delta = self.labels[label] - offset
            if kind == 'adr':
                if not -(1 << 20) <= delta < (1 << 20):
                    raise ValueError('ADR target out of range')
                value = (0x10000000 | ((delta & 3) << 29) |
                         (((delta >> 2) & 0x7ffff) << 5) | register)
            elif kind == 'bne':
                if delta % 4 or not -(1 << 20) <= delta < (1 << 20):
                    raise ValueError('B.NE target out of range')
                value = 0x54000001 | (((delta // 4) & 0x7ffff) << 5)
            else:
                raise ValueError(f'unknown fixup: {kind}')
            struct.pack_into('<I', self.data, offset, value)
        return bytes(self.data)


def realtime_code(round_robin):
    """Make peer execution observable across instruction and syscall boundaries."""
    program = Program()
    program.relative('adr', 'data', 9)
    program.word(0x52800003 | (20000 << 5))  # MOV W3, #20000.
    program.label('leader_loop')
    program.word(0x71000463)  # SUBS W3, W3, #1.
    program.relative('bne', 'leader_loop')
    program.word(0xd2800008 | (178 << 5))  # MOV X8, #178 (gettid).
    program.word(0xd4000001)
    program.word(0xb9400120)  # LDR W0, [X9].
    program.word(0x7100001f | (int(round_robin) << 10))  # CMP W0, #expected.
    program.relative('bne', 'failure')
    program.word(0xd2800008 | (124 << 5))  # MOV X8, #124 (sched_yield).
    program.word(0xd4000001)
    program.word(0xb9400120)  # LDR W0, [X9].
    program.word(0x7100041f)  # CMP W0, #1.
    program.relative('bne', 'failure')
    program.word(0x5280002a)  # MOV W10, #1.
    program.word(0xb900052a)  # STR W10, [X9, #4]: leader assertions completed.
    program.exit(0)
    program.label('failure')
    program.exit(77)

    program.pad_to(PEER_OFFSET)
    program.relative('adr', 'data', 9)
    program.word(0x5280002a)  # MOV W10, #1.
    program.word(0xb900012a)  # STR W10, [X9]: peer ran.
    program.exit(0)

    program.pad_to(BACKGROUND_OFFSET)
    program.word(0x52800003 | (50000 << 5))  # MOV W3, #50000.
    program.label('background_loop')
    program.word(0x71000463)  # SUBS W3, W3, #1.
    program.relative('bne', 'background_loop')
    program.exit(0)
    program.pad_to(DATA_OFFSET)
    program.label('data')
    program.data.extend(b'\0' * 8)
    return program.finish()


def watchdog_code():
    """Busy RT guest keeps making nonblocking syscalls and yielding, never sleeping."""
    data = bytearray(realtime_code(False))
    words = [0xd2801648,  # MOV X8, #178 (gettid).
             0xd4000001,  # SVC #0.
             0xd2800f88,  # MOV X8, #124 (sched_yield).
             0xd4000001,
             0x17fffffc]  # B back to the first syscall.
    data[:len(words) * 4] = struct.pack('<5I', *words)
    # Keep the other vCPU runnable while the harness establishes concurrent
    # execution with the watchdog disabled, then arms its short CPU limit.
    # The dispatch fixtures still use their original finite background loop.
    data[BACKGROUND_OFFSET + 12:BACKGROUND_OFFSET + 16] = struct.pack('<I', 0x17fffffd)
    return bytes(data)


def elf(code):
    """One RWX segment; C++ harness starts peer/background at the fixed offsets."""
    size = CODE_OFFSET + len(code)
    header = struct.pack('<16sHHIQQQIHHHHHH', b'\x7fELF\x02\x01\x01' + b'\0' * 9,
                         2, 183, 1, BASE + CODE_OFFSET, 64, 0, 0, 64, 56, 1, 0, 0, 0)
    segment = struct.pack('<IIQQQQQQ', 1, 7, 0, BASE, BASE, size, size, 4096)
    return (header + segment).ljust(CODE_OFFSET, b'\0') + code


def write_fixtures(root):
    """Write scheduling and watchdog programs into the supplied test root."""
    root.mkdir(parents=True, exist_ok=True)
    for name, round_robin in [('fifo', False), ('rr', True)]:
        path = root / name
        path.write_bytes(elf(realtime_code(round_robin)))
        path.chmod(0o755)
    watchdog = root / 'watchdog'
    watchdog.write_bytes(elf(watchdog_code()))
    watchdog.chmod(0o755)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--api-tests', type=Path, help='Already-built scheduling test executable')
    mode.add_argument('--write-fixtures', type=Path, help='Generate ELF files only; run no executable')
    options = parser.parse_args()
    if options.write_fixtures is not None:
        write_fixtures(options.write_fixtures)
        return
    with tempfile.TemporaryDirectory(prefix='arm64-realtime-') as directory:
        root = Path(directory)
        write_fixtures(root)
        completed = subprocess.run([str(options.api_tests.resolve()), str(root)],
                                   capture_output=True, text=True, timeout=25)
        markers = completed.stdout.splitlines()
        if completed.returncode or SUCCESS not in markers or WATCHDOG_SUCCESS not in markers:
            raise RuntimeError(f'realtime dispatch failed ({completed.returncode})\n'
                               f'{completed.stdout}{completed.stderr}')
        print(completed.stdout, end='')


if __name__ == '__main__':
    main()
