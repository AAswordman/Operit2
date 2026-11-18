import type { Storage } from "../../../../types/storage.js";
import type { CaptureRecord, Ticket } from "./model.js";
import type { Store } from "./engine.js";
import { CHANNELS, type Channel } from "./dependencies/connector_hub/api.js";
let database: Promise<Storage.ObjectDatabase> | null = null;
function open(): Promise<Storage.ObjectDatabase> {
  if (database === null) {
    const opening = Tools.Storage.objects.open({ path: ToolPkg.getSpaceDataDir() + "/maintenance.sqlite" });
    database = opening.catch(error => { database = null; throw error; });
  }
  return database;
}
function parse(value: unknown): Ticket {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("Invalid ticket record");
  const item = value as Ticket;
  if (item.schemaVersion !== 1 || typeof item.id !== "string" || !Array.isArray(item.notes)) throw new Error("Invalid ticket record");
  return item;
}
// Model output is untrusted: only fixed, user-facing phrases can cross the channel boundary.
const PUBLIC_QUESTIONS: Readonly<Record<string, string>> = Object.freeze({
  reproduction: "复现步骤", expected_actual: "预期和实际结果", device: "设备与系统", browser: "浏览器及版本", extension: "相关扩展的名称", error: "出现的错误信息",
});
const PUBLIC_GUIDANCE: Readonly<Record<string, string>> = Object.freeze({
  restart_game: "点击游戏页面中的重新开始按钮", allow_javascript: "允许这个网页运行 JavaScript", check_extension: "检查相关浏览器扩展的设置", reload: "重新加载页面",
});
function publicPhrases(input: unknown, table: Readonly<Record<string, string>>): string[] {
  if (!Array.isArray(input) || input.length > 12) return [];
  const seen = new Set<string>(), result: string[] = [];
  for (const item of input) {
    if (typeof item !== "string" || !Object.prototype.hasOwnProperty.call(table, item) || seen.has(item)) continue;
    seen.add(item); result.push(table[item]);
    if (result.length === 3) break;
  }
  return result;
}
export class Repository implements Store {
  async dashboardSources(): Promise<{captures: CaptureRecord[]}> {
    const db = await open(), captures: CaptureRecord[] = [];
    let after: string | null = null;
    while (true) {
      const page = await db.collection<CaptureRecord>("captured").list(after, 200);
      if (!page.length) break;
      captures.push(...page.map(entry => entry.value)); after = page[page.length - 1].key;
    }
    return {captures};
  }

