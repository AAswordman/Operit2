"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.PACKAGE_ID = void 0;
exports.onEdge = onEdge;
exports.command = command;
const assets_1 = require("./assets");
const protocol_1 = require("./protocol");
exports.PACKAGE_ID = "com.operit.edge_pixel_pet";
const packId = 'pixel-pet-demo';
function nodeId(params) {
    const node = params.node_id || getEnv('EDGE_SCENE_NODE_ID');
    if (!node || typeof node !== 'string')
        throw new Error('设置 EDGE_SCENE_NODE_ID 或传入 node_id；不猜测默认节点');
    return node;
}
// Resolve the config once while a real main-runtime command is active. IPC and
// callback continuations then share the same proxy, not transient global metadata.
let database;
function data() {
    return database ?? (database = PluginConfig.use('pet', { version: 1, state: { taps: 0, sessions: {} } }).catch(error => { database = undefined; throw error; }));
}
async function test() { return (0, protocol_1.sceneCall)(nodeId({}), 'capabilities'); }
async function start_petImpl(params = {}) {
    const node = nodeId(params);
    const db = await data();
    if (db.state.sessions[node]) {
        try {
            await (0, protocol_1.sceneCall)(node, 'lease.close', { lease: db.state.sessions[node].lease });
        }
        catch (e) {
            if (e.code !== 'SCENE_LEASE')
                throw e;
        }
        const sessions = { ...db.state.sessions };
        delete sessions[node];
        await save(db, { ...db.state, sessions });
    }
    const capabilities = await (0, protocol_1.sceneCall)(node, 'capabilities');
    if (capabilities.protocol !== 1 || !capabilities.events?.push)
        throw new Error('Scene v1 with event push required');
    const region = await (0, protocol_1.sceneCall)(node, 'lease.open', { owner: 'pixel.pet.demo', ttlMs: 60000, notify: { packageName: exports.PACKAGE_ID } });
    const lease = region.lease;
    try {
        const cached = capabilities.cache.packs.find((p) => p.id === packId && p.sha256 === assets_1.PACK_SHA256);
        if (!cached) {
            const upload = await (0, protocol_1.sceneCall)(node, 'pack.begin', { lease, packId, bytes: assets_1.PACK_BYTES, sha256: assets_1.PACK_SHA256 });
            try {
                for (const chunk of assets_1.PACK_CHUNKS)
                    await (0, protocol_1.sceneCall)(node, 'pack.chunk', { lease, upload: upload.upload, ...chunk });
                await (0, protocol_1.sceneCall)(node, 'pack.commit', { lease, upload: upload.upload });
            }
            catch (error) {
                try {
                    await (0, protocol_1.sceneCall)(node, 'pack.abort', { lease, upload: upload.upload });
                }
                catch { /* original error wins */ }
                throw error;
            }
        }
        await save(db, { ...db.state, sessions: { ...db.state.sessions, [node]: { lease, cursor: region.cursor } } });
        await (0, protocol_1.sceneCall)(node, 'scene.set', { lease, background: 0x18c3, layers: [
                { asset: `${packId}:floor`, x: 32, y: 152, scale: 8 },
                { asset: `${packId}:pet`, x: 144, y: 166, scale: 6, anchor: 'bc', play: true, target: 'pet' },
                { asset: `${packId}:treat`, x: 250, y: 160, scale: 5, anchor: 'bc', target: 'treat' },
            ] });
        return { node, taps: db.state.taps, ttlMs: region.ttlMs, message: '触摸由 Edge 主动通知 Core；renew_pet 只续显示租约，右上角退出始终可用' };
    }
    catch (error) {
        try {
            await (0, protocol_1.sceneCall)(node, 'lease.close', { lease });
        }
        catch { /* never mask transport/persistence failure */ }
        throw error;
    }
}
async function renew_pet(params = {}) {
    const node = nodeId(params), db = await data(), session = db.state.sessions[node];
    if (!session || session.closed)
        return { node, taps: db.state.taps, closed: session?.closed || true };
    try {
        const result = await (0, protocol_1.sceneCall)(node, 'lease.renew', { lease: session.lease });
        return { node, taps: db.state.taps, closed: false, ttlMs: result.ttlMs };
    }
    catch (error) {
        if (!['SCENE_LEASE', 'SCENE_CLOSED', 'SCENE_EXPIRED'].includes(error.code))
            throw error;
        // The pending exit batch owns persistence; do not advance its cursor here.
        return { node, taps: db.state.taps, closed: true };
    }
}
async function pet_status(params = {}) {
    const node = nodeId(params), db = await data(), session = db.state.sessions[node];
    return { node, taps: db.state.taps, closed: session?.closed || !session,
        cursor: session?.cursor, lostEvents: session?.lostEvents || false };
}
async function close_pet(params = {}) {
    const node = nodeId(params), db = await data(), session = db.state.sessions[node];
    if (session && !session.closed)
        await (0, protocol_1.sceneCall)(node, 'lease.close', { lease: session.lease });
    return { node, taps: db.state.taps, closed: true };
}
async function receiveEdge(payload) {
    const db = await data(), batch = payload.batch, session = db.state.sessions[payload.nodeId];
    if (batch.source !== 'display.scene' || !session || session.lease !== batch.stream)
        return { accepted: false };
    if (batch.next > session.cursor) {
        let taps = db.state.taps, closed = session.closed;
        // An action layer belongs to the consuming plugin. Core does not assume touch/screen data.
        const actions = {
            'target.tap': data => {
                if (data.target !== 'pet' && data.target !== 'treat')
                    throw new Error('Unknown pet target');
                taps += 1;
            },
            'system.exit': () => { closed = 'system.exit'; },
            'lease.closed': () => { closed = 'lease.closed'; },
            'lease.expired': () => { closed = 'lease.expired'; },
        };
        for (const event of batch.events.filter(event => event.seq > session.cursor)) {
            const handler = Object.prototype.hasOwnProperty.call(actions, event.action) ? actions[event.action] : undefined;
            if (!handler)
                return { accepted: false };
            handler(event.data);
        }
        await save(db, { taps, sessions: { ...db.state.sessions, [payload.nodeId]: {
                    lease: session.lease, cursor: batch.next, closed,
                    lostEvents: session.lostEvents || batch.lostBefore > session.cursor,
                } } });
    }
    return { accepted: true, next: batch.next };
}
async function save(db, state) {
    const before = db.state;
    db.state = state;
    try {
        await PluginConfig.flush(db);
    }
    catch (error) {
        db.state = before;
        try {
            await PluginConfig.flush(db);
        }
        catch { /* original error wins */ }
        throw error;
    }
}
let queue = Promise.resolve();
function serial(run) {
    const task = queue.then(run);
    queue = task.catch(() => undefined);
    return task;
}
function onEdge(payload) { return serial(() => receiveEdge(payload)); }
function command(request) {
    return serial(() => {
        switch (request.operation) {
            case 'test': return test();
            case 'start_pet': return start_petImpl(request.params);
            case 'renew_pet': return renew_pet(request.params);
            case 'pet_status': return pet_status(request.params);
            case 'close_pet': return close_pet(request.params);
            default: throw new Error('Unknown pet command');
        }
    });
}
