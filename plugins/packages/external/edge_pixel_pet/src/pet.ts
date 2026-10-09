import { PACK_BYTES, PACK_SHA256, PACK_CHUNKS } from './assets';
import { sceneCall } from './protocol';
interface Session {
    lease: string;
    cursor: number;
    closed?: string;
    lostEvents?: boolean;
}
interface Data {
    version: number;
    state: {
        taps: number;
        sessions: Record<string, Session>;
    };
}
export const PACKAGE_ID = "com.operit.edge_pixel_pet";
const packId = 'pixel-pet-demo';
function nodeId(params: {
    node_id?: string;
}): string {
    const node = params.node_id || getEnv('EDGE_SCENE_NODE_ID');
    if (!node || typeof node !== 'string')
        throw new Error('设置 EDGE_SCENE_NODE_ID 或传入 node_id；不猜测默认节点');
    return node;
}
// Resolve the config once while a real main-runtime command is active. IPC and
// callback continuations then share the same proxy, not transient global metadata.
let database: Promise<Data> | undefined;
function data(): Promise<Data> {
    return database ??= PluginConfig.use<Data>('pet', { version: 1, state: { taps: 0, sessions: {} } }).catch(error => { database = undefined; throw error; });
}
async function test(): Promise<any> { return sceneCall(nodeId({}), 'capabilities'); }
async function start_petImpl(params: {
    node_id?: string;
} = {}): Promise<any> {
    const node = nodeId(params);
    const db = await data();
    if (db.state.sessions[node]) {
        try {
            await sceneCall(node, 'lease.close', { lease: db.state.sessions[node].lease });
        }
        catch (e) {
            if ((e as any).code !== 'SCENE_LEASE')
                throw e;
        }
        const sessions = { ...db.state.sessions };
        delete sessions[node];
        await save(db, { ...db.state, sessions });
    }
    const capabilities = await sceneCall(node, 'capabilities');
    if (capabilities.protocol !== 1 || !capabilities.events?.push)
        throw new Error('Scene v1 with event push required');
    const region = await sceneCall(node, 'lease.open', { owner: 'pixel.pet.demo', ttlMs: 60000, notify: { packageName: PACKAGE_ID } });
    const lease = region.lease;
    try {
        const cached = capabilities.cache.packs.find((p: any) => p.id === packId && p.sha256 === PACK_SHA256);
        if (!cached) {
            const upload = await sceneCall(node, 'pack.begin', { lease, packId, bytes: PACK_BYTES, sha256: PACK_SHA256 });
            try {
                for (const chunk of PACK_CHUNKS)
                    await sceneCall(node, 'pack.chunk', { lease, upload: upload.upload, ...chunk });
                await sceneCall(node, 'pack.commit', { lease, upload: upload.upload });
            }
            catch (error) {
                try {
                    await sceneCall(node, 'pack.abort', { lease, upload: upload.upload });
                }
                catch { /* original error wins */ }
                throw error;
            }
        }
        await save(db, { ...db.state, sessions: { ...db.state.sessions, [node]: { lease, cursor: region.cursor } } });
        await sceneCall(node, 'scene.set', { lease, background: 0x18c3, layers: [
                { asset: `${packId}:floor`, x: 32, y: 152, scale: 8 },
                { asset: `${packId}:pet`, x: 144, y: 166, scale: 6, anchor: 'bc', play: true, target: 'pet' },
                { asset: `${packId}:treat`, x: 250, y: 160, scale: 5, anchor: 'bc', target: 'treat' },
            ] });
        return { node, taps: db.state.taps, ttlMs: region.ttlMs, message: '触摸由 Edge 主动通知 Core；renew_pet 只续显示租约，右上角退出始终可用' };
    }
    catch (error) {
        try {
            await sceneCall(node, 'lease.close', { lease });
        }
        catch { /* never mask transport/persistence failure */ }
        throw error;
    }
}
async function renew_pet(params: {
    node_id?: string;
} = {}): Promise<any> {
    const node = nodeId(params), db = await data(), session = db.state.sessions[node];
    if (!session || session.closed)
        return { node, taps: db.state.taps, closed: session?.closed || true };
    try {
        const result = await sceneCall(node, 'lease.renew', { lease: session.lease });
        return { node, taps: db.state.taps, closed: false, ttlMs: result.ttlMs };
    }
    catch (error) {
        if (!['SCENE_LEASE', 'SCENE_CLOSED', 'SCENE_EXPIRED'].includes((error as any).code))
            throw error;
        // The pending exit batch owns persistence; do not advance its cursor here.
        return { node, taps: db.state.taps, closed: true };
    }
}
async function pet_status(params: {
    node_id?: string;
} = {}): Promise<any> {
    const node = nodeId(params), db = await data(), session = db.state.sessions[node];
    return { node, taps: db.state.taps, closed: session?.closed || !session,
        cursor: session?.cursor, lostEvents: session?.lostEvents || false };
}
async function close_pet(params: {
    node_id?: string;
} = {}): Promise<any> {
    const node = nodeId(params), db = await data(), session = db.state.sessions[node];
    if (session && !session.closed)
        await sceneCall(node, 'lease.close', { lease: session.lease });
    return { node, taps: db.state.taps, closed: true };
}
export interface ScenePayload {
    chatId: string;
    nodeId: string;
    scene: {
        v: number;
        lease: string;
        next: number;
        lostBefore: number;
        closed: string | null;
        events: {
            seq: number;
            type: string;
            target: string | null;
        }[];
    };
}
async function receiveScene(payload: ScenePayload): Promise<any> {
    const db = await data(), batch = payload.scene, session = db.state.sessions[payload.nodeId];
    if (!session || session.lease !== batch.lease)
        return { accepted: false };
    const events = batch.events.filter(event => event.seq > session.cursor);
    if (batch.next > session.cursor) {
        const taps = db.state.taps + events.filter(event => event.type === 'target.tap').length;
        const closed = events.find(event => ['system.exit', 'lease.closed', 'lease.expired'].includes(event.type))?.type || session.closed;
        await save(db, { taps, sessions: { ...db.state.sessions, [payload.nodeId]: {
                    lease: session.lease, cursor: batch.next, closed,
                    lostEvents: session.lostEvents || batch.lostBefore > session.cursor,
                } } });
    }
    // Replay (including a closed tombstone) ACKs without writing or double counting.
    return { accepted: true, next: batch.next };
}
async function save(db: Data, state: Data['state']): Promise<void> {
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
let queue: Promise<unknown> = Promise.resolve();
function serial<T>(run: () => Promise<T>): Promise<T> {
    const task = queue.then(run);
    queue = task.catch(() => undefined);
    return task;
}
export function onScene(payload: ScenePayload): Promise<any> { return serial(() => receiveScene(payload)); }
export function command(request: {
    operation: string;
    params?: {
        node_id?: string;
    };
}): Promise<any> {
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
