# Extension startup: measurement and removal of duplicate loading paths

Measurements use the current Windows debug CLI and an isolated profile. Each
sample starts a fresh CLI process. OS caches are warm; these are not Android or
release benchmarks. Registry times sum five non-overlapping Host-clock phases;
process times use Python's performance counter and include CLI initialization.

## Exactly 30 enabled ToolPkg plugins before the final async algorithm change

The workload contains **30 ToolPkg containers plus 7 ordinary packages**:
37 enabled top-level packages, 50 catalog entries including subpackages. All five
runs loaded all 30 plugins with no load issues.

| Existing resource cache | Run 1 | Run 2 | Run 3 | Run 4 | Run 5 | Median |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Registry scan | 1140 ms | 1211 ms | 1122 ms | 1137 ms | 1160 ms | **1140 ms** |
| Complete CLI process | 1327.29 ms | 1460.20 ms | 1308.45 ms | 1316.73 ms | 1368.64 ms | **1327.29 ms** |

Moving only this diagnostic profile's extracted `cache/toolpkg` directory aside
and starting again took **3143 ms** for registry scanning and **3451.63 ms** for
the complete process. This creates missing caches, not an OS-cold benchmark.
Both following runs reused the new cache: registry 1343/1281 ms, process
1554.27/1506.30 ms. All three runs loaded all 30 plugins with no load issues.
Initial archive extraction therefore remains a one-time cost for missing caches.

## Comparable before/after workload

The original workload had 30 enabled top-level packages: **23 ToolPkg containers
and 7 ordinary packages**, not 30 ToolPkg plugins. Catalog: 41 entries including
subpackages, 37 enabled. Both series below have that identical workload.

| Registry scan | Run 1 | Run 2 | Run 3 |
| --- | ---: | ---: | ---: |
| Before | 2454 ms | 2398 ms | 2380 ms |
| After resource/pool/ownership changes | 977 ms | 955 ms | 931 ms |

Both series load all 30 packages without errors. The median reduction is about
60%. Complete process times after the changes are 1257.74, 1117.42 and 1090.36 ms.
The later diagnostic baseline was 2607/2711/2518 ms; logging and machine load
contribute variation. Results do not establish single-digit-millisecond startup.

## Removed startup paths

- The loader's all-entry UTF-8 snapshot. Registration reads requested modules
  from the same extracted revision cache used by normal runtime resource reads.
  Missing or changed caches still require extraction; unchanged caches are reused.
- Full archive installation-seal verification and record enumeration per external
  package. Startup loads sources from installation records and reads the ZIP
  index for source classification. Authentication remains at import/update boundaries.
  Unregistered files in package directories are not automatically installed by startup.
- Per-package ownership catalog reads and indirect environment owner resolution.
  Both use the installation snapshot captured at scan start; first-run built-in
  registration and source/scope conflict checks remain.
- Temporary registration-engine creation/destruction. Registration uses the
  existing four-slot package executor pool; sequential registration reuses one
  engine. Stateful package main/page contexts retain their explicit lifetimes.
- Unused archive-loader wrappers and the separate per-resource archive read path.

Pooled registration exposed a route installation bug: replacing `ToolPkg` left
the old route-install marker set. Route state now belongs to the actual API
object, so replacement installs the correct route API and clears obsolete state.
A regression registers 24 owners through one engine, includes a failed owner,
and checks API availability, capture separation and engine identity.

The original detailed sample spent 736 ms on text snapshots, 474 ms executing
registration entries, 406 ms classifying external packages, 293 ms preparing
ownership and 156 ms creating registration engines. Post-change ownership takes
4–7 ms; JS registration and revision fingerprints remain substantial work.
Registration already skips `.ui.js` execution via placeholders; UI import logs
alone were not evidence of UI evaluation.

Local raw logs and measurements are retained under ignored
`target/extension-startup-profile/`: `thirty`, `thirty-detailed`, `after-routing`
for the comparable workload; `exact-thirty` and `exact-thirty-cache-create` for
the final 30-plugin workload. The profiling script and build/test logs are
retained there too. Only five registry phase timers remain in runtime code.

Validation: CLI build succeeded; 10 JavaScript registration tests passed,
including the pooled owner/route regression; all 23 local package update tests
passed, including registration failure and publication rollback. Fault injection
now observes registration calls instead of assuming each call creates an engine.

## Async conversion algorithm

Commit `432945f6bbf2a1998c6ad2192c5371bcac2aedb0` introduced the common Oxc
lowering boundary on November 14. It makes native async operations use the
runtime's owned Promise continuations, preserves original serialized callback
source and async route method kinds, and protects plugin-authored Babel bindings.
Those contracts remain necessary across execution contexts and platforms.

The final change keeps the full lexical scope analysis, then removes synchronous
top-level statements from the transformation input. Only statements containing
async code and synthesized helpers are transformed and generated. Untouched
statements are copied from their original source in their original order.
Directive prefixes, statement boundaries and complete scope information remain
available, including synchronous bindings that could collide with generated
helper names. Completely synchronous inputs skip semantic analysis, lowering
and source generation after parsing. Semantic vectors grow without a separate
whole-tree capacity-counting pass. No converted-source disk cache or package
precompilation was added.

An alternating 25-pair comparison on the actual 574,515-byte character-card
entry measured the original conversion at a **125 ms median** and selective
conversion at **110 ms**, a 12% reduction, in the same Windows debug benchmark.
Every sample performs conversion again. Raw results are in
`target/extension-startup-profile/algorithm/selective-debug.log`. Output sizes
were 832,223 and 814,835 bytes. A subsequent small cleanup borrows original
callback text rather than making an extra arena copy.

An earlier source-position design reduced output size but did not reduce the
conversion median; that design, its factory ABI changes and dependency feature
changes were removed. Optimized native conversion of the original algorithm
was about 13 ms; this is a build-optimization difference, not an algorithm gain.

The last complete CLI sample before selective conversion loaded all 30 plugins
without issues: registry 1105/1113/1166/1088/1052 ms (median **1105 ms**) and
process 1291.77/1276.31/1360.77/1244.51/1235.73 ms (median **1276.31 ms**).
These samples are under `algorithm-final-thirty`; they do not establish the
final selective algorithm's whole-application timing. Following the requested
stop to further investigation, the final source was validated through the bridge
tests without another complete CLI rebuild and benchmark cycle.

Final bridge validation: **162 passed, 4 ignored**, with one existing bundled
character WebView fixture excluded. That fixture also fails with the original
compiler: it assumes the former WebView management screen, while the current
plugin renders native management. Added regressions exercise original Unicode
callback source across later compilations, generated-name collisions with
untouched bindings, synchronous source preservation, statement order and strict
function directives. The broad Node run had 45 passing tests, three failures
(a fixture missing the separately registered package-import binding and two
Linux-shell tests requiring `/bin/bash`) and four skips on Windows.
