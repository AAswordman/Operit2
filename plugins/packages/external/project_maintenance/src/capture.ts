import { required, type CaptureRecord, type CapturedMessage, type Ticket } from "./model.js";
import { ProjectBusy, type MaintenanceEngine } from "./engine.js";
import type { Repository } from "./storage.js";
import { isStatusQuestion } from "./app-recovery.js";

type StoredCapture = { key: string; version: string; value: CaptureRecord };
const QUIET_MS = 12000;
const MAX_WAIT_MS = 60000;

/** Stores raw arrivals before upstream acknowledgement and groups nearby conversation messages. */
export class CaptureCoordinator {
  private working = false;
  constructor(private readonly repository: Repository, private readonly engine: MaintenanceEngine,
    private readonly now: () => number, private readonly report: (error: unknown) => void) {}

  async capture(parentChatId: string, projectPath: string, message: CapturedMessage): Promise<{ saved: boolean }> {
    parentChatId = required(parentChatId, "parentChatId"); projectPath = required(projectPath, "projectPath");
    if (message === null || typeof message !== "object" || (message.channel !== "qq" && message.channel !== "wechat" && message.channel !== "app")) throw new Error("Invalid channel message");
    for (const [key, value] of Object.entries({ key: message.key, accountId: message.accountId, messageId: message.messageId, conversationId: message.conversationId, authorId: message.authorId })) {
      required(value, key);
      if (value.length > (key === "key" ? 1000 : 500)) throw new Error(key + " is too long");
    }
    if (message.key.length > 1000 || typeof message.text !== "string" || typeof message.timestamp !== "string" || message.timestamp.length > 100 || message.context === null || typeof message.context !== "object" || Array.isArray(message.context)) throw new Error("Invalid channel message fields");
    if (message.text.length > 16000 || JSON.stringify(message).length > 200000) throw new Error("Channel message is too large");
    const statusQuery = typeof message.autoReplyEnabled === "boolean" && hasOnlyPlainText(message) && isStatusQuestion(message.text);
    const saved = await this.repository.capture({ message, parentChatId, projectPath, receivedAt: this.now(), ticketId: null,
      ...(statusQuery ? {ignored: message.autoReplyEnabled ? "status_query_auto" as const : "status_query" as const} : {}) });
    return { saved };
  }

  /** A short quiet period collects chat fragments before a diagnosis is launched. */
  async tick(): Promise<void> {
    if (this.working) return;
    this.working = true;
    try {
      const pending = await this.repository.pendingCaptures();
      const groups = new Map<string, StoredCapture[]>();
      for (const item of pending) {
        const { message, parentChatId, projectPath } = item.value;
        const key = JSON.stringify([message.channel, message.accountId, message.conversationId, message.authorId, parentChatId, projectPath]);
        const group = groups.get(key) ?? []; group.push(item); groups.set(key, group);
      }
      for (const group of groups.values()) {
        try {
          group.sort((a, b) => a.value.receivedAt - b.value.receivedAt || a.key.localeCompare(b.key));
          const now = this.now();
          if (now - group[group.length - 1].value.receivedAt < QUIET_MS && now - group[0].value.receivedAt < MAX_WAIT_MS) continue;
          while (group.length) {
            const batch: StoredCapture[] = []; let size = 0;
            const batchRefs = new Set<string>();
            while (group.length && batch.length < 50) {
              const next = group[0], length = formatMessage(next.value.message).length + 1;
              const nextRefs = ticketReferences(next.value.message.text);
              if (batch.length && (size + length > 30000 || startsNewTopic(next.value.message.text) || (batchRefs.size > 0 && nextRefs.some(ref => !batchRefs.has(ref))))) break;
              batch.push(group.shift()!); size += length;
              for (const ref of nextRefs) batchRefs.add(ref);
            }
            if (batch.every(item => isPlainAcknowledgement(item.value.message))) {
              await this.repository.ignoreAcknowledgements(batch);
              continue;
            }
            const first = batch[0].value;
            const feedback = batch.map(item => formatMessage(item.value.message)).join("\n");
            const references = new Set(batch.flatMap(item => ticketReferences(item.value.message.text)));
            if (references.size === 1) {
              const result = await this.repository.publishFollowup([...references][0], batch, feedback, this.now());
              if (result === "defer") continue;
              if (result === "linked") { const existing = await this.engine.get([...references][0]); if (existing.stage === "new") await this.startNew(existing); continue; }
            }
            if (references.size === 0 && isConversationalFollowup(batch.map(item => item.value.message.text))) {
              const inferred = await this.repository.inferFollowup(batch, feedback, this.now());
              if (inferred?.result === "defer") continue;
              if (inferred?.result === "linked") { const existing = await this.engine.get(inferred.id); if (existing.stage === "new") await this.startNew(existing); continue; }
              // An earlier qualifying ticket may have changed while publishing;
              // keep the batch for the next tick instead of guessing a new ticket.
              if (inferred?.result === "not_found") continue;
            }
            const ticket = this.engine.prepare(first.parentChatId, first.projectPath, first.message.channel, feedback);
            ticket.autoStart = true;
            await this.repository.publish(ticket, batch);
            await this.startNew(ticket);
          }
        } catch (error) {
          // The pending records remain durable; a bad group must not block
          // independent conversations or recovery of already published tickets.
          this.report(error);
        }
      }
      // Manual notes on a clarification request are requeued only after the
      // previous owner releases its reservation, including across restarts.
      for (const ticket of await this.engine.list()) {
        if (ticket.stage !== "needs_info" || ticket.autoResumeAfterInfo !== true) continue;
        try { await this.repository.queuePendingInfo(ticket.id, this.now()); } catch (error) { this.report(error); }
      }
      // A crash between publishing and starting leaves a durable, safely startable ticket.
      for (const ticket of (await this.engine.list()).sort((a, b) => a.createdAt - b.createdAt || a.id.localeCompare(b.id))) if (ticket.autoStart === true && ticket.stage === "new") await this.startNew(ticket);
    } finally { this.working = false; }
  }

