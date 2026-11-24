import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl.js";
import { Agents } from "../client.js";
import { MAX_ACTIVE_AGENTS, errorText, type AgentSettings, type Capacity } from "../model.js";

/** Presents all space-wide subagent preferences on one themed settings page. */
export default function Settings(ctx: ComposeDslContext): ComposeNode {
  const colors = ctx.MaterialTheme.colorScheme;
  const [capacity, setCapacity] = ctx.useState<Capacity | null>("capacity", null);
  const [settings, setSettings] = ctx.useState<AgentSettings | null>("settings", null);
  const [busy, setBusy] = ctx.useState("busy", false);
  const [error, setError] = ctx.useState("error", "");
  /** Loads both actual owner snapshots and surfaces host errors. */
  async function refresh(): Promise<void> {
    setBusy(true); setError("");
    try { const [current, preferences] = await Promise.all([Agents.capacity(), Agents.settings()]); setCapacity(current); setSettings(preferences); }
    catch (failure) { setError(errorText(failure)); ctx.reportError(failure); }
    finally { setBusy(false); }
  }
  /** Commits one explicit change before showing it as saved. */
  async function setLimit(limit: number): Promise<void> {
    setBusy(true); setError("");
    try { setCapacity(await Agents.setLimit(limit)); }
    catch (failure) { setError(errorText(failure)); ctx.reportError(failure); }
    finally { setBusy(false); }
  }
  /** Commits the complete behavior snapshot through the retained owner. */
  async function update(patch: Partial<AgentSettings>): Promise<void> {
    if (settings === null) return;
    setBusy(true); setError("");
    try { setSettings(await Agents.setSettings({ ...settings, ...patch })); }
    catch (failure) { setError(errorText(failure)); ctx.reportError(failure); }
    finally { setBusy(false); }
  }
  /** Renders a compact themed section matching other plugin settings. */
  function card(icon: string, title: string, children: ComposeNode[]): ComposeNode {
    return ctx.UI.Card({ fillMaxWidth: true, containerColor: colors.surfaceContainerLow, shape: { cornerRadius: 16 } },
      ctx.UI.Column({ fillMaxWidth: true, padding: 16, spacing: 12 }, [
        ctx.UI.Row({ spacing: 12, verticalAlignment: "center" }, [ctx.UI.Icon({ name: icon, size: 22, tint: colors.primary }), ctx.UI.Text({ text: title, style: "titleMedium" })]), ...children,
      ]));
  }
  /** Renders supporting text using the active theme. */
  function description(text: string): ComposeNode { return ctx.UI.Text({ text, style: "bodyMedium", color: colors.onSurfaceVariant }); }
  /** Renders a setting with its explanation and switch on the same row. */
  function toggle(title: string, text: string, checked: boolean, onChange: (value: boolean) => void): ComposeNode {
    return ctx.UI.Row({ fillMaxWidth: true, spacing: 16, verticalAlignment: "center" }, [
      ctx.UI.Column({ weight: 1, spacing: 4 }, [ctx.UI.Text({ text: title, style: "titleSmall" }), description(text)]),
      ctx.UI.Switch({ checked, enabled: !busy && settings !== null, onCheckedChange: onChange }),
    ]);
  }
  ctx.useEffect("load", () => { void refresh(); return () => {}; }, []);
  return ctx.UI.ScrollArea({ fillMaxSize: true }, ctx.UI.Column({ fillMaxWidth: true, padding: 20, spacing: 16 }, [
    ctx.UI.Text({ text: "子代理", style: "headlineSmall" }),
    description("设置应用于当前空间中的所有聊天。"),
    card("AccountTree", "并发执行", [
      ctx.UI.Text({ text: "同时运行的子代理", style: "titleSmall" }),
      description("所有聊天及嵌套任务共享名额。调低上限不会中断正在运行的任务。"),
      ctx.UI.Row({ fillMaxWidth: true, spacing: 12, verticalAlignment: "center" }, [
        ctx.UI.Button({ text: "−", enabled: !busy && capacity !== null && capacity.limit > 1, onClick: () => { if (capacity !== null) void setLimit(capacity.limit - 1); } }),
        ctx.UI.Text({ text: capacity === null ? "…" : `${capacity.limit} / ${MAX_ACTIVE_AGENTS}`, style: "titleMedium" }),
        ctx.UI.Button({ text: "+", enabled: !busy && capacity !== null && capacity.limit < MAX_ACTIVE_AGENTS, onClick: () => { if (capacity !== null) void setLimit(capacity.limit + 1); } }),
      ]),
      description(capacity === null ? "正在读取运行状态…" : `运行或启动中 ${capacity.active} · 可用 ${capacity.available}`),
      ctx.UI.TextButton({ text: "刷新运行状态", enabled: !busy, onClick: refresh }),
    ]),
    card("Settings", "任务行为", [
      toggle("允许子代理继续分派", "关闭后，已有子代理不能再创建下一级任务。", settings?.allowNested ?? true, value => { void update({ allowNested: value }); }),
      ctx.UI.Text({ text: "默认等待时间", style: "titleSmall" }),
      description("主代理未指定等待时间时使用；超时不会停止子代理。"),
      ctx.UI.Row({ spacing: 12, verticalAlignment: "center" }, [
        ctx.UI.Button({ text: "−", enabled: !busy && settings !== null && settings.defaultWaitMs > 5000, onClick: () => { if (settings !== null) void update({ defaultWaitMs: Math.max(5000, settings.defaultWaitMs - 5000) }); } }),
        ctx.UI.Text({ text: settings === null ? "…" : `${settings.defaultWaitMs / 1000} 秒`, style: "titleMedium" }),
        ctx.UI.Button({ text: "+", enabled: !busy && settings !== null && settings.defaultWaitMs < 30000, onClick: () => { if (settings !== null) void update({ defaultWaitMs: Math.min(30000, settings.defaultWaitMs + 5000) }); } }),
      ]),
    ]),
    card("ViewList", "聊天显示", [
      toggle("默认展开任务列表", "有运行中任务时，在聊天输入框上方显示任务详情。", settings?.expandPanel ?? false, value => { void update({ expandPanel: value }); }),
    ]),
    ...(error ? [ctx.UI.Text({ text: error, color: colors.error })] : []),
  ]));
}
