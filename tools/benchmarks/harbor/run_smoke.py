"""Run two pinned Terminal-Bench tasks locally, once each, using Podman."""

import argparse
import os
import subprocess
from pathlib import Path


# Resolve local inputs and dispatch the two sequential Harbor trials.
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--job-name", default="operit2-first-two")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve(strict=True)
    dataset = root / "terminal-bench-2-1-d49e28f1e4ddd13d289e85a5f312a66750951932" / "tasks"
    bundle = root / "package" / "operit2-harbor-linux-x86_64.tar.gz"
    providers = root / "secrets" / "providers.json"
    for path in (dataset, bundle, providers):
        if not path.exists():
            raise FileNotFoundError(path)
    environment = dict(os.environ)
    environment["PYTHONPATH"] = str(Path(__file__).resolve().parent)
    environment["PODMAN_COMPOSE_PROVIDER"] = "/usr/bin/podman-compose"
    environment["PODMAN_COMPOSE_WARNING_LOGS"] = "false"
    command = [str(Path.home() / ".local/bin/harbor"), "run",
               "-p", str(dataset), "-i", "regex-log", "-i", "cancel-async-tasks",
               "-a", "operit_agent:OperitAgent", "-m", "deepseek-v4-flash",
               "--ak", f"bundle_path={bundle}", "--ak", f"providers_path={providers}",
               "--ak", "thinking=high", "-e", "podman", "-k", "1", "-n", "1", "-r", "0",
               "--job-name", args.job_name, "--jobs-dir", str(root / "jobs")]
    if args.dry_run:
        command.append("--dry-run")
    raise SystemExit(subprocess.call(command, cwd=root, env=environment))


if __name__ == "__main__":
    main()
