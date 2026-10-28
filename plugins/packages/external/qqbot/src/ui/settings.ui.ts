import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl";
import { call } from "../rpc.js";
import type { AutoReplyParams, ConfigureParams, InboundEvent, ReplyRole, Status } from "../model.js";
import { errorText } from "../model.js";

type Section = "status" | "connection" | "reply" | "diagnostics";

/** Guides setup around connecting QQ, choosing a role and starting the bot. */
export default function settings(ctx: ComposeDslContext): ComposeNode {
  const initialized = ctx.useRef("initialized", false);
  const working = ctx.useRef("working", false);
  const [busy, setBusy] = ctx.useState("busy", "");
  const [status, setStatus] = ctx.useState<Status | null>("status", null);
  const [errors, setErrors] = ctx.useState<Record<string, string>>("errors", {});
  const [messages, setMessages] = ctx.useState<Record<string, string>>("messages", {});
  const [roles, setRoles] = ctx.useState<ReplyRole[]>("roles", []);
  const [roleError, setRoleError] = ctx.useState("roleError", "");
  const [events, setEvents] = ctx.useState<InboundEvent[] | null>("events", null);
  const [editConnection, setEditConnection] = ctx.useState("editConnection", false);
  const [advanced, setAdvanced] = ctx.useState("advanced", false);
  const [diagnostics, setDiagnostics] = ctx.useState("diagnostics", false);
  const [appId, setAppId] = ctx.useState("appId", "");
  const [secret, setSecret] = ctx.useState("secret", "");
  const [sandbox, setSandbox] = ctx.useState("sandbox", false);
  const [c2c, setC2c] = ctx.useState("c2c", true);
  const [group, setGroup] = ctx.useState("group", true);
  const [selection, setSelection] = ctx.useState("selection", "");
  const [instruction, setInstruction] = ctx.useState("instruction", "");
  const [chatGroup, setChatGroup] = ctx.useState("chatGroup", "QQ Bot");
  const [waifuProfile, setWaifuProfile] = ctx.useState("waifuProfile", "default");
  const [deadline, setDeadline] = ctx.useState("deadline", "120");
  const colors = ctx.MaterialTheme.colorScheme;

  /** Populates drafts only on first load; subsequent status reads never replace edits. */
  function initialize(value: Status): void {
    setStatus(value); setAppId(value.appId); setSandbox(value.useSandbox);
    setC2c(value.autoReply.config.c2cEnabled); setGroup(value.autoReply.config.groupEnabled);
    setSelection(value.autoReply.config.characterSelection); setInstruction(value.autoReply.config.assistantInstruction);
    setChatGroup(value.autoReply.config.chatGroup); setDeadline(String(value.autoReply.config.aiTimeoutMs / 1000));
    setWaifuProfile(value.waifuProfile);
  }

  /** Reads runtime state independently from all editable fields. */
  async function refresh(): Promise<void> {
    const result = await call({ action: "status" });
    if ("status" in result) setStatus(result.status);
  }

  /** Keeps outcomes beside their controls and rejects duplicate actions before a re-render. */
  async function operate(section: Section, label: string, operation: () => Promise<string>): Promise<void> {
    if (working.current) return;
    working.current = true; setBusy(label);
    setErrors({ ...errors, [section]: "" });
    setMessages({ ...messages, [section]: "" });
    let failureMessage = "";
    try {
      const message = await operation();
      setMessages({ ...messages, [section]: message });
    } catch (failure) {
      console.error("[qqbot.ui]", failure);
      failureMessage = errorText(failure);
      setErrors({ ...errors, [section]: failureMessage });
    } finally {
      try { await refresh(); }
      catch (failure) { setErrors({ ...errors, [section]: failureMessage || `状态读取失败：${errorText(failure)}` }); }
      working.current = false; setBusy("");
    }
  }

  /** Requires usable current credentials without ever returning the saved secret to the UI. */
  function credentials(): ConfigureParams {
    if (appId.trim() === "") throw new Error("请先填写 QQ 机器人的 AppID。");
    if (secret.trim() === "" && (!status?.configured || appId.trim() !== status.appId)) throw new Error("请填写这个机器人的 AppSecret。");
    return { app_id: appId.trim(), use_sandbox: sandbox, ...(secret.trim() === "" ? {} : { app_secret: secret.trim() }) };
  }

  /** Converts user-facing seconds once and validates all reply settings before saving. */
  function replies(): AutoReplyParams {
    if (!c2c && !group) throw new Error("请至少选择一种回复范围：私聊或群消息。");
    const seconds = Number(deadline);
    if (deadline.trim() === "" || !Number.isFinite(seconds) || !/^\d+(?:\.\d{1,3})?$/.test(deadline.trim()) || seconds < 1 || seconds > 600) throw new Error("回复等待时间请填写 1～600 秒。");
    if (waifuProfile.trim() === "") throw new Error("请填写回复效果配置，默认使用 default。");
    return { c2c_enabled: c2c, group_enabled: group, character_selection: selection, assistant_instruction: instruction,
      chat_group: chatGroup.trim(), waifu_profile: waifuProfile.trim(), ai_timeout_ms: Math.round(seconds * 1000) };
  }

  /** Saves and verifies the values currently visible in the connection form. */
  async function connect(): Promise<void> {
    await operate("connection", "正在验证连接…", async () => {
      await call({ action: "configure", params: { ...credentials(), test_connection: true } });
      setSecret(""); setEditConnection(false);
      return "接入信息已保存，连接验证通过。";
    });
  }

  /** Saves reply edits without enabling a stopped bot. */
  async function saveReplies(): Promise<void> {
    await operate("reply", "正在保存回复设置…", async () => {
      await call({ action: "auto_configure", params: { ...replies(), start_now: status?.autoReply.running === true } });
      return "回复设置已保存。";
    });
  }

  /** Starts with the current drafts; the backend owns persistence and authenticated startup. */
  async function start(): Promise<void> {
    await operate("reply", "正在启动机器人…", async () => {
      const connection = credentials(), reply = replies();
      await call({ action: "auto_configure", params: { ...reply, enabled: true, start_now: false } });
      await call({ action: "configure", params: { ...connection, listener_enabled: true, test_connection: true } });
      setSecret(""); setEditConnection(false);
      return "机器人已启动，可以在 QQ 中发消息试试。下次打开应用时会自动恢复运行。";
    });
  }

  /** Loads human-readable role choices through the declared character dependency. */
  async function loadRoles(): Promise<void> {
    setRoleError("");
    try {
      const result = await call({ action: "reply_roles" });
      if ("roles" in result) setRoles(result.roles);
    } catch (failure) { setRoleError(`角色列表读取失败：${errorText(failure)}`); }
  }

  function description(text: string): ComposeNode {
    return ctx.UI.Text({ text, style: "bodySmall", color: colors.onSurfaceVariant });
  }

  function field(key: string, label: string, value: string, update: (value: string) => void, options: { sensitive?: boolean; multiline?: boolean } = {}): ComposeNode {
    return ctx.UI.OutlinedTextField({ key, label, value, onValueChange: update, enabled: busy === "" && status !== null,
      singleLine: !options.multiline, minLines: options.multiline ? 3 : 1, maxLines: options.multiline ? 6 : 1,
      isPassword: options.sensitive === true, fillMaxWidth: true });
  }

  function toggle(key: string, label: string, checked: boolean, update: (value: boolean) => void): ComposeNode {
    return ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "center", spacing: 12 }, [
      ctx.UI.Text({ text: label, weight: 1 }), ctx.UI.Switch({ key, checked, onCheckedChange: update, enabled: busy === "" && status !== null }),
    ]);
  }

  function button(key: string, label: string, onClick: () => void | Promise<void>, primary = false): ComposeNode {
    return ctx.UI[primary ? "Button" : "TextButton"]({ key, text: label, onClick, enabled: busy === "" && status !== null, ...(primary ? { fillMaxWidth: true } : {}) });
  }

  function feedback(section: Section): ComposeNode[] {
    return [
      ...(errors[section] ? [ctx.UI.Text({ text: errors[section], color: colors.error, style: "bodyMedium" })] : []),
      ...(messages[section] ? [description(messages[section])] : []),
    ];
  }

  function card(key: string, title: string, children: ComposeNode[], highlight = false): ComposeNode {
    return ctx.UI.Card({ key, fillMaxWidth: true, containerColor: highlight ? colors.secondaryContainer : colors.surfaceContainerLow, shape: { cornerRadius: 16 } },
      ctx.UI.Column({ fillMaxWidth: true, padding: 16, spacing: 12 }, [ctx.UI.Text({ text: title, style: "titleMedium" }), ...children]));
  }

  const connectionChanged = status !== null && (appId.trim() !== status.appId || secret !== "" || sandbox !== status.useSandbox);
  const saved = status?.autoReply.config;
  const replyChanged = saved !== undefined && (c2c !== saved.c2cEnabled || group !== saved.groupEnabled || selection !== saved.characterSelection || instruction !== saved.assistantInstruction
    || chatGroup.trim() !== saved.chatGroup || waifuProfile.trim() !== status?.waifuProfile || Number(deadline) * 1000 !== saved.aiTimeoutMs);
  const active = status !== null && (status.service.status === "ready" || status.service.status === "connecting" || status.autoReply.running);
  const state = status === null ? "正在读取状态…" : !status.configured ? "尚未接入 QQ" : {
    stopped: "机器人已停止", connecting: "正在连接 QQ…", ready: status.autoReply.running ? "机器人运行中" : "QQ 已连接，自动回复未运行", error: "QQ 连接失败",
  }[status.service.status];
  const choices: ReplyRole[] = [{ selection: "", name: "跟随当前角色" }, { selection: "model:chat", name: "模型对话" }, ...roles];
  if (!choices.some(role => role.selection === selection)) choices.push({ selection, name: "已保存的角色（不在当前列表中）" });
  const connectionFields: ComposeNode[] = status !== null && (editConnection || !status.configured) ? [
    description("从 QQ 开放平台的机器人管理页面获取 AppID 和 AppSecret。"),
    field("appId", "AppID", appId, setAppId),
    field("secret", status.configured && appId.trim() === status.appId ? "AppSecret（已保存，留空保留）" : "AppSecret", secret, setSecret, { sensitive: true }),
    toggle("sandbox", "使用沙箱环境（测试机器人）", sandbox, setSandbox),
    button("connect", "保存并验证", connect, true),
  ] : [
    description(status === null ? "正在读取接入信息…" : `AppID：${status.appId} · ${status.useSandbox ? "沙箱环境" : "正式环境"} · 密钥已保存`),
    ...(connectionChanged ? [description("接入信息有未保存的修改，启动时会一起保存并验证。")] : []),
    button("editConnection", "修改接入信息", () => setEditConnection(true)),
  ];

  const replyFields: ComposeNode[] = [
    ctx.UI.DropdownMenu({ key: "role", label: "回复角色", text: choices.find(role => role.selection === selection)!.name,
      enabled: busy === "" && status !== null, fillMaxWidth: true, onClick: index => setSelection(choices[index].selection) },
      choices.map(role => ctx.UI.Text({ text: role.name, paddingVertical: 8 }))),
    description("每个私聊和群聊都有独立会话。更换角色用于新会话，已有会话会保留原来的角色。"),
    ...(roleError ? [ctx.UI.Text({ text: roleError, color: colors.error }), button("reloadRoles", "重新读取角色", loadRoles)] : []),
    toggle("c2c", "回复私聊消息", c2c, setC2c),
    toggle("group", "回复群消息", group, setGroup),
    button("advanced", advanced ? "收起高级设置" : "高级设置", () => setAdvanced(!advanced)),
  ];
  if (advanced) replyFields.push(
    field("deadline", "回复等待时间（秒，1～600）", deadline, setDeadline),
    field("instruction", "回复要求", instruction, setInstruction, { multiline: true }),
    field("chatGroup", "聊天记录分组", chatGroup, setChatGroup),
    field("waifuProfile", "回复效果配置", waifuProfile, setWaifuProfile),
    description("默认使用 default。分句、表情等效果在 Waifu 插件中配置。"),
  );
  if (replyChanged) replyFields.push(button("saveReplies", "保存回复设置", saveReplies), description(active ? "修改尚未保存。保存后会用于后续回复。" : "修改尚未保存。启动机器人时也会一起保存。"));
  const stop = () => operate("reply", "正在停止机器人…", async () => {
    await call({ action: "stop" });
    return "机器人已停止，下次打开应用时也会保持停止。";
  });
  if (status?.autoReply.running || status?.service.status === "connecting") replyFields.push(button("stop", "停止机器人", stop, true));
  else {
    replyFields.push(button("start", connectionChanged || replyChanged ? "保存并启动机器人" : "启动机器人", start, true));
    if (active) replyFields.push(button("stop", "停止机器人", stop));
  }
  replyFields.push(...feedback("reply"));

  const children: ComposeNode[] = [
    ctx.UI.Text({ text: "QQ 机器人", style: "headlineMedium" }),
    description("接入 QQ，选择一个角色，就能自动回复消息。"),
    card("statusCard", state, [
      description(status?.autoReply.running ? `已回复 ${status.autoReply.repliedCount} 条消息` : "先完成接入和回复设置，再启动机器人。"),
      ...(busy ? [description(busy)] : []),
      ...(status?.service.lastError ? [ctx.UI.Text({ text: `连接错误：${status.service.lastError}`, color: colors.error })] : []),
      ...(status?.autoReply.lastError ? [ctx.UI.Text({ text: `回复错误：${status.autoReply.lastError}`, color: colors.error })] : []),
      ctx.UI.TextButton({ key: "refresh", text: status === null ? "重新读取配置" : "刷新状态", enabled: busy === "", onClick: async () => {
        if (status === null) { initialized.current = false; await load(); }
        else await operate("status", "正在刷新状态…", async () => { await refresh(); return ""; });
      } }),
      ...feedback("status"),
    ], status?.autoReply.running === true),
    card("connectionCard", "1. 接入 QQ", [...connectionFields, ...feedback("connection")]),
    card("replyCard", "2. 设置回复", replyFields),
    button("diagnostics", diagnostics ? "收起诊断信息" : "诊断信息", () => setDiagnostics(!diagnostics)),
  ];
  if (diagnostics) children.push(card("diagnosticsCard", "诊断信息", [
    description(`收到事件：${status?.service.receivedCount ?? 0} · 待处理：${status?.queue.size ?? 0}/${status?.queue.capacity ?? 200}`),
    button("recentEvents", "查看待处理消息", () => operate("diagnostics", "正在读取消息…", async () => {
      const result = await call({ action: "receive", params: { limit: 10, consume: false } });
      if ("events" in result) setEvents(result.events);
      return "";
    })),
    ...(events === null ? [] : events.length === 0 ? [description("暂无待处理消息。自动回复已处理的消息可在聊天记录中查看。")] : events.map(event =>
      description(`${event.scene === "c2c" ? "私聊" : event.scene === "group" ? "群消息" : event.eventType} · ${event.timestamp || event.receivedAt}\n${event.content || "[非文本消息]"}`))),
    ...feedback("diagnostics"),
    button("clearEvents", "清空待处理事件", () => operate("diagnostics", "正在清空事件…", async () => {
      const result = await call({ action: "clear" }); setEvents([]);
      return "clearedCount" in result ? `已清空 ${result.clearedCount} 条待处理事件。` : "";
    })),
  ]));

  /** Loads drafts once, leaving a failed initial load available for an explicit retry. */
  async function load(): Promise<void> {
    if (initialized.current || working.current) return;
    initialized.current = true; working.current = true; setBusy("正在读取配置…");
    try {
      const result = await call({ action: "status" });
      if ("status" in result) initialize(result.status);
      setErrors({ ...errors, status: "" });
    } catch (failure) { initialized.current = false; setErrors({ ...errors, status: `配置读取失败：${errorText(failure)}` }); }
    finally { working.current = false; setBusy(""); }
    await loadRoles();
  }

  return ctx.UI.LazyColumn({ fillMaxSize: true, padding: 16, spacing: 16, onLoad: load }, children);
}
