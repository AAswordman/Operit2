// Generated from operit-plugin-sdk Rust declarations.

import type { MemoryLinkQueryResultData, MemoryLinkResultData, MemoryQueryResultData } from "./results";

/**
 * Queries and mutates memories, folders, and relationships within owner-key stores.
 *
 * Memory owner keys use one of these exact forms:
 * - character:<character-card-id>
 * - shared:<shared-memory-id>
 */
export namespace Memory {
  /**
   * Supplies the existing title required by the object-style memory update overload.
   */
  export interface HostUpdateOptionsIntersection2 {
    /**
     * Identifies the memory entry before any title update is applied.
     */
    oldTitle: string;
  }

  /**
   * Combines update fields with the required title of the memory being changed.
   */
  export type HostUpdateOptionsIntersection = UpdateOptions & HostUpdateOptionsIntersection2;

  /**
   * Accepts either one memory title or a list of titles to move.
   */
  export type HostMoveTitles = string[] | string;

  /**
   * Accepts either one memory title or a list of titles in object-style move options.
   */
  export type MoveOptionsTitles = string[] | string;

  /**
   * Create a new memory in one memory owner.
   */
  function create(title: string, content: string, targetOwnerKey: string, contentType?: string, source?: string, folderPath?: string, tags?: string): Promise<string>;
  /**
   * Creates a memory from a structured owner-scoped request.
   */
  function create(options: CreateOptions): Promise<string>;
  /**
   * Delete an existing memory link inside one memory owner.
   */
  function deleteLink(targetOwnerKey: string, linkId?: number, sourceTitle?: string, targetTitle?: string, linkType?: string): Promise<string>;
  /**
   * Locates and removes a memory relationship using structured options.
   */
  function deleteLink(options: DeleteLinkOptions): Promise<string>;
  /**
   * Delete a memory from one memory owner.
   */
  function deleteMemory(title: string, targetOwnerKey: string): Promise<string>;
  /**
   * Deletes an exact-titled memory using a structured owner-scoped request.
   */
  function deleteMemory(options: DeleteOptions): Promise<string>;
  /**
   * Get a memory by exact title from one memory owner.
   */
  function getByTitle(title: string, targetOwnerKey: string, chunkIndex?: number, chunkRange?: string, query?: string, limit?: number): Promise<MemoryQueryResultData>;
  /**
   * Reads an exact-titled memory using structured chunk and query options.
   */
  function getByTitle(options: GetByTitleOptions): Promise<MemoryQueryResultData>;
  /**
   * Create a link between two memories inside one memory owner.
   */
  function link(sourceTitle: string, targetTitle: string, targetOwnerKey: string, linkType?: string, weight?: number, description?: string): Promise<MemoryLinkResultData>;
  /**
   * Creates a typed, weighted relationship between two memories from structured options.
   */
  function link(options: LinkOptions): Promise<MemoryLinkResultData>;
  /**
   * Move memories inside one memory owner.
   */
  function move(targetFolderPath: string, targetOwnerKey: string, titles?: HostMoveTitles, sourceFolderPath?: string): Promise<string>;
  /**
   * Moves selected memories between folders using structured options.
   */
  function move(options: MoveOptions): Promise<string>;
  /**
   * Query memory links inside one memory owner.
   */
  function queryLinks(targetOwnerKey: string, linkId?: number, sourceTitle?: string, targetTitle?: string, linkType?: string, limit?: number): Promise<MemoryLinkQueryResultData>;
  /**
   * Finds memory relationships using structured identity and type filters.
   */
  function queryLinks(options: QueryLinksOptions): Promise<MemoryLinkQueryResultData>;
  /**
   * Query memory. When targetOwnerKey is provided, only that owner is queried.
   * During agent execution, omitting targetOwnerKey queries the current role's readable memory owners.
   */
  function query(query: string, folderPath?: string, limit?: number, startTime?: string, endTime?: string, snapshotId?: string, threshold?: number, targetOwnerKey?: string): Promise<MemoryQueryResultData>;
  /**
   * Queries memory using a structured set of owner, time, folder, and relevance filters.
   */
  function query(options: QueryOptions): Promise<MemoryQueryResultData>;
  /**
   * Update an existing memory link inside one memory owner.
   */
  function updateLink(targetOwnerKey: string, linkId?: number, sourceTitle?: string, targetTitle?: string, linkType?: string, newLinkType?: string, weight?: number, description?: string): Promise<MemoryLinkQueryResultData>;
  /**
   * Locates and changes a memory relationship using structured options.
   */
  function updateLink(options: UpdateLinkOptions): Promise<MemoryLinkQueryResultData>;
  /**
   * Overwrite USER.md for one memory owner.
   */
  function updateUserPreferences(content: string, targetOwnerKey: string): Promise<string>;
  /**
   * Replaces the selected owner's user-preference document from structured options.
   */
  function updateUserPreferences(options: UserPreferencesOptions): Promise<string>;
  /**
   * Update an existing memory in one memory owner.
   */
  function update(oldTitle: string, targetOwnerKey: string, updates?: UpdateOptions): Promise<string>;
  /**
   * Updates a memory using a structured request that includes its current title.
   */
  function update(options: HostUpdateOptionsIntersection): Promise<string>;
  /**
   * Optionally restricts a memory query to one owner store.
   */
  export interface OwnerScopedOptions {
    /**
     * Identifies the character or shared-memory owner to query.
     */
    targetOwnerKey?: string;
  }

