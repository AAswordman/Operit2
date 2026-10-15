/* METADATA
{
    "name": "extended_chat",
    "display_name": {
        "zh": "增强对话",
        "en": "Extended Chat"
    },
    "description": {
        "zh": "通用对话工具包：列出、查找、重命名、删除对话，读取消息并向指定运行时参与者发送消息。角色卡由其所属插件管理。",
        "en": "Generic chat tools: list, find, rename, delete and read conversations, and send to an explicit runtime participant. Character cards are managed by their owning plugin."
    },
    "enabledByDefault": true,
    "category": "Chat",
    "tools": [
        {
            "name": "list_chats",
            "description": {
                "zh": "列出并筛选对话（用于获取 chat_id）。",
                "en": "List and filter chats (to discover chat_id)."
            },
            "parameters": [
                {
                    "name": "query",
                    "description": {
                        "zh": "可选：标题筛选关键字",
                        "en": "Optional title keyword"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "match",
                    "description": {
                        "zh": "可选：contains/exact/regex（默认 contains）",
                        "en": "Optional: contains/exact/regex (default contains)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "limit",
                    "description": {
                        "zh": "可选：最多返回条数（默认 50）",
                        "en": "Optional max results (default 50)"
                    },
                    "type": "number",
                    "required": false
                },
                {
                    "name": "sort_by",
                    "description": {
                        "zh": "可选：updatedAt/createdAt/messageCount（默认 updatedAt）",
                        "en": "Optional: updatedAt/createdAt/messageCount (default updatedAt)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "sort_order",
                    "description": {
                        "zh": "可选：asc/desc（默认 desc）",
                        "en": "Optional: asc/desc (default desc)"
                    },
                    "type": "string",
                    "required": false
                }
            ]
        },
        {
            "name": "find_chat",
            "description": {
                "zh": "按标题查找一个对话并返回 chat_id。",
                "en": "Find a single chat by title and return chat_id."
            },
            "parameters": [
                {
                    "name": "query",
                    "description": {
                        "zh": "标题关键字/正则",
                        "en": "Title keyword/regex"
                    },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "match",
                    "description": {
                        "zh": "可选：contains/exact/regex（默认 contains）",
                        "en": "Optional: contains/exact/regex (default contains)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "index",
                    "description": {
                        "zh": "可选：当匹配多个时选择第 N 个（默认 0）",
                        "en": "Optional: pick Nth when multiple matches (default 0)"
                    },
                    "type": "number",
                    "required": false
                }
            ]
        },
        {
            "name": "read_messages",
            "description": {
                "zh": "读取指定对话的消息（可按 chat_id 或 chat_title 指定）。",
                "en": "Read messages from a chat (by chat_id or chat_title)."
            },
            "parameters": [
                {
                    "name": "chat_id",
                    "description": {
                        "zh": "目标对话 ID（可选）",
                        "en": "Target chat id (optional)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "chat_title",
                    "description": {
                        "zh": "目标对话标题（可选；当 chat_id 为空时使用）",
                        "en": "Target chat title (optional; used when chat_id is empty)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "chat_query",
                    "description": {
                        "zh": "可选：标题筛选关键字（当 chat_id/chat_title 为空时使用）",
                        "en": "Optional title keyword (used when chat_id/chat_title is empty)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "chat_index",
                    "description": {
                        "zh": "可选：当筛选结果有多个时选择第 N 个（默认 0）",
                        "en": "Optional: pick Nth when multiple matches (default 0)"
                    },
                    "type": "number",
                    "required": false
                },
                {
                    "name": "match",
                    "description": {
                        "zh": "可选：contains/exact/regex（默认 contains）",
                        "en": "Optional: contains/exact/regex (default contains)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "order",
                    "description": {
                        "zh": "可选：asc/desc（默认 desc）",
                        "en": "Optional: asc/desc (default desc)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "limit",
                    "description": {
                        "zh": "可选：返回消息条数（默认 20）",
                        "en": "Optional: max number of messages (default 20)"
                    },
                    "type": "number",
                    "required": false
                }
            ]
        },
        {
            "name": "rename_chat",
            "description": {
                "zh": "重命名指定对话（可按 chat_id 或 chat_title 指定）。",
                "en": "Rename a chat (by chat_id or chat_title)."
            },
            "parameters": [
                {
                    "name": "new_title",
                    "description": {
                        "zh": "新的对话标题",
                        "en": "New chat title"
                    },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "chat_id",
                    "description": {
                        "zh": "目标对话 ID（可选）",
                        "en": "Target chat id (optional)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "chat_title",
                    "description": {
                        "zh": "目标对话标题（可选；当 chat_id 为空时使用）",
                        "en": "Target chat title (optional; used when chat_id is empty)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "chat_query",
                    "description": {
                        "zh": "可选：标题筛选关键字（当 chat_id/chat_title 为空时使用）",
                        "en": "Optional title keyword (used when chat_id/chat_title is empty)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "chat_index",
                    "description": {
                        "zh": "可选：当筛选结果有多个时选择第 N 个（默认 0）",
                        "en": "Optional: pick Nth when multiple matches (default 0)"
                    },
                    "type": "number",
                    "required": false
                },
                {
                    "name": "match",
                    "description": {
                        "zh": "可选：contains/exact/regex（默认 contains）",
                        "en": "Optional: contains/exact/regex (default contains)"
                    },
                    "type": "string",
                    "required": false
                }
            ]
        },
        {
            "name": "delete_chat",
            "description": {
                "zh": "删除指定对话（可按 chat_id 或 chat_title 指定）。",
                "en": "Delete a chat (by chat_id or chat_title)."
            },
            "parameters": [
                {
                    "name": "chat_id",
                    "description": {
                        "zh": "目标对话 ID（可选）",
                        "en": "Target chat id (optional)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "chat_title",
                    "description": {
                        "zh": "目标对话标题（可选；当 chat_id 为空时使用）",
                        "en": "Target chat title (optional; used when chat_id is empty)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "chat_query",
                    "description": {
                        "zh": "可选：标题筛选关键字（当 chat_id/chat_title 为空时使用）",
                        "en": "Optional title keyword (used when chat_id/chat_title is empty)"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "chat_index",
                    "description": {
                        "zh": "可选：当筛选结果有多个时选择第 N 个（默认 0）",
                        "en": "Optional: pick Nth when multiple matches (default 0)"
                    },
                    "type": "number",
                    "required": false
                },
                {
                    "name": "match",
                    "description": {
                        "zh": "可选：contains/exact/regex（默认 contains）",
                        "en": "Optional: contains/exact/regex (default contains)"
                    },
                    "type": "string",
                    "required": false
                }
            ]
        },
        {
            "name": "chat_with_agent",
            "description": {
                "zh": "向指定运行时的已有对话发送消息。可显式指定参与者；不读取、创建或修改角色卡绑定。",
                "en": "Send to an existing conversation in an explicit runtime, optionally selecting a participant. Does not read or modify character-card bindings."
            },
            "parameters": [
                {
                    "name": "message",
                    "description": {
                        "zh": "消息原文",
                        "en": "Original message"
                    },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "chat_id",
                    "description": {
                        "zh": "已有对话 ID",
                        "en": "Existing chat ID"
                    },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "runtime",
                    "description": {
                        "zh": "对话运行时：main 或 floating",
                        "en": "Chat runtime: main or floating"
                    },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "participant_id",
                    "description": {
                        "zh": "可选：执行参与者 ID",
                        "en": "Optional execution participant ID"
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "timeout",
                    "description": {
                        "zh": "可选：等待秒数，默认 180；超时后任务仍继续",
                        "en": "Optional wait seconds, default 180; the task continues after timeout"
                    },
                    "type": "number",
                    "required": false
                },
                {
                    "name": "notify_reply",
                    "description": {
                        "zh": "可选：回复通知，默认 false",
                        "en": "Optional reply notification, default false"
                    },
                    "type": "boolean",
                    "required": false
                }
            ]
        },
        {
            "name": "agent_status",
            "description": {
                "zh": "查询对话的输入处理状态。",
                "en": "Check a chat's input processing status."
            },
            "parameters": [
                {
                    "name": "chat_id",
                    "description": {
                        "zh": "目标对话 ID",
                        "en": "Target chat id"
                    },
                    "type": "string",
                    "required": true
                }
            ]
        }
    ]
}*/

