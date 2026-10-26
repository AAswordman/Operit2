"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.PACKAGE_ID = void 0;
exports.command = command;
const assets_1 = require("./assets");
const protocol_1 = require("./protocol");
exports.PACKAGE_ID = "com.operit.edge_pixel_pet";
const packId = 'pixel-pet-demo';
function nodeId(params = {}) {
    const node = params.node_id || getEnv('EDGE_SCENE_NODE_ID');
    if (!node || typeof node !== 'string')
        throw new Error('设置 EDGE_SCENE_NODE_ID 或传入环境变量 EDGE_SCENE_NODE_ID');
    return node;
}
async function inspect(params = {}) {
    return (0, protocol_1.sceneCall)(nodeId(params), 'capabilities');
}
async function applyConfiguration(params = {}) {
    const node = nodeId(params);
    const capabilities = await inspect(params);
    if (capabilities.protocol !== 1 || capabilities.input?.local !== true)
        throw new Error('Scene v1 with device-local input is required');
    const cached = capabilities.cache.packs.find((pack) => pack.id === packId && pack.sha256 === assets_1.PACK_SHA256);
    if (!cached) {
        const upload = await (0, protocol_1.sceneCall)(node, 'pack.begin', {
            packId, bytes: assets_1.PACK_BYTES, sha256: assets_1.PACK_SHA256,
        });
        try {
            for (const chunk of assets_1.PACK_CHUNKS)
                await (0, protocol_1.sceneCall)(node, 'pack.chunk', { upload: upload.upload, ...chunk });
            await (0, protocol_1.sceneCall)(node, 'pack.commit', { upload: upload.upload });
        }
        catch (error) {
            try {
                await (0, protocol_1.sceneCall)(node, 'pack.abort', { upload: upload.upload });
            }
            catch { /* keep the original upload failure */ }
            throw error;
        }
    }
    const scene = await (0, protocol_1.sceneCall)(node, 'config.apply', {
        enabled: true,
        background: 0x18c3,
        layers: [
            { asset: `${packId}:floor`, x: 32, y: 152, scale: 8 },
            { asset: `${packId}:pet`, x: 144, y: 166, scale: 6, anchor: 'bc', play: true, target: 'pet' },
            { asset: `${packId}:treat`, x: 250, y: 160, scale: 5, anchor: 'bc', target: 'treat' },
        ],
    });
    return { node, packId, revision: scene.revision, message: '配置已写入设备；设备运行不依赖此 ToolPkg' };
}
async function clearConfiguration(params = {}) {
    const node = nodeId(params);
    await (0, protocol_1.sceneCall)(node, 'config.clear');
    return { node, cleared: true };
}
let queue = Promise.resolve();
function serial(run) {
    const task = queue.then(run);
    queue = task.catch(() => undefined);
    return task;
}
function command(request) {
    return serial(() => {
        switch (request.operation) {
            case 'inspect': return inspect(request.params);
            case 'apply_configuration': return applyConfiguration(request.params);
            case 'clear_configuration': return clearConfiguration(request.params);
            default: throw new Error('Unknown configuration operation');
        }
    });
}