  /** A deliberate per-conversation binding avoids watching unrelated chats. */
  async bindChat(chatId: string, projectPath: string | null, since: number): Promise<void> {
    const collection = (await open()).collection<{ projectPath: string | null; since?: number }>("chat_bindings");
    const previous = await collection.get(chatId);
    await (await open()).commit([{op: "put", collection: "chat_bindings", key: chatId, value: {projectPath, since}, expectedVersion: previous?.version ?? null}]);
  }
  async chatBinding(chatId: string): Promise<{ projectPath: string | null; since: number | null } | null> {
    const entry = await (await open()).collection<{ projectPath: string | null; since?: number }>("chat_bindings").get(chatId);
    if (entry === null) return null;
    return { projectPath: entry.value.projectPath, since: Number.isSafeInteger(entry.value.since) ? entry.value.since! : null };
  }
  async chatProject(chatId: string): Promise<string | null> { return (await this.chatBinding(chatId))?.projectPath ?? null; }
  async chatBindings(): Promise<{ chatId: string; projectPath: string | null; since: number | null }[]> {
    const collection = (await open()).collection<{ projectPath: string | null; since?: number }>("chat_bindings");
    const result: { chatId: string; projectPath: string | null; since: number | null }[] = [];
    let after: string | null = null;
    while (true) {
      const page = await collection.list(after, 200);
      if (!page.length) return result;
      for (const entry of page) result.push({chatId: entry.key, projectPath: entry.value.projectPath, since: Number.isSafeInteger(entry.value.since) ? entry.value.since! : null});
      after = page[page.length - 1].key;
    }
  }
  /** Reservations coordinate writers that share this object database, not separate device replicas. */
  async claimProject(path: string, ticketId: string, owner: string): Promise<boolean> {
    const key = projectKey(path), db = await open(), collection = db.collection<{path: string; ticketId: string; owner: string}>("project_claims");
    const existing = await collection.get(key);
    if (existing !== null) return existing.value.owner === owner && existing.value.ticketId === ticketId;
    try {
      await db.commit([{op: "put", collection: "project_claims", key, value: {path, ticketId, owner}, expectedVersion: null}]);
      return true;
    } catch (error) {
      const raced = await collection.get(key);
      if (raced === null) throw error;
      return raced.value.owner === owner && raced.value.ticketId === ticketId;
    }
  }
  async projectClaim(path: string): Promise<{ticketId: string} | null> {
    const entry = await (await open()).collection<{ticketId: string}>("project_claims").get(projectKey(path));
    if (entry === null) return null;
    if (typeof entry.value.ticketId !== "string" || !entry.value.ticketId) throw new Error("Invalid project reservation");
    return {ticketId: entry.value.ticketId};
  }
  /** Atomically hands an existing reservation to the same failed ticket and transitions it. */
  async reclaimFailed(id: string, expectedAgentId: string, owner: string, now: number): Promise<{ticket: Ticket; resumeReview: boolean}> {
    const db = await open(), tickets = db.collection<unknown>("tickets");
    const entry = await tickets.get(id);
    if (entry === null) throw new Error("Ticket does not exist");
    const ticket = parse(entry.value);
    if (ticket.id !== id || ticket.stage !== "failed" || ticket.agentId !== expectedAgentId) throw new Error("Ticket changed while checking recovery");
    const key = projectKey(ticket.projectPath);
    const claims = db.collection<{path: string; ticketId: string; owner: string}>("project_claims"), claim = await claims.get(key);
    if (claim === null || claim.value.ticketId !== id) throw new Error("Project reservation changed while checking recovery");
    const resumeReview = ticket.notes.some(note => note.kind === "implement");
    ticket.stage = resumeReview ? "review" : "triage"; ticket.error = null; ticket.agentId = null; ticket.updatedAt = now;
    await db.commit([
      {op:"put", collection:"project_claims", key, value:{...claim.value, owner}, expectedVersion:claim.version},
      {op:"put", collection:"tickets", key:id, value:ticket as unknown as Storage.Value, expectedVersion:entry.version},
    ]);
    return {ticket, resumeReview};
  }
  async releaseProject(path: string, owner: string): Promise<void> {
    const key = projectKey(path), collection = (await open()).collection<{owner: string}>("project_claims"), existing = await collection.get(key);
    if (existing === null) return;
    if (existing.value.owner !== owner) throw new Error("Project reservation owner changed");
    await collection.remove(key, {expectedVersion: existing.version});
  }
  private readonly versions = new WeakMap<Ticket, string>();
  async list(): Promise<Ticket[]> {
    const collection = (await open()).collection<unknown>("tickets");
    const result: Ticket[] = []; let after: string | null = null;
    while (true) {
      const page = await collection.list(after, 200);
      if (page.length === 0) return result;
      for (const entry of page) { const ticket = parse(entry.value); if (ticket.id !== entry.key) throw new Error("Ticket identity mismatch"); result.push(ticket); }
      after = page[page.length - 1].key;
    }
  }
  async get(id: string): Promise<Ticket | null> {
    const result = await (await open()).collection<unknown>("tickets").get(id);
    if (result === null) return null;
    const ticket = parse(result.value);
    this.versions.set(ticket, result.version);
    return ticket;
  }
  async save(ticket: Ticket, create = false): Promise<void> {
    const version = this.versions.get(ticket);
    if (!create && version === undefined) throw new Error("Ticket must be read before updating");
    await (await open()).commit([{ op: "put", collection: "tickets", key: ticket.id,
      value: ticket as unknown as Storage.Value, expectedVersion: create ? null : version! }]);
  }
  /** Requeue a clarified ticket only after the preceding run releases its project. */
  async queuePendingInfo(id: string, now: number): Promise<boolean> {
    const db = await open(), tickets = db.collection<unknown>("tickets"), entry = await tickets.get(id);
    if (entry === null) return false;
    const ticket = parse(entry.value);
    if (ticket.id !== id || ticket.stage !== "needs_info" || ticket.error !== null || ticket.autoResumeAfterInfo !== true) return false;
    const key = projectKey(ticket.projectPath);
    if (await db.collection<unknown>("project_claims").get(key) !== null) return false;
    ticket.stage = "new"; ticket.agentId = null; ticket.autoStart = true;
    ticket.forceTriage = true; ticket.autoResumeAfterInfo = false; ticket.updatedAt = now;
    await db.commit([{op:"put", collection:"tickets", key:id, value:ticket as unknown as Storage.Value, expectedVersion:entry.version}],
      [{collection:"project_claims", key, expectedVersion:null}]);
    return true;
  }
  /** Atomically deduplicates each upstream message across retries and restarts. */
  async capture(record: CaptureRecord): Promise<boolean> {
    const db = await open(), key = record.message.key;
    const collection = db.collection<CaptureRecord>("captured");
    const previous = await collection.get(key);
    if (previous !== null) {
      if ((previous.value.message.channel !== record.message.channel || previous.value.message.accountId !== record.message.accountId || previous.value.message.messageId !== record.message.messageId || previous.value.message.conversationId !== record.message.conversationId || previous.value.message.authorId !== record.message.authorId) || previous.value.parentChatId !== record.parentChatId || previous.value.projectPath !== record.projectPath) throw new Error("Message is already bound to a different project or conversation");
      return false;
    }
    try {
      await db.commit([{ op: "put", collection: "captured", key, value: record as unknown as Storage.Value, expectedVersion: null }]);
      return true;
    } catch (error) {
      const raced = await collection.get(key);
      if (raced === null) throw error;
      if ((raced.value.message.channel !== record.message.channel || raced.value.message.accountId !== record.message.accountId || raced.value.message.messageId !== record.message.messageId || raced.value.message.conversationId !== record.message.conversationId || raced.value.message.authorId !== record.message.authorId) || raced.value.parentChatId !== record.parentChatId || raced.value.projectPath !== record.projectPath) throw new Error("Message is already bound to a different project or conversation");
      return false;
    }
  }
  async pendingCaptures(): Promise<{ key: string; version: string; value: CaptureRecord }[]> {
    const collection = (await open()).collection<CaptureRecord>("captured");
    const pending: { key: string; version: string; value: CaptureRecord }[] = [];
    let after: string | null = null;
    while (true) {
      const page = await collection.list(after, 200);
      if (page.length === 0) return pending;
      pending.push(...page.filter(entry => entry.value.ticketId === null && !entry.value.ignored));
      after = page[page.length - 1].key;
    }
  }
  /** Record a non-actionable conversational acknowledgement without losing deduplication. */
  async ignoreAcknowledgements(captures: {key: string; version: string; value: CaptureRecord}[]): Promise<void> {
    if (captures.length === 0 || captures.length > 50) throw new Error("Invalid acknowledgement batch");
    await (await open()).commit(captures.map(item => ({op: "put", collection: "captured", key: item.key,
      value: {...item.value, ignored: "acknowledgement"} as unknown as Storage.Value, expectedVersion: item.version})));
  }
  /** Return terminal notices only for captures owned by the exact configured binding. */
  async channelNotifications(channel: Channel, parentChatId: string, projectPath: string, conversationId: string, since: number): Promise<{eventId: string; messageKey: string; text: string}[]> {
    if (!CHANNELS.includes(channel) || !parentChatId || !projectPath || !conversationId || !Number.isSafeInteger(since) || since <= 0) throw new Error("Invalid channel notification binding");
    const db = await open(), captured = db.collection<CaptureRecord>("captured"), selected = new Map<string, CaptureRecord>(), queries: CaptureRecord[] = [];
    let after: string | null = null;
    while (true) {
      const page = await captured.list(after, 200);
      for (const entry of page) {
        const item = entry.value, id = item.ticketId;
        if (item.receivedAt < since || item.parentChatId !== parentChatId || item.projectPath !== projectPath || item.message.channel !== channel || item.message.conversationId !== conversationId) continue;
        if (id === null) { if (item.ignored === "status_query") queries.push(item); continue; }
        const old = selected.get(id);
        if (old === undefined || item.receivedAt > old.receivedAt || (item.receivedAt === old.receivedAt && item.message.key > old.message.key)) selected.set(id, item);
      }
      if (page.length < 200) break;
      after = page[page.length - 1].key;
    }
    const result: {eventId: string; messageKey: string; text: string}[] = [];
    const stageLabels: Record<string, string> = {new:"等待处理",triage:"正在诊断",needs_info:"等待补充",not_actionable:"无需修改",implement:"正在修复",review:"正在验收",delivery:"正在交付",completed:"已完成",interrupted:"处理中断",failed:"需要关注"};
    for (const item of queries) {
      const source = item.message;
      const status = await this.channelStatus({channel, parentChatId, projectPath, accountId:source.accountId, conversationId, authorId:source.authorId});
      const explicit = /\b[a-z0-9]{6,16}-[a-f0-9]{32}\b/i.exec(source.text)?.[0];
      const entries: Ticket[] = [];
      for (const id of status.ids) {
        if (explicit && id !== explicit) continue;
        const entry = await db.collection<unknown>("tickets").get(id);
        if (entry === null) continue;
        const ticket = parse(entry.value);
        if (ticket.id === id && ticket.parentChatId === parentChatId && ticket.projectPath === projectPath && ticket.source === channel) entries.push(ticket);
      }
      entries.sort((a,b) => b.updatedAt - a.updatedAt);
      const details = entries.slice(0,5).map(ticket => `工单 ${ticket.id}：${stageLabels[ticket.stage] ?? "状态未知"}`);
      if (!explicit && status.pending) details.push(`${status.pending} 条反馈正在整理`);
      if (entries.length > 5) details.push(`另有 ${entries.length - 5} 张工单`);
      const text = details.length ? `目前进度：${details.join("；")}。` : "暂时没有找到与你的这条查询对应的工单；如刚提交反馈，可能仍在收集中。";
      result.push({eventId:JSON.stringify(["status_query", source.key]),messageKey:source.key,text});
    }
    for (const [id, item] of selected) {
      const entry = await db.collection<unknown>("tickets").get(id);
      if (entry === null) continue;
      const ticket = parse(entry.value);
      if (ticket.parentChatId !== parentChatId || ticket.projectPath !== projectPath || ticket.source !== channel) continue;
      let marker: Ticket["notes"][number] | undefined, text = "";
      if (ticket.stage === "completed") {
        marker = [...ticket.notes].reverse().find(note => note.kind === "delivery_verification");
        text = `反馈已完成处理，修复已通过验收并生成本地提交。工单：${id}`;
      } else if (ticket.stage === "needs_info" && ticket.error === null) {
        marker = [...ticket.notes].reverse().find(note => note.kind === "triage");
        try {
          const report = marker === undefined ? null : JSON.parse(marker.text);
          if (report?.decision !== "needs_info") marker = undefined;
          const questions = publicPhrases(report?.publicQuestions, PUBLIC_QUESTIONS);
          text = questions.length ? `为了弄清楚这个问题，方便补充${questions.join("、")}吗？回复时请带上工单：${id}`
            : `这条反馈还需要一些信息：请补充复现步骤、你预期的结果和实际情况，回复时请带上工单：${id}`;
        } catch { marker = undefined; }
      } else if (ticket.stage === "not_actionable") {
        marker = [...ticket.notes].reverse().find(note => note.kind === "triage");
        try {
          const report = marker === undefined ? null : JSON.parse(marker.text);
          if (report?.decision !== "not_actionable") marker = undefined;
          const guidance = publicPhrases(report?.publicGuidance, PUBLIC_GUIDANCE);
          text = guidance.length ? `目前看起来无需修改代码，可以试试${guidance.join("，然后")}。如果仍有问题，请补充情况。工单：${id}`
            : `目前判断这条反馈无需修改代码；如果仍有问题，请补充具体情况。工单：${id}`;
        } catch { marker = undefined; }
      }
      if (marker !== undefined && Number.isSafeInteger(marker.at)) result.push({eventId: JSON.stringify([id, ticket.stage, marker.at]), messageKey: item.message.key, text});
    }
    return result;
  }

