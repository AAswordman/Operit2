import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import { defaultRule, type Rule } from "./model";
import type { Snapshot } from "./service";
// These declarations name functions registered in the main runtime, never local UI owners.
declare function snapshot(): Promise<Snapshot>;
declare function saveRule(rule: Rule): Promise<Snapshot>;
declare function removeRule(chatId: string): Promise<void>;
declare function prepare(): Promise<void>;
declare function setForeground(value: boolean): Promise<void>;
type NumericField = "intervalMinutes" | "randomMinutes" | "probability" | "prepareCount";
interface View { snapshot: Snapshot | null; draft: Rule | null; numeric: Partial<Record<NumericField, string>>; busy: boolean; error: string }
const labels = { preparing: "正在准备", ready: "已准备", scheduling: "预约待确认", scheduled: "系统已预约", delivering: "正在发送", delivered: "已写入对话", skipped: "已跳过", failed: "失败", cancelled: "已撤销" };
export default function screen(ctx: ComposeDslContext): ComposeNode {
  const UI = ctx.UI;
  const [view, setView] = ctx.useState<View>("proactive-view", { snapshot: null, draft: null, numeric: {}, busy: false, error: "" });
  const live = ctx.useRef("proactive-live", view); live.current = view;
  function update(patch: Partial<View>): void { live.current = { ...live.current, ...patch }; setView(live.current); }
  async function reload(): Promise<void> { update({ snapshot: await withContext("main", {}, () => snapshot()) }); }
  async function run(action: () => Promise<void>): Promise<void> {
    if (live.current.busy) return;
    update({ busy: true, error: "" });
    try { await action(); await reload(); }
    catch (error) { update({ error: String(error) }); }
    finally { update({ busy: false }); }
  }
  function patch(patch: Partial<Rule>): void { if (live.current.draft) update({ draft: { ...live.current.draft, ...patch } }); }
  function field(title: string, value: string, change: (value: string) => void, lines = 1): ComposeNode {
    return UI.OutlinedTextField({ label: title, value, fillMaxWidth: true, enabled: !view.busy,
      singleLine: lines === 1, minLines: lines, onValueChange: change });
  }
  function numericField(title: string, key: NumericField): ComposeNode {
    return field(title, view.numeric[key] ?? String(view.draft![key]), value => {
      update({ numeric: { ...live.current.numeric, [key]: value } });
      patch({ [key]: value.trim() ? Number(value) : NaN });
    });
  }
  const data = view.snapshot;
  const draft = view.draft;
  const nodes: ComposeNode[] = [
    UI.Text({ text: "AI 主动消息", style: "headlineSmall" }),
    UI.Text({ text: "在前台提前决定发送时间、抽取触发概率并准备消息。每个对话可单独配置。", color: "onSurfaceVariant" }),
    UI.Text({ text: "所有平台均由宿主预约和投递消息。间隔、概率和内容在前台准备；iOS 由系统到时展示通知，恢复前台后由宿主同步对话。", style: "bodySmall", color: "onSurfaceVariant" }),
    UI.Text({ text: "间隔 = 固定分钟数 + 0～随机追加分钟数；每次独立抽取触发概率。未命中时不调用 AI。", style: "bodySmall" }),
    UI.Text({ text: "停用或卸载插件前，请先在此关闭规则，以撤销已预约的消息。", style: "bodySmall", color: "onSurfaceVariant" }),
    UI.Row({ spacing: 8 }, [
      UI.Button({ text: view.busy ? "处理中…" : "补充发送计划", enabled: !view.busy,
        onClick: () => run(async () => { await withContext("main", {}, () => prepare()); }) }),
      UI.TextButton({ text: "刷新", enabled: !view.busy, onClick: () => run(reload) }),
    ]),
  ];
  if (view.error) nodes.push(UI.Text({ text: view.error, color: "error" }));
  if (view.busy) nodes.push(UI.Text({ text: "正在准备或保存，请稍候…", color: "primary" }));
  for (const rule of data?.state.rules ?? []) {
    nodes.push(UI.Card({ fillMaxWidth: true }, UI.Column({ padding: 16, spacing: 8 }, [
      UI.Text({ text: rule.title || rule.chatId, style: "titleMedium" }),
      UI.Text({ text: `${rule.enabled ? "已开启" : "已关闭"} · ${rule.intervalMinutes} 分钟 + 随机 0～${rule.randomMinutes} 分钟 · 概率 ${rule.probability}%` }),
      UI.Row({ spacing: 8 }, [
        UI.TextButton({ text: "编辑", enabled: !view.busy, onClick: () => update({ draft: { ...rule }, numeric: {} }) }),
        UI.TextButton({ text: rule.enabled ? "关闭并撤销待发消息" : "开启", enabled: !view.busy,
          onClick: () => run(async () => { const input = { ...rule, enabled: !rule.enabled }; await withContext("main", { input }, () => saveRule(input)); }) }),
        UI.TextButton({ text: "删除", enabled: !view.busy,
          onClick: () => run(async () => { const chatId = rule.chatId; await withContext("main", { chatId }, () => removeRule(chatId)); }) }),
      ]),
    ])));
  }
  if (data && !draft) {
    nodes.push(UI.Text({ text: "选择对话以添加规则", style: "titleMedium" }));
    for (const chat of data.chats.filter(chat => !data.state.rules.some(rule => rule.chatId === chat.id)))
      nodes.push(UI.TextButton({ text: chat.title || chat.id, enabled: !view.busy, onClick: () => update({ draft: defaultRule(chat.id, chat.title), numeric: {} }) }));
    if (!data.chats.length) nodes.push(UI.Text({ text: "请先创建一个对话，再刷新此页。" }));
  }
  if (draft) nodes.push(UI.Card({ fillMaxWidth: true }, UI.Column({ padding: 16, spacing: 12 }, [
    UI.Text({ text: `配置：${draft.title || draft.chatId}`, style: "titleMedium" }),
    UI.Row({ verticalAlignment: "center" }, [UI.Text({ text: "启用主动消息", weight: 1 }), UI.Switch({ checked: draft.enabled, enabled: !view.busy, onCheckedChange: enabled => patch({ enabled }) })]),
    numericField("固定间隔（分钟，1～10080）", "intervalMinutes"),
    numericField("随机追加间隔（分钟，0 表示关闭）", "randomMinutes"),
    numericField("每次触发概率（0～100%）", "probability"),
    numericField("提前准备几次检查（1～8）", "prepareCount"),
    field("主动消息提示词", draft.prompt, prompt => patch({ prompt }), 4),
    UI.Text({ text: "保存会撤销旧计划并重新抽取时间和概率。准备消息使用默认 CHAT 功能模型，并参考最近 20 条对话；可在提示词中填写角色设定。所有对话最多保留 32 次未来检查。", style: "bodySmall", color: "onSurfaceVariant" }),
    UI.Row({ spacing: 8 }, [
      UI.Button({ text: "保存并准备", enabled: !view.busy, onClick: () => run(async () => {
        const input = live.current.draft!; await withContext("main", { input }, () => saveRule(input)); update({ draft: null });
      }) }), UI.TextButton({ text: "取消", enabled: !view.busy, onClick: () => update({ draft: null }) }),
    ]),
  ])));
  const jobs = [...(data?.state.jobs ?? [])].sort((a, b) => b.dueAt - a.dueAt).slice(0, 48);
  if (jobs.length) nodes.push(UI.Text({ text: "发送计划与最近记录", style: "titleMedium" }));
  for (const job of jobs) nodes.push(UI.Card({ fillMaxWidth: true }, UI.Column({ padding: 12, spacing: 6 }, [
    UI.Text({ text: `${job.title} · ${labels[job.status]}`, style: "titleSmall" }),
    UI.Text({ text: new Date(job.dueAt).toLocaleString(), style: "bodySmall" }),
    UI.Text({ text: job.text || (job.status === "skipped" ? "本次概率未命中，不生成消息" : "暂无内容"), maxLines: 4, overflow: "ellipsis" }),
    ...(job.error ? [UI.Text({ text: job.error, color: "error", style: "bodySmall" })] : []),
  ])));
  return UI.Box({ fillMaxSize: true, topBarTitle: UI.Text({ text: "AI 主动消息" }),
    onLoad: () => run(async () => { await withContext("main", {}, () => setForeground(true)); }) },
    UI.LazyColumn({ fillMaxSize: true, padding: 20, spacing: 12 }, nodes));
}
