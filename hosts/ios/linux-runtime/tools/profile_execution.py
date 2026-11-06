#!/usr/bin/env python3
"""Measure the unchanged Python workload and optionally sample its hot window.

Acceptance runs use production binaries, without syscall tracing or sampling.
The --sample phase is separate: CLOCK_MONOTONIC syscall traces delimit the
original workload's two perf_counter calls. No Python source is modified.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import statistics
import subprocess
import sys
import time

WORKLOAD_SHA256 = "736118e90c7989f0352d26bce340ef3118a4efcf7d6b465ba4a71a99849e5eec"
CHECKSUM = 1267865


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def command(binary, rootfs, source, trace=False):
    args = [str(binary), "--root", str(rootfs), "--stats", "--max-steps", "2000000000"]
    if trace:
        args.append("--trace-syscalls")
    return args + ["--", "/usr/bin/python3", "-B", "-c", source]


def checked(stdout, stderr, returncode):
    match = re.search(r"loop_seconds=([\d.]+) checksum=(\d+)", stdout)
    if returncode or not match or int(match[2]) != CHECKSUM:
        raise RuntimeError(f"workload failed: exit={returncode}\n{stdout}\n{stderr}")
    stats = next((json.loads(line) for line in stderr.splitlines()
                  if line.startswith('{"status"')), None)
    if not stats or stats["unsupported_syscalls"] or stats["accelerated_calls"]:
        raise RuntimeError(f"unexpected execution mode: {stats}")
    return float(match[1]), stats


def measure(binary, rootfs, source):
    start = time.perf_counter()
    args = command(binary, rootfs, source)
    if sys.platform == "darwin":
        args = ["/usr/bin/time", "-l"] + args
    result = subprocess.run(args, capture_output=True, text=True, timeout=120)
    elapsed = time.perf_counter() - start
    loop, stats = checked(result.stdout, result.stderr, result.returncode)
    # macOS time -l exposes per-process hardware totals without a sampling
    # session. These include startup and shutdown, not just the Python loop.
    host_counters = {}
    for key, label in (("instructions_retired", "instructions retired"),
                       ("cycles_elapsed", "cycles elapsed"),
                       ("maximum_resident_set_bytes", "maximum resident set size"),
                       ("peak_memory_footprint_bytes", "peak memory footprint")):
        found = re.search(r"^\s*(\d+)\s+" + label + r"$", result.stderr, re.MULTILINE)
        if found:
            host_counters[key] = int(found[1])
    return dict(loop_seconds=loop, process_seconds=elapsed, stats=stats,
                whole_process_host_counters=host_counters,
                stdout=result.stdout, stderr=result.stderr)


def sample(binary, rootfs, source, prefix):
    if sys.platform != "darwin":
        raise RuntimeError("--sample uses the macOS sample utility")
    started = time.perf_counter()
    process = subprocess.Popen(command(binary, rootfs, source, trace=True),
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               text=True, bufsize=1)
    sampler = None
    clocks, lines = [], []
    try:
        for line in process.stderr:
            lines.append(line)
            if line.startswith("syscall 113(0x1,"):
                clocks.append(time.perf_counter() - started)
                if len(clocks) == 1:
                    sampler = subprocess.Popen(
                        ["/usr/bin/sample", str(process.pid), "30", "1", "-mayDie",
                         "-file", str(prefix) + ".sample.txt"],
                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                elif len(clocks) == 2 and sampler.poll() is None:
                    sampler.send_signal(signal.SIGINT)
        stdout, stderr = process.stdout.read(), "".join(lines)
        checked(stdout, stderr, process.wait(timeout=120))
        if len(clocks) != 2 or sampler is None:
            raise RuntimeError(f"expected two monotonic boundaries, saw {clocks}")
        if sampler.poll() is None:
            sampler.send_signal(signal.SIGINT)
        out, err = sampler.communicate(timeout=15)
        Path(str(prefix) + ".run.log").write_text(stdout + stderr)
        Path(str(prefix) + ".sampler.log").write_text(out + err)
        return dict(clock_boundaries_seconds=clocks, sampler_returncode=sampler.returncode,
                    sample=str(prefix) + ".sample.txt")
    finally:
        for child in (sampler, process):
            if child is not None and child.poll() is None:
                child.kill()
                child.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--after", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--sample", type=int, default=0,
                        help="separate hot-window sample runs per binary (macOS)")
    parser.add_argument("--target-seconds", type=float, default=0.5)
    args = parser.parse_args()
    if args.repeats < 1 or args.warmup < 0 or args.sample < 0 or args.target_seconds <= 0:
        parser.error("invalid repeat, warmup, sample or target value")
    workload = Path(__file__).resolve().parents[1] / "benchmarks/python_loop.py"
    if digest(workload) != WORKLOAD_SHA256:
        raise RuntimeError("the acceptance workload changed")
    source, binaries = workload.read_text(), {"before": args.before.resolve(), "after": args.after.resolve()}
    for binary in binaries.values():
        if not binary.is_file() or not os.access(binary, os.X_OK):
            parser.error(f"not executable: {binary}")
    args.output.mkdir(parents=True, exist_ok=True)
    metadata = dict(workload=str(workload), workload_sha256=digest(workload), rootfs=str(args.root.resolve()),
                    binaries={mode: dict(path=str(path), sha256=digest(path)) for mode, path in binaries.items()},
                    target_seconds=args.target_seconds, sampling_is_acceptance=False,
                    timing_policy="Run alone; do not overlap builds, tests, other benchmarks or sampling.")
    (args.output / "environment.json").write_text(json.dumps(metadata, indent=2) + "\n")
    rows = []
    for iteration in range(-args.warmup, args.repeats):
        for mode in (list(binaries) if iteration % 2 == 0 else list(binaries)[::-1]):
            row = dict(iteration=iteration, mode=mode, **measure(binaries[mode], args.root, source))
            rows.append(row)
            (args.output / "measurements.json").write_text(json.dumps(rows, indent=2) + "\n")
            print(f"{mode} {iteration}: loop={row['loop_seconds']:.6f}s process={row['process_seconds']:.6f}s", flush=True)
    medians = {mode: {key: statistics.median(r[key] for r in rows if r["mode"] == mode and r["iteration"] >= 0)
                      for key in ("loop_seconds", "process_seconds")} for mode in binaries}
    hardware = {}
    for mode in binaries:
        measured = [r["whole_process_host_counters"] for r in rows
                    if r["mode"] == mode and r["iteration"] >= 0]
        keys = set.intersection(*(set(r) for r in measured))
        hardware[mode] = {key: statistics.median(r[key] for r in measured) for key in sorted(keys)}
    pairs = []
    for iteration in range(args.repeats):
        pair = {r["mode"]: r["loop_seconds"] for r in rows if r["iteration"] == iteration}
        pairs.append(dict(iteration=iteration, before_seconds=pair["before"],
                          after_seconds=pair["after"],
                          saved_seconds=pair["before"] - pair["after"],
                          speedup=pair["before"] / pair["after"]))
    summary = dict(medians=medians, checksum=CHECKSUM,
                   whole_process_host_counters=hardware,
                   speedup=medians["before"]["loop_seconds"] / medians["after"]["loop_seconds"],
                   paired=dict(runs=pairs,
                               wins=sum(p["saved_seconds"] > 0 for p in pairs),
                               median_saved_seconds=statistics.median(p["saved_seconds"] for p in pairs),
                               median_speedup=statistics.median(p["speedup"] for p in pairs)),
                   target_met=medians["after"]["loop_seconds"] < args.target_seconds)
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary), flush=True)
    samples = []
    for iteration in range(args.sample):
        for mode, binary in binaries.items():
            samples.append(dict(iteration=iteration, mode=mode,
                                **sample(binary, args.root, source, args.output / f"hot-{mode}-{iteration}")))
            (args.output / "sampling.json").write_text(json.dumps(samples, indent=2) + "\n")


if __name__ == "__main__":
    main()
