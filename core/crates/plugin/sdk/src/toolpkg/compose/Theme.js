/// Validates immutable host theme snapshots and delivers changes.
var OperitComposeTheme = function () {
  function makeColorToken(name, alpha) {
    return {
      __colorToken: name,
      alpha: alpha,
      copy: function (options) {
        return makeColorToken(name, options && typeof options.alpha === 'number' ? options.alpha : alpha);
      }
    };
  }
  function create(runtime, options) {
    var colorSchemeNames = ['primary', 'onPrimary', 'primaryContainer', 'onPrimaryContainer', 'secondary', 'onSecondary', 'secondaryContainer', 'onSecondaryContainer', 'tertiary', 'onTertiary', 'tertiaryContainer', 'onTertiaryContainer', 'error', 'onError', 'errorContainer', 'onErrorContainer', 'background', 'onBackground', 'surface', 'onSurface', 'surfaceVariant', 'onSurfaceVariant', 'outline', 'outlineVariant', 'inverseSurface', 'inverseOnSurface', 'inversePrimary', 'surfaceTint', 'scrim', 'shadow', 'surfaceDim', 'surfaceBright', 'surfaceContainerLowest', 'surfaceContainerLow', 'surfaceContainer', 'surfaceContainerHigh', 'surfaceContainerHighest', 'primaryFixed', 'primaryFixedDim', 'onPrimaryFixed', 'onPrimaryFixedVariant', 'secondaryFixed', 'secondaryFixedDim', 'onSecondaryFixed', 'onSecondaryFixedVariant', 'tertiaryFixed', 'tertiaryFixedDim', 'onTertiaryFixed', 'onTertiaryFixedVariant'];
    var colorScheme = {};
    for (var c = 0; c < colorSchemeNames.length; c += 1) {
      colorScheme[colorSchemeNames[c]] = makeColorToken(colorSchemeNames[c]);
    }
    var themeSnapshot;
    var themeListeners = new Set();
    /** Validates and detaches one resolved UI host theme. */
    function readThemeSnapshot(value) {
      if (!value || value.brightness !== 'light' && value.brightness !== 'dark' || !value.colors || typeof value.colors !== 'object' || Array.isArray(value.colors)) {
        throw new Error('Theme requires a resolved UI host snapshot');
      }
      var colors = Object.assign({}, value.colors);
      Object.keys(colors).forEach(function (role) {
        if (typeof colors[role] !== 'string' || !/^#[0-9a-fA-F]{8}$/.test(colors[role])) {
          throw new Error('Invalid Theme color: ' + role);
        }
      });
      return Object.freeze({
        brightness: value.brightness,
        colors: Object.freeze(colors)
      });
    }
    if (options.theme !== undefined) themeSnapshot = readThemeSnapshot(options.theme);
    /** Delivers host theme changes through the existing UI action lifecycle. */
    runtime.actions.set("__operit_theme_changed", async function (value) {
      var next = readThemeSnapshot(value);
      if (JSON.stringify(next) === JSON.stringify(themeSnapshot)) return;
      themeSnapshot = next;
      OperitComposeReactive.invalidateRoot(runtime.ctx);
      for (var listener of Array.from(themeListeners)) await listener(next);
    });
    return {
      api: {
        /** Returns the immutable snapshot owned by this UI context. */
        getCurrent: function () {
          if (!themeSnapshot) throw new Error('Theme is unavailable outside a configured UI host');
          return themeSnapshot;
        },
        /** Registers a listener until explicitly removed or the UI context is released. */
        subscribe: function (listener) {
          if (typeof listener !== 'function') throw new TypeError('Theme listener must be a function');
          themeListeners.add(listener);
          return function () {
            themeListeners.delete(listener);
          };
        }
      },
      material: {
        colorScheme: colorScheme
      },
      update: function (value) {
        themeSnapshot = readThemeSnapshot(value);
      }
    };
  }
  return {
    create: create
  };
}();
