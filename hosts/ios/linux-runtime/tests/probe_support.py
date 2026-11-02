"""Shared subprocess capture for guest application acceptance probes."""
import json
import resource
import subprocess
import time

def execute(command, timeout):
    start = time.monotonic()
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    try:
        result = subprocess.run(command, capture_output=True, timeout=timeout)
        stdout, stderr = result.stdout.decode(errors='replace'), result.stderr.decode(errors='replace')
        returncode = result.returncode
        timed_out = False
    except subprocess.TimeoutExpired as error:
        stdout = (error.stdout or b'').decode(errors='replace')
        stderr = (error.stderr or b'').decode(errors='replace')
        returncode = None
        timed_out = True
    elapsed = time.monotonic() - start
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu_seconds = after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime
    stats = {}
    for line in stderr.splitlines():
        if line.startswith('{'):
            try:
                candidate = json.loads(line)
                if 'status' in candidate and 'instructions' in candidate:
                    stats = candidate
            except ValueError:
                pass
    return {'command': [str(part) for part in command], 'returncode': returncode, 'timed_out': timed_out, 'wall_seconds': elapsed, 'host_cpu_seconds': cpu_seconds,
            'average_host_cores': cpu_seconds / elapsed if elapsed else 0,
            'stdout': stdout, 'stderr': stderr, 'stats': stats}
