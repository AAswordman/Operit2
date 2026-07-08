var PluginConfig = (function() {
    function normalizeName(value) {
        var name = String(value == null ? "config" : value).trim();
        if (!name) throw new Error("PluginConfig name is empty");
        return name;
    }

    function normalizeFileName(name) {
        return normalizeName(name)
            .replace(/[\\/:*?"<>|\x00-\x1F]/g, "_")
            .replace(/^\.+|\.+$/g, "") + ".json";
    }

    function assertDefaults(defaults) {
        if (!defaults || typeof defaults !== "object" || Array.isArray(defaults)) {
            throw new Error("PluginConfig defaults must be an object");
        }
    }

    function cloneJson(value) {
        return JSON.parse(JSON.stringify(value));
    }

    function activeRuntime() {
        var root = typeof globalThis !== "undefined" ? globalThis : this;
        var runtime = root && root.__operit_call_runtime_ref;
        if (!runtime || typeof runtime !== "object") {
            throw new Error("PluginConfig active runtime is unavailable");
        }
        return runtime;
    }

    function configDir() {
        var runtime = activeRuntime();
        if (typeof runtime.getPluginConfigDir !== "function") {
            throw new Error("PluginConfig runtime config directory API is unavailable");
        }
        var dir = String(runtime.getPluginConfigDir() || "")
            .replace(/\\/g, "/")
            .replace(/\/+$/g, "");
        if (!dir) throw new Error("PluginConfig directory is empty");
        return dir;
    }

    function logWriteError(name, path, error) {
        var message = "PluginConfig write error: name=" + name +
            ", path=" + path +
            ", error=" + (error && error.message ? error.message : String(error));
        if (typeof console !== "undefined" && console && typeof console.error === "function") {
            console.error(message);
        } else if (typeof NativeInterface !== "undefined" && NativeInterface && typeof NativeInterface.logError === "function") {
            NativeInterface.logError(message);
        }
    }

    async function loadValues(path, defaults) {
        var values = cloneJson(defaults);
        var existsResult = await Tools.Files.exists(path);
        if (!existsResult.exists) return values;

        var readResult = await Tools.Files.read(path);
        var raw = String(readResult && readResult.content != null ? readResult.content : "").trim();
        if (!raw) throw new Error("PluginConfig file is empty: " + path);

        var parsed = JSON.parse(raw);
        if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
            throw new Error("PluginConfig file must contain an object: " + path);
        }
        Object.keys(parsed).forEach(function(key) {
            values[key] = parsed[key];
        });
        return values;
    }

    function createProxy(name, path, dir, values) {
        var writeScheduled = false;

        async function writeNow() {
            var mkdirResult = await Tools.Files.mkdir(dir, true);
            if (mkdirResult && mkdirResult.successful === false) {
                throw new Error(mkdirResult.details || "PluginConfig mkdir failed: " + dir);
            }
            var writeResult = await Tools.Files.write(path, JSON.stringify(values), false);
            if (writeResult && writeResult.successful === false) {
                throw new Error(writeResult.details || "PluginConfig write failed: " + path);
            }
        }

        function scheduleWrite() {
            if (writeScheduled) return;
            writeScheduled = true;
            Promise.resolve().then(function() {
                writeScheduled = false;
                return writeNow();
            }).catch(function(error) {
                logWriteError(name, path, error);
            });
        }

        return new Proxy(values, {
            get: function(target, prop) {
                return target[prop];
            },
            set: function(target, prop, value) {
                target[prop] = value;
                scheduleWrite();
                return true;
            },
            deleteProperty: function(target, prop) {
                delete target[prop];
                scheduleWrite();
                return true;
            }
        });
    }

    return {
        use: async function(nameOrDefaults, maybeDefaults) {
            var hasExplicitName = arguments.length > 1;
            var name = hasExplicitName ? normalizeName(nameOrDefaults) : "config";
            var defaults = hasExplicitName ? maybeDefaults : nameOrDefaults;
            assertDefaults(defaults);

            var dir = configDir();
            var path = dir + "/" + normalizeFileName(name);
            var values = await loadValues(path, defaults);
            return createProxy(name, path, dir, values);
        }
    };
})();

__operitExpose('PluginConfig', PluginConfig);
