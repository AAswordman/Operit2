/**
 * Preserves the copied v1 environment-variable signatures using the canonical v2 host methods.
 * @param {Pick<typeof import('../../../../../../../plugins/types/software_settings').SoftwareSettings, 'readEnvironmentVariable' | 'writeEnvironmentVariable'>} current
 * @param {() => string} packageName
 * @returns {Pick<typeof import('../../../../../../../plugins/types-v1/software_settings').SoftwareSettings, 'readEnvironmentVariable' | 'writeEnvironmentVariable'>}
 */
function __operitCreateV1SoftwareSettings(current, packageName) {
    return {
        /** Reads the exact configured variable and preserves the host result and rejection. */
        readEnvironmentVariable(key) {
            return current.readEnvironmentVariable(packageName(), key);
        },
        /** Writes through the canonical host without changing the legacy optional value contract. */
        writeEnvironmentVariable(key, value) {
            return current.writeEnvironmentVariable(packageName(), key, value);
        },
    };
}

/**
 * Adapts historical one-shot system speech to the canonical playback request.
 * @param {(request: import('../../../../../../../plugins/types/tts').Tts.SpeakRequest) => Promise<import('../../../../../../../plugins/types/results').TtsSpeakResultData>} speak
 * @returns {typeof import('../../../../../../../plugins/types-v1/software_settings').SoftwareSettings.testTtsPlayback}
 */
function __operitCreateV1TtsPlayback(speak) {
    /** Preserves utterance overrides and reports the actual host start without changing saved settings. */
    return async function testTtsPlayback(text, options) {
        if (typeof text !== 'string' || !text.trim()) {
            throw new TypeError('testTtsPlayback requires nonblank text');
        }
        if (options !== undefined && (!options || typeof options !== 'object' || Array.isArray(options))) {
            throw new TypeError('testTtsPlayback options must be an object');
        }
        /** @type {import('../../../../../../../plugins/types/tts').Tts.SpeakRequest} */
        var request = { text: text };
        if (options !== undefined) {
            for (var key of Object.keys(options)) {
                if (key !== 'interrupt' && key !== 'speech_rate' && key !== 'pitch') {
                    throw new TypeError('Unknown testTtsPlayback option: ' + key);
                }
            }
            if (options.interrupt !== undefined) {
                if (typeof options.interrupt !== 'boolean') throw new TypeError('interrupt must be a boolean');
                request.interrupt = options.interrupt;
            }
            if (options.speech_rate !== undefined) {
                if (!Number.isFinite(options.speech_rate) || options.speech_rate <= 0) {
                    throw new TypeError('speech_rate must be a finite positive number');
                }
                request.speed = options.speech_rate;
            }
            if (options.pitch !== undefined) {
                if (!Number.isFinite(options.pitch) || options.pitch <= 0) {
                    throw new TypeError('pitch must be a finite positive number');
                }
                request.pitch = options.pitch;
            }
        }
        var result = await speak(request);
        if (result.providerType !== 'SYSTEM_TTS') {
            throw new Error('Unsupported v1 system speech provider: ' + result.providerType);
        }
        return {
            ttsServiceType: 'SIMPLE_TTS',
            providerClass: 'TtsPlaybackHost',
            initialized: true,
            playbackTriggered: result.started,
            interrupt: result.interrupt,
            textLength: text.length,
            speechRate: result.speed,
            pitch: result.pitch,
            /** Formats the historical structured playback receipt. */
            toString() { return JSON.stringify(this); },
        };
    };
}

/**
 * Adapts Android's synchronous SharedPreferences vocabulary to existing plugin configuration storage.
 * Values retain their Java type alongside JSON data so numeric getters cannot silently change types.
 * @param {(name: string, edits: Array<{op: 'set', key: string, value: {type: string, value: any}} | {op: 'remove', key: string}> | null, clear: boolean) => {values: Record<string, {type: string, value: any}>}} request
 */
