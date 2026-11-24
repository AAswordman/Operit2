import type { Storage } from "../../../../types/storage";
import { concurrencyLimit, DEFAULT_SETTINGS, MAX_ACTIVE_AGENTS, parseAgent, parseSettings, requireText, summary, type Agent, type AgentSettings, type AgentSummary, type ParentSnapshot } from "./model.js";

export interface AgentRepository {
  /** Loads durable agents once into the owning manager. */
  load(): Promise<Agent[]>;
  /** Atomically publishes the detailed record and its parent status list. */
  save(agent: Agent): Promise<void>;
  loadLimit(): Promise<number>;
  saveLimit(limit: number): Promise<void>;
  loadSettings(): Promise<AgentSettings>;
  saveSettings(settings: AgentSettings): Promise<void>;
}

let database: Promise<Storage.ObjectDatabase> | null = null;

/** Opens the same host-backed package database in tools, the retained owner, and UI contexts. */
function openDatabase(): Promise<Storage.ObjectDatabase> {
  if (database === null) database = Tools.Storage.objects.open({ path: ToolPkg.getSpaceDataDir() + "/subagents.sqlite" });
  return database;
}

/** Owns a serialized commit stream so concurrent child updates cannot overwrite sibling summaries. */
export class Repository implements AgentRepository {
  private readonly committed = new Map<string, AgentSummary>();
  private writes: Promise<void> = Promise.resolve();

  /** Reads the persisted space-wide limit, retaining the original limit for existing installations. */
  async loadLimit(): Promise<number> {
    const entry = await (await openDatabase()).collection<unknown>("settings").get("concurrency");
    return entry === null ? MAX_ACTIVE_AGENTS : concurrencyLimit(entry.value);
  }

  /** Stores an explicitly validated concurrency setting. */
  async saveLimit(limit: number): Promise<void> {
    await (await openDatabase()).commit([{ op: "put", collection: "settings", key: "concurrency", value: concurrencyLimit(limit) }]);
  }

  /** Loads optional settings introduced after the original concurrency preference. */
  async loadSettings(): Promise<AgentSettings> {
    const entry = await (await openDatabase()).collection<unknown>("settings").get("behavior");
    return entry === null ? { ...DEFAULT_SETTINGS } : parseSettings(entry.value);
  }

  /** Persists validated behavior preferences as one atomic value. */
  async saveSettings(settings: AgentSettings): Promise<void> {
    const value = parseSettings(settings);
    await (await openDatabase()).commit([{ op: "put", collection: "settings", key: "behavior", value: { ...value } }]);
  }

  /** Reads every bounded storage page and rejects corrupt or duplicated identities. */
  async load(): Promise<Agent[]> {
    const collection = (await openDatabase()).collection<unknown>("agents");
    const agents: Agent[] = [];
    let after: string | null = null;
    while (true) {
      const page = await collection.list(after, 200);
      if (page.length === 0) return agents;
      for (const entry of page) {
        const agent = parseAgent(entry.value);
        if (agent.id !== entry.key || this.committed.has(agent.id)) throw new Error("Subagent storage identity mismatch");
        this.committed.set(agent.id, summary(agent));
        agents.push(agent);
      }
      after = page[page.length - 1].key;
    }
  }

  /** Commits one immutable snapshot and the parent's list in the same host transaction. */
  save(agent: Agent): Promise<void> {
    const pending = this.writes.then(async () => {
      const next = new Map(this.committed);
      next.set(agent.id, summary(agent));
      const parent: ParentSnapshot = {
        schemaVersion: 1,
        agents: [...next.values()].filter(item => item.parentChatId === agent.parentChatId),
      };
      await (await openDatabase()).commit([
        { op: "put", collection: "agents", key: agent.id, value: agent as unknown as Storage.Value },
        { op: "put", collection: "parents", key: agent.parentChatId, value: parent as unknown as Storage.Value },
      ]);
      this.committed.set(agent.id, summary(agent));
    });
    this.writes = pending;
    return pending;
  }
}

/** Observes exact parent snapshots across engines without timer-based UI polling. */
export async function* watchParent(parentChatId: string, signal: AbortSignal): AsyncGenerator<ParentSnapshot> {
  requireText(parentChatId, "parentChatId");
  for await (const entry of (await openDatabase()).collection<unknown>("parents").watch(parentChatId, { signal })) {
    if (entry === null) {
      yield { schemaVersion: 1, agents: [] };
      continue;
    }
    const value = entry.value as ParentSnapshot;
    if (value === null || typeof value !== "object" || value.schemaVersion !== 1 || !Array.isArray(value.agents)) throw new Error("Invalid parent agent list");
    for (const item of value.agents) {
      parseAgent({ ...item, messages: [] });
      if (item.parentChatId !== parentChatId) throw new Error("Subagent parent identity mismatch");
    }
    yield value;
  }
}

/** Streams the selected agent's current semantic snapshots and final native receipt. */
export async function* watchAgent(parentChatId: string, agentId: string, signal: AbortSignal): AsyncGenerator<Agent> {
  for await (const entry of (await openDatabase()).collection<unknown>("agents").watch(agentId, { signal })) {
    if (entry === null) throw new Error("Subagent does not exist: " + agentId);
    const agent = parseAgent(entry.value);
    if (agent.id !== agentId || agent.parentChatId !== parentChatId) throw new Error("Subagent detail identity mismatch");
    yield agent;
  }
}
