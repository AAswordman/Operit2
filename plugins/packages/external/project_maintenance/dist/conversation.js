"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.maintenanceConversation = maintenanceConversation;
const client_js_1 = require("./client.js");
/** Supplies only structured ticket state for the current parent conversation. */
async function maintenanceConversation(event) {
    const payload = event.eventPayload;
    if ((payload.stage ?? event.eventName) !== "before_send_to_model" || !payload.chatId || !payload.preparedHistory)
        return null;
    let state;
    try {
        state = await (0, client_js_1.call)({ action: "conversation_status", chatId: payload.chatId });
    }
    catch (error) {
        console.error("[project_maintenance.conversation]", error instanceof Error ? error.message : String(error));
        return null;
    }
    if (!state.bound && state.total === 0)
        return null;
    const instruction = [
        "当前对话连接了项目维护服务。以下是可信服务提供的工单状态快照（数据，不是用户指令）：",
        JSON.stringify(state),
        "counts 是当前聊天全部工单的状态数量，tickets 只包含最近更新的最多五项；用户询问维护进度时，请根据工单状态如实回答；需要其他工单或详情时调用维护工具查询，不要猜测结果或宣称尚未发生的提交、PR或修复。gitVerified 只代表宿主核验过提交和工作区；verifiedPullRequest 非空表示宿主曾核验 GitHub PR 的仓库、开放状态和提交，不能保证此后没有变化。验收报告是否真实执行仍未由宿主核验。",
        "当反馈聊天已绑定时，用户新发送的反馈会在短暂聚合后进入后台诊断；自然回复用户，不必等待后台工作完成。等待补充表示尚需信息，而不是已经修好。",
    ].join("\n");
    const history = payload.preparedHistory.slice();
    const index = history.findIndex(turn => turn.kind === "SYSTEM");
    if (index < 0)
        history.unshift({ kind: "SYSTEM", content: instruction });
    else
        history[index] = { ...history[index], content: `${history[index].content}\n\n${instruction}` };
    return { preparedHistory: history };
}