function __operitCreateV1SharedPreferences(request) {
    /** Requires exact string identities, including empty keys supported by Android. @param {unknown} value */
    function text(value) {
        if (typeof value !== 'string') throw new TypeError('SharedPreferences names and keys must be strings');
        return value;
    }
    /** Validates a value without coercing or truncating persisted Java numeric types. @param {string} type @param {any} value */
    function checked(type, value) {
        switch (type) {
            case 'boolean': if (typeof value === 'boolean') return value; break;
            case 'string': if (typeof value === 'string') return value; break;
            case 'int': if (Number.isInteger(value) && value >= -2147483648 && value <= 2147483647) return value; break;
            case 'long': if (Number.isSafeInteger(value)) return value; break;
            case 'float': if (typeof value === 'number' && Number.isFinite(value) && Number.isFinite(Math.fround(value))) return Math.fround(value); break;
            case 'string_set':
                if (value && typeof value[Symbol.iterator] === 'function' && typeof value !== 'string') {
                    var strings = Array.from(value);
                    if (strings.every(function(item) { return typeof item === 'string'; })) return Array.from(new Set(strings));
                }
                break;
        }
        throw new TypeError('Invalid SharedPreferences ' + type + ' value');
    }
    /** Returns a defensive value, including a fresh set rather than mutable persisted arrays. @param {{type: string, value: any}} entry */
    function decoded(entry) {
        var value = checked(entry.type, entry.value);
        return entry.type === 'string_set' ? new Set(value) : value;
    }
    /** Opens a named store lazily; every read sees the current native snapshot across UI and tool engines. @param {string} name @param {number} mode */
    return function getSharedPreferences(name, mode) {
        text(name);
        // Accept the historical flag while retaining the existing config transport consistency policy.
        if (mode !== 0 && mode !== 4) throw new Error('SharedPreferences supports MODE_PRIVATE and MODE_MULTI_PROCESS');
        /** Reads a typed entry while distinguishing absence from false, zero and empty strings. @param {string} key @param {string} type @param {any} fallback */
        function get(key, type, fallback) {
            var values = request(name, null, false).values;
            if (!Object.prototype.hasOwnProperty.call(values, text(key))) return fallback;
            var entry = values[key];
            if (!entry || entry.type !== type) throw new TypeError('SharedPreferences type mismatch for ' + key + ': expected ' + type);
            return decoded(entry);
        }
        return {
            /** @param {string} key @param {boolean} fallback */
            getBoolean: function(key, fallback) { return get(key, 'boolean', fallback); },
            /** @param {string} key @param {string | null} fallback */
            getString: function(key, fallback) { return get(key, 'string', fallback); },
            /** @param {string} key @param {number} fallback */
            getInt: function(key, fallback) { return get(key, 'int', fallback); },
            /** @param {string} key @param {number} fallback */
            getLong: function(key, fallback) { return get(key, 'long', fallback); },
            /** @param {string} key @param {number} fallback */
            getFloat: function(key, fallback) { return get(key, 'float', fallback); },
            /** @param {string} key @param {Set<string> | null} fallback */
            getStringSet: function(key, fallback) { return get(key, 'string_set', fallback); },
            /** @param {string} key */
            contains: function(key) { return Object.prototype.hasOwnProperty.call(request(name, null, false).values, text(key)); },
            getAll: function() {
                var values = request(name, null, false).values;
                /** @type {Record<string, any>} */
                var result = Object.create(null);
                Object.keys(values).forEach(function(key) { result[key] = decoded(values[key]); });
                return result;
            },
            edit: function() {
                /** @type {Map<string, {op: 'set', key: string, value: {type: string, value: any}} | {op: 'remove', key: string}>} */
                var changes = new Map();
                var clear = false;
                /** Applies clear before every pending put/remove, as Android does regardless of call order. */
                function publish() {
                    request(name, Array.from(changes.values()), clear);
                    changes.clear();
                    clear = false;
                }
                /** Queues one typed mutation without publishing a partial editor. @param {string} key @param {string} type @param {any} value */
                function put(key, type, value) {
                    text(key);
                    if (value === null && (type === 'string' || type === 'string_set')) return editor.remove(key);
                    changes.set(key, { op: 'set', key: key, value: { type: type, value: checked(type, value) } });
                    return editor;
                }
                var editor = {
                    /** @param {string} key @param {boolean} value */
                    putBoolean: function(key, value) { return put(key, 'boolean', value); },
                    /** @param {string} key @param {string | null} value */
                    putString: function(key, value) { return put(key, 'string', value); },
                    /** @param {string} key @param {number} value */
                    putInt: function(key, value) { return put(key, 'int', value); },
                    /** @param {string} key @param {number} value */
                    putLong: function(key, value) { return put(key, 'long', value); },
                    /** @param {string} key @param {number} value */
                    putFloat: function(key, value) { return put(key, 'float', value); },
                    /** @param {string} key @param {Iterable<string> | null} value */
                    putStringSet: function(key, value) { return put(key, 'string_set', value); },
                    /** @param {string} key */
                    remove: function(key) { text(key); changes.set(key, { op: 'remove', key: key }); return editor; },
                    clear: function() { clear = true; return editor; },
                    // Persist before returning: a released page must not discard a pending apply timer.
                    apply: function() { publish(); },
                    commit: function() { try { publish(); return true; } catch (_) { return false; } },
                };
                return editor;
            },
        };
    };
}

