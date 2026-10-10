/**
 * Converts legacy memory overloads to the role provider's executable tools.
 * @param {(name: string, params: Record<string, unknown>) => Promise<unknown>} invoke
 * @returns {typeof import('../../../../../../../plugins/types-v1/memory').Memory}
 */
function __operitCreateV1Memory(invoke) {
    /** @type {Record<string, string>} */
    var fields = { callerCardId: 'caller_card_id', folderPath: 'folder_path', startTime: 'start_time', endTime: 'end_time',
        snapshotId: 'snapshot_id', chunkIndex: 'chunk_index', chunkRange: 'chunk_range', contentType: 'content_type',
        oldTitle: 'old_title', newTitle: 'new_title', targetFolderPath: 'target_folder_path', sourceFolderPath: 'source_folder_path',
        sourceTitle: 'source_title', targetTitle: 'target_title', linkType: 'link_type', newLinkType: 'new_link_type', linkId: 'link_id' };
    /** Converts one exact overload, omitting absent fields rather than overwriting provider defaults.
     * @param {string[]} names
     * @param {ArrayLike<unknown>} args
     * @returns {Record<string, unknown>}
     */
    function options(names, args) {
        /** @type {Record<string, unknown>} */
        var input = args[0] !== null && typeof args[0] === 'object' ? Object.fromEntries(Object.entries(args[0])) : Object.fromEntries(names.map(
            /** Copies each documented positional argument into its explicit option name. */
            (key, index) => [key, args[index]],
        ));
        /** @type {Record<string, unknown>} */
        var output = {};
        for (var key of Object.keys(input)) {
            if (names.indexOf(key) === -1) throw new Error('Unknown legacy memory option: ' + key);
            var value = input[key];
            if (value === undefined) continue;
            if (key === 'linkId') {
                if (typeof value !== 'number' || !Number.isSafeInteger(value) || value <= 0) throw new Error('Legacy memory linkId must be a positive safe integer');
                output.link_id = String(value);
                continue;
            }
            if (key === 'titles' && Array.isArray(value)) {
                if (value.some(
                    /** Rejects title spellings that the executable tool delimiters cannot preserve. */
                    item => typeof item !== 'string' || /[,\n|]/.test(item),
                )) throw new Error('Legacy title arrays must be representable by the memory tool delimiters');
                output.titles = value.join('\n');
            } else output[Object.prototype.hasOwnProperty.call(fields, key) ? fields[key] : key] = input[key];
        }
        return output;
    }
    /** Requires the declared textual result from memory mutation tools.
     * @param {unknown} value
     * @returns {string}
     */
    function text(value) {
        if (typeof value === 'string') return value;
        if (value !== null && typeof value === 'object') return JSON.stringify(value);
        throw new Error('Invalid role-provider memory result');
    }
    /** Validates the exact link fields shared by both contracts.
     * @param {unknown} value
     * @returns {import('../../../../../../../plugins/types-v1/results').MemoryLinkResultData}
     */
    function link(value) {
        if (value === null || typeof value !== 'object' || !('sourceTitle' in value) || !('targetTitle' in value) || !('linkType' in value) || !('weight' in value) || !('description' in value) ||
            typeof value.sourceTitle !== 'string' || typeof value.targetTitle !== 'string' || typeof value.linkType !== 'string' || typeof value.weight !== 'number' || typeof value.description !== 'string') throw new Error('Invalid memory link result');
        return { sourceTitle: value.sourceTitle, targetTitle: value.targetTitle, linkType: value.linkType, weight: value.weight, description: value.description };
    }
    /** Validates and projects one structured memory query from the actual provider result.
     * @param {unknown} value
     * @returns {import('../../../../../../../plugins/types-v1/results').MemoryQueryResultData}
     */
    function queryResult(value) {
        if (value === null || typeof value !== 'object' || !('memories' in value) || !Array.isArray(value.memories) ||
            !('snapshotId' in value) || (value.snapshotId !== null && typeof value.snapshotId !== 'string') ||
            !('snapshotCreated' in value) || typeof value.snapshotCreated !== 'boolean' ||
            !('excludedBySnapshotCount' in value) || typeof value.excludedBySnapshotCount !== 'number') throw new Error('Invalid memory query result');
        /** Validates one provider match against the legacy structured record.
         * @param {unknown} item
         * @returns {import('../../../../../../../plugins/types-v1/results').MemoryQueryResultMemoryInfo}
         */
        function memory(item) {
            if (item === null || typeof item !== 'object' || !('title' in item) || typeof item.title !== 'string' ||
                !('content' in item) || typeof item.content !== 'string' || !('source' in item) || typeof item.source !== 'string' ||
                !('createdAt' in item) || typeof item.createdAt !== 'string' || !('tags' in item) || !Array.isArray(item.tags) || item.tags.some(
                    /** Requires actual tag names rather than coercing provider data. */
                    tag => typeof tag !== 'string',
                ) ||
                !('chunkInfo' in item) || (item.chunkInfo !== null && typeof item.chunkInfo !== 'string') ||
                !('chunkIndices' in item) || (item.chunkIndices !== null && (!Array.isArray(item.chunkIndices) || item.chunkIndices.some(
                    /** Preserves actual document indices without a numeric coercion. */
                    index => !Number.isSafeInteger(index),
                )))) throw new Error('Invalid memory query match');
            return { title: item.title, content: item.content, source: item.source, tags: item.tags.slice(), createdAt: item.createdAt,
                chunkInfo: item.chunkInfo, chunkIndices: item.chunkIndices === null ? null : item.chunkIndices.slice() };
        }
        return { memories: value.memories.map(memory), snapshotId: value.snapshotId, snapshotCreated: value.snapshotCreated, excludedBySnapshotCount: value.excludedBySnapshotCount };
    }
    return {
        /** Queries the persisted participant-bound memory store.
         * @param {...unknown} args
         */
        async query(...args) {
            var params = options(['query', 'folderPath', 'limit', 'startTime', 'endTime', 'snapshotId', 'threshold', 'callerCardId'], args);
            if (params.limit === undefined) params.limit = 20;
            return queryResult(await invoke('query_memory', params));
        },
        /** Reads text or exact document query data.
         * @param {...unknown} args
         */
        async getByTitle(...args) {
            var result = await invoke('get_memory_by_title', options(['title', 'chunkIndex', 'chunkRange', 'query', 'limit', 'callerCardId'], args));
            return typeof result === 'string' ? result : JSON.stringify(queryResult(result));
        },
        /** Creates a real persisted memory.
         * @param {...unknown} args
         */
        async create(...args) { return text(await invoke('create_memory', options(['title', 'content', 'contentType', 'source', 'folderPath', 'tags', 'callerCardId'], args))); },
        /** Merges only the documented positional update envelope into a fresh object.
         * @param {...unknown} args
         */
        async update(...args) {
            var input = typeof args[0] === 'object' ? options(['oldTitle', 'newTitle', 'content', 'contentType', 'source', 'credibility', 'importance', 'folderPath', 'tags', 'callerCardId'], args) : options(['oldTitle', 'newTitle', 'content', 'contentType', 'source', 'credibility', 'importance', 'folderPath', 'tags', 'callerCardId'], [Object.assign({}, args[1], { oldTitle: args[0], ...(args[2] === undefined ? {} : { callerCardId: args[2] }) })]);
            return text(await invoke('update_memory', input));
        },
        /** Deletes one exact persisted memory.
         * @param {...unknown} args
         */
        async deleteMemory(...args) { return text(await invoke('delete_memory', options(['title', 'callerCardId'], args))); },
        /** Moves memories between actual folders.
         * @param {...unknown} args
         */
        async move(...args) { return text(await invoke('move_memory', options(['targetFolderPath', 'titles', 'sourceFolderPath', 'callerCardId'], args))); },
        /** Creates a persisted relationship and validates its result.
         * @param {...unknown} args
         */
        async link(...args) { return link(await invoke('link_memories', options(['sourceTitle', 'targetTitle', 'linkType', 'weight', 'description', 'callerCardId'], args))); },
        /** Converts lossless decimal relationship identities to the legacy safe integer contract.
         * @param {...unknown} args
         */
        async queryLinks(...args) {
            var result = await invoke('query_memory_links', options(['linkId', 'sourceTitle', 'targetTitle', 'linkType', 'limit', 'callerCardId'], args));
            if (result === null || typeof result !== 'object' || !('links' in result) || !Array.isArray(result.links) ||
                !('totalCount' in result) || typeof result.totalCount !== 'number') throw new Error('Invalid memory links result');
            return { totalCount: result.totalCount, links: result.links.map(
                /** Rejects lossy numeric conversion instead of changing the persisted relationship identity. */
                item => {
                    if (item === null || typeof item !== 'object' || typeof item.linkId !== 'string' || !/^[1-9][0-9]*$/.test(item.linkId)) throw new Error('Invalid memory relationship identity');
                    var id = Number(item.linkId);
                    if (!Number.isSafeInteger(id) || String(id) !== item.linkId) throw new Error('Memory linkId exceeds the ToolPkg v1 safe integer range');
                    return Object.assign(link(item), { linkId: id });
                },
            ) };
        },
        /** Updates the unique title-selected relationship and preserves the legacy single-link envelope.
         * @param {...unknown} args
         */
        async updateLink(...args) {
            var result = await invoke('update_memory_link', options(['linkId', 'sourceTitle', 'targetTitle', 'linkType', 'newLinkType', 'weight', 'description', 'callerCardId'], args));
            if (result === null || typeof result !== 'object' || !('links' in result) || !Array.isArray(result.links) || result.links.length !== 1) throw new Error('ToolPkg v1 updateLink requires exactly one updated relationship');
            return link(result.links[0]);
        },
        /** Deletes the unique title-selected relationship.
         * @param {...unknown} args
         */
        async deleteLink(...args) { return text(await invoke('delete_memory_link', options(['linkId', 'sourceTitle', 'targetTitle', 'linkType', 'callerCardId'], args))); },
    };
}
