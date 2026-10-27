/** Owns function lookup and durable references for every ToolPkg CommonJS module. */
(function(root) {
  if (root.OperitToolPkgExports) return;
  var own = Object.prototype.hasOwnProperty;
  function isNamespace(value) { return value !== null && (typeof value === 'object' || typeof value === 'function'); }
  /** Resolves only an actual module export; a callable CommonJS module is its default export. */
  function get(namespace, name) {
    if (!isNamespace(namespace)) return null;
    if (own.call(namespace, name) && typeof namespace[name] === 'function') return namespace[name];
    return name === 'default' && typeof namespace === 'function' ? namespace : null;
  }
  function requireFunction(namespace, name) {
    var fn = get(namespace, name);
    if (!fn) throw new Error('ToolPkg module export is not a function: ' + name);
    return fn;
  }
  function names(namespace) {
    if (!isNamespace(namespace)) return [];
    var result = Object.getOwnPropertyNames(namespace).filter(function(name) { return typeof namespace[name] === 'function'; });
    if (typeof namespace === 'function' && result.indexOf('default') < 0) result.unshift('default');
    return result;
  }
  function nameOf(namespace, fn) {
    var keys = names(namespace);
    for (var i = 0; i < keys.length; i++) if (get(namespace, keys[i]) === fn) return keys[i];
    return '';
  }
  /** Tags callable modules and namespace members with the same module/export identity. */
  function tag(path, namespace) {
    if (!isNamespace(namespace)) return;
    namespace.__operit_toolpkg_module_path = path;
    names(namespace).forEach(function(name) {
      var fn = get(namespace, name);
      fn.__operit_toolpkg_module_path = path;
      fn.__operit_toolpkg_export_name = name;
    });
  }
  function reference(value, label) {
    var fn = typeof value === 'function' ? value : get(value, 'default');
    var path = fn && fn.__operit_toolpkg_module_path;
    var name = fn && fn.__operit_toolpkg_export_name;
    if (typeof path !== 'string' || !path.trim() || typeof name !== 'string' || !name.trim()) {
      throw new Error(label + ' requires an exported ToolPkg function reference');
    }
    return { path: path.trim().replace(/\\/g, '/'), name: name };
  }
  /** Captures named/default UI references without running screen code during registration. */
  function placeholder(path) {
    var exports = Object.create(null);
    function exported(name) {
      if (!own.call(exports, name)) {
        var fn = function() { throw new Error('Registration screen references cannot be executed'); };
        fn.__operit_toolpkg_module_path = path;
        fn.__operit_toolpkg_export_name = name;
        exports[name] = fn;
      }
      return exports[name];
    }
    exported('default');
    return new Proxy(exports, { get: function(_target, name) {
      if (name === '__esModule') return true;
      return typeof name === 'string' ? exported(name) : undefined;
    } });
  }
  root.OperitToolPkgExports = { get: get, require: requireFunction, names: names, nameOf: nameOf, tag: tag, reference: reference, placeholder: placeholder };
})(globalThis);
