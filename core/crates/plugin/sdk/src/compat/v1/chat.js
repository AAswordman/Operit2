/**
 * Adapts the v1.0.1 model-call contract without changing the current Chat namespace.
 * @param {typeof import('../../../../../../../plugins/types/chat').Chat.call} currentCall
 * @returns {typeof import('../../../../../../../plugins/types-v1/chat').Chat.call}
 */
function __operitCreateV1ChatCall(currentCall) {
    /**
     * Validates a JSON boundary value without widening the legacy metadata contract.
     * @param {unknown} value
     * @returns {import('../../../../../../../plugins/types-v1/toolpkg').ToolPkg.JsonValue}
     */
    function jsonValue(value) {
        if (value === null || typeof value === 'string' || typeof value === 'boolean') return value;
        if (typeof value === 'number' && Number.isFinite(value)) return value;
        if (Array.isArray(value)) return value.map(jsonValue);
        if (typeof value === 'object' && value !== null) return jsonObject(value);
        throw new Error('ToolPkg v1 Chat.call received non-JSON metadata.');
    }

    /**
     * Converts a metadata object while retaining every declared JSON field.
     * @param {object} value
     * @returns {import('../../../../../../../plugins/types-v1/toolpkg').ToolPkg.JsonObject}
     */
    function jsonObject(value) {
        if (Object.prototype.toString.call(value) !== '[object Object]') {
            throw new Error('ToolPkg v1 Chat.call metadata must be a JSON object.');
        }
        /** @type {import('../../../../../../../plugins/types-v1/toolpkg').ToolPkg.JsonObject} */
        var output = {};
        for (var [key, entry] of Object.entries(value)) {
            Object.defineProperty(output, key, { value: jsonValue(entry), enumerable: true });
        }
        return output;
    }

    /**
     * Validates the explicit v1 prompt-role vocabulary.
     * @param {string} kind
     * @returns {import('../../../../../../../plugins/types-v1/toolpkg').ToolPkg.PromptTurnKind}
     */
    function promptKind(kind) {
        switch (kind) {
            case 'SYSTEM': case 'USER': case 'ASSISTANT':
            case 'TOOL_CALL': case 'TOOL_RESULT': case 'SUMMARY': return kind;
            default: throw new Error('Unsupported ToolPkg v1 prompt turn kind: ' + kind);
        }
    }

    /**
     * Converts nullable tool names to the optional current field and validates returned enums.
     * @param {import('../../../../../../../plugins/types-v1/chat').Chat.ChatCallOptions} options
     * @returns {Promise<import('../../../../../../../plugins/types-v1/results').ChatCallResultData>}
     */
    return async function call(options) {
        /** @type {import('../../../../../../../plugins/types/chat').Chat.CallOptions} */
        var request = {
            functionType: options.functionType,
            /** Copies each turn so the legacy caller's data stays unchanged. */
            turns: options.turns.map(function(turn) {
                var { toolName, ...rest } = turn;
                if (toolName === null) return rest;
                return Object.assign({}, rest, { toolName: toolName });
            }),
            recordTokenUsage: options.recordTokenUsage,
            enableThinking: options.enableThinking
        };
        var response = await currentCall(request);
        if (response.finishReason !== 'stop' && response.finishReason !== 'tool_call') {
            throw new Error('Unsupported ToolPkg v1 model finish reason: ' + response.finishReason);
        }
        return {
            text: response.text,
            /** Converts the current open role string to the closed legacy role contract. */
            turns: response.turns.map(function(turn) {
                /** @type {import('../../../../../../../plugins/types-v1/toolpkg').ToolPkg.PromptTurn} */
                var converted = {
                    kind: promptKind(turn.kind),
                    content: turn.content,
                    metadata: jsonObject(turn.metadata)
                };
                if (turn.toolName !== undefined) converted.toolName = turn.toolName;
                return converted;
            }),
            finishReason: response.finishReason,
            metadata: jsonObject(response.metadata),
            receivedAt: response.receivedAt
        };
    };
}
/**
 * Bridges legacy positional chat calls to generic chat requests and plugin-owned role input.
 * @param {typeof import('../../../../../../../plugins/types/chat').Chat} current
 * @param {(id: string) => Promise<unknown>} requireCard
 * @param {(params: Record<string, unknown>) => Promise<import('../../../../../../../plugins/types/results').MessageSendResultData>} sendLegacy
 * @param {() => Promise<import('../../../../../../../plugins/types-v1/results').CharacterCardListResultData>} listCards
 */
