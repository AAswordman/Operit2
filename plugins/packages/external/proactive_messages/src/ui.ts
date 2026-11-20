import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import { defaultRule, pending, type Rule } from "./model";
import type { Snapshot } from "./service";

declare function snapshot(includeChats: boolean): Promise<Snapshot>;
declare function saveRule(rule: Rule): Promise<Snapshot>;
declare function removeRule(chatId: string): Promise<void>;
declare function prepare(): Promise<void>;
declare function setForeground(value: boolean): Promise<void>;

type Page = "home" | "picker" | "rules" | "editor" | "activity";
type NumericField = "intervalMinutes" | "randomMinutes" | "probability" | "prepareCount";
interface View {
  page: Page;
  snapshot: Snapshot | null;
  query: string;
  offset: number;
  draft: Rule | null;
  numeric: Partial<Record<NumericField, string>>;
  busy: boolean;
  error: string;
  confirmDelete: boolean;
  advanced: boolean;
}
const titles = { home: "AI 主动消息", picker: "选择对话", rules: "对话规则", editor: "编辑规则", activity: "发送计划与记录" };
const labels = { preparing: "正在准备", ready: "待准备", scheduling: "正在准备并预约", scheduled: "系统已预约", delivering: "正在发送", delivered: "已写入对话", skipped: "已跳过", failed: "失败", cancelled: "已撤销", invalidated: "上下文已变化" };
const pageSize = 6;

