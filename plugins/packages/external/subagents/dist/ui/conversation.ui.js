"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = Conversation;
const client_js_1 = require("../client.js");
const model_js_1 = require("../model.js");
const shared_js_1 = require("./shared.js");
/** Owns the selected child's presentation; closing this tab only releases its observers. */
function Conversation(ctx) {
    const [input] = ctx.useState("input", { parentChatId: "", agentId: "" });
    const chatId = (0, model_js_1.requireText)(input.parentChatId, "parentChatId");
    const selectedId = (0, model_js_1.requireText)(input.agentId, "agentId");
    const [agent, setAgent] = ctx.useState("subagents.detail", null);
    const [message, setMessage] = ctx.useState("subagents.message", "");
    const [expanded, setExpanded] = ctx.useState("subagents.input-expanded", false);
    const [busy, setBusy] = ctx.useState("subagents.busy", false);
    const [waiting, setWaiting] = ctx.useState("subagents.waiting", false);
    const [error, setError] = ctx.useState("subagents.error", null);
    const [notice, setNotice] = ctx.useState("subagents.notice", null);
    const selectedRef = ctx.useRef("subagents.selected", selectedId);
    selectedRef.current = selectedId;
    /** Streams only the chosen task's current execution and cancels the old collector on selection changes. */
    ctx.useEffect("subagents.detail-watch", () => {
        setAgent(null);
        setError(null);
        setMessage("");
        setNotice(null);
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
    /** Runs a concrete control action and keeps its actual error visible in the workspace conversation. */
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
    /** Preserves the selected task's identity while closing its native execution and descendants. */
    async function close(agentId) {
        await act(async () => { await client_js_1.Agents.close(chatId, agentId); });
    }
    /** Sends a new instruction only through the owner's explicit interrupt policy. */
    async function send(interrupt) {
        const target = selectedId;
        await act(async () => {
            await client_js_1.Agents.send(chatId, target, message, interrupt);
            if (selectedRef.current === target)
                setMessage("");
        });
    }
    /** Lets the user keep waiting in bounded intervals while other execution controls remain available. */
    async function wait() {
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
    const editable = agent !== null && agent.status !== "closed" && agent.status !== "closing";
    const active = agent !== null && (0, model_js_1.isActive)(agent.status);
    const canSend = editable && !busy && message.trim() !== "";
    const sendLabel = busy ? "正在处理…" : !editable ? (agent === null ? "正在读取…" : shared_js_1.labels[agent.status]) : active ? "打断并发送" : "发送新指令";
    return ctx.UI.Column({ key: "subagents.conversation", fillMaxWidth: true, fillMaxHeight: true, padding: 12, spacing: 8 }, [
        ctx.UI.Text({ text: agent === null ? "子代理会话" : agent.name, style: "titleMedium", maxLines: 1, overflow: "ellipsis" }),
        ...(agent === null ? [ctx.UI.Text({ text: "正在读取子代理…" })] : [
            ctx.UI.Text({ text: shared_js_1.labels[agent.status] + " · 第 " + agent.runNumber + " 轮", style: "labelMedium" }),
            ctx.UI.AiChat({ key: "subagents.execution", chatId: agent.chatId, showInput: false, weight: 1, fillMaxWidth: true }),
            ...(agent.error === null ? [] : [ctx.UI.Text({ text: agent.error, color: ctx.MaterialTheme.colorScheme.error })]),
        ]),
        ...(notice === null ? [] : [ctx.UI.Text({ key: "subagents.notice", text: notice, style: "bodySmall" })]),
        ...(error === null ? [] : [ctx.UI.Text({ text: error, style: "bodySmall", color: ctx.MaterialTheme.colorScheme.error })]),
        ctx.UI.Card({ key: "subagents.input-surface", fillMaxWidth: true, containerColor: "surfaceContainer", themeSurface: "card", shape: { cornerRadius: 22 }, elevation: 0 }, ctx.UI.Column({ fillMaxWidth: true, padding: { start: 12, top: 14, end: 12, bottom: 8 }, spacing: 8 }, [
            ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: "topStart" }, [
                ctx.UI.TextField({ key: "subagents.followup", weight: 1, placeholder: editable ? "给子代理发送新指令" : sendLabel,
                    value: message, onValueChange: setMessage, minLines: expanded ? 5 : 1, maxLines: expanded ? 12 : 6,
                    readOnly: busy || !editable, style: { fontSize: 14 },
                    decoration: { borderless: true, filled: false, contentPadding: { start: 16, top: 10, end: 8, bottom: 8 }, hintStyle: { color: "onSurfaceVariant" } } }),
                ctx.UI.IconButton({ key: "subagents.input-expand", width: 32, height: 32,
                    content: ctx.UI.Icon({ name: expanded ? "fullscreenExit" : "fullscreen", size: 18, tint: "onSurfaceVariant" }),
                    tooltip: expanded ? "收起输入框" : "展开输入框",
                    onClick: () => { setExpanded(!expanded); } }),
            ]),
            ctx.UI.Row({ key: "subagents.input-actions", fillMaxWidth: true, spacing: 4, verticalAlignment: "center" }, [
                ctx.UI.IconButton({ key: "subagents.wait", width: 32, height: 32,
                    content: ctx.UI.Icon({ name: "hourglassEmpty", size: 18 }),
                    tooltip: waiting ? "等待中…" : "等待 30 秒", enabled: !waiting && active, onClick: wait }),
                ctx.UI.IconButton({ key: "subagents.close", width: 32, height: 32,
                    content: ctx.UI.Icon({ name: "close", size: 18 }),
                    tooltip: "关闭子代理", enabled: editable && !busy,
                    onClick: () => close(selectedId) }),
                ctx.UI.Text({ key: "subagents.send-label", text: sendLabel, style: "labelMedium", color: "onSurfaceVariant", weight: 1, maxLines: 1, overflow: "ellipsis" }),
                ctx.UI.Tooltip({ message: active ? "打断当前任务并发送新指令" : "发送新指令" }, ctx.UI.FilledIconButton({ key: active ? "subagents.interrupt" : "subagents.send", width: 40, height: 40,
                    content: ctx.UI.Icon({ name: "arrowUpward", size: 20 }), shape: { cornerRadius: 20 }, enabled: canSend,
                    onClick: () => send(active) })),
            ]),
        ])),
    ]);
}