/** Keeps the legacy Java facade and its persistence entirely inside the v1 adapter.
 * @param {Record<string, any>} runtime
 * @param {any} api
 */
function __operitInstallV1SharedPreferences(runtime, api) {
    var java = runtime.Java;
    if (!java || typeof java.getApplicationContext !== 'function') return;
    /** Uses the installation-owned synchronous config transport already used by getEnv/setEnv.
     * @type {Parameters<typeof __operitCreateV1SharedPreferences>[0]}
     */
    var configRequest = function(name, edits, clear) {
        if (name.length > 4096 || name.includes('\0')) throw new TypeError('SharedPreferences name is invalid');
        // Encode UTF-16 code units so empty names, Unicode and punctuation cannot collide.
        var key = '__OPERIT_V1_PREFS_';
        for (var i = 0; i < name.length; i++) key += name.charCodeAt(i).toString(16).padStart(4, '0');
        var callId = runtime.__operitCurrentCallId;
        var raw = runtime.__operitNativeGetEnvForCall(callId, key);
        var saved = raw == null || raw === '' ? {format: 1, values: Object.create(null)} : JSON.parse(raw);
        if (!saved || saved.format !== 1 || !saved.values || typeof saved.values !== 'object' || Array.isArray(saved.values)) {
            throw new Error('Invalid v1 preference configuration');
        }
        var values = clear ? Object.create(null) : saved.values;
        if (edits !== null) {
            for (var edit of edits) {
                if (edit.op === 'remove') delete values[edit.key];
                else Object.defineProperty(values, edit.key, {value: edit.value, enumerable: true, configurable: true, writable: true});
            }
            // One synchronous write publishes the whole editor; failure is visible to commit/apply.
            runtime.__operitNativeSetEnv(callId, key, JSON.stringify({format: 1, values: values}));
        }
        return {values: values};
    };
    var request = api.method().between('1.0.0', '2.0.0', configRequest).build('Java.SharedPreferences');
    var preferences = __operitCreateV1SharedPreferences(request);
    /** Reads the active API on every use, including retained Java context objects. */
    function legacy() {
        var version = api.currentVersion();
        return version === '1.0.0' || version === '1.0.1';
    }
    /** Adds only the historical context methods; all other Java calls retain the original proxy.
     * @param {any} context
     * @returns {any}
     */
    function wrap(context) {
        if (!context || !legacy()) return context;
        return new Proxy(context, {
            get: function(target, prop) {
                if (legacy() && prop === 'getSharedPreferences') return preferences;
                if (prop === 'getApplicationContext') return function() { return wrap(target.getApplicationContext()); };
                if (prop === 'call') return /** @param {string} method @param {...any} args */ function(method, ...args) {
                    if (legacy() && method === 'getSharedPreferences') return preferences(args[0], args[1]);
                    return method === 'getApplicationContext' ? wrap(target.call(method, ...args)) : target.call(method, ...args);
                };
                return Reflect.get(target, prop);
            },
        });
    }
    var applicationContext = java.getApplicationContext;
    // getContext() and Kotlin use this same Java object, so their existing aliases are preserved.
    java.getApplicationContext = function() { return wrap(applicationContext.apply(java, arguments)); };
}

