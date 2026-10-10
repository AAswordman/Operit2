/** Installs v1 adapters beside the generated v2 implementations without replacing their contracts. */
(function installV1Adapters() {
    var api = globalThis.__operitToolPkgApi;
    var currentFiles = Object.assign({}, Tools.Files);
    var legacyFiles = __operitCreateV1Files(currentFiles);
    for (var method of Object.keys(legacyFiles)) {
        Tools.Files[method] = api.method()
            .between('1.0.0', '2.0.0', legacyFiles[method])
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
    for (var name of Object.keys(workflow)) {
        Tools.Workflow[name] = api.method()
            .between('1.0.0', '2.0.0', workflow[name])
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
            return globalThis.toolCall('send_message_to_ai', params);
        },
        characters.listCharacterCards,
    );
    for (var chatMethod of Object.keys(legacyChat)) {
        var chatBuilder = api.method().between('1.0.0', '2.0.0', legacyChat[chatMethod]);
        if (chatMethod !== 'listCharacterCards') chatBuilder.since('2.0.0', currentChat[chatMethod]);
        Tools.Chat[chatMethod] = chatBuilder.build('Tools.Chat.' + chatMethod);
    }
    var legacyMemory = __operitCreateV1Memory(
        /** Sends one package tool request with the active authenticated caller fields. */
        function(name, params) {
            var state = globalThis.__operitGetCallState(api.currentCallId());
            if (state.params.__operit_package_caller_participant_id !== undefined) legacyCallerCardId();
            for (var key of ['__operit_package_caller_name', '__operit_package_chat_id', '__operit_package_caller_participant_id', '__operit_package_caller_owner']) {
                if (state.params[key] !== undefined) params[key] = state.params[key];
            }
            return globalThis.toolCall('character_memory:' + name, params).then(
                /** Decodes the package executor's JSON payload, including JSON-encoded string results. */
                function(serialized) {
                    if (typeof serialized !== 'string') throw new Error('Package memory tool must return its serialized execution result');
                    return JSON.parse(serialized);
                },
            );
        },
    );
    Tools.Memory = {};
    for (var memoryMethod of Object.keys(legacyMemory)) {
        Tools.Memory[memoryMethod] = api.method()
            .between('1.0.0', '2.0.0', legacyMemory[memoryMethod])
            .build('Tools.Memory.' + memoryMethod);
    }
    /** Projects the role provider's authenticated participant into the historical card namespace. */
    function legacyCallerCardId() {
        var params = globalThis.__operitGetCallState(api.currentCallId()).params;
        var participant = params.__operit_package_caller_participant_id;
        if (participant === undefined) return undefined;
        if (params.__operit_package_caller_owner !== 'com.operit.character_cards') {
            throw new Error('ToolPkg v1 getCallerCardId requires the character provider; another provider participant is not a card ID.');
        }
        return participant;
    }
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
})();