  private async startNew(ticket: Ticket): Promise<void> {
    try { await this.engine.start(ticket.id); } catch (error) { if (!(error instanceof ProjectBusy)) this.report(error); }
  }
}

function formatMessage(message: CapturedMessage): string {
  return `[${message.authorId} @ ${message.timestamp}] ${message.text || "[非文本消息，原始附件信息已保存]"}`;
}

/** Only an explicit full ticket identifier is used to associate a clarification. */
function ticketReferences(text: string): string[] {
  return Array.from(text.matchAll(/工单\s*[:：#]?\s*([A-Za-z0-9][A-Za-z0-9_-]{2,127})/g), match => match[1]);
}

/** Only suppress an exact acknowledgement when the channel payload proves it contains no media. */
function isPlainAcknowledgement(message: CapturedMessage): boolean {
  const value = message.text.trim();
  if (!value || value.length > 80 || ticketReferences(value).length > 0) return false;
  if (!/^(?:补充[：:,，\s]*)?(?:(?:谢谢(?:你|了|啦)?|多谢|感谢|收到|好的|好|嗯|了解|明白|辛苦了|ok(?:ay)?|thanks|thank you)[\s，。！？,.!~～]*)+$/iu.test(value)) return false;
  return hasOnlyPlainText(message);
}

/** Require channel evidence that no attachment accompanies the text. */
function hasOnlyPlainText(message: CapturedMessage): boolean {
  if (message.channel === "wechat") {
    const items = message.context.items;
    return Array.isArray(items) && items.every(item => item !== null && typeof item === "object" && !Array.isArray(item) && item.type === 1 && Object.keys(item).every(key => key === "type" || key === "text_item"));
  }
  if (message.channel === "qq") {
    const raw = message.context.raw;
    if (raw === null || typeof raw !== "object" || Array.isArray(raw)) return false;
    const payload = raw as Record<string, unknown>;
    const known = new Set(["id", "content", "timestamp", "author", "group_openid", "channel_id", "guild_id", "attachments"]);
    return Object.keys(payload).every(key => known.has(key)) &&
      (payload.attachments === undefined || payload.attachments === null || (Array.isArray(payload.attachments) && payload.attachments.length === 0));
  }
  // In-app capture does not expose attachment metadata yet.
  return false;
}

/** Strong wording only: a general new report must not be attached by proximity alone. */
function isConversationalFollowup(messages: string[]): boolean {
  const value = messages[0]?.trim() ?? "";
  if (messages.some(text => /(?:另一个|另外一个|另有|新问题|新反馈|顺便)/u.test(text))) return false;
  return /^(?:补充(?:一下|说明)?|再补充|刚才(?:的|说的|那个)?|关于刚才|之前(?:说的|那个)|我说的(?:是|那个))/u.test(value);
}

/** Split only an explicit change of issue, preserving ordinary details in the same batch. */
function startsNewTopic(text: string): boolean {
  return /^(?:(?:另外|另有|还有|另)(?:还有)?(?:一个|一条|一件)?(?:新的?)?|新(?:的)?)(?:问题|反馈|故障|bug)(?=$|[\s，。！？:：,.!?]|是|就是)/iu.test(text.trim());
}
