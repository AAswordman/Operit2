# Device scene configuration

The native `edge.scene` service owns bounded indexed assets, durable display
configuration, strip painting and local touch hit testing. It has no JS runtime.
Device applications construct `SceneService`, attach a RuntimeStorageHost with
atomic writes, route native calls, and handle returned touch targets locally.

Configuration and referenced resources are saved as one atomic device snapshot.
After restart the device restores its configuration; disabling the host editor
has no effect on device operation. System exit stops the current scene, while
config.clear also clears the saved display configuration.

`config.show` validates and displays a configuration in RAM without writing
storage or replacing the saved scene. Devices advertise
`configuration.transient: true`. Frequent task status updates use this path;
a reboot restores the previous persistent configuration.

`SceneService::new` retains the default 4096-byte pack and 8192-byte cache
budgets. Products may choose bounded `SceneLimits` via `new_with_limits` before
storage restoration. The same limits govern uploads, capabilities and restored
snapshots. Board budgets belong in the product composition root; this SDK owns
only the generic ESP1/ESI1/ESR1 formats and renderer. Task hooks, mood selection
and artwork generation belong in the host ToolPkg.

See plugins/docs/edge-scene.md for the wire and asset formats. Run:

```sh
cargo test --manifest-path edge/sdk/scene/Cargo.toml --lib
```
