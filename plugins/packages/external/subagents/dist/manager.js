"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.AgentManager = void 0;
const model_js_1 = require("./model.js");
/** Runs one space's child conversations independently of tool invocation and UI lifetimes. */
class AgentManager {
    /** Binds durable storage, native Chat operations, clocks and explicit background diagnostics. */
    constructor(repository, chat, clock, reportError) {
        this.repository = repository;
        this.chat = chat;
        this.clock = clock;
        this.reportError = reportError;
        this.initialization = null;
        this.agents = new Map();
        this.executions = new Map();
        this.commands = new Set();
        this.waiters = new Set();
        this.pendingStarts = 0;
        this.creatingChildren = new Map();
    }
    /** Loads records once and truthfully marks observations lost through a runtime restart. */
    ready() {
        if (this.initialization === null)
            this.initialization = this.initialize();
        return this.initialization;
    }
    /** Preserves history while exposing lost ownership instead of claiming that a native turn finished. */
    async initialize() {
        for (const saved of await this.repository.load()) {
            if (this.agents.has(saved.id))
                throw new Error("Duplicate subagent identity");
            const agent = (0, model_js_1.isActive)(saved.status)
                ? { ...saved, status: "interrupted", updatedAt: this.clock.now(), error: "Plugin runtime restarted; the previous turn's observation was lost. Native execution may still be active." }
                : saved;
            this.agents.set(agent.id, agent);
            if (agent !== saved)
                await this.repository.save(agent);
        }
    }
    /** Dispatches the same explicit operations for model tools and composer controls. */
    async request(request) {
        await this.ready();
        (0, model_js_1.requireText)(request.parentChatId, "parentChatId");
        switch (request.action) {
            case "spawn": return this.spawn(request);
            case "list": return this.list(request.parentChatId);
            case "wait": return this.wait(request.parentChatId, request.agentIds, request.timeoutMs);
            case "send": return this.command(request.parentChatId, request.agentId, () => this.sendInput(request));
            case "close": return this.command(request.parentChatId, request.agentId, () => this.close(request.parentChatId, request.agentId));
            default: throw new Error("Unknown subagent operation");
        }
    }
    /** Rejects foreign IDs before any wait, cancellation or conversation mutation. */
    owned(parentChatId, agentId) {
        (0, model_js_1.requireText)(agentId, "agentId");
        const agent = this.agents.get(agentId);
        if (agent === undefined || agent.parentChatId !== parentChatId)
            throw new Error("Subagent does not belong to this conversation: " + agentId);
        return agent;
    }
    /** Returns one exact current agent; a disappeared record is an internal state error. */
    current(agentId) {
        const agent = this.agents.get(agentId);
        if (agent === undefined)
            throw new Error("Missing managed subagent: " + agentId);
        return agent;
    }
    /** Publishes immutable snapshots; persistence failures remain failures of the originating operation. */
    async save(agent) {
        this.agents.set(agent.id, agent);
        await this.repository.save(agent);
        if ((0, model_js_1.isActive)(agent.status))
            this.publish(agent);
    }
    /** Lists both active and retained finished children of the selected parent. */
    list(parentChatId) {
        return { schemaVersion: 1, agents: [...this.agents.values()].filter(agent => agent.parentChatId === parentChatId).map(model_js_1.summary) };
    }
    /** Reserves creation capacity and protects active parent controls without locking retained closed conversations. */
    async spawn(request) {
        (0, model_js_1.requireText)(request.task, "task");
        (0, model_js_1.requireText)(request.name, "name");
        if (request.participantId !== null)
            (0, model_js_1.requireText)(request.participantId, "participantId");
        const managedParent = this.agents.get(request.parentChatId);
        if (managedParent !== undefined && (managedParent.status === "closing" || this.commands.has(managedParent.id)))
            throw new Error("The parent subagent is closing or has a control operation in progress");
        this.reserveStart();
        const creating = this.creatingChildren.get(request.parentChatId);
        this.creatingChildren.set(request.parentChatId, creating === undefined ? 1 : creating + 1);
        try {
            const chatId = (0, model_js_1.requireText)(await this.chat.create(request.parentChatId), "created chatId");
            if (this.agents.has(chatId) || chatId === request.parentChatId)
                throw new Error("Native creation returned a reused conversation identity");
            const now = this.clock.now();
            const agent = {
                schemaVersion: 1, id: chatId, chatId, parentChatId: request.parentChatId,
                name: request.name, task: request.task, participantId: request.participantId,
                status: "starting", runNumber: 1, lastInput: request.task,
                createdAt: now, updatedAt: now, error: null, result: null, messages: [],
            };
            await this.save(agent);
            this.launch(agent);
            return (0, model_js_1.summary)(agent);
        }
        finally {
            this.pendingStarts--;
            const count = this.creatingChildren.get(request.parentChatId);
            if (count === undefined)
                throw new Error("Missing child creation reservation");
            if (count === 1)
                this.creatingChildren.delete(request.parentChatId);
            else
                this.creatingChildren.set(request.parentChatId, count - 1);
        }
    }
    /** Reserves global capacity for both creation and follow-up sends before their first await. */
    reserveStart() {
        if ([...this.executions.values()].filter(execution => !execution.settled).length + this.pendingStarts >= model_js_1.MAX_ACTIVE_AGENTS)
            throw new Error("At most " + model_js_1.MAX_ACTIVE_AGENTS + " subagents may run at once");
        this.pendingStarts++;
    }
    /** Prevents competing interrupt and close requests from scheduling overlapping turns. */
    async command(parentChatId, agentId, operation) {
        this.owned(parentChatId, agentId);
        if (this.commands.has(agentId))
            throw new Error("Another control operation is still in progress for this subagent");
        this.commands.add(agentId);
        try {
            return await operation();
        }
        finally {
            this.commands.delete(agentId);
        }
    }
    /** Starts generation without holding the caller's tool or composer action open. */
    launch(agent) {
        let resolveDone;
        let rejectDone;
        const done = new Promise((resolve, reject) => { resolveDone = resolve; rejectDone = reject; });
        const execution = { agentId: agent.id, runNumber: agent.runNumber, settled: false, failure: null, done };
        this.executions.set(agent.id, execution);
        void this.execute(execution).then(resolveDone, rejectDone);
        void done.catch(error => {
            this.failWaiters(execution, error);
            this.reportError(error);
        });
    }
    /** Consumes native semantic replacements and records only the actual originating terminal receipt. */
    async execute(execution) {
        const agent = this.current(execution.agentId);
        const request = {
            kind: "submit", chatId: agent.chatId, runtime: "main", notifyReply: false,
            input: { text: agent.lastInput, attachments: [], replyToMessageTimestamp: null },
            turn: agent.participantId === null ? { kind: "execute" } : { kind: "execute", participantId: agent.participantId },
        };
        let iterator;
        try {
            iterator = this.chat.send(request)[Symbol.asyncIterator]();
        }
        catch (error) {
            try {
                return await this.finishFailure(execution, (0, model_js_1.errorText)(error));
            }
            finally {
                execution.settled = true;
            }
        }
        try {
            while (true) {
                // Catch only native observation errors; failed storage writes must not be retried as task updates.
                const pulled = await iterator.next().then(value => ({ type: "value", value }), error => ({ type: "error", error }));
                if (pulled.type === "error")
                    return await this.finishFailure(execution, (0, model_js_1.errorText)(pulled.error));
                if (pulled.value.done)
                    return await this.finishFailure(execution, "Native observation ended without its terminal receipt");
                const event = pulled.value.value;
                const current = this.current(execution.agentId);
                if (current.runNumber !== execution.runNumber)
                    throw new Error("Subagent turn identity changed during observation");
                const invalid = this.eventError(current, event);
                if (invalid !== null)
                    return await this.finishFailure(execution, invalid);
                if (event.type === "part") {
                    const index = current.messages.findIndex(message => message.messageTimestamp === event.messageTimestamp);
                    const messages = current.messages.slice();
                    const snapshot = { messageTimestamp: event.messageTimestamp, revision: event.revision, parts: event.parts };
                    if (index === -1)
                        messages.push(snapshot);
                    else
                        messages[index] = snapshot;
                    await this.save({ ...current, status: current.status === "closing" ? "closing" : "running", messages, updatedAt: this.clock.now() });
                    continue;
                }
                if (event.type !== "completed")
                    throw new Error("Invalid native subagent completion");
                const outcome = { ...event.result.outcome };
                let status;
                let error = null;
                switch (outcome.type) {
                    case "committed":
                    case "not_persisted":
                        status = outcome.status;
                        break;
                    case "blocked":
                        status = "failed";
                        error = outcome.message === undefined ? "The chat input hook blocked this task" : outcome.message;
                        break;
                    case "consumed":
                        status = "failed";
                        error = "A chat input hook consumed this turn; its later execution is not owned by this subagent";
                        break;
                    default: throw new Error("Unknown native subagent outcome");
                }
                const finished = {
                    ...current, status: current.status === "closing" ? "closed" : status,
                    updatedAt: this.clock.now(), error,
                    result: { response: event.result.aiResponse === undefined ? null : event.result.aiResponse, outcome },
                };
                await this.save(finished);
                this.publish(finished);
                return finished;
            }
        }
        finally {
            try {
                if (iterator.return !== undefined)
                    await iterator.return();
            }
            finally {
                execution.settled = true;
            }
        }
    }
    /** Validates native identities and revision order before any event can mutate the durable task. */
    eventError(agent, event) {
        if (event.type === "part") {
            if (event.chatId !== agent.chatId)
                return "Native snapshot names a different conversation";
            if (!Number.isSafeInteger(event.messageTimestamp) || !Number.isSafeInteger(event.revision) || event.revision < 1 || !Array.isArray(event.parts))
                return "Native snapshot has invalid fields";
            const previous = agent.messages.find(message => message.messageTimestamp === event.messageTimestamp);
            if (previous !== undefined && event.revision <= previous.revision)
                return "Native snapshot revision did not advance";
            return null;
        }
        if (event.type !== "completed" || event.result.chatId !== agent.chatId)
            return "Native completion names a different conversation";
        const outcome = event.result.outcome;
        if (outcome.type === "committed" || outcome.type === "not_persisted") {
            if (outcome.status !== "completed" && outcome.status !== "cancelled")
                return "Native completion has an invalid status";
        }
        else if (outcome.type !== "blocked" && outcome.type !== "consumed")
            return "Native completion has an invalid outcome";
        return null;
    }
    /** Persists an actual observation failure without inventing a successful native receipt. */
    async finishFailure(execution, error) {
        const current = this.current(execution.agentId);
        const failed = { ...current, status: "failed", updatedAt: this.clock.now(), error, result: null };
        await this.save(failed);
        this.publish(failed);
        return failed;
    }
    /** Waits for cancellation admission or settlement of this same turn while the command lock excludes newer turns. */
    async cancelExecution(execution) {
        while (!execution.settled) {
            const acknowledgement = await this.chat.cancel(execution.agentId);
            if (acknowledgement.chatId !== execution.agentId || typeof acknowledgement.cancelRequested !== "boolean")
                throw new Error("Invalid native cancellation acknowledgement");
            if (acknowledgement.cancelRequested)
                break;
            await this.clock.delay(50);
        }
        await execution.done;
    }
    /** Interrupts only the owned turn, then creates one new turn after its genuine finalization. */
    async sendInput(request) {
        (0, model_js_1.requireText)(request.message, "message");
        if (typeof request.interrupt !== "boolean")
            throw new Error("interrupt must be boolean");
        const current = this.owned(request.parentChatId, request.agentId);
        if (current.status === "closed" || current.status === "closing")
            throw new Error("Subagent is closed or closing");
        const execution = this.executions.get(current.id);
        if (execution !== undefined && !execution.settled) {
            if (!request.interrupt)
                throw new Error("Subagent is running; set interrupt=true to replace its current turn");
            await this.cancelExecution(execution);
        }
        this.reserveStart();
        try {
            if (await this.chat.isProcessing(current.chatId))
                throw new Error("The native conversation still has an active execution; a new turn cannot start");
            const latest = this.current(current.id);
            const next = { ...latest, status: "starting", runNumber: latest.runNumber + 1, lastInput: request.message,
                updatedAt: this.clock.now(), result: null, error: null, messages: [] };
            await this.save(next);
            this.launch(next);
            return (0, model_js_1.summary)(next);
        }
        finally {
            this.pendingStarts--;
        }
    }
    /** Cancels the owned execution before closing the task while retaining its conversation and history. */
    async close(parentChatId, agentId) {
        const agent = this.owned(parentChatId, agentId);
        if (agent.status === "closed")
            return (0, model_js_1.summary)(agent);
        if (this.creatingChildren.has(agent.chatId))
            throw new Error("A child creation is still in progress; wait for its creation acknowledgement before closing the parent");
        const children = [...this.agents.values()].filter(child => child.parentChatId === agent.chatId && child.status !== "closed");
        for (const child of children)
            await this.command(child.parentChatId, child.id, () => this.close(child.parentChatId, child.id));
        const execution = this.executions.get(agentId);
        if (execution !== undefined && !execution.settled) {
            await this.save({ ...this.current(agentId), status: "closing", updatedAt: this.clock.now() });
            await this.cancelExecution(execution);
        }
        // A runtime restart loses the observation; close still requires the actual host conversation to become idle.
        if (await this.chat.isProcessing(agent.chatId)) {
            const acknowledgement = await this.chat.cancel(agent.chatId);
            if (acknowledgement.chatId !== agent.chatId || !acknowledgement.cancelRequested)
                throw new Error("The active native conversation could not be cancelled by this package");
            while (await this.chat.isProcessing(agent.chatId))
                await this.clock.delay(100);
        }
        const closed = { ...this.current(agentId), status: "closed", updatedAt: this.clock.now() };
        await this.save(closed);
        this.publish(closed);
        return (0, model_js_1.summary)(closed);
    }
    /** Subscribes to the exact current turns and removes every listener and timer when this wait ends. */
    wait(parentChatId, agentIds, timeoutMs) {
        if (!Array.isArray(agentIds) || agentIds.length < 1 || agentIds.length > 16 || new Set(agentIds).size !== agentIds.length)
            throw new Error("agent_ids must contain 1..16 unique identities");
        if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 30000)
            throw new Error("timeout_ms must be an integer from 1 to 30000");
        const agents = agentIds.map(id => this.owned(parentChatId, id));
        for (const agent of agents) {
            const execution = this.executions.get(agent.id);
            if (execution !== undefined && execution.runNumber === agent.runNumber && execution.failure !== null)
                return Promise.reject(execution.failure);
        }
        if (agents.some(agent => !(0, model_js_1.isActive)(agent.status)))
            return Promise.resolve({ timed_out: false, agents: agents.map(model_js_1.summary) });
        return new Promise((resolve, reject) => {
            const waiter = {
                targets: new Map(agents.map(agent => [agent.id, agent.runNumber])),
                observed: new Map(agents.map(agent => [agent.id, (0, model_js_1.summary)(agent)])),
                resolve, reject, cancelTimer: () => { },
            };
            waiter.cancelTimer = this.clock.schedule(() => this.finishWait(waiter, true), timeoutMs);
            this.waiters.add(waiter);
        });
    }
    /** Completes a wait with only its captured turns, even if another command already started a later turn. */
    finishWait(waiter, timedOut) {
        waiter.cancelTimer();
        this.waiters.delete(waiter);
        waiter.resolve({ timed_out: timedOut, agents: [...waiter.observed.values()] });
    }
    /** Sends settled snapshots to matching turns without waking waits for unrelated or newer work. */
    publish(agent) {
        for (const waiter of [...this.waiters]) {
            if (waiter.targets.get(agent.id) !== agent.runNumber)
                continue;
            waiter.observed.set(agent.id, (0, model_js_1.summary)(agent));
            if (!(0, model_js_1.isActive)(agent.status))
                this.finishWait(waiter, false);
        }
    }
    /** Propagates infrastructure failures to live callers rather than allowing a false timeout or completion. */
    failWaiters(execution, error) {
        execution.failure = error;
        for (const waiter of [...this.waiters]) {
            if (waiter.targets.get(execution.agentId) !== execution.runNumber)
                continue;
            waiter.cancelTimer();
            this.waiters.delete(waiter);
            waiter.reject(error);
        }
    }
}
exports.AgentManager = AgentManager;
