# Local Harbor smoke evaluation

The adapter runs the packaged Operit2 CLI inside the original task environment.
Its model requests, prompts, tools, chat lifecycle, and execution loop remain
owned by Operit2. Each trial gets a new isolated identity and a workspace mounted
at the task's native `/app`. It enables full tool permissions inside that sandbox.
The adapter saves the native chat transcript; it does not yet export ATIF or
independently meter model calls and cost.

This initial run uses Terminal-Bench 2.1 at
`d49e28f1e4ddd13d289e85a5f312a66750951932`, selecting `regex-log` and
`cancel-async-tasks`, one attempt each, sequentially, with no trial retries.
The configured model is `deepseek-v4-flash`, with high thinking effort.
This is a two-task integration check, not a full leaderboard result.

Run the Linux build on the WSL native filesystem to reuse its Cargo cache:

```sh
.venv/bin/python tools/build_scripts/build_cli_linux.py --arches host
```

`package_linux.py` makes a separate evaluation bundle containing a stripped CLI
and the exact dynamic loader and libraries it was linked against. This allows a
binary built on Fedora to run inside the original Ubuntu/Debian task images.
The bundle's entry point expects installation under `/opt/operit`; it is not a
general distribution archive or a replacement for the normal CLI release.

```sh
python3 tools/benchmarks/harbor/package_linux.py \
  apps/cli/target/release/operit2 ~/prog/operit-benchmark/package
```

Place the pinned dataset and package under a dedicated local benchmark root.
Export the desired CLI providers privately to `secrets/providers.json`; never
commit that file or upload it with the evaluation results. The adapter copies
credentials into the task sandbox, imports them through the CLI, and deletes the
temporary import document. Credentials remain in the sandbox's private Operit
configuration until Harbor deletes that sandbox. Host artifacts and secrets are
copied into containers rather than bind mounting the source checkout.

Install Harbor and Podman in WSL. `podman-compose` is used directly through
Podman's provider setting; no Docker Desktop or permanent Podman API service is
required. No changes are made to the original tasks or graders.

```sh
python3 tools/benchmarks/harbor/run_smoke.py --root ~/prog/operit-benchmark --dry-run
python3 tools/benchmarks/harbor/run_smoke.py --root ~/prog/operit-benchmark
```

Harbor writes the run configuration, each trial's verifier logs and score, and
the CLI's receipt and chat transcript under `jobs/`. Containers are deleted after
their trials. The two task images are retained for subsequent local evaluations.
