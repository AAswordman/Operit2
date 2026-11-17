"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.AppRecovery = void 0;
exports.appMessage = appMessage;
exports.isStatusQuestion = isStatusQuestion;
/** Rescans only explicitly bound conversations, so missed persistence hooks are repairable. */
class AppRecovery {
    constructor(repository, capture, report, workspace, isBotChat) {
        this.repository = repository;
        this.capture = capture;
        this.report = report;
        this.workspace = workspace;
        this.isBotChat = isBotChat;
        this.running = false;
    }
    async run() {
        if (this.running)
            return;
        this.running = true;
        try {
            for (const binding of await this.repository.chatBindings()) {
                if (binding.projectPath === null || binding.since === null)
                    continue;
                try {
                    if (await this.isBotChat?.(binding.chatId))
                        continue;
                    const path = this.workspace ? await this.workspace(binding.chatId) : binding.projectPath;
                    if (path !== binding.projectPath) {
                        await this.repository.bindChat(binding.chatId, path, Date.now());
                        continue;
                    }
                    const found = await Tools.Chat.listChats({ chat_ids: [binding.chatId], limit: 1 });
                    const chat = found.chats.find(item => item.id === binding.chatId);
                    if (chat === undefined)
                        continue;
                    if (!Number.isSafeInteger(chat.messageCount) || chat.messageCount < 0)
                        throw new Error("Invalid chat message count");
                    // The host filters summary rows after applying range bounds. Iterate to the
                    // actual message count rather than treating a short page as end-of-history.
                    for (let start = 0; start < chat.messageCount; start += 100) {
                        const page = await Tools.Chat.getMessagesRange(binding.chatId, { order: "asc", start, end: start + 99 });
                        for (const item of page.messages) {
                            if (item.sender !== "user" || !Number.isSafeInteger(item.timestamp) || item.timestamp < binding.since || typeof item.content !== "string" || !item.content.trim() || isStatusQuestion(item.content))
                                continue;
                            const current = await this.repository.chatBinding(binding.chatId);
                            if (current?.projectPath !== binding.projectPath || current.since !== binding.since)
                                break;
                            await this.capture.capture(binding.chatId, path, appMessage(binding.chatId, item.timestamp, item.content));
                        }
                    }
                }
                catch (error) {
                    this.report(error);
                }
            }
        }
        finally {
            this.running = false;
        }
    }
}
exports.AppRecovery = AppRecovery;
function appMessage(chatId, timestamp, text) {
    const id = String(timestamp);
    return { key: ["app", chatId, id].map(encodeURIComponent).join(":"), channel: "app", accountId: "local", messageId: id,
        conversationId: chatId, authorId: "user", text, timestamp: id, context: {} };
}
/** Only recognize unambiguous short status questions; mixed feedback remains collectable. */
function isStatusQuestion(text) {
    // Only strip identifiers with the complete generated shape, so arbitrary
    // feedback following the word 工单 cannot disappear as a supposed ID.
    const value = text.trim().replace(/\b[a-z0-9]{6,16}-[a-f0-9]{32}\b/gi, "").trim();
    if (!value || value.length > 100)
        return false;
    if (!/(?:进度|状态|进展|修好|完成|处理|修复|到哪)/.test(value))
        return false;
    if (!/(?:[?？]|查询|查看|看看|怎么样|如何|到哪|了吗|进度|状态|进展)/.test(value))
        return false;
    return /^(?:请问|麻烦|帮我|想问|查询|查看|看看|当前|现在|最新|我的|这个|那个|上次|之前|刚才|项目|维护|修复|工单|问题|反馈|任务|后台|一下|下|进度|状态|进展|怎么样|如何|到哪一步|到哪里|到哪儿|到哪|处理|进展|完了|完成|修好|是什么|告诉我|已经|了吗|了|吗|呢|的|得|请|是|有|怎么样了|如何了|编号|id|ID|[\s，。！？,.!?:：#])+$/u.test(value);
}
