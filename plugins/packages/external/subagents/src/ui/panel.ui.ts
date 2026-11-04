import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl.js";
import { Agents } from "../client.js";
import { errorText, isActive, type AgentSummary, type ParentSnapshot } from "../model.js";
import { CONVERSATION_ROUTE, labels, statusIcons } from "./shared.js";

/** Owns only subscriptions and presentation; every execution and control lives in the retained main service. */
export default function Screen(ctx: ComposeDslContext): ComposeNode {
  const [chatId] = ctx.useState<string>("chatId", "");
  const [parent, setParent] = ctx.useState<ParentSnapshot>("subagents.parent", { schemaVersion: 1, agents: [] });
  const [loaded, setLoaded] = ctx.useState("subagents.loaded", false);
  const [expanded, setExpanded] = ctx.useState("subagents.expanded", false);
  const [showClosed, setShowClosed] = ctx.useState("subagents.showClosed", false);
  const [busy, setBusy] = ctx.useState("subagents.busy", false);
  const [error, setError] = ctx.useState<string | null>("subagents.error", null);
  /** Collects host-backed status changes and disposes the subscription when the parent changes. */
  ctx.useEffect("subagents.parent-watch", () => {
    const controller = new AbortController();
    setLoaded(false);
    setParent({ schemaVersion: 1, agents: [] });
    setError(null);
    /** Reports real observation failures while treating explicit view disposal as cancellation. */
    void (async () => {
      try {
        for await (const value of Agents.watch(chatId, controller.signal)) {
          if (controller.signal.aborted) return;
          setParent(value);
          setLoaded(true);
        }
      } catch (failure) {
        if (!controller.signal.aborted) { setError(errorText(failure)); ctx.reportError(failure); }
      }
    })();
    /** Releases UI observation without cancelling background agents. */
    return () => { controller.abort(); };
  }, [chatId]);

  /** Runs a concrete control action and keeps its actual error visible in the composer. */
  async function act(operation: () => Promise<void>): Promise<void> {
    setBusy(true);
    setError(null);
    try { await operation(); }
    catch (failure) { setError(errorText(failure)); ctx.reportError(failure); }
    finally { setBusy(false); }
  }

  /** Preserves the selected task's identity while closing its native execution and descendants. */
  async function close(agentId: string): Promise<void> {
    await act(async () => { await Agents.close(chatId, agentId); });
  }

  /** Opens one stable workspace tab for this child without changing the main conversation. */
  async function openConversation(child: AgentSummary): Promise<void> {
    await act(async () => {
      await ctx.navigate(CONVERSATION_ROUTE, {
        parentChatId: chatId,
        agentId: child.id,
        workspaceTabId: JSON.stringify([chatId, child.id]),
        workspaceTabTitle: child.name,
      });
    });
  }

  const visible = parent.agents.filter(child => showClosed || isActive(child.status));
  const activeCount = parent.agents.filter(child => isActive(child.status)).length;
  const content: ComposeNode[] = [];
  // Idle agent lists never reserve composer space.
  if (activeCount > 0) {
    const rows: ComposeNode[] = [
      ctx.UI.Row({ key: "subagents.header", fillMaxWidth: true,
        padding: { horizontal: 12, vertical: 4 }, spacing: 4, verticalAlignment: "center",
        /** Expands task rows only when the user asks to inspect them. */
        onClick: () => { setExpanded(!expanded); },
      }, [
        ctx.UI.Icon({ key: "subagents.status-icon", name: "accountTree", tint: "onSurfaceVariant", size: 16 }),
        ctx.UI.Text({ key: "subagents.title", text: "子代理 · " + activeCount + " 运行中", style: "labelMedium", color: "onSurfaceVariant", weight: 1, maxLines: 1, overflow: "ellipsis" }),
        ctx.UI.IconButton({ key: "subagents.history", content: ctx.UI.Icon({ name: "history", size: 20 }), tooltip: showClosed ? "隐藏已结束任务" : "查看任务历史", width: 32, height: 32, enabled: loaded,
          /** Shows retained tasks inside the expanded queue surface. */
          onClick: () => { setShowClosed(!showClosed); setExpanded(true); } }),
        ctx.UI.IconButton({ key: "subagents.expand", content: ctx.UI.Icon({ name: expanded ? "keyboardArrowUp" : "keyboardArrowDown", size: 20 }), tooltip: expanded ? "收起子代理" : "展开子代理", width: 32, height: 32,
          /** Toggles the task list without changing any running agent. */
          onClick: () => { setExpanded(!expanded); } }),
      ]),
    ];
    if (expanded) rows.push(ctx.UI.Column({ key: "subagents.tasks", fillMaxWidth: true, paddingHorizontal: 8, paddingBottom: 8, spacing: 8 },
      visible.map(child => ctx.UI.Card({ key: "subagents.task-surface:" + child.id, fillMaxWidth: true, containerColor: "surface", themeSurface: "control", shape: { cornerRadius: 8 }, elevation: 0 },
        ctx.UI.Row({ key: "agent-" + child.id, fillMaxWidth: true, padding: { horizontal: 6, vertical: 2 }, spacing: 6, verticalAlignment: "center",
        /** Focuses the workspace conversation for this exact child identity. */
        onClick: () => openConversation(child),
      }, [
        ctx.UI.Icon({ name: statusIcons[child.status], size: 16, tint: child.status === "failed" ? "error" : "onSurfaceVariant" }),
        ctx.UI.Text({ text: child.name, style: "bodySmall", weight: 1, maxLines: 1, overflow: "ellipsis" }),
        ctx.UI.IconButton({ content: ctx.UI.Icon({ name: "close", size: 16 }), tooltip: "关闭子代理", width: 28, height: 28, enabled: !busy && child.status !== "closing" && child.status !== "closed",
          /** Cancels and closes the actual child while retaining its transcript. */
          onClick: () => close(child.id) }),
      ]))),
    ));
    if (error !== null) rows.push(ctx.UI.Text({ key: "subagents.error", text: error, paddingHorizontal: 12, paddingBottom: 8, style: "bodySmall", color: ctx.MaterialTheme.colorScheme.error }));
    content.push(ctx.UI.Card({ key: "subagents.surface", fillMaxWidth: true, containerColor: "surfaceContainer", themeSurface: "card", shape: { cornerRadius: 8 }, elevation: 0 },
      ctx.UI.Column({ fillMaxWidth: true }, rows),
    ));
  }

  return ctx.UI.Column({ key: "subagents.composer", fillMaxWidth: true, paddingHorizontal: 0, paddingBottom: activeCount > 0 ? 8 : 0 }, content);
}
