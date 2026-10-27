#!/usr/bin/env python3
"""Sample a macOS process without attaching a profiler.

Usage: python3 tools/benchmarks/macos_idle.py PID --seconds 30 --output idle.json
CPU percentages use one core as 100%. Wakeups are process counters, not watts.
"""
import argparse
import ctypes
import json
import statistics
import time
from pathlib import Path

class Usage(ctypes.Structure):
    _fields_ = [('uuid', ctypes.c_uint8 * 16)] + [(name, ctypes.c_uint64) for name in (
        'user_time', 'system_time', 'pkg_idle_wkups', 'interrupt_wkups', 'pageins',
        'wired_size', 'resident_size', 'phys_footprint', 'proc_start_abstime',
        'proc_exit_abstime', 'child_user_time', 'child_system_time',
        'child_pkg_idle_wkups', 'child_interrupt_wkups', 'child_pageins',
        'child_elapsed_abstime', 'diskio_bytesread', 'diskio_byteswritten')]

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('pid', type=int)
parser.add_argument('--seconds', type=int, default=30)
parser.add_argument('--output', required=True)
args = parser.parse_args()
if args.seconds < 1:
    parser.error('--seconds must be positive')
class Timebase(ctypes.Structure):
    _fields_ = [('numer', ctypes.c_uint32), ('denom', ctypes.c_uint32)]
timebase = Timebase()
ctypes.CDLL('/usr/lib/libSystem.B.dylib').mach_timebase_info(ctypes.byref(timebase))
ns_per_tick = timebase.numer / timebase.denom
lib = ctypes.CDLL('/usr/lib/libproc.dylib', use_errno=True)
lib.proc_pid_rusage.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_void_p]
lib.proc_pid_rusage.restype = ctypes.c_int

def read():
    usage = Usage()
    if lib.proc_pid_rusage(args.pid, 2, ctypes.byref(usage)) != 0:
        raise OSError(ctypes.get_errno(), 'proc_pid_rusage failed')
    return time.monotonic(), {name: getattr(usage, name) for name, _ in Usage._fields_ if name != 'uuid'}

start_t, start = previous_t, previous = read()
samples = []
for _ in range(args.seconds):
    time.sleep(1)
    now_t, now = read()
    dt = now_t - previous_t
    samples.append({
        'elapsed_seconds': now_t - start_t,
        'cpu_percent_one_core': ((now['user_time'] + now['system_time']) - (previous['user_time'] + previous['system_time'])) * ns_per_tick / 1e9 / dt * 100,
        'interrupt_wakeups_per_second': (now['interrupt_wkups'] - previous['interrupt_wkups']) / dt,
        'package_idle_wakeups_per_second': (now['pkg_idle_wkups'] - previous['pkg_idle_wkups']) / dt,
        'footprint_mib': now['phys_footprint'] / 1024**2,
    })
    previous_t, previous = now_t, now
elapsed = now_t - start_t
summary = {
    'pid': args.pid,
    'cpu_time_conversion': f'mach absolute ticks: {timebase.numer}/{timebase.denom} ns per tick',
    'duration_seconds': elapsed,
    'cpu_percent_one_core': ((now['user_time'] + now['system_time']) - (start['user_time'] + start['system_time'])) * ns_per_tick / 1e9 / elapsed * 100,
    'cpu_percent_median': statistics.median(s['cpu_percent_one_core'] for s in samples),
    'cpu_percent_max_1s': max(s['cpu_percent_one_core'] for s in samples),
    'interrupt_wakeups_per_second': (now['interrupt_wkups'] - start['interrupt_wkups']) / elapsed,
    'package_idle_wakeups_per_second': (now['pkg_idle_wkups'] - start['pkg_idle_wkups']) / elapsed,
    'footprint_mib': now['phys_footprint'] / 1024**2,
    'disk_read_bytes': now['diskio_bytesread'] - start['diskio_bytesread'],
    'disk_write_bytes': now['diskio_byteswritten'] - start['diskio_byteswritten'],
}
Path(args.output).write_text(json.dumps({'summary': summary, 'samples': samples}, indent=2) + '\n')
print(json.dumps(summary, indent=2))