function planTime(timestamp: number): string {
  const date = new Date(timestamp);
  const today = new Date();
  const day = date.toDateString() === today.toDateString() ? "今天" : `${date.getMonth() + 1}月${date.getDate()}日`;
  return `${day} ${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
}

export default function screen(ctx: ComposeDslContext): ComposeNode {
  const UI = ctx.UI;
  const colors = ctx.MaterialTheme.colorScheme;
  const [view, setView] = ctx.useState<View>("proactive-workspace", {
    page: "home", snapshot: null, query: "", offset: 0, draft: null, numeric: {}, busy: false, error: "", confirmDelete: false, advanced: false,
  });
  const live = ctx.useRef("proactive-live", view);
  live.current = view;
  function update(patch: Partial<View>): void {
    live.current = { ...live.current, ...patch };
    setView(live.current);
  }
  async function run(action: () => Promise<void>): Promise<void> {
    if (live.current.busy) return;
    update({ busy: true, error: "" });
    try { await action(); }
    catch (error) { update({ error: String(error) }); }
    finally { update({ busy: false }); }
  }
  async function reload(includeChats = false): Promise<void> {
    update({ snapshot: await withContext("main", { includeChats }, () => snapshot(includeChats)) });
  }
  function go(page: Page): void { update({ page, error: "", confirmDelete: false }); }
  function edit(rule: Rule): void { update({ page: "editor", draft: { ...rule }, numeric: {}, error: "", advanced: false }); }
  function patchDraft(changes: Partial<Rule>): void {
    if (live.current.draft) update({ draft: { ...live.current.draft, ...changes } });
  }
  function note(text: string): ComposeNode {
    return UI.Text({ text, style: "bodySmall", color: colors.onSurfaceVariant });
  }
  function panel(children: ComposeNode[], highlight = false): ComposeNode {
    return UI.Card({ fillMaxWidth: true, shape: { cornerRadius: 20 }, elevation: 0,
      containerColor: highlight ? colors.primaryContainer : colors.surfaceContainerLow },
    UI.Column({ fillMaxWidth: true, padding: highlight ? 22 : 16, spacing: 12 }, children));
  }
  function entry(key: string, icon: string, title: string, detail: string, open: () => void): ComposeNode {
    return UI.Surface({ key, fillMaxWidth: true, shape: { cornerRadius: 14 }, containerColor: colors.surfaceContainerLow,
      onClick: () => { if (!live.current.busy) open(); } },
    UI.Row({ padding: 16, spacing: 14, verticalAlignment: "center" }, [
      UI.Icon({ name: icon, size: 22, tint: colors.primary }),
      UI.Column({ weight: 1, spacing: 3 }, [UI.Text({ text: title, style: "titleMedium", maxLines: 2, overflow: "ellipsis" }), ...(detail ? [note(detail)] : [])]),
      UI.Icon({ name: "chevronRight", size: 18, tint: colors.onSurfaceVariant }),
    ]));
  }
  function field(label: string, value: string, onValueChange: (value: string) => void, lines = 1): ComposeNode {
    return UI.OutlinedTextField({ key: label, label, value, fillMaxWidth: true, enabled: !view.busy,
      singleLine: lines === 1, minLines: lines, onValueChange });
  }
  function numericField(title: string, key: NumericField): ComposeNode {
    return field(title, view.numeric[key] ?? String(view.draft![key]), value => {
      update({ numeric: { ...live.current.numeric, [key]: value } });
      patchDraft({ [key]: value.trim() ? Number(value) : NaN });
    });
  }

  const data = view.snapshot;
  const rules = data?.state.rules ?? [];
  const jobs = data?.state.jobs ?? [];
  const draft = view.draft;
  const existing = rules.some(rule => rule.chatId === draft?.chatId);
  const nodes: ComposeNode[] = [];
  if (view.page !== "home") {
    const back = view.page === "editor" ? (existing ? "rules" : "picker") : "home";
    nodes.push(UI.Row({ verticalAlignment: "center", spacing: 8 }, [
      UI.Text({ weight: 1, text: view.page === "editor" && !existing ? "新建规则" : titles[view.page], style: "titleLarge" }),
      UI.TextButton({ text: view.page === "editor" ? "取消" : "返回概览", enabled: !view.busy, onClick: () => go(back) }),
    ]));
  }
  if (view.error) nodes.push(UI.Text({ text: view.error, color: colors.error }));
  if (!data) {
    if (view.error) nodes.push(UI.Button({ text: "重试", enabled: !view.busy, onClick: () => run(loadHome) }));
    else nodes.push(note("正在读取主动消息设置…"));
  } else if (view.page === "home") {
    const enabled = rules.filter(rule => rule.enabled).length;
    const upcoming = jobs.filter(job => job.status === "scheduled" && job.dueAt > Date.now()).sort((first, second) => first.dueAt - second.dueAt)[0];
    const waiting = jobs.filter(pending).length;
    nodes.push(panel([
      UI.Row({ spacing: 8, verticalAlignment: "center" }, [UI.Icon({ name: "schedule", size: 18, tint: colors.onPrimaryContainer }),
        UI.Text({ text: upcoming ? "下次发送" : "主动消息", style: "labelLarge", color: colors.onPrimaryContainer })]),
      UI.Text({ text: upcoming ? planTime(upcoming.dueAt) : enabled ? "等待安排" : rules.length ? "所有规则已关闭" : "添加第一个对话", style: "headlineMedium", color: colors.onPrimaryContainer }),
      UI.Text({ text: upcoming ? `发往「${upcoming.title || upcoming.chatId}」` : enabled ? "准备好消息后，这里会显示发送时间。" : rules.length ? "在对话规则中开启后，AI 会按设定准备消息。" : "设定发送节奏，让 AI 主动联系你。", style: "bodyMedium", color: colors.onPrimaryContainer, maxLines: 2, overflow: "ellipsis" }),
      UI.Button({ text: rules.length >= 8 ? "已达 8 个对话上限" : "添加对话", enabled: !view.busy && rules.length < 8, onClick: () => run(async () => {
        await reload(true);
        update({ page: "picker", query: "", offset: 0 });
      }) }),
    ], true),
    entry("rules", "tune", titles.rules, `${rules.length} 个对话 · ${enabled} 个已开启`, () => go("rules")),
    entry("activity", "history", titles.activity, waiting ? `${waiting} 条待处理计划` : "暂无待处理计划", () => go("activity")));
  } else if (view.page === "picker") {
    const available = data.chats.filter(chat => !rules.some(rule => rule.chatId === chat.id));
    const query = view.query.trim().toLowerCase();
    const matches = available.filter(chat => (chat.title || chat.id).toLowerCase().includes(query));
    nodes.push(field("搜索对话", view.query, value => update({ query: value, offset: 0 })));
    const rows: ComposeNode[] = [];
    for (const chat of matches.slice(view.offset, view.offset + pageSize)) {
      if (rows.length) rows.push(UI.HorizontalDivider({ color: colors.outlineVariant, thickness: 1 }));
      rows.push(UI.Surface({ key: chat.id, containerColor: colors.surface, onClick: () => edit(defaultRule(chat.id, chat.title)) },
        UI.Row({ fillMaxWidth: true, paddingVertical: 14, spacing: 12, verticalAlignment: "center" }, [
          UI.Icon({ name: "chatBubbleOutline", size: 18, tint: colors.onSurfaceVariant }),
          UI.Text({ text: chat.title || chat.id, weight: 1, style: "bodyLarge", maxLines: 2, overflow: "ellipsis" }),
          UI.Icon({ name: "chevronRight", size: 16, tint: colors.onSurfaceVariant }),
        ])));
    }
    if (rows.length) nodes.push(UI.Column({ fillMaxWidth: true }, rows));
    if (!matches.length) nodes.push(note(!data.chats.length ? "还没有对话。创建对话后，再来添加规则。" : !available.length ? "所有对话都已配置规则。" : "没有匹配的对话，试试其他关键词。"));
    if (matches.length > pageSize) nodes.push(UI.Row({ fillMaxWidth: true, horizontalArrangement: "spaceBetween", verticalAlignment: "center" }, [
      UI.TextButton({ text: "上一页", enabled: view.offset > 0, onClick: () => update({ offset: view.offset - pageSize }) }),
      note(`${1 + Math.floor(view.offset / pageSize)} / ${Math.ceil(matches.length / pageSize)}`),
      UI.TextButton({ text: "下一页", enabled: view.offset + pageSize < matches.length, onClick: () => update({ offset: view.offset + pageSize }) }),
    ]));
  } else if (view.page === "rules") {
    for (const rule of rules) nodes.push(entry(rule.chatId, rule.enabled ? "notificationsActive" : "notificationsOff", rule.title || rule.chatId,
      `${rule.enabled ? "已开启" : "已关闭"} · ${rule.intervalMinutes}${rule.randomMinutes ? `～${rule.intervalMinutes + rule.randomMinutes}` : ""} 分钟 · ${rule.probability}% 触发`, () => edit(rule)));
    if (!rules.length) nodes.push(note("还没有规则。返回首页添加第一个对话。"));
    else nodes.push(note("停用或卸载插件前，请先关闭规则，以撤销已预约的消息。"));
  } else if (view.page === "editor" && draft) {
    nodes.push(panel([
      UI.Text({ text: draft.title || draft.chatId, style: "titleLarge" }),
      UI.Row({ verticalAlignment: "center", spacing: 12 }, [UI.Text({ text: "启用主动消息", weight: 1 }), UI.Switch({ checked: draft.enabled, enabled: !view.busy, onCheckedChange: enabled => patchDraft({ enabled }) })]),
    ]), panel([
      UI.Text({ text: "发送节奏", style: "titleMedium" }),
      numericField("固定间隔（1～10080 分钟）", "intervalMinutes"), numericField("触发概率（0～100%）", "probability"),
      UI.TextButton({ text: view.advanced ? "收起高级设置" : "高级设置", onClick: () => update({ advanced: !view.advanced }) }),
      ...(view.advanced ? [numericField("随机追加（0～10080 分钟）", "randomMinutes"), numericField("提前准备次数（1～8）", "prepareCount")] : []),
    ]), panel([
      UI.Text({ text: "消息内容", style: "titleMedium" }), field("提示词", draft.prompt, prompt => patchDraft({ prompt }), 4),
      note("AI 使用目标对话的角色、模型、记忆和摘要后的上下文。对话或角色变化后，旧计划会失效并在前台重新准备。"),
    ]), note("保存会撤销原有预约。已开启的规则会重新准备消息。"),
    UI.Button({ text: view.busy ? "处理中…" : "保存规则", enabled: !view.busy, onClick: () => run(async () => {
      const input = live.current.draft!;
      const saved = await withContext("main", { input }, () => saveRule(input));
      update({ snapshot: saved, draft: null, page: "rules" });
    }) }));
    if (existing) nodes.push(UI.TextButton({ text: "删除规则", enabled: !view.busy, onClick: () => update({ confirmDelete: true }) }));
    if (view.confirmDelete) nodes.push(UI.AlertDialog({ title: "删除这条规则？", text: "此对话已预约的消息也会撤销。", confirmText: "删除", dismissText: "取消",
      onDismissRequest: () => update({ confirmDelete: false }), onDismiss: () => update({ confirmDelete: false }), onConfirm: () => run(async () => {
        update({ confirmDelete: false });
        const chatId = draft.chatId;
        await withContext("main", { chatId }, () => removeRule(chatId));
        go("rules");
        await reload();
      }) }));
  } else if (view.page === "activity") {
    nodes.push(UI.Row({ spacing: 8 }, [UI.Button({ text: view.busy ? "处理中…" : "补充发送计划", enabled: !view.busy, onClick: () => run(async () => {
      await withContext("main", {}, () => prepare());
      await reload();
    }) }), UI.TextButton({ text: "刷新", enabled: !view.busy, onClick: () => run(() => reload()) })]));
    const upcoming = jobs.filter(pending).sort((first, second) => first.dueAt - second.dueAt);
    const history = jobs.filter(job => !pending(job)).sort((first, second) => second.dueAt - first.dueAt).slice(0, 48);
    for (const [title, items] of [["待处理", upcoming], ["最近记录", history]] as const) {
      if (items.length) nodes.push(UI.Text({ text: title, style: "titleMedium" }));
      for (const job of items) nodes.push(panel([
        UI.Text({ text: job.title || job.chatId, style: "titleSmall" }), note(`${labels[job.status]} · ${new Date(job.dueAt).toLocaleString()}`),
        ...(job.text ? [UI.Text({ text: job.text, maxLines: 4, overflow: "ellipsis" })] : []),
        ...(job.error ? [UI.Text({ text: job.error, color: colors.error })] : []),
      ]));
    }
    if (!jobs.length) nodes.push(note("暂无发送计划或记录。开启对话规则后，可在这里查看进度。"));
  }
  async function loadHome(): Promise<void> {
    await withContext("main", {}, () => setForeground(true));
    await reload();
  }
  return UI.Box({ fillMaxSize: true, topBarTitle: UI.Text({ text: titles.home }), onLoad: () => run(loadHome) },
    UI.LazyColumn({ key: view.page, fillMaxSize: true, padding: 16, spacing: 12 }, nodes));
}
