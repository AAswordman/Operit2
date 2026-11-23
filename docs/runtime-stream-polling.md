# Runtime delivery, polling and ANR diagnostics

Batching is a transport operation over encoded, ready frames. The FFI writer
waits for the first ready item, drains up to 64 available items and hands their
frames to Dart or the wire queue. It never waits for a full batch, a Core call,
a watch open, source conversion or a cancellation response. Calls and watches
share this bounded outbox; producers await capacity without holding a delivery
lock. The writer is the sole normal delivery caller. Admission errors can be
posted directly through the same thread-safe, nonblocking platform sink.

Each watch encodes on its own source continuation before entering the outbox.
The browser also receives pre-encoded deliveries and submits cancellation without
awaiting the owner inside its delivery loop. A browser callback must return;
cooperative execution cannot preempt synchronous JavaScript.

The connection owner handles commands, generations and completion notifications.
Both owner and native writer use movable coordination tasks. Core opens remain
affine because the shared interface returns `!Send` futures. Their `Send` results
cross a channel to independently movable source polls on existing blocking
capacity. Ordinary calls continue to share coordination scheduling: an async call
must yield rather than block its worker.

The application-update check now submits synchronous installation discovery and
cache filesystem work through `scheduleHostRuntimeTask`, then awaits a oneshot.
Cache cleanup reads the Host's package-retention policy; it no longer requests
Android's installed APK through a synchronous owner interaction. Actual package
access remains part of update preparation when its bytes are needed.

`scheduleHostRuntimePooledTask` accepts an optional affine opener. Native Hosts
atomically reserve both stages on blocking capacity when present; without one,
movable work uses coordination capacity. Rejection starts neither stage. Wakes
coalesce, polls never overlap, and polling/destruction holds no queue lock.
Queued, executing and suspended tasks count toward each pool's 4,096-entry limit.
Worker counts and bounded connection backpressure are unchanged. Web Hosts
explicitly admit work on their cooperative executor.

The remaining scheduler mutexes protect short queue bookkeeping; no user poll,
platform callback or asynchronous wait runs under them. One blocked synchronous
poll still occupies its worker. Movable tasks can resume on other workers, but
suspended affine futures cannot migrate. Finite pools cannot guarantee progress
if every eligible worker blocks. This is containment, not an async replacement
for synchronous waits.

A native `operit-anr` observer samples active polls every 100 ms independently
of worker execution. `HostRuntimeANR` reports a poll exceeding 2,000 ms once,
with task, thread, elapsed time and `phase=polling|returned`. Openers include
`phase=open` in their names. Idle `Pending` subscriptions produce no ANRs.

Regression cases cover production owner progress during a blocked open, source
conversion, cancellation, bounded admission, worker migration, wake coalescing
and Android cache cleanup without owner interaction. Runtime/device verification
is still needed to identify the original startup stall conclusively.

The transport fixture imports `BridgeTransport.rs`, `FfiTransport.rs` and
`BridgeCodec.rs` directly and uses the native Host scheduler. Only the business
Core, bridge lifecycle shell and final frame receiver are scripted. It does not
run Dart, the VM port, a socket writer or the full application lifecycle.

```powershell
cargo test --locked --manifest-path apps/flutter/native/operit-flutter-bridge/tests/transport-fixture/Cargo.toml --target-dir core/target --lib -- --test-threads=1 --nocapture
cargo test --locked --manifest-path hosts/common/operit-host-native-scheduler/Cargo.toml --no-default-features --lib -- --test-threads=1
cargo test --locked --manifest-path core/Cargo.toml -p operit-util --test incremental_updates startup_cleans_only_obsolete_update_files -- --exact
```

The mixed scenario holds a synchronous call and a mapped-source poll blocked
for at least 2.4 seconds, opens four more watches while blocked, and verifies
eight healthy watches, a sporadic watch, ordinary call replies and cancellation
acknowledgements keep arriving. Sequence assertions detect loss, duplication or
reordering. Before releasing either blocker, it requires exactly their two ANR
reports and no idle/delivery ANR. It then checks both recover and all eight
healthy subscriptions acknowledge closing and deliver terminal frames.

The ignored Release benchmark exercises the same production transport with
bounded sources (64 queued items and at most 16 outstanding deliveries per
source), validates every source sequence, and injects ordinary calls during
delivery. Run it explicitly:

```powershell
cargo test --locked --release --manifest-path apps/flutter/native/operit-flutter-bridge/tests/transport-fixture/Cargo.toml --target-dir core/target --lib concurrent_stream_throughput -- --ignored --nocapture --test-threads=1
```

On Windows with an AMD Ryzen 9 7940HX (16 cores, 32 threads), three complete
runs passed all 27 cases. Each case delivered 500,000 events, except 64 KiB
text cases, which delivered 32,768 events (2 GiB of logical text). Medians
across the three runs were:

| Streams | Payload | Events/s | Text MiB/s | Delivery P99 (ms) | Call P99 (ms) | Mean batch size |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 KiB text | 315,856 | 308.5 | 1 | 1 | 4.3 |
| 8 | 1 KiB text | 481,232 | 470.0 | 1 | 1 | 58.6 |
| 32 | 1 KiB text | 428,449 | 418.4 | 3 | 1 | 44.2 |
| 1 | 64 KiB text | 44,522 | 2,782.6 | 1 | 1 | 5.1 |
| 8 | 64 KiB text | 39,056 | 2,441.0 | 4 | 4 | 63.5 |
| 32 | 64 KiB text | 38,236 | 2,389.7 | 25 | 4 | 63.3 |
| 1 | 64 KiB shared binary | 309,598 | — | 1 | 1 | 5.0 |
| 8 | 64 KiB shared binary | 460,405 | — | 1 | 1 | 60.7 |
| 32 | 64 KiB shared binary | 415,282 | — | 4 | 1 | 44.8 |

For eight 1 KiB text streams, throughput ranged from 480,307 to 483,559 events/s.
For 32 streams of 64 KiB text, delivery P99 ranged from 24 to 26 ms, and call
P99 from 4 to 5 ms. The largest observed batch was 64 items.

Latency runs from producer submission to entry into the batch receiver and uses
the Host's millisecond clock. The receiver decodes frames in Rust, which also
affects throughput. Binary cases reuse a shared buffer and measure reference
delivery rather than generating or copying distinct 64 KiB buffers. These are
short, bounded-load desktop transport measurements; they exclude Dart decoding,
UI rendering, real socket output and mobile-device behavior.
