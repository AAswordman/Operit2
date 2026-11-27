# Local Harbor smoke evaluation

The adapter runs the packaged Operit2 CLI inside the original task environment.
Its model requests, prompts, tools, chat lifecycle, and execution loop remain
owned by Operit2. Each trial gets a new isolated identity and a workspace rooted
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
  apps/cli/target/release/operit2 ~/prog/operit-benchmark/package-dbus
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
The launcher selects a local `podman.conf` using iptables for container bridge
rules, because this WSL kernel lacks nftables' IPv6 FIB expression. This requires
the small `iptables-nft` package and does not change the user's Podman defaults.
`podman_local.py` keeps Harbor's Podman orchestration and executes commands and
uploads files through the native Podman CLI. A local compose command measured
about eight seconds, so Harbor's five-second per-probe deadline could never
succeed. The local adapter uses one probe with the existing 90-second total
readiness deadline. Native `podman cp` also avoids unsupported compose uploads
and buffering a second copy of the CLI archive in memory. Agent and verifier
time limits are unchanged.

The evaluation bundle includes the small `dbus-daemon` program and its linked
libraries. The adapter starts separate private system/session buses inside each
task, as required by the Linux Host event stream and plugin IPC. These buses
live only for the duration of that container. They do not connect to host buses.
The launcher uses LiteLLM's bundled model price metadata explicitly; the adapter
does not use LiteLLM to call the model or calculate billed usage.

```sh
python3 tools/benchmarks/harbor/run_smoke.py --root ~/prog/operit-benchmark --dry-run
python3 tools/benchmarks/harbor/run_smoke.py --root ~/prog/operit-benchmark --job-name install-check --install-only
python3 tools/benchmarks/harbor/run_smoke.py --root ~/prog/operit-benchmark
python3 tools/benchmarks/harbor/run_smoke.py --root ~/prog/operit-benchmark --task cancel-async-tasks --job-name cancel-after-fix
```

Omitting `--task` runs the original two tasks. Pass it once for a single task or
repeat it to select several; each selected task still runs once with no retries.

Harbor writes the run configuration, each trial's verifier logs and score, and
the CLI's receipt and chat transcript under `jobs/`. Containers are deleted after
their trials. The two task images are retained for subsequent local evaluations.

Inspect the verifier log as well as `reward.txt`. The original shell graders can
write a zero reward when their dependency downloads fail, before pytest runs.
That is an infrastructure failure and does not establish whether the answer
passes. Preserve the original result and answer, then regrade that same answer
with the original tests and pinned Python/pytest dependencies. Record a separate
verifier-only result; do not submit the task to the agent again. Dependency caches
can be copied into the original image and the original pytest command run
offline. Keep the answer hash, command, CTRF output and provisioning details with
the regrade evidence.

For direct networking after disabling a proxy, clear `HTTP_PROXY`, `HTTPS_PROXY`,
`ALL_PROXY` and their lowercase variants in the launching shell. The WSL build
environment may have set these variables earlier in the session.
