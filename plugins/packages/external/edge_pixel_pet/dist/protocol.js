"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.sceneCall = sceneCall;
/** Core-side wrapper. Protocol failures are distinct from authorization/Link failures. */
async function sceneCall(node, action, fields = {}) {
    const args = { v: 1, ...fields };
    // All schema fields/chunks are ASCII in this example. JSON bytes <= 1024.
    if (JSON.stringify(args).length > 1024)
        throw new Error('Scene request exceeds 1024 bytes');
    const { data } = await Tools.Edge.execute(node, { pluginId: 'display.scene', action }, args);
    if (data?.v !== 1 || typeof data.ok !== 'boolean')
        throw new Error('Invalid scene protocol reply');
    if (!data.ok) {
        const error = new Error(`${data.error.code}: ${data.error.message}`);
        error.code = data.error.code;
        throw error;
    }
    return data.result;
}
