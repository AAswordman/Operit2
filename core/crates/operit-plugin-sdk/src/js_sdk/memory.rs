//! Owner-scoped memory storage, retrieval, organization, and relationship APIs.
use super::results::*;
use super::{JsDate, JsFuture};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Supplies the existing title required by the object-style memory update overload.
pub struct MemoryHostUpdateOptionsIntersection2 {
    /// Identifies the memory entry before any title update is applied.
    #[serde(rename = "oldTitle")]
    pub old_title: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Combines update fields with the required title of the memory being changed.
pub struct MemoryHostUpdateOptionsIntersection {
    /// Contains the owner scope and values to update.
    #[serde(flatten)]
    pub member_1: MemoryUpdateOptions,
    /// Contains the required current title used to locate the entry.
    #[serde(flatten)]
    pub member_2: MemoryHostUpdateOptionsIntersection2,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts either one memory title or a list of titles to move.
pub enum MemoryHostMoveTitles {
    Variant1(Vec<String>),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts either one memory title or a list of titles in object-style move options.
pub enum MemoryMoveOptionsTitles {
    Variant1(Vec<String>),
    Variant2(String),
}
/// Queries and mutates memories, folders, and relationships within owner-key stores.
///
///Memory owner keys use one of these exact forms:
///- character:<character-card-id>
///- shared:<shared-memory-id>
///
pub trait MemoryHost: Send + Sync {
    ///
    ///Query memory. When targetOwnerKey is provided, only that owner is queried.
    ///During agent execution, omitting targetOwnerKey queries the current role's readable memory owners.
    ///
    fn query_overload_1(
        &self,
        query: String,
        folderPath: Option<String>,
        limit: Option<f64>,
        startTime: Option<String>,
        endTime: Option<String>,
        snapshotId: Option<String>,
        threshold: Option<f64>,
        targetOwnerKey: Option<String>,
    ) -> JsFuture<MemoryQueryResultData>;
    /// Queries memory using a structured set of owner, time, folder, and relevance filters.
    fn query_overload_2(
        &self,
        options: MemoryQueryOptions,
    ) -> JsFuture<MemoryQueryResultData>;
    ///
    ///Get a memory by exact title from one memory owner.
    ///
    fn getByTitle_overload_1(
        &self,
        title: String,
        targetOwnerKey: String,
        chunkIndex: Option<f64>,
        chunkRange: Option<String>,
        query: Option<String>,
        limit: Option<f64>,
    ) -> JsFuture<MemoryQueryResultData>;
    /// Reads an exact-titled memory using structured chunk and query options.
    fn getByTitle_overload_2(
        &self,
        options: MemoryGetByTitleOptions,
    ) -> JsFuture<MemoryQueryResultData>;
    ///
    ///Create a new memory in one memory owner.
    ///
    fn create_overload_1(
        &self,
        title: String,
        content: String,
        targetOwnerKey: String,
        contentType: Option<String>,
        source: Option<String>,
        folderPath: Option<String>,
        tags: Option<String>,
    ) -> JsFuture<String>;
    /// Creates a memory from a structured owner-scoped request.
    fn create_overload_2(&self, options: MemoryCreateOptions) -> JsFuture<String>;
    ///
    ///Update an existing memory in one memory owner.
    ///
    fn update_overload_1(
        &self,
        oldTitle: String,
        targetOwnerKey: String,
        updates: Option<MemoryUpdateOptions>,
    ) -> JsFuture<String>;
    /// Updates a memory using a structured request that includes its current title.
    fn update_overload_2(
        &self,
        options: MemoryHostUpdateOptionsIntersection,
    ) -> JsFuture<String>;
    ///
    ///Overwrite USER.md for one memory owner.
    ///
    fn updateUserPreferences_overload_1(
        &self,
        content: String,
        targetOwnerKey: String,
    ) -> JsFuture<String>;
    /// Replaces the selected owner's user-preference document from structured options.
    fn updateUserPreferences_overload_2(
        &self,
        options: MemoryUserPreferencesOptions,
    ) -> JsFuture<String>;
    ///
    ///Delete a memory from one memory owner.
    ///
    fn deleteMemory_overload_1(
        &self,
        title: String,
        targetOwnerKey: String,
    ) -> JsFuture<String>;
    /// Deletes an exact-titled memory using a structured owner-scoped request.
    fn deleteMemory_overload_2(&self, options: MemoryDeleteOptions) -> JsFuture<String>;
    ///
    ///Move memories inside one memory owner.
    ///
    fn move_overload_1(
        &self,
        targetFolderPath: String,
        targetOwnerKey: String,
        titles: Option<MemoryHostMoveTitles>,
        sourceFolderPath: Option<String>,
    ) -> JsFuture<String>;
    /// Moves selected memories between folders using structured options.
    fn move_overload_2(&self, options: MemoryMoveOptions) -> JsFuture<String>;
    ///
    ///Create a link between two memories inside one memory owner.
    ///
    fn link_overload_1(
        &self,
        sourceTitle: String,
        targetTitle: String,
        targetOwnerKey: String,
        linkType: Option<String>,
        weight: Option<f64>,
        description: Option<String>,
    ) -> JsFuture<MemoryLinkResultData>;
    /// Creates a typed, weighted relationship between two memories from structured options.
    fn link_overload_2(
        &self,
        options: MemoryLinkOptions,
    ) -> JsFuture<MemoryLinkResultData>;
    ///
    ///Query memory links inside one memory owner.
    ///
    fn queryLinks_overload_1(
        &self,
        targetOwnerKey: String,
        linkId: Option<f64>,
        sourceTitle: Option<String>,
        targetTitle: Option<String>,
        linkType: Option<String>,
        limit: Option<f64>,
    ) -> JsFuture<MemoryLinkQueryResultData>;
    /// Finds memory relationships using structured identity and type filters.
    fn queryLinks_overload_2(
        &self,
        options: MemoryQueryLinksOptions,
    ) -> JsFuture<MemoryLinkQueryResultData>;
    ///
    ///Update an existing memory link inside one memory owner.
    ///
    fn updateLink_overload_1(
        &self,
        targetOwnerKey: String,
        linkId: Option<f64>,
        sourceTitle: Option<String>,
        targetTitle: Option<String>,
        linkType: Option<String>,
        newLinkType: Option<String>,
        weight: Option<f64>,
        description: Option<String>,
    ) -> JsFuture<MemoryLinkQueryResultData>;
    /// Locates and changes a memory relationship using structured options.
    fn updateLink_overload_2(
        &self,
        options: MemoryUpdateLinkOptions,
    ) -> JsFuture<MemoryLinkQueryResultData>;
    ///
    ///Delete an existing memory link inside one memory owner.
    ///
    fn deleteLink_overload_1(
        &self,
        targetOwnerKey: String,
        linkId: Option<f64>,
        sourceTitle: Option<String>,
        targetTitle: Option<String>,
        linkType: Option<String>,
    ) -> JsFuture<String>;
    /// Locates and removes a memory relationship using structured options.
    fn deleteLink_overload_2(
        &self,
        options: MemoryDeleteLinkOptions,
    ) -> JsFuture<String>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Optionally restricts a memory query to one owner store.
pub struct MemoryOwnerScopedOptions {
    /// Identifies the character or shared-memory owner to query.
    #[serde(rename = "targetOwnerKey")]
    pub target_owner_key: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects the owner store affected by a memory mutation or exact lookup.
pub struct MemoryRequiredOwnerScopedOptions {
    /// Identifies the character or shared-memory owner whose store is accessed.
    #[serde(rename = "targetOwnerKey")]
    pub target_owner_key: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures semantic memory retrieval by owner, folder, time range, and score.
pub struct MemoryQueryOptions {
    /// Optionally restricts the search to a specific memory owner.
    #[serde(flatten)]
    pub base_owner_scoped_options: MemoryOwnerScopedOptions,
    /// Contains the semantic query or search text.
    #[serde(rename = "query")]
    pub query: String,
    /// Restricts results to memories under this folder path.
    #[serde(rename = "folderPath")]
    pub folder_path: Option<String>,
    /// Limits the number of memory matches returned.
    #[serde(rename = "limit")]
    pub limit: Option<f64>,
    /// Excludes memories created before this time boundary.
    #[serde(rename = "startTime")]
    pub start_time: Option<String>,
    /// Excludes memories created after this time boundary.
    #[serde(rename = "endTime")]
    pub end_time: Option<String>,
    /// Reuses a stable result snapshot for deterministic paging or follow-up queries.
    #[serde(rename = "snapshotId")]
    pub snapshot_id: Option<String>,
    /// Sets the minimum relevance score required for a match.
    #[serde(rename = "threshold")]
    pub threshold: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures an exact-title lookup and optional chunk selection within one owner store.
pub struct MemoryGetByTitleOptions {
    /// Identifies the memory owner whose store is searched.
    #[serde(flatten)]
    pub base_required_owner_scoped_options: MemoryRequiredOwnerScopedOptions,
    /// Contains the exact memory title to retrieve.
    #[serde(rename = "title")]
    pub title: String,
    /// Selects one indexed chunk from chunked memory content.
    #[serde(rename = "chunkIndex")]
    pub chunk_index: Option<f64>,
    /// Selects a range of chunks from chunked memory content.
    #[serde(rename = "chunkRange")]
    pub chunk_range: Option<String>,
    /// Prioritizes content relevant to this query within the selected memory.
    #[serde(rename = "query")]
    pub query: Option<String>,
    /// Limits the number of matching chunks returned.
    #[serde(rename = "limit")]
    pub limit: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Supplies the content, organization, and provenance of a new memory.
pub struct MemoryCreateOptions {
    /// Identifies the memory owner that will receive the new entry.
    #[serde(flatten)]
    pub base_required_owner_scoped_options: MemoryRequiredOwnerScopedOptions,
    /// Sets the title used to identify the memory.
    #[serde(rename = "title")]
    pub title: String,
    /// Contains the information stored in the memory.
    #[serde(rename = "content")]
    pub content: String,
    /// Labels the format or semantic kind of the stored content.
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    /// Records where the stored information originated.
    #[serde(rename = "source")]
    pub source: Option<String>,
    /// Places the memory under this organizational folder.
    #[serde(rename = "folderPath")]
    pub folder_path: Option<String>,
    /// Supplies tags used to categorize and retrieve the memory.
    #[serde(rename = "tags")]
    pub tags: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Describes the owner-scoped fields that may be changed on an existing memory.
pub struct MemoryUpdateOptions {
    /// Identifies the memory owner whose entry is changed.
    #[serde(flatten)]
    pub base_required_owner_scoped_options: MemoryRequiredOwnerScopedOptions,
    /// Identifies the existing entry before an optional rename.
    #[serde(rename = "oldTitle")]
    pub old_title: Option<String>,
    /// Replaces the memory title when present.
    #[serde(rename = "newTitle")]
    pub new_title: Option<String>,
    /// Replaces the stored memory content when present.
    #[serde(rename = "content")]
    pub content: Option<String>,
    /// Replaces the content format or semantic kind when present.
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    /// Replaces the recorded information source when present.
    #[serde(rename = "source")]
    pub source: Option<String>,
    /// Sets the confidence assigned to the stored information.
    #[serde(rename = "credibility")]
    pub credibility: Option<f64>,
    /// Sets the retrieval importance assigned to the memory.
    #[serde(rename = "importance")]
    pub importance: Option<f64>,
    /// Moves the memory to this organizational folder when present.
    #[serde(rename = "folderPath")]
    pub folder_path: Option<String>,
    /// Replaces the tags used to categorize the memory when present.
    #[serde(rename = "tags")]
    pub tags: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Supplies the complete user-preference document for one memory owner.
pub struct MemoryUserPreferencesOptions {
    /// Identifies the owner whose `USER.md` document is replaced.
    #[serde(flatten)]
    pub base_required_owner_scoped_options: MemoryRequiredOwnerScopedOptions,
    /// Contains the complete replacement contents of `USER.md`.
    #[serde(rename = "content")]
    pub content: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies an exact-titled memory to remove from one owner store.
pub struct MemoryDeleteOptions {
    /// Identifies the memory owner whose entry is removed.
    #[serde(flatten)]
    pub base_required_owner_scoped_options: MemoryRequiredOwnerScopedOptions,
    /// Contains the exact title of the memory to delete.
    #[serde(rename = "title")]
    pub title: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects memories and the destination folder for an owner-scoped move.
pub struct MemoryMoveOptions {
    /// Identifies the memory owner whose folder structure is changed.
    #[serde(flatten)]
    pub base_required_owner_scoped_options: MemoryRequiredOwnerScopedOptions,
    /// Sets the destination folder for the selected memories.
    #[serde(rename = "targetFolderPath")]
    pub target_folder_path: String,
    /// Selects one or more exact-titled memories to move.
    #[serde(rename = "titles")]
    pub titles: Option<MemoryMoveOptionsTitles>,
    /// Restricts selection to memories currently under this folder.
    #[serde(rename = "sourceFolderPath")]
    pub source_folder_path: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Describes a new typed relationship between two memories in one owner store.
pub struct MemoryLinkOptions {
    /// Identifies the memory owner that contains both linked entries.
    #[serde(flatten)]
    pub base_required_owner_scoped_options: MemoryRequiredOwnerScopedOptions,
    /// Identifies the memory at the relationship's source.
    #[serde(rename = "sourceTitle")]
    pub source_title: String,
    /// Identifies the memory at the relationship's target.
    #[serde(rename = "targetTitle")]
    pub target_title: String,
    /// Labels the semantic relationship between the memories.
    #[serde(rename = "linkType")]
    pub link_type: Option<String>,
    /// Sets the relative strength of the relationship.
    #[serde(rename = "weight")]
    pub weight: Option<f64>,
    /// Explains the meaning or origin of the relationship.
    #[serde(rename = "description")]
    pub description: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Filters relationships stored between memories owned by one memory store.
pub struct MemoryQueryLinksOptions {
    /// Identifies the memory owner whose relationships are searched.
    #[serde(flatten)]
    pub base_required_owner_scoped_options: MemoryRequiredOwnerScopedOptions,
    /// Selects one relationship by its stable identifier.
    #[serde(rename = "linkId")]
    pub link_id: Option<f64>,
    /// Restricts matches to relationships originating at this memory.
    #[serde(rename = "sourceTitle")]
    pub source_title: Option<String>,
    /// Restricts matches to relationships ending at this memory.
    #[serde(rename = "targetTitle")]
    pub target_title: Option<String>,
    /// Restricts matches to one semantic relationship type.
    #[serde(rename = "linkType")]
    pub link_type: Option<String>,
    /// Limits the number of relationships returned.
    #[serde(rename = "limit")]
    pub limit: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Locates a memory relationship and supplies the values to change.
pub struct MemoryUpdateLinkOptions {
    /// Identifies the memory owner whose relationship is changed.
    #[serde(flatten)]
    pub base_required_owner_scoped_options: MemoryRequiredOwnerScopedOptions,
    /// Selects the relationship by its stable identifier when known.
    #[serde(rename = "linkId")]
    pub link_id: Option<f64>,
    /// Identifies the relationship source when locating it by endpoints.
    #[serde(rename = "sourceTitle")]
    pub source_title: Option<String>,
    /// Identifies the relationship target when locating it by endpoints.
    #[serde(rename = "targetTitle")]
    pub target_title: Option<String>,
    /// Restricts endpoint matching to the current relationship type.
    #[serde(rename = "linkType")]
    pub link_type: Option<String>,
    /// Replaces the semantic relationship type when present.
    #[serde(rename = "newLinkType")]
    pub new_link_type: Option<String>,
    /// Replaces the relationship strength when present.
    #[serde(rename = "weight")]
    pub weight: Option<f64>,
    /// Replaces the relationship description when present.
    #[serde(rename = "description")]
    pub description: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Locates a memory relationship to remove from one owner store.
pub struct MemoryDeleteLinkOptions {
    /// Identifies the memory owner whose relationship is removed.
    #[serde(flatten)]
    pub base_required_owner_scoped_options: MemoryRequiredOwnerScopedOptions,
    /// Selects the relationship by its stable identifier when known.
    #[serde(rename = "linkId")]
    pub link_id: Option<f64>,
    /// Identifies the relationship source when locating it by endpoints.
    #[serde(rename = "sourceTitle")]
    pub source_title: Option<String>,
    /// Identifies the relationship target when locating it by endpoints.
    #[serde(rename = "targetTitle")]
    pub target_title: Option<String>,
    /// Restricts endpoint matching to one semantic relationship type.
    #[serde(rename = "linkType")]
    pub link_type: Option<String>,
}
