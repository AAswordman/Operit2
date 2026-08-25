# Link Performance Scenarios

This directory is for end-to-end Link performance runs that cross app or host
boundaries. Keep protocol-only benchmarks in `core/crates/foundation/link/benches`.

## Scope

- Measure CLI, host, and access-layer paths that embed `operit-link`.
- Keep scenario inputs in `scenarios/` as stable JSON files.
- Write generated run output under `reports/`.
- Record machine, build profile, command, sample count, and payload size with
  every report.

## Scenario Files

跨节点性能场景必须针对 `CoreNodeRouter -> PeerLink -> CoreNode` 编写。旧的应用层
HTTP RPC 场景已经删除，避免把本地 Proxy 链路误当成 Space 链路。

## Commands

```powershell
.\tools\link-perf\run_link_perf.ps1 -Scenario .\tools\link-perf\scenarios\<peer-scenario>.json
```

Rust warnings are ignored for performance runs. The script records commands and
timestamps, but the concrete process wiring belongs to the selected scenario.