  /**
   * Selects the owner store affected by a memory mutation or exact lookup.
   */
  export interface RequiredOwnerScopedOptions {
    /**
     * Identifies the character or shared-memory owner whose store is accessed.
     */
    targetOwnerKey: string;
  }

  /**
   * Configures semantic memory retrieval by owner, folder, time range, and score.
   */
  export interface QueryOptions extends OwnerScopedOptions {
    /**
     * Contains the semantic query or search text.
     */
    query: string;
    /**
     * Restricts results to memories under this folder path.
     */
    folderPath?: string;
    /**
     * Limits the number of memory matches returned.
     */
    limit?: number;
    /**
     * Excludes memories created before this time boundary.
     */
    startTime?: string;
    /**
     * Excludes memories created after this time boundary.
     */
    endTime?: string;
    /**
     * Reuses a stable result snapshot for deterministic paging or follow-up queries.
     */
    snapshotId?: string;
    /**
     * Sets the minimum relevance score required for a match.
     */
    threshold?: number;
  }

  /**
   * Configures an exact-title lookup and optional chunk selection within one owner store.
   */
  export interface GetByTitleOptions extends RequiredOwnerScopedOptions {
    /**
     * Contains the exact memory title to retrieve.
     */
    title: string;
    /**
     * Selects one indexed chunk from chunked memory content.
     */
    chunkIndex?: number;
    /**
     * Selects a range of chunks from chunked memory content.
     */
    chunkRange?: string;
    /**
     * Prioritizes content relevant to this query within the selected memory.
     */
    query?: string;
    /**
     * Limits the number of matching chunks returned.
     */
    limit?: number;
  }

  /**
   * Supplies the content, organization, and provenance of a new memory.
   */
  export interface CreateOptions extends RequiredOwnerScopedOptions {
    /**
     * Sets the title used to identify the memory.
     */
    title: string;
    /**
     * Contains the information stored in the memory.
     */
    content: string;
    /**
     * Labels the format or semantic kind of the stored content.
     */
    contentType?: string;
    /**
     * Records where the stored information originated.
     */
    source?: string;
    /**
     * Places the memory under this organizational folder.
     */
    folderPath?: string;
    /**
     * Supplies tags used to categorize and retrieve the memory.
     */
    tags?: string;
  }

