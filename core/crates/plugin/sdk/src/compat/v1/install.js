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

(/**
 * Installs typed v1 adapters in the runtime's dynamic registration namespace.
 * @param {Record<string, any>} runtime
 */
function installV1Adapters(runtime) {
    var Tools = runtime.Tools;
    var api = runtime.__operitToolPkgApi;
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
