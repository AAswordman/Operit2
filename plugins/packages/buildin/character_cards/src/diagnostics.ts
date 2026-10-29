/** Compact send diagnostics; normal repository and UI reads stay silent. */
export type DiagnosticFields = Record<string, string | number | boolean | null>;
const reportedErrors = new WeakSet<object>();

/** Keeps just enough identity to correlate a send without dumping its payload. */
export function logCharacterStatus(message: string, fields: DiagnosticFields, elapsedMs?: number): void {
  const chat = typeof fields.chatId === "string" ? " chat=" + fields.chatId.slice(0, 8) : "";
  const task = typeof fields.submissionId === "string" ? " task=" + fields.submissionId : "";
  const elapsed = elapsedMs === undefined ? "" : " " + elapsedMs + "ms";
  console.log("[character_cards] " + message + chat + task + elapsed);
}

/** Emits one short failure per original error, even as it crosses multiple await boundaries. */
export function logCharacterFailure(message: string, fields: DiagnosticFields, error: unknown, elapsedMs?: number): void {
  if (typeof error === "object" && error !== null) {
    if (reportedErrors.has(error)) return;
    reportedErrors.add(error);
  }
  const reason = typeof error === "object" && error !== null && "message" in error ? String(error.message) : String(error);
  const chat = typeof fields.chatId === "string" ? " chat=" + fields.chatId.slice(0, 8) : "";
  const task = typeof fields.submissionId === "string" ? " task=" + fields.submissionId : "";
  const elapsed = elapsedMs === undefined ? "" : " " + elapsedMs + "ms";
  console.error("[character_cards] " + message + chat + task + elapsed + ": " + reason.replace(/\s+/g, " ").slice(0, 240));
}

/** Tracks await stages silently; only send decisions, native sends and failures are printed. */
export class CharacterTrace {
  private readonly startedAt = Date.now();
  private stage = "";
  private details: DiagnosticFields;

  constructor(private readonly operation: string, fields: DiagnosticFields) {
    this.details = { ...fields };
    if (operation === "group.input.submit") logCharacterStatus("检查发送", this.details);
    if (operation === "group.native.send") logCharacterStatus(fields.turn === "record_only" ? "保存群聊消息" : "请求角色回复", this.details);
  }

  event(stage: string, fields: DiagnosticFields = {}): void {
    Object.assign(this.details, fields);
    if (stage === "input.admission.wait") logCharacterStatus("群聊接管，等待保存", this.details);
    if (stage !== "end") return;
    const elapsedMs = Date.now() - this.startedAt;
    if (this.operation === "group.input.submit") {
      logCharacterStatus(this.details.reason === "existing_submission" ? "群聊任务未结束，阻止重复提交" : this.details.type ? "群聊已接管" : "放行", this.details, elapsedMs);
    } else if (elapsedMs >= 1000) {
      logCharacterStatus(this.operation + " 完成", this.details, elapsedMs);
    }
  }

  async step<T>(stage: string, work: () => Promise<T>): Promise<T> {
    this.stage = stage;
    const result = await work();
    this.stage = "";
    return result;
  }

  fail(error: unknown): void {
    logCharacterFailure(this.operation + (this.stage ? "/" + this.stage : "") + " 失败", this.details, error, Date.now() - this.startedAt);
  }
}

/** Preserves the original result and rejection while reporting a compact operation outcome. */
export async function traceCharacterWork<T>(operation: string, fields: DiagnosticFields,
  work: (trace: CharacterTrace) => Promise<T>): Promise<T> {
  const trace = new CharacterTrace(operation, fields);
  try {
    const result = await work(trace);
    trace.event("end");
    return result;
  } catch (error) {
    trace.fail(error);
    throw error;
  }
}