(/**
 * Installs typed v1 adapters in the runtime's dynamic registration namespace.
 * @param {Record<string, any>} runtime
 */
function installV1Adapters(runtime) {
    var Tools = runtime.Tools;
    var api = runtime.__operitToolPkgApi;
    __operitInstallV1SharedPreferences(runtime, api);
    var currentSettings = Object.assign({}, Tools.SoftwareSettings);
    /** Resolves the native installation owner retained by the exact active call. */
    function environmentPackageName() {
        var state = runtime.__operitGetCallState(api.currentCallId());
        var owner = state.params.__operit_environment_owner;
        if (typeof owner !== 'string' || !owner.trim()) {
            throw new Error('Environment access requires an owning package installation');
        }
        return owner;
    }
    var legacySettings = __operitCreateV1SoftwareSettings(currentSettings, environmentPackageName);
    for (var [settingsMethod, settingsImplementation] of Object.entries(legacySettings)) {
        Tools.SoftwareSettings[settingsMethod] = api.method()
            .between('1.0.0', '2.0.0', settingsImplementation)
            .since('2.0.0', currentSettings[settingsMethod])
            .build('Tools.SoftwareSettings.' + settingsMethod);
    }
    var legacyTtsPlayback = api.method()
        .between('1.0.0', '2.0.0', __operitCreateV1TtsPlayback(
            /** Uses the shared tool pipeline while preserving the active invocation owner. */
            function(request) { return runtime.toolCall('tts_speak', { request: request }); },
        ))
        .build('Tools.SoftwareSettings.testTtsPlayback');
    Object.defineProperty(Tools.SoftwareSettings, 'testTtsPlayback', {
        configurable: true,
        /** Exposes the historical settings method only to admitted v1 calls. */
        get: function() {
            var version = api.currentVersion();
            return version === '1.0.0' || version === '1.0.1' ? legacyTtsPlayback : undefined;
        },
    });
    var currentFiles = Object.assign({}, Tools.Files);
    var legacyFiles = __operitCreateV1Files(currentFiles);
    for (var [method, filesImplementation] of Object.entries(legacyFiles)) {
        Tools.Files[method] = api.method()
            .between('1.0.0', '2.0.0', filesImplementation)
            .since('2.0.0', currentFiles[method])
            .build('Tools.Files.' + method);
    }
    var workflow = __operitCreateV1Workflow(
        /** Invokes the declared workflow prerequisite's published method. */
        function(method, payload) {
            return api.callDependency('com.operit.workflow', method, payload);
        },
    );
    Tools.Workflow = {};
    for (var [name, workflowImplementation] of Object.entries(workflow)) {
        Tools.Workflow[name] = api.method()
            .between('1.0.0', '2.0.0', workflowImplementation)
            .build('Tools.Workflow.' + name);
    }
    var characters = __operitCreateV1Characters(
        /** Invokes the declared role prerequisite's exact public domain method. */
        function(method, payload) {
            return api.callDependency('com.operit.character_cards', method, payload);
        },
    );
    var currentChat = Object.assign({}, Tools.Chat);
    var legacyChat = __operitCreateV1Chat(
        currentChat,
        /** Requires the actual role record before creating or sending a conversation. */
        function(id) {
            return api.callDependency('com.operit.character_cards', 'character.get', { id: id });
        },
        /** Preserves native ordinary-send controls while the adapter translates role identity. */
        function(params) {
            return runtime.toolCall('send_message_to_ai', params);
        },
        characters.listCharacterCards,
    );
    for (var [chatMethod, chatImplementation] of Object.entries(legacyChat)) {
        var chatBuilder = api.method().between('1.0.0', '2.0.0', chatImplementation);
        if (chatMethod !== 'listCharacterCards') chatBuilder.since('2.0.0', currentChat[chatMethod]);
        Tools.Chat[chatMethod] = chatBuilder.build('Tools.Chat.' + chatMethod);
    }
    var legacyMemory = __operitCreateV1Memory(
        /** Sends one package tool request with the active authenticated caller fields. */
        function(name, params) {
            var state = runtime.__operitGetCallState(api.currentCallId());
            if (state.params.__operit_package_caller_participant_id !== undefined) legacyCallerCardId();
            for (var key of ['__operit_package_caller_name', '__operit_package_chat_id', '__operit_package_caller_participant_id', '__operit_package_caller_owner']) {
                if (state.params[key] !== undefined) params[key] = state.params[key];
            }
            return runtime.toolCall('character_memory:' + name, params).then(
                /**
                 * Decodes the package executor's serialized JSON result.
                 * @param {unknown} serialized
                 */
                function(serialized) {
                    if (typeof serialized !== 'string') throw new Error('Package memory tool must return its serialized execution result');
                    return JSON.parse(serialized);
                },
            );
        },
    );
    Tools.Memory = {};
    for (var [memoryMethod, memoryImplementation] of Object.entries(legacyMemory)) {
        Tools.Memory[memoryMethod] = api.method()
            .between('1.0.0', '2.0.0', memoryImplementation)
            .build('Tools.Memory.' + memoryMethod);
    }
    /** Projects the role provider's authenticated participant into the historical card namespace. */
    function legacyCallerCardId() {
        var params = runtime.__operitGetCallState(api.currentCallId()).params;
        var participant = params.__operit_package_caller_participant_id;
        if (participant === undefined) return undefined;
        if (params.__operit_package_caller_owner !== 'com.operit.character_cards') {
            throw new Error('ToolPkg v1 getCallerCardId requires the character provider; another provider participant is not a card ID.');
        }
        return participant;
    }
    /**
     * Registers base64 image bytes and returns the historical link string.
     * @param {string} base64
     * @param {string} [mimeType]
     */
    function registerImageFromBase64(base64, mimeType) {
        if (typeof base64 !== 'string' || base64.trim() === '') {
            throw new Error('registerImageFromBase64 requires image data');
        }
        return __operitRegisterV1Image('base64', base64, mimeType);
    }
    /**
     * Registers an Android device image path and returns the historical link string.
     * @param {string} path
     */
    function registerImageFromPath(path) {
        return __operitRegisterV1Image('path', path, null);
    }
    /**
     * Exposes one historical global only while the active call uses API 1.0.0 or 1.0.1.
     * @param {string} name
     * @param {Function} implementation
     */
    function exposeLegacyFunction(name, implementation) {
        var gated = api.method().between('1.0.0', '2.0.0', implementation).build(name);
        Object.defineProperty(globalThis, name, {
            configurable: true,
            get: function() {
                var version = api.currentVersion();
                return version === '1.0.0' || version === '1.0.1' ? gated : undefined;
            },
        });
    }
    exposeLegacyFunction('registerImageFromBase64', registerImageFromBase64);
    exposeLegacyFunction('registerImageFromPath', registerImageFromPath);
    var legacyGetter = api.method().between('1.0.0', '2.0.0', legacyCallerCardId).build('getCallerCardId');
    Object.defineProperty(globalThis, 'getCallerCardId', {
        configurable: true,
        /** Exposes the historical getter only for admitted legacy contracts on the active call. */
        get: function() {
            var version = api.currentVersion();
            return version === '1.0.0' || version === '1.0.1' ? legacyGetter : undefined;
        },
    });
    var currentCall = Tools.Chat.call;
    Tools.Chat.call = api.method()
        .between('1.0.1', '2.0.0', __operitCreateV1ChatCall(currentCall))
        .since('2.0.0', currentCall)
        .build('Tools.Chat.call');
})(globalThis);
