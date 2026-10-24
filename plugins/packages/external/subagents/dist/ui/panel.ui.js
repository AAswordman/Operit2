"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = Screen;
const client_js_1 = require("../client.js");
const model_js_1 = require("../model.js");
const labels = {
    starting: "正在启动", running: "运行中", completed: "已完成", cancelled: "已取消",
    failed: "执行失败", closing: "正在关闭", closed: "已关闭", interrupted: "观察已中断",
};
const statusIcons = {
    starting: "hourglassEmpty", running: "playArrow", completed: "checkCircle",
    cancelled: "cancel", failed: "errorOutline", closing: "hourglassEmpty",
    closed: "close", interrupted: "pause",
};
/** Owns only subscriptions and presentation; every execution and control lives in the retained main service. */
function Screen(ctx) {
    const [chatId] = ctx.useState("chatId", "");
    const [parent, setParent] = ctx.useState("subagents.parent", { schemaVersion: 1, agents: [] });
    const [loaded, setLoaded] = ctx.useState("subagents.loaded", false);
    const [dialog, setDialog] = ctx.useState("subagents.dialog", { kind: "none" });
    const [agent, setAgent] = ctx.useState("subagents.detail", null);
    const [expanded, setExpanded] = ctx.useState("subagents.expanded", false);
    const [showClosed, setShowClosed] = ctx.useState("subagents.showClosed", false);
    const [name, setName] = ctx.useState("subagents.name", "");
    const [task, setTask] = ctx.useState("subagents.task", "");
    const [message, setMessage] = ctx.useState("subagents.message", "");
    const [busy, setBusy] = ctx.useState("subagents.busy", false);
    const [waiting, setWaiting] = ctx.useState("subagents.waiting", false);
    const [error, setError] = ctx.useState("subagents.error", null);
    const [notice, setNotice] = ctx.useState("subagents.notice", null);
    const selectedId = dialog.kind === "agent" ? dialog.agentId : null;
    const selectedRef = ctx.useRef("subagents.selected", selectedId);
    selectedRef.current = selectedId;
    /** Collects host-backed status changes and disposes the subscription when the parent changes. */
    ctx.useEffect("subagents.parent-watch", () => {
        const controller = new AbortController();
        setLoaded(false);
        setParent({ schemaVersion: 1, agents: [] });
        setError(null);
        /** Reports real observation failures while treating explicit view disposal as cancellation. */
        void (async () => {
            try {
                for await (const value of client_js_1.Agents.watch(chatId, controller.signal)) {
                    if (controller.signal.aborted)
                        return;
                    setParent(value);
                    setLoaded(true);
                }
            }
            catch (failure) {
                if (!controller.signal.aborted) {
                    setError((0, model_js_1.errorText)(failure));
                    ctx.reportError(failure);
                }
            }
        })();
        /** Releases UI observation without cancelling background agents. */
        return () => { controller.abort(); };
    }, [chatId]);
    /** Streams only the chosen task's current execution and cancels the old collector on selection changes. */
    ctx.useEffect("subagents.detail-watch", () => {
        setAgent(null);
        setMessage("");
        setNotice(null);
        if (selectedId === null)
            return () => { };
        const controller = new AbortController();
        /** Applies full native message replacements through the package's synchronized detail record. */
        void (async () => {
            try {
                for await (const value of client_js_1.Agents.watchDetail(chatId, selectedId, controller.signal)) {
                    if (controller.signal.aborted)
                        return;
                    setAgent(value);
                }
            }
            catch (failure) {
                if (!controller.signal.aborted) {
                    setError((0, model_js_1.errorText)(failure));
                    ctx.reportError(failure);
                }
            }
        })();
        /** Releases the detail observer while preserving its task and native execution. */
        return () => { controller.abort(); };
    }, [chatId, selectedId]);
    /** Runs a concrete control action and keeps its actual error visible in the composer and dialog. */
    async function act(operation) {
        setBusy(true);
        setError(null);
        try {
            await operation();
        }
        catch (failure) {
            setError((0, model_js_1.errorText)(failure));
            ctx.reportError(failure);
        }
        finally {
            setBusy(false);
        }
    }
    /** Creates a source-configured child with the user-authored name and self-contained task. */
    async function create() {
        await act(async () => {
            const child = await client_js_1.Agents.spawn(chatId, name, task, null);
            setName("");
            setTask("");
            setDialog({ kind: "agent", agentId: child.id });
        });
    }
    /** Preserves the selected task's identity while closing its native execution and descendants. */
    async function close(agentId) {
        await act(async () => { await client_js_1.Agents.close(chatId, agentId); });
    }
    /** Sends a new instruction only through the owner's explicit interrupt policy. */
    async function send(interrupt) {
        if (selectedId === null)
            throw new Error("Select a subagent before sending");
        const target = selectedId;
        await act(async () => {
            await client_js_1.Agents.send(chatId, target, message, interrupt);
            if (selectedRef.current === target)
                setMessage("");
        });
    }
    /** Lets the user keep waiting in bounded intervals while other execution controls remain available. */
    async function wait() {
        if (selectedId === null)
            throw new Error("Select a subagent before waiting");
        const target = selectedId;
        setWaiting(true);
        setNotice(null);
        setError(null);
        try {
            const result = await client_js_1.Agents.wait(chatId, [target], model_js_1.DEFAULT_WAIT_MS);
            if (selectedRef.current === target)
                setNotice(result.timed_out ? "已等待 30 秒，任务仍在运行，可以继续等待。" : "观察的轮次已结束。");
        }
        catch (failure) {
            if (selectedRef.current === target) {
                setError((0, model_js_1.errorText)(failure));
                ctx.reportError(failure);
            }
        }
        finally {
            setWaiting(false);
        }
    }
    /** Opens the canonical conversation containing all earlier turns as well as the latest one. */
    async function openConversation() {
        if (agent === null)
            throw new Error("The selected subagent has not loaded");
        const target = agent.chatId;
        await act(async () => { await Tools.Chat.switchTo(target); setDialog({ kind: "none" }); });
    }
    /** Dismisses only the modal presentation. */
    function dismiss() { setDialog({ kind: "none" }); }
    const visible = parent.agents.filter(child => showClosed || (0, model_js_1.isActive)(child.status));
    const activeCount = parent.agents.filter(child => (0, model_js_1.isActive)(child.status)).length;
    const content = [];
    // Empty slots and retained dialogs never reserve composer space.
    if (activeCount > 0) {
        const rows = [
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
                ctx.UI.IconButton({ key: "subagents.create", content: ctx.UI.Icon({ name: "add", size: 20 }), tooltip: "创建子代理", width: 32, height: 32, enabled: loaded && !busy,
                    /** Opens the task form without starting an execution. */
                    onClick: () => { setError(null); setDialog({ kind: "create" }); } }),
                ctx.UI.IconButton({ key: "subagents.expand", content: ctx.UI.Icon({ name: expanded ? "keyboardArrowUp" : "keyboardArrowDown", size: 20 }), tooltip: expanded ? "收起子代理" : "展开子代理", width: 32, height: 32,
                    /** Toggles the task list without changing any running agent. */
                    onClick: () => { setExpanded(!expanded); } }),
            ]),
        ];
        if (expanded)
            rows.push(ctx.UI.Column({ key: "subagents.tasks", fillMaxWidth: true, paddingHorizontal: 8, paddingBottom: 8, spacing: 8 }, visible.map(child => ctx.UI.Card({ key: "subagents.task-surface:" + child.id, fillMaxWidth: true, containerColor: "surface", themeSurface: "control", shape: { cornerRadius: 8 }, elevation: 0 }, ctx.UI.Row({ key: "agent-" + child.id, fillMaxWidth: true, padding: { horizontal: 6, vertical: 2 }, spacing: 6, verticalAlignment: "center",
                /** Opens the host chat and detailed status for this exact child identity. */
                onClick: () => { setError(null); setDialog({ kind: "agent", agentId: child.id }); },
            }, [
                ctx.UI.Icon({ name: statusIcons[child.status], size: 16, tint: child.status === "failed" ? "error" : "onSurfaceVariant" }),
                ctx.UI.Text({ text: child.name, style: "bodySmall", weight: 1, maxLines: 1, overflow: "ellipsis" }),
                ctx.UI.IconButton({ content: ctx.UI.Icon({ name: "close", size: 16 }), tooltip: "关闭子代理", width: 28, height: 28, enabled: !busy && child.status !== "closing" && child.status !== "closed",
                    /** Cancels and closes the actual child while retaining its transcript. */
                    onClick: () => close(child.id) }),
            ])))));
        if (error !== null)
            rows.push(ctx.UI.Text({ key: "subagents.error", text: error, paddingHorizontal: 12, paddingBottom: 8, style: "bodySmall", color: ctx.MaterialTheme.colorScheme.error }));
        content.push(ctx.UI.Card({ key: "subagents.surface", fillMaxWidth: true, containerColor: "surfaceContainer", themeSurface: "card", shape: { cornerRadius: 8 }, elevation: 0 }, ctx.UI.Column({ fillMaxWidth: true }, rows)));
    }
    if (dialog.kind === "create")
        content.push(ctx.UI.Dialog({ key: "subagents.create-dialog", onDismissRequest: dismiss }, ctx.UI.Column({ width: 560, padding: 16, spacing: 12 }, [
            ctx.UI.Text({ text: "创建子代理", style: "titleMedium" }),
            ctx.UI.TextField({ key: "subagents.name", label: "名称", value: name, onValueChange: setName, singleLine: true }),
            ctx.UI.TextField({ key: "subagents.task", label: "任务与必要背景", value: task, onValueChange: setTask, minLines: 3, maxLines: 6 }),
            ctx.UI.Text({ text: "子代理使用独立会话，继承当前聊天的配置。任务说明需要包含它执行所需的背景。", style: "bodySmall" }),
            ...(error === null ? [] : [ctx.UI.Text({ text: error, color: ctx.MaterialTheme.colorScheme.error })]),
            ctx.UI.Row({ spacing: 8 }, [
                ctx.UI.TextButton({ text: "取消", enabled: !busy, onClick: dismiss }),
                ctx.UI.Button({ key: "subagents.start", text: busy ? "正在创建…" : "开始执行", enabled: !busy && name.trim() !== "" && task.trim() !== "", onClick: create }),
            ]),
        ])));
    if (dialog.kind === "agent") {
        const detailId = dialog.agentId;
        const canSend = agent !== null && agent.status !== "closed" && agent.status !== "closing" && !busy && message.trim() !== "";
        content.push(ctx.UI.Dialog({ key: "subagents.detail-dialog", onDismissRequest: dismiss }, ctx.UI.Column({ width: 680, height: 560, padding: 16, spacing: 8 }, [
            ctx.UI.Row({ fillMaxWidth: true, spacing: 4, verticalAlignment: "center" }, [
                ctx.UI.Text({ text: agent === null ? "子代理执行" : agent.name, style: "titleMedium", weight: 1 }),
                ctx.UI.IconButton({ icon: "Close", tooltip: "返回聊天", onClick: dismiss }),
            ]),
            ...(agent === null ? [ctx.UI.Text({ text: "正在读取子代理…" })] : [
                ctx.UI.Text({ text: labels[agent.status] + " · 第 " + agent.runNumber + " 轮", style: "labelMedium" }),
                ctx.UI.AiChat({ key: "subagents.execution", chatId: agent.chatId, showInput: false, weight: 1, fillMaxWidth: true }),
                ...(agent.error === null ? [] : [ctx.UI.Text({ text: agent.error, color: ctx.MaterialTheme.colorScheme.error })]),
            ]),
            ...(notice === null ? [] : [ctx.UI.Text({ key: "subagents.notice", text: notice, style: "bodySmall" })]),
            ...(error === null ? [] : [ctx.UI.Text({ text: error, style: "bodySmall", color: ctx.MaterialTheme.colorScheme.error })]),
            ctx.UI.Row({ fillMaxWidth: true, spacing: 4, verticalAlignment: "center" }, [
                ctx.UI.TextButton({ key: "subagents.wait", text: waiting ? "等待中…" : "等待 30 秒", enabled: !waiting && agent !== null && (0, model_js_1.isActive)(agent.status), onClick: wait }),
                ctx.UI.TextButton({ text: "完整会话", enabled: agent !== null && !busy, onClick: openConversation }),
                ctx.UI.TextButton({ text: "关闭子代理", enabled: agent !== null && !busy && agent.status !== "closed" && agent.status !== "closing",
                    /** Closes the selected task while preserving this inspectable detail view. */
                    onClick: () => close(detailId) }),
            ]),
            ctx.UI.TextField({ key: "subagents.followup", label: "给子代理发送新指令", value: message, onValueChange: setMessage, minLines: 1, maxLines: 3, readOnly: busy || agent?.status === "closed" }),
            ctx.UI.Row({ fillMaxWidth: true, spacing: 8, verticalAlignment: "center" }, [
                ctx.UI.Button({ key: "subagents.send", text: "发送", enabled: canSend && agent !== null && !(0, model_js_1.isActive)(agent.status),
                    /** Starts a normal follow-up only when the preceding turn has ended. */
                    onClick: () => send(false) }),
                ctx.UI.FilledTonalButton({ key: "subagents.interrupt", text: busy ? "正在处理…" : "打断并发送", enabled: canSend && agent !== null && (0, model_js_1.isActive)(agent.status),
                    /** Waits for real cancellation before sending the entered replacement instruction. */
                    onClick: () => send(true) }),
            ]),
        ])));
    }
    return ctx.UI.Column({ key: "subagents.composer", fillMaxWidth: true, paddingHorizontal: 0, paddingBottom: activeCount > 0 ? 8 : 0 }, content);
}