function __operitCreateV1Chat(current, requireCard, sendLegacy, listCards) {
    /** Builds one persisted submission after rejecting unsupported legacy controls.
     * @param {string} message
     * @param {string | undefined} chatId
     * @param {string | undefined} roleCardId
     * @param {string | undefined} senderName
     * @param {import('../../../../../../../plugins/types-v1/chat').Chat.SendMessageOptions} options
     * @returns {Promise<import('../../../../../../../plugins/types/chat').Chat.SendRequest>}
     */
    async function request(message, chatId, roleCardId, senderName, options) {
        if (senderName !== undefined) throw new Error('ToolPkg v1 senderName has no equivalent in persisted native submissions');
        if (options.persist_turn !== undefined && options.persist_turn !== true) throw new Error('Legacy streaming persist_turn=false requires an ephemeral streaming host operation');
        if (options.hide_user_message !== undefined && options.hide_user_message !== false) throw new Error('Legacy streaming hide_user_message=true requires a hidden-input streaming host operation');
        if (options.disable_warning !== undefined && options.disable_warning !== false) throw new Error('Legacy streaming disable_warning=true is not exposed by the structured streaming host');
        if (options.timeout_ms !== undefined) throw new Error('Legacy streaming timeout_ms is not exposed by the structured streaming host');
        if (chatId === undefined) {
            var chats = await current.listAll();
            if (chats.currentChatId === null) throw new Error('ToolPkg v1 send requires a current chat or an explicit chatId');
            chatId = chats.currentChatId;
        }
        if (roleCardId !== undefined) await requireCard(roleCardId);
        return { kind: 'submit', chatId, runtime: options.runtime === undefined ? 'main' : options.runtime,
            input: { text: message, attachments: [], replyToMessageTimestamp: null },
            turn: { kind: 'execute', ...(roleCardId === undefined ? {} : { participantId: roleCardId }) },
            notifyReply: options.notify_reply === undefined ? false : options.notify_reply };
    }
    return {
        /** Sends the explicit role as provider-scoped creation input, then assigns native folder membership.
         * @type {typeof import('../../../../../../../plugins/types-v1/chat').Chat.createNew}
         */
        async createNew(group, setAsCurrentChat, characterCardId) {
            if (characterCardId !== undefined) await requireCard(characterCardId);
            var result = await current.createNew({ ...(setAsCurrentChat === undefined ? {} : { setAsCurrentChat }),
                sourceChatId: null,
                input: characterCardId === undefined ? null : { 'com.operit.character_cards': { version: 1, selection: 'card:' + characterCardId } } });
            if (group !== undefined) await current.updateGroup([result.chatId], group);
            return result;
        },
        /** Reads role identities from the prerequisite provider. */
        listCharacterCards: listCards,
        /** Converts the v1 positional signature to one native persisted submission.
         * @type {typeof import('../../../../../../../plugins/types-v1/chat').Chat.sendMessage}
         */
        async sendMessage(message, chatId, roleCardId, senderName, options = {}) {
            if (roleCardId !== undefined) await requireCard(roleCardId);
            return sendLegacy(Object.assign({}, options, { message },
                chatId === undefined ? {} : { chat_id: chatId },
                roleCardId === undefined ? {} : { participant_id: roleCardId },
                senderName === undefined ? {} : { sender_name: senderName }));
        },
        /** Converts ordered growing text snapshots to the original callback and terminal Promise contract.
         * @type {typeof import('../../../../../../../plugins/types-v1/chat').Chat.sendMessageStreaming}
         */
        async sendMessageStreaming(message, chatId, roleCardId, senderName, options = {}) {
            var submission = await request(message, chatId, roleCardId, senderName, options);
            if (options.waifu === true) throw new Error('Legacy waifu sentence aggregation requires an explicit streaming aggregation contract');
            var delivered = '', chunkIndex = 0;
            if (options.onIntermediateResult !== undefined) options.onIntermediateResult({ type: 'start', chatId: submission.chatId, message, waifu: false, receivedChars: 0 });
            for await (var event of current.sendMessageStreaming(submission)) {
                if (event.type === 'completed') return event.result;
                if (options.onIntermediateResult === undefined) continue;
                var text = event.parts.filter(
                    /** Projects visible answer text without parsing provider markup. */
                    part => part.kind === 'markdown',
                ).map(
                    /** Preserves the host's ordered semantic snapshot. */
                    part => part.content,
                ).join('');
                if (!text.startsWith(delivered)) throw new Error('Legacy chunk callbacks cannot retract text revised by an atomic message snapshot');
                var chunk = text.slice(delivered.length);
                delivered = text;
                if (chunk !== '' && options.onIntermediateResult !== undefined) options.onIntermediateResult({ type: 'chunk', chatId: submission.chatId, message, waifu: false, chunk, chunkIndex: chunkIndex++, receivedChars: delivered.length });
            }
            throw new Error('Chat stream ended without its committed completion receipt');
        },
    };
}
