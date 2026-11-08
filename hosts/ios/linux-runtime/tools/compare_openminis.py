#!/usr/bin/env python3
"""Serial, rotating comparisons of identical ARM64 guests across three CLIs.

Requires two upstream builds (jit=false and jit=true), an Operit production CLI,
and a disposable ordinary rootfs containing Python and /tmp/runtime-compare.
Build runtime_compare.c natively to supply --native-oracle. Neither the oracle's
speed nor a different Python build is used as a cross-runtime speed baseline.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
import time

WORKLOAD_HASH = "736118e90c7989f0352d26bce340ef3118a4efcf7d6b465ba4a71a99849e5eec"
ENVIRONMENT = ["HOME=/root", "PATH=/usr/local/bin:/usr/bin:/bin", "TERM=xterm-256color",
               "PYTHONMALLOC=malloc", "PYTHONDONTWRITEBYTECODE=1"]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def c_results(stdout):
    rows = re.findall(r"^(\w+) ns=(\d+) checksum=(\d+)$", stdout, re.M)
    if len(rows) != 4 or {r[0] for r in rows} != {"integer", "memory", "floating", "calls"}:
        raise RuntimeError(f"missing C benchmark results: {stdout}")
    return {name: {"seconds": int(ns) / 1e9, "checksum": int(check)}
            for name, ns, check in rows}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for arg in ["operit", "gadgets", "jit", "root", "output", "native-oracle"]:
        p.add_argument("--" + arg, type=Path, required=True)
    p.add_argument("--repeats", type=int, default=9)
    p.add_argument("--default-repeats", type=int, default=5)
    args = p.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    root = args.root.resolve()
    source_file = Path(__file__).resolve().parents[1] / "benchmarks/python_loop.py"
    assert sha(source_file) == WORKLOAD_HASH
    source = source_file.read_text()
    binaries = {"operit": args.operit.resolve(), "gadgets": args.gadgets.resolve(), "jit": args.jit.resolve()}
    oracle = subprocess.run([str(args.native_oracle.resolve())], capture_output=True, text=True, check=True)
    expected = {k: v["checksum"] for k, v in c_results(oracle.stdout).items()}
    (args.output / "native-oracle.txt").write_text(oracle.stdout)
    env = {k: v for k, v in os.environ.items() if not k.startswith(("ISH_", "OPERIT_"))}
    env["TERM"] = "xterm-256color"
    metadata = {
        "timestamp_local": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "root": str(root), "binaries": {k: {"path": str(v), "sha256": sha(v)} for k, v in binaries.items()},
        "files": {str(f): sha(f) for f in [source_file, root / "usr/bin/python3.12",
                    root / "usr/lib/libpython3.12.so.1.0", root / "lib/ld-musl-aarch64.so.1",
                    root / "tmp/runtime-compare"]},
        "normalized_guest_environment": ENVIRONMENT, "c_checksums": expected,
        "warmup_rounds_per_workload": 1, "repeats": args.repeats,
        "default_repeats": args.default_repeats,
        "host": subprocess.check_output(["sw_vers"], text=True),
        "cpu": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string", "hw.ncpu", "hw.memsize"], text=True),
        "notes": ["No binary/function offload or AOT images linked into upstream CLI.",
                  "Default Python runs retain each CLI's environment; normalized runs use the same guest env -i.",
                  "All process times include the time utility/spawn overhead; counters cover entire process.",
                  "The native oracle checks results only; host compilation/ABI differs from Linux ELF."]}
    (args.output / "environment.json").write_text(json.dumps(metadata, indent=2) + "\n")
    rows = []

    def measure(mode, workload, iteration, position):
        prefix = (["--root", str(root), "--stats", "--max-steps", "10000000000", "--"]
                  if mode == "operit" else ["-r", str(root)])
        guest = (["/tmp/runtime-compare"] if workload == "c"
                 else ["/usr/bin/python3", "-B", "-c", source])
        if workload == "python-normalized":
            guest = ["/usr/bin/env", "-i", *ENVIRONMENT, *guest]
        command = ["/usr/bin/time", "-l", str(binaries[mode]), *prefix, *guest]
        start = time.perf_counter()
        result = subprocess.run(command, capture_output=True, text=True, env=env, timeout=120)
        elapsed = time.perf_counter() - start
        stem = f"{workload}.{iteration}.{position}.{mode}"
        (args.output / (stem + ".stdout")).write_text(result.stdout)
        (args.output / (stem + ".stderr")).write_text(result.stderr)
        row = dict(mode=mode, workload=workload, iteration=iteration, position=position,
                   process_seconds=elapsed, returncode=result.returncode, command=command,
                   stdout=result.stdout, stderr=result.stderr)
        if result.returncode:
            raise RuntimeError(f"failed: {stem}\n{result.stdout}\n{result.stderr}")
        if workload == "c":
            checks = c_results(result.stdout)
            if {k: v["checksum"] for k, v in checks.items()} != expected:
                raise RuntimeError(f"C checksum mismatch: {stem}: {checks}")
            row["metrics"] = {k: v["seconds"] for k, v in checks.items()}
        else:
            match = re.search(r"loop_seconds=([\d.]+) checksum=(\d+)", result.stdout)
            if not match or int(match[2]) != 1267865:
                raise RuntimeError(f"Python checksum mismatch: {stem}")
            row["metrics"] = {"loop_seconds": float(match[1])}
        if mode == "operit":
            stats = next((json.loads(line) for line in result.stderr.splitlines() if line.startswith('{"status"')), None)
            if not stats or stats["status"] or stats["accelerated_calls"] or stats["unsupported_syscalls"]:
                raise RuntimeError(f"unexpected Operit mode: {stats}")
            row["operit_stats"] = stats
        row["counters"] = {}
        for key, label in [("instructions", "instructions retired"), ("cycles", "cycles elapsed"),
                           ("rss_bytes", "maximum resident set size"), ("footprint_bytes", "peak memory footprint")]:
            match = re.search(r"^\s*(\d+)\s+" + label + r"$", result.stderr, re.M)
            if match:
                row["counters"][key] = int(match[1])
        rows.append(row)
        (args.output / "measurements.json").write_text(json.dumps(rows, indent=2) + "\n")
        print(stem, json.dumps(row["metrics"]), f"process={elapsed:.6f}", flush=True)

    for workload, repeats in [("python-normalized", args.repeats), ("python-default", args.default_repeats), ("c", args.repeats)]:
        # All six permutations repeat evenly: each mode occupies every position.
        orders = [("operit", "gadgets", "jit"), ("jit", "gadgets", "operit"),
                  ("gadgets", "operit", "jit"), ("jit", "operit", "gadgets"),
                  ("operit", "jit", "gadgets"), ("gadgets", "jit", "operit")]
        for iteration in range(-1, repeats):
            for position, mode in enumerate(orders[iteration % len(orders)]):
                measure(mode, workload, iteration, position)

    summary = {}
    for workload in ["python-normalized", "python-default", "c"]:
        included = [r for r in rows if r["workload"] == workload and r["iteration"] >= 0]
        metrics = [*included[0]["metrics"], "process_seconds"]
        mode_stats = {}
        for mode in binaries:
            subset = [r for r in included if r["mode"] == mode]
            mode_stats[mode] = {"n": len(subset), "medians": {}, "ranges": {}, "counter_medians": {}}
            for key in metrics:
                vals = [r["process_seconds"] if key == "process_seconds" else r["metrics"][key] for r in subset]
                mode_stats[mode]["medians"][key] = statistics.median(vals)
                mode_stats[mode]["ranges"][key] = [min(vals), max(vals)]
            for key in subset[0]["counters"]:
                mode_stats[mode]["counter_medians"][key] = statistics.median(r["counters"][key] for r in subset)
        comparisons = {}
        for mode in ["gadgets", "jit"]:
            wins = {}
            for key in metrics:
                pairs = []
                for i in range(mode_stats[mode]["n"]):
                    a = next(r for r in included if r["iteration"] == i and r["mode"] == "operit")
                    b = next(r for r in included if r["iteration"] == i and r["mode"] == mode)
                    pairs.append((a["process_seconds"], b["process_seconds"]) if key == "process_seconds"
                                 else (a["metrics"][key], b["metrics"][key]))
                wins[key] = {"faster_rounds": sum(b < a for a, b in pairs), "n": len(pairs),
                             "ratio_of_medians": mode_stats["operit"]["medians"][key] / mode_stats[mode]["medians"][key],
                             "median_paired_ratio": statistics.median(a / b for a, b in pairs)}
            comparisons[mode] = wins
        summary[workload] = {"modes": mode_stats, "vs_operit": comparisons}
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
