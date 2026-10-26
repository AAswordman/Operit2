# Markdown native pipeline benchmark

Run from the repository root on macOS, with the project's Flutter dependencies installed:

```sh
python3 tools/benchmarks/markdown_pipeline.py
```

The runner builds the native fixture in **Rust release** mode and launches the
Flutter integration benchmark in **profile/AOT** mode. The report records both
modes and the runner rejects a result from another mode. Each case warms up once
and reports the median of five runs, including individual samples.

The measured path is:

1. Open a watch with fixed UTF-8 Markdown input and a Unicode character chunk size.
2. Parse chunks using `MarkdownRenderEventStream`, including its EOF flush.
3. Convert each semantic event with `toCoreValue` and enqueue a `CoreEvent`, as in
   `MessageProcessingDelegate`. Only root completion terminates the transport.
4. Use the **production source files**, included by the fixture without copying
   their implementations: `BridgeTransport`, `BridgeCodec`, and `FfiTransport`.
   These provide the stream owner, bounded delivery queue, frame encoding,
   batching, native scheduler, `DartPort` delivery and native buffer finalizers.
5. Receive through production `FfiCoreTransport`, `FfiCoreProxy`,
   `ProxyCoreRuntimeBridge.openEmbeddedCoreStream`, and generated
   `MarkdownStreamEvent.fromMessagePack`.
6. Apply events to `StreamMarkdownRendererState`, complete the node builder and
   create its final stable root node snapshot.

The application/provider source is a fixture. It replaces model generation,
network timing, conversation storage and application startup; it does not replace
the transport, parser, codec or root node builder. Wall time includes watch
opening and input-request encoding/decoding. Connection setup is warmed first.
The fixture's one-time ABI descriptor uses the production setup format, while
Markdown events use the production MessagePack frames.

The timer stops after stream completion and the final root node snapshot. Source
content equality, semantic event counts, native metrics requests and state cleanup
are checked outside the timed region. A separate diagnostic pass instruments
native parsing and CoreValue conversion; these concurrent stage times must not be
summed to estimate wall time.

This measures a burst input stream. It does not measure model token latency,
periodic widget snapshots, nested quote/table widget consumers, layout, painting,
animations or device FPS. The actual `StreamMarkdownRenderer` and its nested
consumers are mounted separately as a correctness smoke check. The original
`markdown_parse_bench` remains a parser-only microbenchmark for isolating parser
cost; its numbers are not native pipeline timings.

Output is written under `apps/flutter/app/build/markdown_benchmark/` by default:
`markdown_pipeline.json`, `flutter_profile.log`, and `native_build.log`.
Use `--output /absolute/path` to choose another output directory.
