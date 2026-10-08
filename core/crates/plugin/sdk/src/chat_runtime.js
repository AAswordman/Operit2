// Canonical current Chat send bindings. Internal transport handles never reach plugin authors.
var __operitChatCallbackSequence = 0;

/** Invokes the exact typed native Chat method without exposing callbacks in the public contract. */
function __operitChatInvoke(method, payload) {
    return new Promise(
        /** Registers one response callback for a single native pull or control operation. */
        function(resolve, reject) {
            var callbackId = '__operit_chat_send_' + (++__operitChatCallbackSequence);
            /** Delivers the actual typed host response and removes its private callback immediately. */
            globalThis[callbackId] = function(result, isError) {
                delete globalThis[callbackId];
                try {
                    var value = JSON.parse(result);
                    if (isError) reject(new Error(value.message));
                    else resolve(value);
                } catch (error) { reject(error); }
            };
            try { __operitNativeChatAsync(callbackId, method, payload); }
            catch (error) { delete globalThis[callbackId]; reject(error); }
        }
    );
}

/** Captures one complete send request before its caller can mutate it across asynchronous pulls. */
function __operitChatRequestJson(request) {
    if (request === null || typeof request !== 'object' || Array.isArray(request)) throw new Error('Chat requires one send request object');
    __operitRequireChatJsonObject(request);
    return JSON.stringify(request);
}

/** Returns one actual finalized receipt through the same native execution pipeline as streaming. */
function __operitChatSend(request) {
    if (arguments.length !== 1) return Promise.reject(new Error('Chat.sendMessage requires exactly one request'));
    try { return __operitChatInvoke('send', __operitChatRequestJson(request)); }
    catch (error) { return Promise.reject(error); }
}

/** Returns one bounded pull-based iterator whose disposal never cancels generation. */
function __operitChatStream(request) {
    if (arguments.length !== 1) throw new Error('Chat.sendMessageStreaming requires exactly one request');
    var requestJson = __operitChatRequestJson(request);
    var opening = null, streamId = null, disposal = null, closed = false, pulling = false;

    /** Starts the sole accepted send lazily on its first consumer pull. */
    function open() {
        if (opening === null) {
            opening = __operitChatInvoke('open', requestJson).then(
                /** Retains only the private native observation identity. */
                function(id) { streamId = id; return id; }
            );
        }
        return opening;
    }

    /** Detaches immediately, including while a native next call is waiting for model output. */
    function dispose() {
        closed = true;
        if (disposal === null) {
            /** Waits only for admission, then disposes the actual observation once. */
            disposal = (async function() {
                if (opening === null) return;
                var id = await opening;
                await __operitChatInvoke('close', JSON.stringify(id));
            })();
        }
        return disposal;
    }

    /** Preserves the actual observation error and any independent disposal failure. */
    async function rejectAndDispose(originalError) {
        closed = true;
        if (streamId !== null) {
            try { await dispose(); }
            catch (cleanupError) {
                if (cleanupError !== originalError) {
                    var combined = new Error('Chat observation and disposal both failed');
                    combined.cause = originalError; combined.cleanupError = cleanupError;
                    throw combined;
                }
            }
        }
        throw originalError;
    }

    /** Performs exactly one native pull without queuing additional consumer requests in JavaScript. */
    async function pull() {
        try {
            var id = await open();
            if (closed) { await dispose(); return { done: true, value: undefined }; }
            var event = await __operitChatInvoke('next', JSON.stringify(id));
            if (closed || event === null) { await dispose(); return { done: true, value: undefined }; }
            if (event.type === 'completed') await dispose();
            return { done: false, value: event };
        } catch (error) { return rejectAndDispose(error); }
        finally { pulling = false; }
    }

    return {
        /** Exposes this single-consumer iterator through the standard async iterable protocol. */
        [Symbol.asyncIterator]: function() { return this; },
        /** Rejects concurrent pulls instead of creating an unbounded local Promise queue. */
        next: function() {
            if (closed) return Promise.resolve({ done: true, value: undefined });
            if (pulling) return Promise.reject(new Error('Chat observation permits only one pending next call'));
            pulling = true;
            return pull();
        },
        /** Stops only observation and wakes a pending native pull through explicit disposal. */
        return: async function() { await dispose(); return { done: true, value: undefined }; },
        /** Disposes observation while retaining the consumer's original error. */
        throw: function(error) { return rejectAndDispose(error); }
    };
}

/** Requests cancellation of only the immutable calling plugin's captured execution in this chat. */
function __operitChatCancel(chatId) {
    if (arguments.length !== 1 || typeof chatId !== 'string' || chatId.trim() === '') return Promise.reject(new Error('Chat.cancel requires one nonempty chatId'));
    return __operitChatInvoke('cancel', JSON.stringify(chatId));
}
