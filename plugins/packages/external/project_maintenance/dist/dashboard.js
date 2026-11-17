"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.feedbackItem = feedbackItem;
exports.projectSummaries = projectSummaries;
/** Explicit projection excludes tokens, reply contexts, raw attachments and upstream account payloads. */
function feedbackItem(record) {
    return { key: record.message.key, channel: record.message.channel, authorId: record.message.authorId, conversationId: record.message.conversationId,
        projectPath: record.projectPath, text: record.message.text.slice(0, 4000), receivedAt: record.receivedAt, ticketId: record.ticketId, ignored: record.ignored ?? null };
}
function projectSummaries(tickets, bindings, channels) {
    const paths = new Set([...tickets.map(t => t.projectPath), ...bindings.map(b => b.projectPath), ...channels.flatMap(c => c.sessions.filter(s => s.enabled).map(s => s.workspacePath))].filter((p) => !!p));
    return [...paths].map(path => {
        const list = tickets.filter(t => t.projectPath === path).sort((a, b) => b.updatedAt - a.updatedAt);
        return { path, tickets: list.length, active: list.filter(t => ["triage", "implement", "review", "delivery"].includes(t.stage)).length,
            attention: list.filter(t => ["new", "needs_info", "failed", "interrupted"].includes(t.stage)).length,
            completed: list.filter(t => t.stage === "completed").length, updatedAt: list[0]?.updatedAt ?? null,
            chats: bindings.filter(b => b.projectPath === path).map(b => b.chatId),
            channels: channels.filter(c => c.sessions.some(s => s.enabled && s.workspacePath === path)).map(c => c.channel) };
    });
}