  /** Only return status for messages from this exact authenticated channel sender. */
  async channelStatus(scope: {channel: Channel; parentChatId: string; projectPath: string; accountId: string; conversationId: string; authorId: string}): Promise<{pending: number; ids: string[]}> {
    const rows = (await open()).collection<CaptureRecord>("captured"), ids = new Set<string>();
    let after: string | null = null, pending = 0;
    while (true) {
      const page = await rows.list(after, 200);
      for (const {value} of page) {
        const m = value.message;
        if (value.parentChatId !== scope.parentChatId || value.projectPath !== scope.projectPath || m.channel !== scope.channel ||
          m.accountId !== scope.accountId || m.conversationId !== scope.conversationId || m.authorId !== scope.authorId) continue;
        if (value.ticketId !== null) ids.add(value.ticketId);
        else if (!value.ignored) pending++;
      }
      if (page.length < 200) break;
      after = page[page.length - 1].key;
    }
    return {pending, ids: [...ids]};
  }

  /** Attach clarification only to the same sender and exact original source. */
  async publishFollowup(id: string, captures: {key: string; version: string; value: CaptureRecord}[], text: string, now: number,
    extraGuards: {collection: string; key: string; expectedVersion: string | null}[] = []): Promise<"linked" | "defer" | "not_found"> {
    if (captures.length === 0 || captures.length > 50 || !Number.isSafeInteger(now) || !text || text.length > 32000) throw new Error("Invalid clarification batch");
    const db = await open(), tickets = db.collection<unknown>("tickets"), existing = await tickets.get(id);
    if (existing === null) return "not_found";
    const ticket = parse(existing.value), first = captures[0].value, source = first.message;
    const active = ["new", "triage", "implement", "review"].includes(ticket.stage);
    if (ticket.id !== id || (!active && ticket.stage !== "needs_info" && ticket.stage !== "delivery") || ticket.error !== null || ticket.parentChatId !== first.parentChatId || ticket.projectPath !== first.projectPath || ticket.source !== source.channel) return "not_found";
    for (const item of captures) {
      const m = item.value.message;
      if (item.value.ticketId !== null || item.value.parentChatId !== first.parentChatId || item.value.projectPath !== first.projectPath || m.channel !== source.channel || m.accountId !== source.accountId || m.conversationId !== source.conversationId || m.authorId !== source.authorId) throw new Error("Clarification batch has mixed sources");
    }
    if (ticket.stage === "needs_info") {
      const triage = [...ticket.notes].reverse().find(note => note.kind === "triage");
      try { if (triage === undefined || JSON.parse(triage.text).decision !== "needs_info") return "not_found"; } catch { return "not_found"; }
    }
    const all = db.collection<CaptureRecord>("captured"); let after: string | null = null, original = false;
    while (true) {
      const page = await all.list(after, 200);
      if (page.some(row => row.value.ticketId === id && row.value.parentChatId === first.parentChatId && row.value.projectPath === first.projectPath && row.value.message.channel === source.channel && row.value.message.accountId === source.accountId && row.value.message.conversationId === source.conversationId && row.value.message.authorId === source.authorId)) { original = true; break; }
      if (page.length < 200) break;
      after = page[page.length - 1].key;
    }
    if (!original) return "not_found";
    // Do not append to an already reviewed delivery: keep the incoming batch
    // pending, then publish it as a new ticket after delivery settles.
    if (ticket.stage === "delivery") return "defer";
    const claimKey = projectKey(ticket.projectPath);
    if (!active && await db.collection<unknown>("project_claims").get(claimKey) !== null) return "defer";
    ticket.notes.push({at:now,kind:"user",text});
    if (!active) { ticket.stage = "new"; ticket.error = null; ticket.agentId = null; ticket.autoStart = true; ticket.forceTriage = true; }
    ticket.updatedAt = now;
    const mutations: Storage.ObjectMutation[] = [{op:"put",collection:"tickets",key:id,value:ticket as unknown as Storage.Value,expectedVersion:existing.version}];
    for (const item of captures) mutations.push({op:"put",collection:"captured",key:item.key,value:{...item.value,ticketId:id} as unknown as Storage.Value,expectedVersion:item.version});
    // The previous run must have fully released its claim, even if a competing
    // executor races the check above; otherwise a crash can strand the ticket.
    await db.commit(mutations, [...(!active ? [{collection:"project_claims",key:claimKey,expectedVersion:null}] : []), ...extraGuards]);
    return "linked";
  }

