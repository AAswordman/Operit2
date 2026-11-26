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
    revision: number;
    probe?: Promise<void>;
    closing?: Promise<void>;
}

const IDLE_TIMEOUT_MS = 10 * 60 * 1000;
const sessions = new Map<string, ManagedSession>();
const families = new Map<string, number>();

/** Routes all tool runtimes to the container's single session owner. */
export async function manageSessions(request: SessionRequest): Promise<void> {
    await ToolPkg.ipc.call("super_admin.sessions", request, { targetRuntime: "main" });
}

/** Records only sessions actually acquired by this plugin and activity directed at those sessions. */
export async function receiveSessionRequest(request: SessionRequest): Promise<void> {
    const now = Date.now();
    switch (request.action) {
        case "acquire_family":
        case "release_family": {
            const change = request.action === "acquire_family" ? 1 : -1;
            const count = (families.get(request.sessionName) ?? 0) + change;
            if (count < 0) throw new Error("Terminal family lease is unbalanced");
            if (count === 0) families.delete(request.sessionName);
            else families.set(request.sessionName, count);
            if (request.action === "acquire_family") {
                // Hold the family before yielding so no new close can overtake this acquisition.
                await Promise.all([...sessions.values()]
                    .filter(session => session.sessionName === request.sessionName && session.closing)
                    .map(session => session.closing));
            }
            return;
        }
        case "track": {
            if (!sessions.has(request.sessionId)) sessions.set(request.sessionId, {
                sessionName: request.sessionName, lastUsed: now, users: 0,
                wasRunning: false, reportedError: false, revision: 0,
            });
            const session = sessions.get(request.sessionId)!;
            session.users++;
            session.lastUsed = now;
            session.revision++;
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
            session.revision++;
            if (request.action === "acquire_session") await session.closing;
            return;
        }
    }
}

/** Checks current ownership and leases without holding any asynchronous Host operation. */
function isIdle(sessionId: string, session: ManagedSession): boolean {
    const now = Date.now();
    if (session.lastUsed > now) session.lastUsed = now;
    return sessions.get(sessionId) === session && session.users === 0
        && !families.has(session.sessionName) && now - session.lastUsed >= IDLE_TIMEOUT_MS;
}

/** Revalidates an idle observation and reserves only its own session during atomic Host close. */
async function inspectIdleSession(sessionId: string, session: ManagedSession): Promise<void> {
    try {
        if (!isIdle(sessionId, session)) return;
        const revision = session.revision;
        const screen = await Tools.System.terminal.screen(sessionId);
        if (revision !== session.revision || !isIdle(sessionId, session)) return;
        if (screen.commandRunning || session.wasRunning) {
            session.lastUsed = Date.now();
            session.wasRunning = screen.commandRunning;
            return;
        }
        let release!: () => void;
        session.closing = new Promise<void>(resolve => { release = resolve; });
        try {
            const result = await Tools.System.terminal.closeIfIdle(sessionId, session.sessionName);
            if (!result.success) throw new Error(result.message);
            if (sessions.get(sessionId) === session) sessions.delete(sessionId);
            console.log(`[super_admin] Closed idle terminal: ${sessionId}`);
        } finally {
            session.closing = undefined;
            release();
        }
    } catch (error) {
        if (!session.reportedError) console.error(`[super_admin] Idle terminal cleanup failed for ${sessionId}: ${String(error)}`);
        session.reportedError = true;
    }
}

/** Probes sessions independently so a suspended Host read cannot block other profiles or leases. */
export async function sweepIdleSessions(): Promise<void> {
    const probes: Promise<void>[] = [];
    for (const [sessionId, session] of sessions) {
        if (!session.probe && isIdle(sessionId, session)) {
            session.probe = inspectIdleSession(sessionId, session).finally(() => { session.probe = undefined; });
        }
        if (session.probe) probes.push(session.probe);
    }
    await Promise.all(probes);
}
