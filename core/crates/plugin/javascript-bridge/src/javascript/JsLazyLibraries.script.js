/** Installs trusted libraries on first export access within their owning runtime. */
function __operitInstallBootstrapLibraries(libraries) {
    libraries.forEach(
    /** Owns one library initialization state without sharing it across runtimes. */
    function (library) {
        var name = library[0];
        var exports = library[1];
        var initializing = false;
        var failed = false;
        var failure;

        /** Initializes all exports once and preserves an exact failure without retrying. */
        function load() {
            if (failed) throw failure;
            if (initializing) throw new Error('Circular bootstrap library access: ' + name);
            initializing = true;
            try {
                var source = __operitNativeReadBootstrapLibrary(name);
                (0, eval)(source);
                exports.forEach(
                /** Requires all declared exports to be installed before initialization completes. */
                function (key) {
                    var descriptor = Object.getOwnPropertyDescriptor(globalThis, key);
                    if (!descriptor || !Object.prototype.hasOwnProperty.call(descriptor, 'value') || descriptor.value === undefined) {
                        throw new Error('Bootstrap library did not expose ' + key + ': ' + name);
                    }
                });
            } catch (error) {
                failed = true;
                failure = error;
                exports.forEach(install);
                throw error;
            } finally {
                initializing = false;
            }
        }

        /** Replaces a lazy accessor with the exact public value supplied by the library or caller. */
        function publish(key, value) {
            Object.defineProperty(globalThis, key, {
                configurable: true, enumerable: true, writable: true, value: value
            });
        }

        /** Keeps public lookup and assignment synchronous while avoiding eager compilation. */
        function install(key) {
            Object.defineProperty(globalThis, key, {
                configurable: true,
                enumerable: true,
                /** Loads the complete library before exposing any requested export. */
                get: function () {
                    load();
                    return globalThis[key];
                },
                /** Preserves the ordinary global assignment contract. */
                set: function (value) { publish(key, value); }
            });
        }
        exports.forEach(install);
    });
}
