#!/usr/bin/env python3
"""Serial, rotating comparison of two Operit builds against identical guests."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
import time

from compare_aot import GUEST_ENV, UNTRAINED


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ("control", "candidate", "root", "output"):
        p.add_argument("--" + name, type=Path, required=True)
    p.add_argument("--repeats", type=int, default=9)
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=True)
    result_path = a.output / "measurements.jsonl"
    if result_path.exists():
        raise RuntimeError("output already contains measurements; use a fresh directory")
    binaries = {name: getattr(a, name).resolve() for name in ("control", "candidate")}
    source_path = Path(__file__).resolve().parents[1] / "benchmarks/python_loop.py"
    workloads = {"trained": ["/usr/bin/python3", "-B", "-c", source_path.read_text()],
                 "untrained": ["/usr/bin/python3", "-B", "-c", UNTRAINED],
                 "c": ["/tmp/runtime-compare"]}
    root = a.root.resolve()
    env = {k: v for k, v in os.environ.items() if not k.startswith(("ISH_", "OPERIT_"))}
    env["TERM"] = "xterm-256color"
    metadata = {"binaries": {k: {"path": str(v), "sha256": sha(v), "bytes": v.stat().st_size}
                             for k, v in binaries.items()},
                "guest_files": {str(f): sha(f) for f in (source_path, root / "usr/bin/python3.12",
                    root / "usr/lib/libpython3.12.so.1.0", root / "lib/ld-musl-aarch64.so.1", root / "tmp/runtime-compare")},
                "guest_environment": GUEST_ENV, "untrained_source": UNTRAINED,
                "host": subprocess.check_output(["sw_vers"], text=True),
                "cpu": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip()}
    (a.output / "environment.json").write_text(json.dumps(metadata, indent=2) + "\n")
    rows, expected = [], {}
    for workload, guest in workloads.items():
        for iteration in range(-1, a.repeats):
            for mode in (("control", "candidate") if iteration % 2 == 0 else ("candidate", "control")):
                cmd = [str(binaries[mode]), "--root", str(root), "--stats", "--max-steps", "10000000000", "--",
                       "/usr/bin/env", "-i", *GUEST_ENV, *guest]
                start = time.perf_counter()
                r = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=120)
                elapsed = time.perf_counter() - start
                if r.returncode:
                    raise RuntimeError(f"{workload} {mode}: {r.returncode}\n{r.stdout}\n{r.stderr}")
                stats = next((json.loads(line) for line in r.stderr.splitlines() if line.startswith('{"status"')), None)
                if not stats or stats["status"] or stats["accelerated_calls"] or stats["unsupported_syscalls"]:
                    raise RuntimeError(f"unexpected stats: {stats}")
                if workload == "c":
                    values = re.findall(r"^(\w+) ns=(\d+) checksum=(\d+)$", r.stdout, re.M)
                    if {x[0] for x in values} != {"integer", "memory", "floating", "calls"}:
                        raise RuntimeError(r.stdout)
                    checks = {name: check for name, _, check in values}
                    metrics = {name: int(ns) / 1e9 for name, ns, _ in values}
                else:
                    m = re.search(r"(?:loop|work)_seconds=([\d.]+) checksum=([\da-f]+)", r.stdout)
                    if not m:
                        raise RuntimeError(r.stdout)
                    checks, metrics = m[2], {"work_seconds": float(m[1])}
                    if workload == "trained" and checks != "1267865":
                        raise RuntimeError(f"bad trained checksum: {checks}")
                if workload in expected and checks != expected[workload]:
                    raise RuntimeError(f"checksum mismatch: {checks} != {expected[workload]}")
                expected[workload] = checks
                row = {"workload": workload, "iteration": iteration, "mode": mode,
                       "process_seconds": elapsed, "metrics": metrics, "checksums": checks,
                       "command": cmd, "stdout": r.stdout, "stderr": r.stderr}
                rows.append(row)
                with result_path.open("a") as out:
                    out.write(json.dumps(row) + "\n")
                print(workload, iteration, mode, metrics, f"process={elapsed:.6f}", flush=True)
    summary = {}
    for workload in workloads:
        subset = [r for r in rows if r["workload"] == workload and r["iteration"] >= 0]
        keys = [*subset[0]["metrics"], "process_seconds"]
        summary[workload] = {mode: {k: statistics.median(r["process_seconds"] if k == "process_seconds" else r["metrics"][k]
                                                        for r in subset if r["mode"] == mode)
                                    for k in keys} for mode in binaries}
    (a.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
