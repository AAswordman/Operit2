# Optional Edge scene service

This device-side library implements the bounded `display.scene` service, indexed
asset parsing, strip painting and an adapter to generic Edge action events.
It contains no ESP32 driver. Device applications opt in by depending on this
crate, registering `ScenePlugin` and connecting `paint_strip` to their display.
Core and the common Edge node/contract crates do not depend on this library.
A sensor-only node does not need to build or register it.

The application supplies dimensions, the height of its reserved system bar and
touch capability with `ScenePlugin::new(width, height, reserved_top, touch)`.
A device without a top system bar may pass zero. The ESP32 application chooses
320×240 with a 24-pixel system bar; other applications choose their own geometry
and retain responsibility for system UI priority and an exit mechanism.

The current bounded RGB565/indexed-asset implementation has the documented fixed
memory/format limits. These limits belong to this optional implementation, not
to the Core event receiver or a requirement that every Edge use this renderer.
The ESP1/ESI1 magic bytes remain unchanged for asset compatibility; they do not
restrict the protocol to ESP32. Other renderers may register their own services.

See `plugins/docs/edge-scene.md` for the wire protocol and
`plugins/docs/edge-events.md` for the generic event envelope.

## Native Integration Migration

Device applications previously using `operit_node_edge::scene::ScenePlugin`
must explicitly depend on this crate and use `operit_edge_scene::ScenePlugin`.
Pass the device-owned system bar height as the new third constructor argument.
Scene-specific Rust contract types are now in `operit_edge_scene::protocol`,
not `operit_edge_contract::scene`. JS ToolPkg calls and the wire protocol are
unchanged; non-display Edge applications need no renderer dependency.

Run the renderer and protocol tests from the repository root:

```powershell
cargo test --locked --manifest-path hosts/common/operit-edge-scene/Cargo.toml --lib
```
