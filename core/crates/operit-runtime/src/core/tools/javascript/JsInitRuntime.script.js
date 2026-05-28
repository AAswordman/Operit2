(function() {
    var root = typeof globalThis !== 'undefined'
        ? globalThis
        : (typeof window !== 'undefined' ? window : this);
    var windowRef = typeof window !== 'undefined' ? window : root;

    function expose(name, value) {
        var key = name == null ? '' : String(name).trim();
        if (!key || value === undefined) {
            return;
        }
        root[key] = value;
        windowRef[key] = value;
    }

    expose('__operitExpose', expose);
    root.window = windowRef;
})();
