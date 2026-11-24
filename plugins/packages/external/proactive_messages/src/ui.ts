import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import { defaultRule, type Rule } from "./model";
import type { Snapshot } from "./service";

declare function snapshot(includeChats: boolean): Promise<Snapshot>;
declare function saveRule(rule: Rule): Promise<Snapshot>;
declare function removeRule(chatId: string): Promise<void>;
declare function openOriginal(chatId: string): Promise<void>;
declare function cancelChat(chatId: string): Promise<void>;
interface View { data: Snapshot | null; draft: Rule | null; query: string; busy: boolean; error: string; confirmDelete: boolean }
const status = { queued: "等待回复", running: "正在回复", completed: "已完成", failed: "失败", interrupted: "已停止" };

export default function screen(ctx: ComposeDslContext): ComposeNode {
  const UI = ctx.UI, colors = ctx.MaterialTheme.colorScheme;
  const [view, setView] = ctx.useState<View>("autonomous-reply-settings", { data: null, draft: null, query: "", busy: false, error: "", confirmDelete: false });
  const live = ctx.useRef("autonomous-reply-live", view); live.current = view;
  function update(patch: Partial<View>): void { live.current = { ...live.current, ...patch }; setView(live.current); }
  async function reload(): Promise<void> { const includeChats = true; update({ data: await withContext("main", { includeChats: true }, () => snapshot(includeChats)) }); }
  async function run(action: () => Promise<void>): Promise<void> {
    if (live.current.busy) return;
    update({ busy: true, error: "" });
    try { await action(); } catch (error) { update({ error: error instanceof Error ? error.message : String(error) }); }
    finally { update({ busy: false }); }
  }
  function note(text: string): ComposeNode { return UI.Text({ text, style: "bodySmall", color: colors.onSurfaceVariant }); }
  function panel(children: ComposeNode[]): ComposeNode {
    return UI.Card({ fillMaxWidth: true, shape: { cornerRadius: 16 }, containerColor: colors.surfaceContainerLow }, UI.Column({ padding: 16, spacing: 12 }, children));
  }
  const nodes: ComposeNode[] = [];
  if (view.error) nodes.push(UI.Text({ text: view.error, color: colors.error }));
  if (!view.data) nodes.push(note("正在加载对话配置…"));
  else if (view.draft) {
    const draft = view.draft;
    function patch(changes: Partial<Rule>): void { if (live.current.draft) update({ draft: { ...live.current.draft, ...changes } }); }
    nodes.push(panel([
      UI.Text({ text: draft.title || "未命名对话", style: "titleLarge" }),
      UI.Row({ verticalAlignment: "center" }, [UI.Text({ text: "启用自主回复", weight: 1 }), UI.Switch({ checked: draft.enabled, enabled: !view.busy, onCheckedChange: enabled => patch({ enabled }) })]),
      note("AI 在原始子对话中正常运行，自主决定向主对话展示哪些内容、分成几段。首次启用会复制已有对话记录。"),
      UI.OutlinedTextField({ label: "展示提示词", value: draft.prompt, fillMaxWidth: true, minLines: 4, enabled: !view.busy, onValueChange: (prompt: string) => patch({ prompt }) }),
      UI.OutlinedTextField({ label: "每字延迟（0～150 毫秒）", value: String(draft.charDelayMs), fillMaxWidth: true, singleLine: true, enabled: !view.busy,
        onValueChange: (value: string) => patch({ charDelayMs: value.trim() ? Number(value) : 0 }) }),
      note("AI 可以为每段单独指定等待时间，最多 3 秒。未指定时使用每字延迟；首段立即展示。"),
      UI.Button({ text: "保存配置", enabled: !view.busy, onClick: () => run(async () => {
        const rule = live.current.draft!;
        await withContext("main", { rule }, () => saveRule(rule)); update({ draft: null }); await reload();
      }) }),
      ...(draft.originalChatId ? [UI.Button({ text: "查看原始子对话", enabled: !view.busy, onClick: () => run(async () => {
        const chatId = draft.chatId; await withContext("main", { chatId }, () => openOriginal(chatId));
      }) })] : []),
      UI.TextButton({ text: "返回", enabled: !view.busy, onClick: () => update({ draft: null, error: "", confirmDelete: false }) }),
      ...(view.data.state.rules.some(rule => rule.chatId === draft.chatId) ? [UI.TextButton({ text: "移除配置", enabled: !view.busy, onClick: () => update({ confirmDelete: true }) })] : []),
    ]));
    if (view.confirmDelete) nodes.push(UI.AlertDialog({ title: "移除自主回复配置？", text: "当前回复会停止。主对话和原始子对话均保留。", confirmText: "移除", dismissText: "取消",
      onDismiss: () => update({ confirmDelete: false }), onConfirm: () => run(async () => {
        const chatId = draft.chatId; await withContext("main", { chatId }, () => removeRule(chatId)); update({ draft: null, confirmDelete: false }); await reload();
      }) }));
  } else {
    nodes.push(panel([
      UI.Text({ text: "让 AI 决定如何回复", style: "headlineSmall" }),
      note("选择一个对话后，AI 自主组织面向用户的消息，并逐段展示。完整的模型回复、思考和工具记录保留在“原始对话”子对话中。"),
      UI.Button({ text: "刷新状态", enabled: !view.busy, onClick: () => run(reload) }),
    ]));
    for (const rule of view.data.state.rules) nodes.push(panel([
      UI.Text({ text: rule.title || "未命名对话", style: "titleMedium" }),
      note(rule.enabled ? "自主回复已启用" : "自主回复已关闭"),
      UI.TextButton({ text: "编辑配置", enabled: !view.busy, onClick: () => update({ draft: { ...rule }, error: "" }) }),
      ...(rule.originalChatId ? [UI.TextButton({ text: "查看原始子对话", enabled: !view.busy, onClick: () => run(async () => { const chatId = rule.chatId; await withContext("main", { chatId }, () => openOriginal(chatId)); }) })] : []),
      ...(view.data.state.tasks.some(task => task.chatId === rule.chatId && ["queued", "running"].includes(task.status)) ? [UI.Button({ text: "停止回复", enabled: !view.busy, onClick: () => run(async () => {
        const chatId = rule.chatId; await withContext("main", { chatId }, () => cancelChat(chatId)); await reload();
      }) })] : []),
    ]));
    nodes.push(UI.Text({ text: "添加对话", style: "titleMedium" }));
    nodes.push(UI.OutlinedTextField({ label: "搜索对话", value: view.query, fillMaxWidth: true, singleLine: true, onValueChange: (query: string) => update({ query }) }));
    const candidates = view.data.chats.filter(chat => !view.data!.state.rules.some(rule => rule.chatId === chat.id) && `${chat.title} ${chat.id}`.toLowerCase().includes(view.query.toLowerCase()));
    for (const chat of candidates.slice(0, 30)) nodes.push(UI.TextButton({ text: chat.title || "未命名对话", key: chat.id, enabled: !view.busy, onClick: () => update({ draft: defaultRule(chat.id, chat.title) }) }));
    if (!candidates.length) nodes.push(note("没有可添加的对话。"));
    const tasks = view.data.state.tasks.slice(-12).reverse();
    if (tasks.length) {
      nodes.push(UI.Text({ text: "最近回复", style: "titleMedium" }));
      for (const task of tasks) nodes.push(panel([
        UI.Text({ text: `${status[task.status]} · 已展示 ${task.displayed} 段` }),
        note(task.input.text.slice(0, 120) || "附件消息"),
        ...(task.error ? [UI.Text({ text: task.error, color: colors.error })] : []),
      ]));
    }
  }
  return UI.Box({ fillMaxSize: true, topBarTitle: UI.Text({ text: "AI 自主回复" }), onLoad: () => run(reload) },
    UI.LazyColumn({ fillMaxSize: true, padding: 16, spacing: 12 }, nodes));
}
