/** Adapts the callback-style API to the same live terminal stream exposed to v2 consumers. */
async function __operitExecTerminalStreaming(sessionId, command, options) {
    if (options === undefined) options = {};
    if (options === null || typeof options !== 'object' || Array.isArray(options)) throw new TypeError('Terminal streaming options must be an object');
    var callback = options.onIntermediateResult;
    if (callback !== undefined && typeof callback !== 'function') throw new TypeError('onIntermediateResult must be a function');
    var request = { sessionId: sessionId, command: command };
    if (options.timeoutMs !== undefined) {
        var timeout = Number(options.timeoutMs);
        if (!Number.isSafeInteger(timeout) || timeout < 0) throw new TypeError('timeoutMs must be a nonnegative integer');
        request.timeoutMs = timeout;
    }
    var stream = __operitCreateToolStream('terminal_observe', request);
    for await (var event of stream) {
        if (event.type === 'completed') return event.result;
        if (callback !== undefined) callback(event);
    }
    throw new Error('Terminal execution ended without a completion receipt');
}
