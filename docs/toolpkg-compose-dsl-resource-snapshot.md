# Compose DSL resource snapshots

Compose DSL screens execute while the Core dispatcher owns the `RuntimePackageManager` mutex. A screen module may load another package file through `require`, which previously called the JavaScript host and tried to acquire that same mutex again. Native builds then had one thread waiting for the JavaScript worker while the worker waited for the mutex; single-threaded Web builds have the same reentry hazard.

Before a Compose render starts, `RuntimePackageManager` builds an immutable map of every UTF-8 entry in the container's extracted package cache. The render stores this map on its page-owned JavaScript engine. Render, rerender, and action execution expose it to `NativeInterface.readToolPkgTextResource`, so CommonJS module resolution reads the map and does not call back into the package manager.

The map lives with the page execution engine and is released when that engine is released. It contains only text entries; binary package resources continue to use their dedicated materialization API.

After Core returns a Compose result, the Flutter launcher parses the result on its UI isolate. The result is ordinary JSON and should not be sent through `Isolate.run`: closures created from a `State` method capture the widget tree, which Dart correctly rejects as an unsendable isolate message. This also keeps Compose DSL result handling on the same path for native builds and single-threaded Web builds. The Rust host remains responsible for executing JavaScript outside Flutter's UI work.

Diagnostics use these messages:

- `ToolPkg compose-render-start` and `compose-render-finish` include context, package, entry count, elapsed time, and result state.
- `OperitQuickJsEngine compose-request-start` and `compose-request-finish` identify the worker phase and snapshot entry count.
- The Flutter launcher logs render start, completion, failure, and elapsed time. It also logs Compose action start, received event count, each event phase, and failures without logging the event payload.

The JavaScript bridge regression test `compose_dsl_resource_snapshot_avoids_host_reentry_for_render_and_action` proves that a screen using `require("../shared")` succeeds for both render and action while the host text-resource read counter remains zero.
