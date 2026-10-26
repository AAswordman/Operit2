# Device scene configuration

The native `edge.scene` service owns bounded indexed assets, durable display
configuration, strip painting and local touch hit testing. It has no JS runtime.
Device applications construct `SceneService`, attach a RuntimeStorageHost with
atomic writes, route native calls, and handle returned touch targets locally.

Configuration and referenced resources are saved as one atomic device snapshot.
After restart the device restores its configuration; disabling the host editor
has no effect on device operation. System exit stops the current scene, while
config.clear also clears the saved display configuration.

See plugins/docs/edge-scene.md for the wire and asset formats. Run:

```sh
cargo test --manifest-path hosts/common/operit-edge-scene/Cargo.toml --lib
```