const HistoryChat = (function () {

    interface ToolResponse<T = unknown> {
        success: boolean;
        message: string;
        data?: T;
    }

    type ResolveChatIdParams = {
        chat_id?: string;
        chat_title?: string;
        chat_query?: string;
        chat_index?: number;
        match?: string;
    };

    type ReadMessagesParams = ResolveChatIdParams & {
        order?: string;
        limit?: number;
    };

    type RenameChatParams = ResolveChatIdParams & {
        new_title: string;
    };

    type DeleteChatParams = ResolveChatIdParams;

    type ListChatsParams = {
        query?: string;
        match?: string;
        limit?: number;
        sort_by?: string;
        sort_order?: string;
    };

    type FindChatParams = {
        query: string;
        match?: string;
        index?: number;
    };

    type AgentStatusParams = {
        chat_id: string;
    };

    type ChatWithAgentParams = {
        message: string;
        chat_id: string;
        runtime: 'main' | 'floating';
        participant_id?: string;
        timeout?: number;
        notify_reply?: boolean;
    };

    function normalizeMatchMode(match?: string): 'contains' | 'exact' | 'regex' {
        const m = (match || '').trim().toLowerCase();
        if (m === 'exact' || m === 'regex' || m === 'contains') return m;
        return 'contains';
    }

    async function list_chats_impl(params: ListChatsParams): Promise<ToolResponse> {
        const query = (params?.query ?? '').toString().trim();
        const matchMode = normalizeMatchMode(params?.match);
        const limitRaw = params && params.limit !== undefined ? Number(params.limit) : undefined;
        const limit = limitRaw !== undefined && !isNaN(limitRaw) ? limitRaw : undefined;
        const sortBy = params?.sort_by ? params.sort_by.toString().trim() : undefined;
        const sortOrder = params?.sort_order ? params.sort_order.toString().trim().toLowerCase() : undefined;

        const listParams: ToolParams = {};
        if (query) listParams.query = query;
        if (matchMode) listParams.match = matchMode;
        if (limit !== undefined) listParams.limit = limit;
        if (sortBy) listParams.sort_by = sortBy;
        if (sortOrder) listParams.sort_order = sortOrder;

        const listResult = await Tools.Chat.listChats(listParams);
        const chats = listResult.chats;
        return {
            success: true,
            message: '对话列表获取完成',
            data: {
                totalCount: listResult?.totalCount ?? chats.length,
                currentChatId: listResult?.currentChatId ?? null,
                matchedCount: listResult?.totalCount ?? chats.length,
                chats,
            }
        };
    }

    async function find_chat_impl(params: FindChatParams): Promise<ToolResponse> {
        const query = (params?.query ?? '').toString().trim();
        if (!query) {
            throw new Error('Missing parameter: query');
        }

        const matchMode = normalizeMatchMode(params?.match);
        const indexRaw = params && params.index !== undefined ? Number(params.index) : 0;
        const index = isNaN(indexRaw) ? 0 : indexRaw;
        const findParams: Parameters<typeof Tools.Chat.findChat>[0] = { query };
        if (matchMode) findParams.match = matchMode;
        if (index !== undefined) findParams.index = index;
        const findResult = await Tools.Chat.findChat(findParams);
        const picked = findResult?.chat ?? null;
        if (!picked) {
            throw new Error(`Chat not found by query: ${query}`);
        }

        return {
            success: true,
            message: '对话查找完成',
            data: {
                chat: picked,
                matchedCount: findResult?.matchedCount ?? 1,
            }
        };
    }

    async function resolveChatId(params: ResolveChatIdParams): Promise<string> {
        if (params && typeof params.chat_id === 'string' && params.chat_id.trim()) {
            return params.chat_id.trim();
        }

        const title = params && typeof params.chat_title === 'string' ? params.chat_title.trim() : '';
        const query = params && typeof params.chat_query === 'string' ? params.chat_query.trim() : '';
        const matchMode = normalizeMatchMode(params?.match);
        const indexRaw = params && params.chat_index !== undefined ? Number(params.chat_index) : 0;
        const index = isNaN(indexRaw) ? 0 : indexRaw;

        if (!title && !query) {
            throw new Error('Missing parameter: chat_id or chat_title or chat_query is required');
        }

        const needle = title || query;
        const findParams: Parameters<typeof Tools.Chat.findChat>[0] = { query: needle };
        findParams.match = title ? 'exact' : matchMode;
        if (index !== undefined) findParams.index = index;
        const findResult = await Tools.Chat.findChat(findParams);
        const picked = findResult?.chat ?? null;
        if (!picked?.id) {
            throw new Error(`Chat not found by query: ${needle}`);
        }
        return picked.id;
    }

    async function read_messages_impl(params: ReadMessagesParams): Promise<ToolResponse> {
        const chatId = await resolveChatId(params || {});

        const orderRaw = params && params.order !== undefined ? String(params.order).trim().toLowerCase() : '';
        const order = (orderRaw === 'asc' || orderRaw === 'desc') ? orderRaw : 'desc';

        const limitRaw = params && params.limit !== undefined ? Number(params.limit) : 20;
        const limit = isNaN(limitRaw) ? 20 : limitRaw;

        const result = await Tools.Chat.getMessages(chatId, {
            order,
            limit,
        });

        const rawMessages = result.messages;
        const text = rawMessages
            .map((m) => {
                const role = (m.sender ?? '').toString() || 'message';
                const ts = (m.timestamp !== undefined && m.timestamp !== null) ? String(m.timestamp) : '';
                const header = ts ? `[${ts}] ${role}` : role;
                return `${header}:\n${(m.content ?? '').toString()}`;
            })
            .join('\n\n');

        return {
            success: true,
            message: '读取对话消息完成',
            data: {
                result,
                text,
            },
        };
    }

    async function rename_chat_impl(params: RenameChatParams): Promise<ToolResponse> {
        const newTitle = (params?.new_title ?? '').toString().trim();
        if (!newTitle) {
            throw new Error('Missing parameter: new_title');
        }

        const chatId = await resolveChatId(params || {});
        const result = await Tools.Chat.updateTitle(chatId, newTitle);

        return {
            success: true,
            message: '对话重命名完成',
            data: {
                chat_id: chatId,
                title: newTitle,
                result,
            },
        };
    }

    async function delete_chat_impl(params: DeleteChatParams): Promise<ToolResponse> {
        const chatId = await resolveChatId(params || {});
        const result = await Tools.Chat.deleteChat(chatId);

        return {
            success: true,
            message: '对话删除完成',
            data: {
                chat_id: chatId,
                result,
            },
        };
    }

    async function agent_status_impl(params: AgentStatusParams): Promise<ToolResponse> {
        const chatId = (params?.chat_id ?? '').toString().trim();
        if (!chatId) {
            throw new Error('Missing parameter: chat_id');
        }
        const result = await Tools.Chat.agentStatus(chatId);
        return {
            success: true,
            message: '对话状态查询完成',
            data: {
                result,
            },
        };
    }

    /** Sends only a generic SDK request; plugin-owned configuration stays opaque. */
    async function chat_with_agent_impl(params: ChatWithAgentParams): Promise<ToolResponse> {
        const message = (params?.message ?? '').toString();
        const chatId = (params?.chat_id ?? '').toString().trim();
        const runtime = params?.runtime;
        if (!message.trim()) throw new Error('Missing parameter: message');
        if (!chatId) throw new Error('Missing parameter: chat_id');
        if (runtime !== 'main' && runtime !== 'floating') {
            throw new Error('runtime must be main or floating');
        }
        for (const key of ['character_card_name', 'persist_turn', 'hide_user_message', 'disable_warning']) {
            if (Object.prototype.hasOwnProperty.call(params, key)) {
                throw new Error(`Unsupported retired parameter: ${key}`);
            }
        }
        const participantId = params.participant_id?.trim();
        if (params.participant_id !== undefined && !participantId) {
            throw new Error('participant_id must be nonblank when supplied');
        }
        const timeoutSec = params.timeout === undefined ? 180 : Number(params.timeout);
        if (!Number.isFinite(timeoutSec) || timeoutSec <= 0) {
            throw new Error('timeout must be a positive finite number');
        }
        const sendPromise = Tools.Chat.sendMessage({
            kind: 'submit', chatId, runtime,
            input: { text: message, attachments: [], replyToMessageTimestamp: null },
            turn: participantId ? { kind: 'execute', participantId } : { kind: 'execute' },
            notifyReply: params.notify_reply ?? false,
        });
        let timer: ReturnType<typeof setTimeout> | undefined;
        try {
            const timeoutPromise = new Promise<null>((resolve) => {
                timer = setTimeout(() => resolve(null), timeoutSec * 1000);
            });
            const result = await Promise.race([sendPromise, timeoutPromise]);
            return {
                success: true,
                message: result === null ? `等待响应超时（${timeoutSec}s），任务仍在运行` : '消息发送完成',
                data: result === null
                    ? { chat_id: chatId, timeout: true, hint: 'Use agent_status to check execution.' }
                    : { chat_id: chatId, result },
            };
        } finally {
            if (timer !== undefined) clearTimeout(timer);
        }
    }

    async function wrapToolExecution<P>(func: (params: P) => Promise<ToolResponse>, params: P): Promise<void> {
        try {
            const result = await func(params);
            complete(result);
        } catch (error: unknown) {
            const message = error instanceof Error ? error.message : String(error);
            console.error(`Tool ${func.name} failed unexpectedly`, error);
            complete({
                success: false,
                message: `读取对话消息失败: ${message}`,
            });
        }
    }

    async function read_messages(params: ReadMessagesParams): Promise<void> {
        return await wrapToolExecution(read_messages_impl, params);
    }

    async function rename_chat(params: RenameChatParams): Promise<void> {
        return await wrapToolExecution(rename_chat_impl, params);
    }

    async function delete_chat(params: DeleteChatParams): Promise<void> {
        return await wrapToolExecution(delete_chat_impl, params);
    }

    async function list_chats(params: ListChatsParams): Promise<void> {
        return await wrapToolExecution(list_chats_impl, params);
    }

    async function find_chat(params: FindChatParams): Promise<void> {
        return await wrapToolExecution(find_chat_impl, params);
    }

    async function agent_status(params: AgentStatusParams): Promise<void> {
        return await wrapToolExecution(agent_status_impl, params);
    }

    async function chat_with_agent(params: ChatWithAgentParams): Promise<void> {
        return await wrapToolExecution(chat_with_agent_impl, params);
    }

    async function main(): Promise<void> {
        complete({
            success: true,
            message: 'extended_chat 工具包已加载',
            data: {
                hint: 'Use extended_chat:read_messages / rename_chat / delete_chat.',
            },
        });
    }

    return {
        list_chats,
        find_chat,
        read_messages,
        rename_chat,
        delete_chat,
        chat_with_agent,
        agent_status,
        main,
    };
})();

exports.list_chats = HistoryChat.list_chats;
exports.find_chat = HistoryChat.find_chat;
exports.read_messages = HistoryChat.read_messages;
exports.rename_chat = HistoryChat.rename_chat;
exports.delete_chat = HistoryChat.delete_chat;
exports.chat_with_agent = HistoryChat.chat_with_agent;
exports.agent_status = HistoryChat.agent_status;
exports.main = HistoryChat.main;