  /** Resolve an explicit conversational follow-up only when this sender has one open ticket. */
  async inferFollowup(captures: {key: string; version: string; value: CaptureRecord}[], text: string, now: number): Promise<{id: string; result: "linked" | "defer" | "not_found"} | null> {
    if (!captures.length) throw new Error("Empty follow-up batch");
    const db = await open(), first = captures[0].value, source = first.message;
    // Changes to the set of published source messages invalidate the snapshot,
    // including a new competing ticket appearing between lookup and commit.
    const epoch = await db.collection<unknown>("capture_epoch").get("all");
    const ids = new Set<string>(), all = db.collection<CaptureRecord>("captured");
    let after: string | null = null;
    while (true) {
      const page = await all.list(after, 200);
      for (const row of page) {
        const item = row.value, m = item.message;
        if (item.ticketId !== null && item.parentChatId === first.parentChatId && item.projectPath === first.projectPath &&
          m.channel === source.channel && m.accountId === source.accountId && m.conversationId === source.conversationId && m.authorId === source.authorId) ids.add(item.ticketId);
      }
      // Prefer a separate ticket over guessing if this conversation has too much history.
      if (ids.size > 200) return null;
      if (page.length < 200) break;
      after = page[page.length - 1].key;
    }
    const rows: {id: string; version: string; ticket: Ticket}[] = [];
    for (const id of ids) {
      const entry = await db.collection<unknown>("tickets").get(id);
      if (entry === null) return null;
      const ticket = parse(entry.value);
      if (ticket.id !== id) throw new Error("Ticket identity mismatch");
      rows.push({id, version:entry.version, ticket});
    }
    const openTickets = rows.filter(row => !["completed", "not_actionable"].includes(row.ticket.stage));
    if (openTickets.length !== 1) return null;
    const chosen = openTickets[0], ticket = chosen.ticket;
    if (ticket.error !== null) return null;
    if (ticket.stage === "needs_info") {
      const triage = [...ticket.notes].reverse().find(note => note.kind === "triage");
      if (triage === undefined || !Number.isSafeInteger(triage.at) || first.receivedAt < triage.at || first.receivedAt - triage.at > 30 * 60 * 1000) return null;
      try { if (JSON.parse(triage.text).decision !== "needs_info") return null; } catch { return null; }
    } else {
      if (!["new", "triage", "implement", "review", "delivery"].includes(ticket.stage) || !Number.isSafeInteger(ticket.createdAt) || first.receivedAt < ticket.createdAt || first.receivedAt - ticket.createdAt > 30 * 60 * 1000) return null;
    }
    const guards = [{collection:"capture_epoch", key:"all", expectedVersion:epoch?.version ?? null},
      ...rows.filter(row => row.id !== chosen.id).map(row => ({collection:"tickets", key:row.id, expectedVersion:row.version}))];
    const result = await this.publishFollowup(chosen.id, captures, text, now, guards);
    return {id: chosen.id, result};
  }

