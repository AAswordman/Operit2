{{flutter_js}}
{{flutter_build_config}}

// Load the exact engine font resources staged and verified by the build.
_flutter.loader.load({
  config: { fontFallbackBaseUrl: new URL("fonts/", document.baseURI).href },
});