  /**
   * Describes the owner-scoped fields that may be changed on an existing memory.
   */
  export interface UpdateOptions extends RequiredOwnerScopedOptions {
    /**
     * Identifies the existing entry before an optional rename.
     */
    oldTitle?: string;
    /**
     * Replaces the memory title when present.
     */
    newTitle?: string;
    /**
     * Replaces the stored memory content when present.
     */
    content?: string;
    /**
     * Replaces the content format or semantic kind when present.
     */
    contentType?: string;
    /**
     * Replaces the recorded information source when present.
     */
    source?: string;
    /**
     * Sets the confidence assigned to the stored information.
     */
    credibility?: number;
    /**
     * Sets the retrieval importance assigned to the memory.
     */
    importance?: number;
    /**
     * Moves the memory to this organizational folder when present.
     */
    folderPath?: string;
    /**
     * Replaces the tags used to categorize the memory when present.
     */
    tags?: string;
  }

  /**
   * Supplies the complete user-preference document for one memory owner.
   */
  export interface UserPreferencesOptions extends RequiredOwnerScopedOptions {
    /**
     * Contains the complete replacement contents of `USER.md`.
     */
    content: string;
  }

  /**
   * Identifies an exact-titled memory to remove from one owner store.
   */
  export interface DeleteOptions extends RequiredOwnerScopedOptions {
    /**
     * Contains the exact title of the memory to delete.
     */
    title: string;
  }

  /**
   * Selects memories and the destination folder for an owner-scoped move.
   */
  export interface MoveOptions extends RequiredOwnerScopedOptions {
    /**
     * Sets the destination folder for the selected memories.
     */
    targetFolderPath: string;
    /**
     * Selects one or more exact-titled memories to move.
     */
    titles?: MoveOptionsTitles;
    /**
     * Restricts selection to memories currently under this folder.
     */
    sourceFolderPath?: string;
  }

  /**
   * Describes a new typed relationship between two memories in one owner store.
   */
  export interface LinkOptions extends RequiredOwnerScopedOptions {
    /**
     * Identifies the memory at the relationship's source.
     */
    sourceTitle: string;
    /**
     * Identifies the memory at the relationship's target.
     */
    targetTitle: string;
    /**
     * Labels the semantic relationship between the memories.
     */
    linkType?: string;
    /**
     * Sets the relative strength of the relationship.
     */
    weight?: number;
    /**
     * Explains the meaning or origin of the relationship.
     */
    description?: string;
  }

  /**
   * Filters relationships stored between memories owned by one memory store.
   */
  export interface QueryLinksOptions extends RequiredOwnerScopedOptions {
    /**
     * Selects one relationship by its stable identifier.
     */
    linkId?: number;
    /**
     * Restricts matches to relationships originating at this memory.
     */
    sourceTitle?: string;
    /**
     * Restricts matches to relationships ending at this memory.
     */
    targetTitle?: string;
    /**
     * Restricts matches to one semantic relationship type.
     */
    linkType?: string;
    /**
     * Limits the number of relationships returned.
     */
    limit?: number;
  }

  /**
   * Locates a memory relationship and supplies the values to change.
   */
  export interface UpdateLinkOptions extends RequiredOwnerScopedOptions {
    /**
     * Selects the relationship by its stable identifier when known.
     */
    linkId?: number;
    /**
     * Identifies the relationship source when locating it by endpoints.
     */
    sourceTitle?: string;
    /**
     * Identifies the relationship target when locating it by endpoints.
     */
    targetTitle?: string;
    /**
     * Restricts endpoint matching to the current relationship type.
     */
    linkType?: string;
    /**
     * Replaces the semantic relationship type when present.
     */
    newLinkType?: string;
    /**
     * Replaces the relationship strength when present.
     */
    weight?: number;
    /**
     * Replaces the relationship description when present.
     */
    description?: string;
  }

  /**
   * Locates a memory relationship to remove from one owner store.
   */
  export interface DeleteLinkOptions extends RequiredOwnerScopedOptions {
    /**
     * Selects the relationship by its stable identifier when known.
     */
    linkId?: number;
    /**
     * Identifies the relationship source when locating it by endpoints.
     */
    sourceTitle?: string;
    /**
     * Identifies the relationship target when locating it by endpoints.
     */
    targetTitle?: string;
    /**
     * Restricts endpoint matching to one semantic relationship type.
     */
    linkType?: string;
  }

}
