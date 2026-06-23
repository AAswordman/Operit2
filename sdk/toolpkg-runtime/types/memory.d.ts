/**
 * Memory management namespace
 * Provides methods for managing owner-key memory stores.
 *
 * Memory owner keys use one of these exact forms:
 * - character:<character-card-id>
 * - shared:<shared-memory-id>
 */

export namespace Memory {
    interface OwnerScopedOptions {
        targetOwnerKey?: string;
    }

    interface RequiredOwnerScopedOptions {
        targetOwnerKey: string;
    }

    interface QueryOptions extends OwnerScopedOptions {
        query: string;
        folderPath?: string;
        limit?: number;
        startTime?: string;
        endTime?: string;
        snapshotId?: string;
        threshold?: number;
    }

    interface GetByTitleOptions extends RequiredOwnerScopedOptions {
        title: string;
        chunkIndex?: number;
        chunkRange?: string;
        query?: string;
        limit?: number;
    }

    interface CreateOptions extends RequiredOwnerScopedOptions {
        title: string;
        content: string;
        contentType?: string;
        source?: string;
        folderPath?: string;
        tags?: string;
    }

    /**
     * Query memory. When targetOwnerKey is provided, only that owner is queried.
     * During agent execution, omitting targetOwnerKey queries the current role's readable memory owners.
     */
    function query(
        query: string,
        folderPath?: string,
        limit?: number,
        startTime?: string,
        endTime?: string,
        snapshotId?: string,
        threshold?: number,
        targetOwnerKey?: string
    ): Promise<import('./results').MemoryQueryResultData>;
    function query(options: QueryOptions): Promise<import('./results').MemoryQueryResultData>;

    /**
     * Get a memory by exact title from one memory owner.
     */
    function getByTitle(title: string, targetOwnerKey: string, chunkIndex?: number, chunkRange?: string, query?: string, limit?: number): Promise<import('./results').MemoryQueryResultData>;
    function getByTitle(options: GetByTitleOptions): Promise<import('./results').MemoryQueryResultData>;

    /**
     * Create a new memory in one memory owner.
     */
    function create(title: string, content: string, targetOwnerKey: string, contentType?: string, source?: string, folderPath?: string, tags?: string): Promise<string>;
    function create(options: CreateOptions): Promise<string>;

    interface UpdateOptions extends RequiredOwnerScopedOptions {
        oldTitle?: string;
        newTitle?: string;
        content?: string;
        contentType?: string;
        source?: string;
        credibility?: number;
        importance?: number;
        folderPath?: string;
        tags?: string;
    }

    /**
     * Update an existing memory in one memory owner.
     */
    function update(oldTitle: string, targetOwnerKey: string, updates?: Omit<UpdateOptions, 'oldTitle' | 'targetOwnerKey'>): Promise<string>;
    function update(options: UpdateOptions & { oldTitle: string }): Promise<string>;

    interface UserPreferencesOptions extends RequiredOwnerScopedOptions {
        content: string;
    }

    /**
     * Overwrite USER.md for one memory owner.
     */
    function updateUserPreferences(content: string, targetOwnerKey: string): Promise<string>;
    function updateUserPreferences(options: UserPreferencesOptions): Promise<string>;

    interface DeleteOptions extends RequiredOwnerScopedOptions {
        title: string;
    }

    /**
     * Delete a memory from one memory owner.
     */
    function deleteMemory(title: string, targetOwnerKey: string): Promise<string>;
    function deleteMemory(options: DeleteOptions): Promise<string>;

    interface MoveOptions extends RequiredOwnerScopedOptions {
        targetFolderPath: string;
        titles?: string[] | string;
        sourceFolderPath?: string;
    }

    /**
     * Move memories inside one memory owner.
     */
    function move(targetFolderPath: string, targetOwnerKey: string, titles?: string[] | string, sourceFolderPath?: string): Promise<string>;
    function move(options: MoveOptions): Promise<string>;

    interface LinkOptions extends RequiredOwnerScopedOptions {
        sourceTitle: string;
        targetTitle: string;
        linkType?: string;
        weight?: number;
        description?: string;
    }

    /**
     * Create a link between two memories inside one memory owner.
     */
    function link(sourceTitle: string, targetTitle: string, targetOwnerKey: string, linkType?: string, weight?: number, description?: string): Promise<import('./results').MemoryLinkResultData>;
    function link(options: LinkOptions): Promise<import('./results').MemoryLinkResultData>;

    interface QueryLinksOptions extends RequiredOwnerScopedOptions {
        linkId?: number;
        sourceTitle?: string;
        targetTitle?: string;
        linkType?: string;
        limit?: number;
    }

    /**
     * Query memory links inside one memory owner.
     */
    function queryLinks(
        targetOwnerKey: string,
        linkId?: number,
        sourceTitle?: string,
        targetTitle?: string,
        linkType?: string,
        limit?: number
    ): Promise<import('./results').MemoryLinkQueryResultData>;
    function queryLinks(options: QueryLinksOptions): Promise<import('./results').MemoryLinkQueryResultData>;

    interface UpdateLinkOptions extends RequiredOwnerScopedOptions {
        linkId?: number;
        sourceTitle?: string;
        targetTitle?: string;
        linkType?: string;
        newLinkType?: string;
        weight?: number;
        description?: string;
    }

    /**
     * Update an existing memory link inside one memory owner.
     */
    function updateLink(
        targetOwnerKey: string,
        linkId?: number,
        sourceTitle?: string,
        targetTitle?: string,
        linkType?: string,
        newLinkType?: string,
        weight?: number,
        description?: string
    ): Promise<import('./results').MemoryLinkQueryResultData>;
    function updateLink(options: UpdateLinkOptions): Promise<import('./results').MemoryLinkQueryResultData>;

    interface DeleteLinkOptions extends RequiredOwnerScopedOptions {
        linkId?: number;
        sourceTitle?: string;
        targetTitle?: string;
        linkType?: string;
    }

    /**
     * Delete an existing memory link inside one memory owner.
     */
    function deleteLink(targetOwnerKey: string, linkId?: number, sourceTitle?: string, targetTitle?: string, linkType?: string): Promise<string>;
    function deleteLink(options: DeleteLinkOptions): Promise<string>;
}
