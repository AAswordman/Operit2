/* METADATA
{
    "name": "extended_memory_tools",

    "display_name": {
        "zh": "增强记忆工具",
        "en": "Extended Memory Tools"
    },
    "description": {
        "zh": "拓展记忆工具包：提供创建、更新、删除、移动、链接记忆，以及更新 USER.md 的能力。",
        "en": "Extended memory tools: create, update, delete, move, link memories, and update USER.md."
    },
    "category": "Memory",
    "enabledByDefault": true,
    "tools": [
        {
            "name": "create_memory",
            "description": { "zh": "创建新的记忆节点。", "en": "Create a new memory node." },
            "parameters": [
                { "name": "target_owner_key", "description": { "zh": "目标记忆库 owner key，例如 character:<character-id> 或 shared:<shared-id>", "en": "Target memory owner key, such as character:<character-id> or shared:<shared-id>" }, "type": "string", "required": true },
                { "name": "title", "description": { "zh": "记忆标题", "en": "Memory title" }, "type": "string", "required": true },
                { "name": "content", "description": { "zh": "记忆内容", "en": "Memory content" }, "type": "string", "required": true },
                { "name": "content_type", "description": { "zh": "可选：内容类型，默认 text/plain", "en": "Optional: content type (default: text/plain)" }, "type": "string", "required": false },
                { "name": "source", "description": { "zh": "可选：来源，默认 ai_created", "en": "Optional: source (default: ai_created)" }, "type": "string", "required": false },
                { "name": "folder_path", "description": { "zh": "可选：文件夹路径，默认空", "en": "Optional: folder path (default: empty)" }, "type": "string", "required": false },
                { "name": "tags", "description": { "zh": "可选：标签（逗号分隔字符串）", "en": "Optional: tags (comma-separated string)" }, "type": "string", "required": false }
            ]
        },
        {
            "name": "update_memory",
            "description": { "zh": "按标题更新已有记忆节点。", "en": "Update an existing memory node by title." },
            "parameters": [
                { "name": "target_owner_key", "description": { "zh": "目标记忆库 owner key，例如 character:<character-id> 或 shared:<shared-id>", "en": "Target memory owner key, such as character:<character-id> or shared:<shared-id>" }, "type": "string", "required": true },
                { "name": "old_title", "description": { "zh": "原始标题（用于定位记忆）", "en": "Old title (to locate the memory)" }, "type": "string", "required": true },
                { "name": "new_title", "description": { "zh": "可选：新标题（重命名）", "en": "Optional: new title (rename)" }, "type": "string", "required": false },
                { "name": "content", "description": { "zh": "可选：新内容", "en": "Optional: new content" }, "type": "string", "required": false },
                { "name": "content_type", "description": { "zh": "可选：内容类型", "en": "Optional: content type" }, "type": "string", "required": false },
                { "name": "source", "description": { "zh": "可选：来源", "en": "Optional: source" }, "type": "string", "required": false },
                { "name": "credibility", "description": { "zh": "可选：可信度 0-1", "en": "Optional: credibility 0-1" }, "type": "number", "required": false },
                { "name": "importance", "description": { "zh": "可选：重要性 0-1", "en": "Optional: importance 0-1" }, "type": "number", "required": false },
                { "name": "folder_path", "description": { "zh": "可选：文件夹路径", "en": "Optional: folder path" }, "type": "string", "required": false },
                { "name": "tags", "description": { "zh": "可选：标签（逗号分隔字符串）", "en": "Optional: tags (comma-separated string)" }, "type": "string", "required": false }
            ]
        },
        {
            "name": "delete_memory",
            "description": { "zh": "按标题删除记忆节点（不可逆）。", "en": "Delete a memory node by title (irreversible)." },
            "parameters": [
                { "name": "target_owner_key", "description": { "zh": "目标记忆库 owner key，例如 character:<character-id> 或 shared:<shared-id>", "en": "Target memory owner key, such as character:<character-id> or shared:<shared-id>" }, "type": "string", "required": true },
                { "name": "title", "description": { "zh": "要删除的记忆标题", "en": "Memory title to delete" }, "type": "string", "required": true }
            ]
        },
        {
            "name": "move_memory",
            "description": { "zh": "批量移动记忆到新文件夹。可按标题列表和来源文件夹筛选。", "en": "Move memories to another folder in batch. Filter by titles and source folder." },
            "parameters": [
                { "name": "target_owner_key", "description": { "zh": "目标记忆库 owner key，例如 character:<character-id> 或 shared:<shared-id>", "en": "Target memory owner key, such as character:<character-id> or shared:<shared-id>" }, "type": "string", "required": true },
                { "name": "target_folder_path", "description": { "zh": "目标文件夹路径（空字符串表示未分类）", "en": "Target folder path (empty string means uncategorized)" }, "type": "string", "required": true },
                { "name": "titles", "description": { "zh": "可选：标题列表（逗号或换行分隔）", "en": "Optional: title list (comma/newline separated)" }, "type": "string", "required": false },
                { "name": "source_folder_path", "description": { "zh": "可选：来源文件夹路径（空字符串表示未分类）", "en": "Optional: source folder path (empty string means uncategorized)" }, "type": "string", "required": false }
            ]
        },
        {
            "name": "link_memories",
            "description": { "zh": "创建两条记忆之间的语义链接。", "en": "Create a semantic link between two memories." },
            "parameters": [
                { "name": "target_owner_key", "description": { "zh": "目标记忆库 owner key，例如 character:<character-id> 或 shared:<shared-id>", "en": "Target memory owner key, such as character:<character-id> or shared:<shared-id>" }, "type": "string", "required": true },
                { "name": "source_title", "description": { "zh": "源记忆标题", "en": "Source memory title" }, "type": "string", "required": true },
                { "name": "target_title", "description": { "zh": "目标记忆标题", "en": "Target memory title" }, "type": "string", "required": true },
                { "name": "link_type", "description": { "zh": "可选：关系类型，默认 related", "en": "Optional: link type (default: related)" }, "type": "string", "required": false },
                { "name": "weight", "description": { "zh": "可选：强度 0-1，默认 0.7", "en": "Optional: weight 0-1 (default: 0.7)" }, "type": "number", "required": false },
                { "name": "description", "description": { "zh": "可选：关系描述", "en": "Optional: relationship description" }, "type": "string", "required": false }
            ]
        },
        {
            "name": "query_memory_links",
            "description": { "zh": "查询记忆链接（可按 ID、源标题、目标标题、关系类型过滤）。", "en": "Query memory links (filter by id, source, target, or type)." },
            "parameters": [
                { "name": "target_owner_key", "description": { "zh": "目标记忆库 owner key，例如 character:<character-id> 或 shared:<shared-id>", "en": "Target memory owner key, such as character:<character-id> or shared:<shared-id>" }, "type": "string", "required": true },
                { "name": "link_id", "description": { "zh": "可选：链接ID", "en": "Optional: link id" }, "type": "number", "required": false },
                { "name": "source_title", "description": { "zh": "可选：源记忆标题", "en": "Optional: source memory title" }, "type": "string", "required": false },
                { "name": "target_title", "description": { "zh": "可选：目标记忆标题", "en": "Optional: target memory title" }, "type": "string", "required": false },
                { "name": "link_type", "description": { "zh": "可选：关系类型", "en": "Optional: relation type" }, "type": "string", "required": false },
                { "name": "limit", "description": { "zh": "可选：返回上限 1-200，默认20", "en": "Optional: limit 1-200, default 20" }, "type": "number", "required": false }
            ]
        },
        {
            "name": "update_memory_link",
            "description": { "zh": "更新记忆链接（按 link_id 或 source/target/link_type 定位）。", "en": "Update a memory link (by link_id or source/target/link_type)." },
            "parameters": [
                { "name": "target_owner_key", "description": { "zh": "目标记忆库 owner key，例如 character:<character-id> 或 shared:<shared-id>", "en": "Target memory owner key, such as character:<character-id> or shared:<shared-id>" }, "type": "string", "required": true },
                { "name": "link_id", "description": { "zh": "可选：链接ID", "en": "Optional: link ID" }, "type": "number", "required": false },
                { "name": "source_title", "description": { "zh": "可选：源记忆标题（未提供 link_id 时使用）", "en": "Optional: source title (used when link_id is not provided)" }, "type": "string", "required": false },
                { "name": "target_title", "description": { "zh": "可选：目标记忆标题（未提供 link_id 时使用）", "en": "Optional: target title (used when link_id is not provided)" }, "type": "string", "required": false },
                { "name": "link_type", "description": { "zh": "可选：当前关系类型（用于唯一定位）", "en": "Optional: current relation type (for unique resolution)" }, "type": "string", "required": false },
                { "name": "new_link_type", "description": { "zh": "可选：新的关系类型", "en": "Optional: new relation type" }, "type": "string", "required": false },
                { "name": "weight", "description": { "zh": "可选：新的强度 0-1", "en": "Optional: new weight 0-1" }, "type": "number", "required": false },
                { "name": "description", "description": { "zh": "可选：新的关系描述", "en": "Optional: new relationship description" }, "type": "string", "required": false }
            ]
        },
        {
            "name": "delete_memory_link",
            "description": { "zh": "删除记忆链接（按 link_id 或 source/target/link_type 定位）。", "en": "Delete a memory link (by link_id or source/target/link_type)." },
            "parameters": [
                { "name": "target_owner_key", "description": { "zh": "目标记忆库 owner key，例如 character:<character-id> 或 shared:<shared-id>", "en": "Target memory owner key, such as character:<character-id> or shared:<shared-id>" }, "type": "string", "required": true },
                { "name": "link_id", "description": { "zh": "可选：链接ID", "en": "Optional: link ID" }, "type": "number", "required": false },
                { "name": "source_title", "description": { "zh": "可选：源记忆标题（未提供 link_id 时使用）", "en": "Optional: source title (used when link_id is not provided)" }, "type": "string", "required": false },
                { "name": "target_title", "description": { "zh": "可选：目标记忆标题（未提供 link_id 时使用）", "en": "Optional: target title (used when link_id is not provided)" }, "type": "string", "required": false },
                { "name": "link_type", "description": { "zh": "可选：关系类型（用于唯一定位）", "en": "Optional: relation type (for unique resolution)" }, "type": "string", "required": false }
            ]
        },
        {
            "name": "update_user_preferences",
            "description": { "zh": "覆盖指定记忆库的 USER.md。", "en": "Overwrite USER.md for the specified memory owner." },
            "parameters": [
                { "name": "target_owner_key", "description": { "zh": "目标记忆库 owner key，例如 character:<character-id> 或 shared:<shared-id>", "en": "Target memory owner key, such as character:<character-id> or shared:<shared-id>" }, "type": "string", "required": true },
                { "name": "content", "description": { "zh": "新的 USER.md 内容", "en": "New USER.md content" }, "type": "string", "required": true }
            ]
        }
    ]
}*/

