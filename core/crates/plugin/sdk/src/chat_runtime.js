// Canonical current Chat bindings use the same scoped host Promise transport as Tools.

/** Captures one complete send request before its caller can mutate it across asynchronous pulls. */
function __operitChatRequestValue(request) {
    if (request === null || typeof request !== 'object' || Array.isArray(request)) throw new Error('Chat requires one send request object');
    return __operitSnapshotJsonObject(request);
}

/** Returns one bounded pull-based iterator whose disposal never cancels generation. */
function __operitCreateToolStream(toolName, request) {
    var requestValue = __operitChatRequestValue(request);
    var opening = null, streamId = null, disposal = null, closed = false, pulling = false;

    /** Starts the sole accepted send lazily on its first consumer pull. */
    function open() {
        if (opening === null) {
            opening = toolCall(toolName, { observation: { op: 'open', request: requestValue } }).then(
                /** Retains only the private native observation identity. */
                function(result) { var id = result.handle; streamId = id; return id; }
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
                await toolCall(toolName, { observation: { op: 'close', handle: id } });
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
            var result = await toolCall(toolName, { observation: { op: 'next', handle: id } });
            var event = result.event;
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

