type SessionRequest =
    | { action: "acquire_family" | "release_family"; sessionName: string }
    | { action: "track"; sessionId: string; sessionName: string }
    | { action: "acquire_session" | "release_session" | "forget"; sessionId: string };

interface ManagedSession {
    sessionName: string;
    lastUsed: number;
    users: number;
    wasRunning: boolean;
    reportedError: boolean;
}

const IDLE_TIMEOUT_MS = 10 * 60 * 1000;
const sessions = new Map<string, ManagedSession>();
const families = new Map<string, number>();
let queue: Promise<void> = Promise.resolve();

/** Serializes acquisition and cleanup so a newly reused session cannot be closed by an old sweep. */
function serialize(work: () => Promise<void> | void): Promise<void> {
    const result = queue.then(work);
    queue = result.then(() => {}, () => {});
    return result;
}

/** Routes all tool runtimes to the container's single session owner. */
export async function manageSessions(request: SessionRequest): Promise<void> {
    await ToolPkg.ipc.call("super_admin.sessions", request, { targetRuntime: "main" });
}

/** Records only sessions actually acquired by this plugin and activity directed at those sessions. */
export function receiveSessionRequest(request: SessionRequest): Promise<void> {
    return serialize(() => {
        const now = Date.now();
        switch (request.action) {
            case "acquire_family":
            case "release_family": {
                const change = request.action === "acquire_family" ? 1 : -1;
                const count = (families.get(request.sessionName) ?? 0) + change;
                if (count < 0) throw new Error("Terminal family lease is unbalanced");
                if (count === 0) families.delete(request.sessionName);
                else families.set(request.sessionName, count);
                return;
            }
            case "track": {
                if (!sessions.has(request.sessionId)) sessions.set(request.sessionId, {
                    sessionName: request.sessionName, lastUsed: now, users: 0,
                    wasRunning: false, reportedError: false,
                });
                const session = sessions.get(request.sessionId)!;
                session.users++;
                session.lastUsed = now;
                return;
            }
            case "forget":
                sessions.delete(request.sessionId);
                return;
            case "acquire_session":
            case "release_session": {
                const session = sessions.get(request.sessionId);
                if (session === undefined) return;
                session.users += request.action === "acquire_session" ? 1 : -1;
                if (session.users < 0) throw new Error("Terminal session lease is unbalanced");
                session.lastUsed = now;
                return;
            }
        }
    });
}

/** Checks the host's running state and uses atomic conditional close without ever forcing termination. */
export function sweepIdleSessions(): Promise<void> {
    return serialize(async () => {
        const now = Date.now();
        for (const [sessionId, session] of sessions) {
            if (session.lastUsed > now) session.lastUsed = now;
            if (session.users > 0 || families.has(session.sessionName) || now - session.lastUsed < IDLE_TIMEOUT_MS) continue;
            try {
                const screen = await Tools.System.terminal.screen(sessionId);
                if (screen.commandRunning || session.wasRunning) {
                    session.lastUsed = Date.now();
                    session.wasRunning = screen.commandRunning;
                    continue;
                }
                const result = await Tools.System.terminal.closeIfIdle(sessionId, session.sessionName);
                if (!result.success) throw new Error(result.message);
                sessions.delete(sessionId);
                console.log(`[super_admin] Closed idle terminal: ${sessionId}`);
            } catch (error) {
                if (!session.reportedError) console.error(`[super_admin] Idle terminal cleanup failed for ${sessionId}: ${String(error)}`);
                session.reportedError = true;
            }
        }
    });
}