const ExtendedMemoryTools = (function () {
    interface ToolResponse<T = unknown> {
        success: boolean;
        message: string;
        data?: T;
    }

    type OwnerParams = {
        target_owner_key: string;
    };

    type CreateMemoryParams = OwnerParams & {
        title: string;
        content: string;
        content_type?: string;
        source?: string;
        folder_path?: string;
        tags?: string;
    };

    type UpdateMemoryParams = OwnerParams & {
        old_title: string;
        new_title?: string;
        content?: string;
        content_type?: string;
        source?: string;
        credibility?: number;
        importance?: number;
        folder_path?: string;
        tags?: string;
    };

    type DeleteMemoryParams = OwnerParams & {
        title: string;
    };

    type MoveMemoryParams = OwnerParams & {
        target_folder_path: string;
        titles?: string;
        source_folder_path?: string;
    };

    type LinkMemoriesParams = OwnerParams & {
        source_title: string;
        target_title: string;
        link_type?: string;
        weight?: number;
        description?: string;
    };

    type QueryMemoryLinksParams = OwnerParams & {
        link_id?: number;
        source_title?: string;
        target_title?: string;
        link_type?: string;
        limit?: number;
    };

    type UpdateMemoryLinkParams = QueryMemoryLinksParams & {
        new_link_type?: string;
        weight?: number;
        description?: string;
    };

    type DeleteMemoryLinkParams = QueryMemoryLinksParams;

    type UpdateUserPreferencesParams = OwnerParams & {
        content: string;
    };

    function requireText(value: unknown, name: string): string {
        const text = String(value ?? '').trim();
        if (!text) {
            throw new Error(`Missing parameter: ${name}`);
        }
        return text;
    }

    function parseTitles(value?: string): string[] | undefined {
        if (value === undefined) {
            return undefined;
        }
        const titles = value.split(/[,\n|]/).map((item) => item.trim()).filter((item) => item.length > 0);
        return titles.length > 0 ? titles : undefined;
    }

    async function create_memory_impl(params: CreateMemoryParams): Promise<ToolResponse<string>> {
        const result = await Tools.Memory.create({
            targetOwnerKey: requireText(params?.target_owner_key, 'target_owner_key'),
            title: requireText(params?.title, 'title'),
            content: requireText(params?.content, 'content'),
            contentType: params.content_type,
            source: params.source,
            folderPath: params.folder_path,
            tags: params.tags,
        });
        return { success: typeof result === 'string' && result.length > 0, message: '记忆创建完成', data: result };
    }

    async function update_memory_impl(params: UpdateMemoryParams): Promise<ToolResponse<string>> {
        const result = await Tools.Memory.update({
            targetOwnerKey: requireText(params?.target_owner_key, 'target_owner_key'),
            oldTitle: requireText(params?.old_title, 'old_title'),
            newTitle: params.new_title,
            content: params.content,
            contentType: params.content_type,
            source: params.source,
            credibility: params.credibility,
            importance: params.importance,
            folderPath: params.folder_path,
            tags: params.tags,
        });
        return { success: typeof result === 'string' && result.length > 0, message: '记忆更新完成', data: result };
    }

    async function delete_memory_impl(params: DeleteMemoryParams): Promise<ToolResponse<string>> {
        const result = await Tools.Memory.deleteMemory({
            targetOwnerKey: requireText(params?.target_owner_key, 'target_owner_key'),
            title: requireText(params?.title, 'title'),
        });
        return { success: typeof result === 'string' && result.length > 0, message: '记忆删除完成', data: result };
    }

    async function move_memory_impl(params: MoveMemoryParams): Promise<ToolResponse<string>> {
        const result = await Tools.Memory.move({
            targetOwnerKey: requireText(params?.target_owner_key, 'target_owner_key'),
            targetFolderPath: requireText(params?.target_folder_path, 'target_folder_path'),
            titles: parseTitles(params?.titles),
            sourceFolderPath: params.source_folder_path,
        });
        return { success: typeof result === 'string' && result.length > 0, message: '记忆移动完成', data: result };
    }

    async function link_memories_impl(params: LinkMemoriesParams): Promise<ToolResponse<unknown>> {
        const result = await Tools.Memory.link({
            targetOwnerKey: requireText(params?.target_owner_key, 'target_owner_key'),
            sourceTitle: requireText(params?.source_title, 'source_title'),
            targetTitle: requireText(params?.target_title, 'target_title'),
            linkType: params.link_type,
            weight: params.weight,
            description: params.description,
        });
        return { success: !!result, message: '记忆链接创建完成', data: result };
    }

    async function query_memory_links_impl(params: QueryMemoryLinksParams): Promise<ToolResponse<unknown>> {
        const result = await Tools.Memory.queryLinks({
            targetOwnerKey: requireText(params?.target_owner_key, 'target_owner_key'),
            linkId: params.link_id,
            sourceTitle: params.source_title,
            targetTitle: params.target_title,
            linkType: params.link_type,
            limit: params.limit,
        });
        return { success: !!result, message: '记忆链接查询完成', data: result };
    }

    async function update_memory_link_impl(params: UpdateMemoryLinkParams): Promise<ToolResponse<unknown>> {
        const result = await Tools.Memory.updateLink({
            targetOwnerKey: requireText(params?.target_owner_key, 'target_owner_key'),
            linkId: params.link_id,
            sourceTitle: params.source_title,
            targetTitle: params.target_title,
            linkType: params.link_type,
            newLinkType: params.new_link_type,
            weight: params.weight,
            description: params.description,
        });
        return { success: !!result, message: '记忆链接更新完成', data: result };
    }

    async function delete_memory_link_impl(params: DeleteMemoryLinkParams): Promise<ToolResponse<string>> {
        const result = await Tools.Memory.deleteLink({
            targetOwnerKey: requireText(params?.target_owner_key, 'target_owner_key'),
            linkId: params.link_id,
            sourceTitle: params.source_title,
            targetTitle: params.target_title,
            linkType: params.link_type,
        });
        return { success: typeof result === 'string' ? result.length > 0 : !!result, message: '记忆链接删除完成', data: result };
    }

    async function update_user_preferences_impl(params: UpdateUserPreferencesParams): Promise<ToolResponse<string>> {
        const result = await Tools.Memory.updateUserPreferences({
            targetOwnerKey: requireText(params?.target_owner_key, 'target_owner_key'),
            content: requireText(params?.content, 'content'),
        });
        return { success: typeof result === 'string' && result.length > 0, message: 'USER.md 更新完成', data: result };
    }

    async function wrapToolExecution<P, T>(func: (params: P) => Promise<ToolResponse<T>>, params: P): Promise<void> {
        try {
            const result = await func(params);
            complete(result);
        } catch (error: unknown) {
            const message = error instanceof Error ? error.message : String(error);
            console.error(`Tool ${func.name} failed unexpectedly`, error);
            complete({
                success: false,
                message: `工具执行失败: ${message}`,
            });
        }
    }

    async function main(): Promise<void> {
        complete({
            success: true,
            message: 'extended_memory_tools 工具包已加载',
            data: {
                tools: [
                    'create_memory',
                    'update_memory',
                    'delete_memory',
                    'move_memory',
                    'link_memories',
                    'query_memory_links',
                    'update_memory_link',
                    'delete_memory_link',
                    'update_user_preferences',
                ],
            },
        });
    }

    return {
        create_memory: (params: CreateMemoryParams) => wrapToolExecution(create_memory_impl, params),
        update_memory: (params: UpdateMemoryParams) => wrapToolExecution(update_memory_impl, params),
        delete_memory: (params: DeleteMemoryParams) => wrapToolExecution(delete_memory_impl, params),
        move_memory: (params: MoveMemoryParams) => wrapToolExecution(move_memory_impl, params),
        link_memories: (params: LinkMemoriesParams) => wrapToolExecution(link_memories_impl, params),
        query_memory_links: (params: QueryMemoryLinksParams) => wrapToolExecution(query_memory_links_impl, params),
        update_memory_link: (params: UpdateMemoryLinkParams) => wrapToolExecution(update_memory_link_impl, params),
        delete_memory_link: (params: DeleteMemoryLinkParams) => wrapToolExecution(delete_memory_link_impl, params),
        update_user_preferences: (params: UpdateUserPreferencesParams) => wrapToolExecution(update_user_preferences_impl, params),
        main,
    };
})();

exports.create_memory = ExtendedMemoryTools.create_memory;
exports.update_memory = ExtendedMemoryTools.update_memory;
exports.delete_memory = ExtendedMemoryTools.delete_memory;
exports.move_memory = ExtendedMemoryTools.move_memory;
exports.link_memories = ExtendedMemoryTools.link_memories;
exports.query_memory_links = ExtendedMemoryTools.query_memory_links;
exports.update_memory_link = ExtendedMemoryTools.update_memory_link;
exports.delete_memory_link = ExtendedMemoryTools.delete_memory_link;
exports.update_user_preferences = ExtendedMemoryTools.update_user_preferences;
exports.main = ExtendedMemoryTools.main;
