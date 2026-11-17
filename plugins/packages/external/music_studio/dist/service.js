"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.StudioService = void 0;
const model_1 = require("./shared/model");
const validation_1 = require("./shared/validation");
const operations_1 = require("./shared/operations");
const composition_1 = require("./shared/composition");
const model_2 = require("./shared/model");
const presets_1 = require("./shared/presets");
const offline = () => ({ connected: false, unlocked: false, playing: false, beat: 0, peak: 0, voices: 0, dropped: 0, updatedAt: 0, projectId: "" });
/** Single main-runtime owner. Injected storage makes concurrency and failure behavior testable. */
class StudioService {
    constructor(storage) {
        this.storage = storage;
        this.store = null;
        this.queue = Promise.resolve();
        this.status = offline();
        this.commands = [];
        this.receipts = [];
        this.history = new Map();
    }
    request(raw) {
        const next = this.queue.then(() => this.dispatch(raw));
        this.queue = next.catch(() => undefined);
        return next;
    }
    async load() {
        if (!this.store) {
            const data = await this.storage.read();
            if (data) {
                if (data.version !== 1)
                    throw new Error("Unsupported studio database version");
                const projects = (0, validation_1.array)(data.projects, model_1.LIMITS.projects, "projects").map(validation_1.parseProject);
                if (!projects.length || new Set(projects.map(p => p.id)).size !== projects.length || !projects.some(p => p.id === data.activeId))
                    throw new Error("Invalid studio project catalog; stored data was not overwritten");
                this.store = { version: 1, activeId: data.activeId, projects };
            }
            else {
                const p = (0, composition_1.demoProject)();
                const initial = { version: 1, activeId: p.id, projects: [p] };
                await this.storage.write((0, model_1.copy)(initial));
                this.store = initial;
            }
        }
        return this.store;
    }
    async commit(next) { await this.storage.write((0, model_1.copy)(next)); this.store = next; }
    current() { return this.store.projects.find(p => p.id === this.store.activeId); }
    snapshot() {
        const connected = Date.now() - this.status.updatedAt < 6000 && this.status.projectId === this.store.activeId;
        return (0, model_1.copy)({ project: this.current(), projects: this.store.projects.map(({ id, name, revision }) => ({ id, name, revision })), status: { ...this.status, connected, playing: connected && this.status.playing }, commands: this.commands, receipts: this.receipts });
    }
    expect(p, r) {
        if (r.projectId !== p.id || r.revision !== p.revision)
            throw new Error(`REVISION_CONFLICT: get project first; active=${p.id}, revision=${p.revision}`);
    }
    cancelCommands(message) {
        for (const c of this.commands)
            this.receipts = [...this.receipts.filter(r => r.id !== c.id), { id: c.id, state: "failed", message }];
        this.receipts = this.receipts.slice(-30);
        this.commands = [];
    }
    async dispatch(raw) {
        const r = (0, validation_1.object)(raw, "request");
        const action = (0, validation_1.text)(r.action, "action");
        const db = await this.load();
        const p = this.current();
        for (const command of [...this.commands])
            if (Date.now() - command.createdAt > (command.type === "render" ? 600000 : 120000)) {
                this.commands = this.commands.filter(c => c.id !== command.id);
                this.receipts = [...this.receipts.filter(c => c.id !== command.id), { id: command.id, state: "failed", message: "UI command timed out" }].slice(-30);
            }
        if (action === "catalog")
            return { presets: presets_1.PRESETS, effects: presets_1.EFFECTS, templates: composition_1.TEMPLATES, synthRanges: validation_1.SYNTH_RANGES, synthIntegers: Array.from(validation_1.SYNTH_INTEGERS), automation: { targets: { level: [0, 1.5], pan: [-1, 1], cutoff: [40, 18000] }, maxLanes: 3, maxPoints: 1024, interpolation: "linear", beatUnit: "quarter-note" }, limits: model_1.LIMITS, units: "start/duration = quarter-note beats; pitch = MIDI; gain/velocity = linear; time is zero-based" };
        if (action === "get")
            return this.snapshot();
        if (action === "sync") {
            if (r.status !== undefined) {
                const s = (0, validation_1.object)(r.status);
                this.status = { connected: true, unlocked: s.unlocked === true, playing: s.playing === true, beat: (0, validation_1.number)(s.beat, 0, model_1.LIMITS.bars * 7, "beat"), peak: (0, validation_1.number)(s.peak, 0, 2, "peak"), voices: (0, validation_1.number)(s.voices, 0, 100000, "voices", true), dropped: (0, validation_1.number)(s.dropped, 0, Number.MAX_SAFE_INTEGER, "dropped", true), updatedAt: Date.now(), projectId: (0, validation_1.id)(s.projectId) };
            }
            const snapshot = this.snapshot();
            return r.projectId === p.id && r.revision === p.revision ? { status: snapshot.status, commands: snapshot.commands, receipts: snapshot.receipts } : snapshot;
        }
        if (action === "ack") {
            const command = this.commands.find(c => c.id === r.id);
            if (!command)
                throw new Error("Unknown or expired command receipt");
            this.commands = this.commands.filter(c => c !== command);
            const state = (0, validation_1.choice)(r.state, ["done", "failed"], "state");
            const receipt = { id: command.id, state, message: (0, validation_1.text)(r.message, "message", 1000) };
            if (r.path !== undefined)
                receipt.path = (0, validation_1.text)(r.path, "path", 1024);
            this.receipts = [...this.receipts.filter(c => c.id !== command.id), receipt].slice(-30);
            return receipt;
        }
        if (action === "command") {
            this.expect(p, r);
            const type = (0, validation_1.choice)(r.type, ["play", "pause", "stop", "seek", "render"], "command");
            const status = this.snapshot().status;
            if (!status.connected)
                throw new Error("音乐工作台未打开。请用户从侧边栏打开音乐工作台；无界面仍可编曲与保存，但不能播放或渲染。");
            if (type === "play" && !status.unlocked)
                throw new Error("需要用户先在工作台点击一次播放以启用音频。");
            if (this.commands.length >= 8)
                throw new Error("Playback command queue is full");
            if (type === "render" && this.commands.some(c => c.type === "render"))
                throw new Error("Render already queued/running");
            if (type === "render" && p.bars * p.beatsPerBar * 60 / p.bpm > model_1.LIMITS.renderSeconds)
                throw new Error(`WAV render limited to ${model_1.LIMITS.renderSeconds}s; shorten project or export JSON`);
            const c = { id: (0, model_1.uid)("cmd"), type, projectId: p.id, revision: p.revision, createdAt: Date.now() };
            if (r.beat !== undefined)
                c.beat = (0, validation_1.number)(r.beat, 0, p.bars * p.beatsPerBar - 0.001, "beat");
            if (type === "seek" && c.beat === undefined)
                throw new Error("seek requires beat");
            this.commands.push(c);
            const receipt = { id: c.id, state: "queued", message: "Queued for WebView; get returns completion/error, queued does not mean completed" };
            this.receipts = [...this.receipts, receipt].slice(-30);
            return (0, model_1.copy)(receipt);
        }
        if (action === "export")
            return { filename: `${p.id}.operitmusic.json`, json: JSON.stringify(p, null, 2) };
        if (action === "open") {
            this.expect(p, r);
            const target = (0, validation_1.id)(r.id);
            if (!db.projects.some(p => p.id === target))
                throw new Error("Project not found");
            await this.commit({ ...db, activeId: target });
            this.cancelCommands("Active project changed");
            this.status = offline();
            return this.snapshot();
        }
        if (["create", "import", "duplicate"].includes(action)) {
            this.expect(p, r);
            if (db.projects.length >= model_1.LIMITS.projects)
                throw new Error("Project catalog is full; delete a project first");
            let next = action === "duplicate" ? (0, model_1.copy)(p) : action === "import" ? (0, validation_1.parseProject)(JSON.parse((0, validation_1.text)(r.json, "json", model_1.LIMITS.jsonBytes))) : r.template ? (0, composition_1.demoProject)((0, validation_1.text)(r.template, "template")) : (0, model_2.emptyProject)((0, validation_1.text)(r.name ?? "Untitled session", "name"));
            next.id = (0, model_1.uid)("project");
            next.revision = 0;
            next.createdAt = next.updatedAt = new Date().toISOString();
            if (action === "duplicate")
                next.name = p.name.slice(0, 110) + " · Copy";
            await this.commit({ ...db, activeId: next.id, projects: [...db.projects, next] });
            this.cancelCommands("Active project changed");
            this.status = offline();
            return this.snapshot();
        }
        if (action === "delete") {
            this.expect(p, r);
            if (r.confirm !== p.id)
                throw new Error("delete requires confirm=active project ID");
            if (db.projects.length === 1)
                throw new Error("Keep at least one project");
            const projects = db.projects.filter(t => t.id !== p.id);
            await this.commit({ ...db, projects, activeId: projects[0].id });
            this.history.delete(p.id);
            this.cancelCommands("Project deleted");
            this.status = offline();
            return this.snapshot();
        }
        if (["batch", "save", "undo", "redo"].includes(action)) {
            this.expect(p, r);
            const oldHistory = this.history.get(p.id) ?? { undo: [], redo: [] };
            const history = { undo: [...oldHistory.undo], redo: [...oldHistory.redo] };
            let next;
            if (action === "undo" || action === "redo") {
                const stack = action === "undo" ? history.undo : history.redo;
                const entry = stack.pop();
                if (!entry)
                    throw new Error(`Nothing to ${action}`);
                (action === "undo" ? history.redo : history.undo).push((0, model_1.copy)(p));
                next = (0, model_1.copy)(entry);
                next.revision = p.revision + 1;
                next.updatedAt = new Date().toISOString();
            }
            else {
                next = action === "batch" ? (0, operations_1.applyOperations)(p, (0, validation_1.array)(r.operations, 100, "operations")) : (0, validation_1.parseProject)(r.project);
                if (next.id !== p.id)
                    throw new Error("save cannot change project ID");
                next.revision = p.revision + 1;
                next.createdAt = p.createdAt;
                next.updatedAt = new Date().toISOString();
                history.undo.push((0, model_1.copy)(p));
                history.redo = [];
            }
            history.undo = history.undo.slice(-20);
            history.redo = history.redo.slice(-20);
            await this.commit({ ...db, projects: db.projects.map(a => a.id === p.id ? next : a) });
            this.history.set(p.id, history);
            this.cancelCommands("Project changed; issue command using the new revision");
            return this.snapshot();
        }
        throw new Error(`Unsupported request: ${action}`);
    }
}
exports.StudioService = StudioService;
