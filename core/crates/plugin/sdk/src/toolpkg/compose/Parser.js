/// Keeps the trusted parser source outside the eager page runtime.
var __operitAcorn = function (readParserSource) {
  var parser = null;
  var initializing = false;
  var failed = false;
  var failure;
  return {
    /// Initializes one parser per engine and preserves exact initialization failures.
    parse: function (source, options) {
      if (failed) throw failure;
      if (initializing) throw new Error('Compose parser initialization was interrupted');
      if (parser === null) {
        initializing = true;
        try {
          var exports = {};
          new Function('exports', 'module', readParserSource())(exports, {
            exports: exports
          });
          if (typeof exports.parse !== 'function') throw new Error('Compose parser initialization failed');
          parser = exports;
        } catch (error) {
          failed = true;
          failure = error;
          throw error;
        } finally {
          readParserSource = null;
          initializing = false;
        }
      }
      return parser.parse(source, options);
    }
  };
}(__OPERIT_COMPOSE_PARSER_SOURCE__);
