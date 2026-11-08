#!/usr/bin/env python3
"""Compare prebuilt AOT and static ARM64 executors on identical guests."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
import time


GUEST_ENV = ["HOME=/root", "PATH=/usr/local/bin:/usr/bin:/bin", "TERM=xterm-256color",
             "PYTHONMALLOC=malloc", "PYTHONDONTWRITEBYTECODE=1"]
UNTRAINED = '''import hashlib, json, re, time
t = time.perf_counter()
h = hashlib.sha256()
for i in range(1500):
    data = json.dumps({"number": i, "items": [i, i * 7, str(i)]}, sort_keys=True)
    h.update(re.sub(r"[0-9]+", "#", data).encode())
print("work_seconds=%.9f checksum=%s" % (time.perf_counter() - t, h.hexdigest()))
'''


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("operit", "static", "aot", "root", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=9)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    if (args.output / "measurements.jsonl").exists():
        raise RuntimeError("output already contains measurements; use a fresh directory")
    binaries = {k: getattr(args, k).resolve() for k in ("operit", "static", "aot")}
    root = args.root.resolve()
    source_path = Path(__file__).resolve().parents[1] / "benchmarks/python_loop.py"
    source = source_path.read_text()
    workloads = {"trained": ["/usr/bin/python3", "-B", "-c", source],
                 "untrained": ["/usr/bin/python3", "-B", "-c", UNTRAINED],
                 "c": ["/tmp/runtime-compare"]}
    env = {k: v for k, v in os.environ.items() if not k.startswith(("ISH_", "OPERIT_"))}
    env["TERM"] = "xterm-256color"
    metadata = {"host": subprocess.check_output(["sw_vers"], text=True),
                "cpu": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip(),
                "binaries": {k: {"path": str(v), "sha256": sha(v), "bytes": v.stat().st_size}
                             for k, v in binaries.items()},
                "guest_files": {str(f): sha(f) for f in
                    (source_path, root / "usr/bin/python3.12", root / "usr/lib/libpython3.12.so.1.0",
                     root / "lib/ld-musl-aarch64.so.1", root / "tmp/runtime-compare")},
                "untrained_source": UNTRAINED, "guest_environment": GUEST_ENV,
                "note": "AOT images trained on the Python loop and linked into a build with jit_emit=false; macOS only."}
    (args.output / "environment.json").write_text(json.dumps(metadata, indent=2) + "\n")
    expected, rows = {}, []
    orders = [("operit", "static", "aot"), ("aot", "static", "operit"),
              ("static", "operit", "aot"), ("aot", "operit", "static"),
              ("operit", "aot", "static"), ("static", "aot", "operit")]
    for workload, program in workloads.items():
        for iteration in range(-1, args.repeats):
            for position, mode in enumerate(orders[iteration % len(orders)]):
                prefix = (["--root", str(root), "--stats", "--max-steps", "10000000000", "--"]
                          if mode == "operit" else ["-r", str(root)])
                command = [str(binaries[mode]), *prefix, "/usr/bin/env", "-i", *GUEST_ENV, *program]
                start = time.perf_counter()
                result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=120)
                elapsed = time.perf_counter() - start
                if result.returncode:
                    raise RuntimeError(f"{workload} {mode}: {result.returncode}\n{result.stdout}\n{result.stderr}")
                if mode == "operit":
                    stats = next((json.loads(line) for line in result.stderr.splitlines()
                                  if line.startswith('{"status"')), None)
                    if not stats or stats["status"] or stats["accelerated_calls"] or stats["unsupported_syscalls"]:
                        raise RuntimeError(f"unexpected Operit stats: {stats}")
                if workload == "c":
                    matches = re.findall(r"^(\w+) ns=(\d+) checksum=(\d+)$", result.stdout, re.M)
                    if {x[0] for x in matches} != {"integer", "memory", "floating", "calls"}:
                        raise RuntimeError(result.stdout)
                    checks = {name: value for name, _, value in matches}
                    metrics = {name: int(ns) / 1e9 for name, ns, _ in matches}
                else:
                    match = re.search(r"(?:loop|work)_seconds=([\d.]+) checksum=([\da-f]+)", result.stdout)
                    if not match:
                        raise RuntimeError(result.stdout)
                    checks, metrics = match[2], {"work_seconds": float(match[1])}
                    if workload == "trained" and checks != "1267865":
                        raise RuntimeError(f"incorrect trained checksum: {checks}")
                if workload in expected and checks != expected[workload]:
                    raise RuntimeError(f"checksum mismatch {workload}: {checks} != {expected[workload]}")
                expected[workload] = checks
                row = {"workload": workload, "iteration": iteration, "position": position,
                       "mode": mode, "process_seconds": elapsed, "metrics": metrics,
                       "checksums": checks, "command": command, "stdout": result.stdout,
                       "stderr": result.stderr}
                rows.append(row)
                with (args.output / "measurements.jsonl").open("a") as f:
                    f.write(json.dumps(row) + "\n")
                print(workload, iteration, mode, metrics, f"process={elapsed:.6f}", flush=True)
    summary = {}
    for workload in workloads:
        sample = [r for r in rows if r["workload"] == workload and r["iteration"] >= 0]
        keys = [*sample[0]["metrics"], "process_seconds"]
        summary[workload] = {mode: {key: statistics.median(
            r["process_seconds"] if key == "process_seconds" else r["metrics"][key]
            for r in sample if r["mode"] == mode) for key in keys} for mode in binaries}
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
