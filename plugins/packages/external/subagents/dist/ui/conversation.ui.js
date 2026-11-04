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
    const canSend = agent !== null && agent.status !== "closed" && agent.status !== "closing" && !busy && message.trim() !== "";
    return ctx.UI.Column({ key: "subagents.conversation", fillMaxWidth: true, fillMaxHeight: true, padding: 12, spacing: 8 }, [
        ctx.UI.Text({ text: agent === null ? "子代理会话" : agent.name, style: "titleMedium", maxLines: 1, overflow: "ellipsis" }),
        ...(agent === null ? [ctx.UI.Text({ text: "正在读取子代理…" })] : [
            ctx.UI.Text({ text: shared_js_1.labels[agent.status] + " · 第 " + agent.runNumber + " 轮", style: "labelMedium" }),
            ctx.UI.AiChat({ key: "subagents.execution", chatId: agent.chatId, showInput: false, weight: 1, fillMaxWidth: true }),
            ...(agent.error === null ? [] : [ctx.UI.Text({ text: agent.error, color: ctx.MaterialTheme.colorScheme.error })]),
        ]),
        ...(notice === null ? [] : [ctx.UI.Text({ key: "subagents.notice", text: notice, style: "bodySmall" })]),
        ...(error === null ? [] : [ctx.UI.Text({ text: error, style: "bodySmall", color: ctx.MaterialTheme.colorScheme.error })]),
        ctx.UI.Row({ fillMaxWidth: true, spacing: 4, verticalAlignment: "center" }, [
            ctx.UI.TextButton({ key: "subagents.wait", text: waiting ? "等待中…" : "等待 30 秒", enabled: !waiting && agent !== null && (0, model_js_1.isActive)(agent.status), onClick: wait }),
            ctx.UI.TextButton({ text: "关闭子代理", enabled: agent !== null && !busy && agent.status !== "closed" && agent.status !== "closing",
                /** Closes the selected task while preserving this inspectable detail view. */
                onClick: () => close(selectedId) }),
        ]),
        ctx.UI.TextField({ key: "subagents.followup", label: "给子代理发送新指令", value: message, onValueChange: setMessage, minLines: 1, maxLines: 3, readOnly: busy || agent === null || agent.status === "closed" || agent.status === "closing" }),
        ctx.UI.Row({ fillMaxWidth: true, spacing: 8, verticalAlignment: "center" }, [
            ctx.UI.Button({ key: "subagents.send", text: "发送", enabled: canSend && agent !== null && !(0, model_js_1.isActive)(agent.status),
                /** Starts a normal follow-up only when the preceding turn has ended. */
                onClick: () => send(false) }),
            ctx.UI.FilledTonalButton({ key: "subagents.interrupt", text: busy ? "正在处理…" : "打断并发送", enabled: canSend && agent !== null && (0, model_js_1.isActive)(agent.status),
                /** Waits for real cancellation before sending the entered replacement instruction. */
                onClick: () => send(true) }),
        ]),
    ]);
}
