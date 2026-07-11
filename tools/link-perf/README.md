# Link Performance Scenarios

This directory is for end-to-end Link performance runs that cross app or host
boundaries. Keep protocol-only benchmarks in `core/crates/operit-link/benches`.

## Scope

- Measure CLI, host, and access-layer paths that embed `operit-link`.
- Keep scenario inputs in `scenarios/` as stable JSON files.
- Write generated run output under `reports/`.
- Record machine, build profile, command, sample count, and payload size with
  every report.

## Scenario Files

- `scenarios/local_call.json`: local one-shot `/link/call` latency.
- `scenarios/local_call_stress.json`: local dispatcher call burst pressure.
- `scenarios/local_watch.json`: local watch channel open and event stream cost.
- `scenarios/cli_to_host_call.json`: CLI to host call path latency.

## Commands

```powershell
.\tools\link-perf\run_link_perf.ps1 -Scenario .\tools\link-perf\scenarios\local_call.json
```

Rust warnings are ignored for performance runs. The script records commands and
timestamps, but the concrete process wiring belongs to the selected scenario.
