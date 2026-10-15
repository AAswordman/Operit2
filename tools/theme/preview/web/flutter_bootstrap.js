{{flutter_js}}
{{flutter_build_config}}

// Studio releases serve Flutter engine bytes from their own immutable origin.
// Do not fetch CanvasKit from a CDN or register a service worker for local
// authoring; both would make a supposedly local preview depend on remote or
// stale code outside the pinned renderer release.
_flutter.loader.load({
  config: {
    renderer: "canvaskit",
    useLocalCanvasKit: true,
  },
});
