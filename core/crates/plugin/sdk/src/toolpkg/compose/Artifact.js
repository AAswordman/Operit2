/// Owns the generated-code ABI shared by build tooling, compiler and page loader.
var OperitComposeArtifact = {
  version: '4',
  prefix: '// @operit-compose-artifact ',
  /// Accepts only an exact versioned header, without inspecting arbitrary script strings.
  read: function(source) {
    if (!source.startsWith(this.prefix)) return null;
    var end = source.indexOf('\n');
    var version = end < 0 ? '' : source.slice(this.prefix.length, end);
    if (version !== this.version) {
      throw new Error('Unsupported Compose artifact version; rebuild the plugin with this SDK');
    }
    return source;
  }
};