  /** Publishing and linking every source message is one transaction. */
  async publish(ticket: Ticket, captures: { key: string; version: string; value: CaptureRecord }[]): Promise<void> {
    if (captures.length === 0 || captures.length > 50) throw new Error("Invalid capture batch size");
    const db = await open(), epoch = await db.collection<unknown>("capture_epoch").get("all");
    const mutations: Storage.ObjectMutation[] = [{ op: "put", collection: "tickets", key: ticket.id, value: ticket as unknown as Storage.Value, expectedVersion: null },
      {op:"put", collection:"capture_epoch", key:"all", value:{ticketId:ticket.id}, expectedVersion:epoch?.version ?? null}];
    for (const item of captures) mutations.push({ op: "put", collection: "captured", key: item.key, value: { ...item.value, ticketId: ticket.id } as unknown as Storage.Value, expectedVersion: item.version });
    await db.commit(mutations);
  }

}

/** Normalize lexical aliases; symlinks and case-insensitive mounts require a canonical configured path. */
function projectKey(path: string): string {
  const input = path.trim().replace(/\\/g, "/"), windows = /^[A-Za-z]:\//.test(input);
  if (!input.startsWith("/") && !windows) throw new Error("projectPath must be absolute");
  const parts: string[] = [];
  for (const part of input.split("/")) {
    if (!part || part === ".") continue;
    if (part === "..") { if (!parts.length || (windows && parts.length === 1)) throw new Error("projectPath escapes its root"); parts.pop(); }
    else parts.push(part);
  }
  const result = (windows ? "" : "/") + parts.join("/");
  return windows ? result.toLowerCase() : result;
}
