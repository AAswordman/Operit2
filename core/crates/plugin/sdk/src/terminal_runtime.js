/** Requires managed terminal waits to retain exact nonnegative integer milliseconds. */
function __operitTerminalWaitBudget(waitMs) {
    if (!Number.isSafeInteger(waitMs) || waitMs < 0) throw new TypeError('Terminal waitMs must be a nonnegative integer');
    return waitMs;
}

/** Extracts the typed managed command receipt from the ordinary tool result. */
function __operitManagedTerminalResult(response) {
    if (!response || typeof response !== 'object' || !('result' in response)) throw new Error('Managed terminal returned no execution result');
    return response.result;
}

/** Starts a session-owned command without tying its lifetime to this JavaScript Promise. */
function __operitRunTerminal(request) {
    var value = __operitSnapshotJsonObject(request);
    return toolCall('terminal_observe', { observation: {
        op: 'run', sessionName: value.sessionName, terminalType: value.terminalType,
        command: value.command, waitMs: __operitTerminalWaitBudget(value.waitMs)
    }}).then(__operitManagedTerminalResult);
}

/** Waits for the original command without submitting input or canceling it on wait expiry. */
function __operitWaitTerminal(sessionId, waitMs) {
    return toolCall('terminal_observe', { observation: {
        op: 'wait', sessionId: sessionId, waitMs: __operitTerminalWaitBudget(waitMs)
    }}).then(__operitManagedTerminalResult);
}

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
