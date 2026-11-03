# Compute benchmark

`compute.c` exercises integer dependencies, scalar FMA, square root, NEON FMA,
large copies, and fork/exec. On an ARM64 macOS host with an Alpine aarch64
sysroot containing development headers and libraries, the same source can be
built for both targets. Run these commands from the repository root, replacing
the runtime and rootfs paths with local ones:

```sh
CLANG=/opt/homebrew/opt/llvm/bin/clang
LLD=/opt/homebrew/bin/ld.lld
ROOT=/path/to/alpine-rootfs
RUNTIME=/path/to/operit-arm64
BENCH=$(mktemp -d)
SRC=hosts/ios/linux-runtime/benchmarks/compute.c

"$CLANG" --target=aarch64-linux-musl --sysroot="$ROOT" --ld-path="$LLD" \
  -O2 -ffp-contract=off -Wall -Wextra -Werror "$SRC" -lm -o "$BENCH/compute-guest"
"$CLANG" -isysroot "$(xcrun --show-sdk-path)" \
  -O2 -ffp-contract=off -Wall -Wextra -Werror "$SRC" -lm -o "$BENCH/compute-native"

for mode in integer fma sqrt neon memcpy fork; do
  "$BENCH/compute-native" "$mode"
  "$RUNTIME" --root "$ROOT" --fixed-acceleration --bind-ro "$BENCH:/bench" \
    --max-steps 10000000000 -- /bench/compute-guest "$mode"
done
```

Repeat and alternate runtime versions before comparing medians. Check the
reported result values and `valid=1` for copies. Fixed helpers only accelerate
the authenticated musl versions supported by the runtime; unrecognized images
fall back to the interpreter. The CLI requires `--fixed-acceleration` to enable
them. The native and guest `fork` workloads execute different platform binaries
(`/usr/bin/true` and `/bin/true` respectively), so cross-platform fork timing is
only indicative. Native and guest compilers can also emit different code even
when given the same source and optimization flags.

## Measurement Snapshot

On 2026-11-04, an Apple M5 macOS host ran the source above with Clang 23.1.2
(`-O2 -ffp-contract=off`). The runtime was a Release build (`-O3`, LTO off),
and the Alpine musl image matched the installed product rootfs. Each row is
the median of three runs, alternating the baseline and optimized runtimes.
The baseline already contained the FMADD decoding correctness fix but not the
subsequent optimizations. Times are seconds; lower is better.

| Workload | Baseline | Optimized | Speedup | macOS native |
| --- | ---: | ---: | ---: | ---: |
| Integer dependencies, 100 million | 0.944440 | 0.728192 | 1.30x | 0.112218 |
| Scalar FMA, 100 million | 7.638453 | 3.972074 | 1.92x | 0.179423 |
| Square root, 20 million | 1.541271 | 1.145994 | 1.34x | 0.009611 |
| NEON FMA, 81.92 million elements | 1.459269 | 1.246272 | 1.17x | 0.033121 |
| `memcpy`, 64 MiB x 10 | 0.458461 | 0.086573 | 5.30x | 0.013576 |
| `fork` + `exec`, 60 children | 0.111414 | 0.080649 | 1.38x | 0.059975 |

These are microbenchmarks, not estimates for npm, compression, or arbitrary
applications. The optimized copy result requires an authenticated musl image;
other libraries use the interpreter. Native and guest use different ABIs and
may use different instructions, even with the same source. The fork comparison
also runs different platform executables. These results do not cover an app
build or an iOS device.
