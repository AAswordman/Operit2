use std::collections::BTreeMap;
use std::sync::Arc;

use operit_host_api::{HttpHost, HttpRequestData, HttpResponseData};
use serde::{Deserialize, Serialize};
use url::Url;

const MARKET_V2_BASE_URL: &str = "https://api.operit.app/market/v2";
const MARKET_V2_STATIC_URL: &str = "https://static.operit.app/market/v2";
const GITHUB_API_BASE_URL: &str = "https://api.github.com";
const USER_AGENT: &str = "Operit-Market-Stats";
const TIMEOUT_SECONDS: u64 = 15;

// ---- v2 Public Models ----

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Summary metadata for one marketplace entry.
pub struct MarketEntrySummary {
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub author: Option<MarketAuthor>,
    #[serde(default)]
    pub publisher: Option<MarketAuthor>,
    #[serde(default)]
    pub contributors: Vec<MarketAuthor>,
    #[serde(rename = "categoryId", default)]
    pub category_id: Option<String>,
    #[serde(rename = "stateCode", default)]
    pub state_code: String,
    #[serde(rename = "allowPublicUpdates", default)]
    pub allow_public_updates: bool,
    #[serde(default)]
    pub featured: bool,
    #[serde(default)]
    pub downloads: i32,
    #[serde(rename = "downloadCount", default)]
    pub download_count: i32,
    #[serde(rename = "createdAt", default)]
    pub created_at: String,
    #[serde(rename = "updatedAt", default)]
    pub updated_at: String,
    #[serde(rename = "publishedAt", default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub source: Option<MarketSource>,
    #[serde(rename = "repoVersion", default)]
    pub repo_version: Option<MarketRepoVersion>,
    #[serde(default)]
    pub artifact: Option<MarketEntryArtifact>,
    #[serde(default)]
    pub assets: Vec<MarketEntryAsset>,
    #[serde(default)]
    pub versions: Vec<MarketEntryVersion>,
    #[serde(rename = "latestVersion", default)]
    pub latest_version: Option<MarketEntryVersion>,
    #[serde(default)]
    pub reactions: Vec<MarketReactionCount>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Source repository metadata for a marketplace entry.
pub struct MarketSource {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Repository version metadata for installable marketplace content.
pub struct MarketRepoVersion {
    #[serde(rename = "refType", default)]
    pub ref_type: String,
    #[serde(rename = "refName", default)]
    pub ref_name: String,
    #[serde(rename = "installConfig", default)]
    pub install_config: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Artifact metadata attached to a marketplace entry version.
pub struct MarketEntryArtifact {
    #[serde(rename = "projectId", default)]
    pub project_id: String,
    #[serde(rename = "runtimePackageId", default)]
    pub runtime_package_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Downloadable asset metadata for a marketplace entry.
pub struct MarketEntryAsset {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "versionId", default)]
    pub version_id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(rename = "assetName", default)]
    pub asset_name: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Version metadata published for a marketplace entry.
pub struct MarketEntryVersion {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub version: String,
    #[serde(rename = "formatVer", default)]
    pub format_ver: String,
    #[serde(rename = "minAppVer", default)]
    pub min_app_ver: String,
    #[serde(rename = "maxAppVer", default)]
    pub max_app_ver: Option<String>,
    #[serde(default)]
    pub changelog: Option<String>,
    #[serde(rename = "projectId", default)]
    pub project_id: Option<String>,
    #[serde(rename = "runtimePackageId", default)]
    pub runtime_package_id: Option<String>,
    #[serde(rename = "installConfig", default)]
    pub install_config: Option<String>,
    #[serde(default)]
    pub publisher: Option<MarketAuthor>,
    #[serde(rename = "publishedAt", default)]
    pub published_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Reaction counts displayed for a marketplace entry.
pub struct MarketReactionCount {
    #[serde(default)]
    pub reaction: String,
    #[serde(default)]
    pub total: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Paginated marketplace entry list response.
pub struct MarketListPage {
    #[serde(rename = "generatedAt", default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub sort: String,
    #[serde(default)]
    pub page: i32,
    #[serde(rename = "pageSize", default)]
    pub page_size: i32,
    #[serde(default)]
    pub total: i32,
    #[serde(default)]
    pub items: Vec<MarketEntrySummary>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Sharded entry lookup response keyed by marketplace id.
pub struct MarketEntriesShard {
    #[serde(rename = "generatedAt", default)]
    pub generated_at: Option<String>,
    #[serde(rename = "entriesById", default)]
    pub entries_by_id: BTreeMap<String, MarketEntrySummary>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Marketplace comment with author and reaction metadata.
pub struct MarketComment {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "entryId", default)]
    pub entry_id: String,
    #[serde(rename = "parentId", default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub author: MarketAuthor,
    #[serde(default)]
    pub body: String,
    #[serde(rename = "createdAt", default)]
    pub created_at: String,
    #[serde(rename = "updatedAt", default)]
    pub updated_at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Author profile metadata attached to marketplace comments.
pub struct MarketAuthor {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "githubId", default)]
    pub github_id: i64,
    #[serde(default)]
    pub login: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Paginated marketplace comment response.
pub struct MarketCommentPage {
    #[serde(default)]
    pub ok: bool,
    #[serde(rename = "entryId", default)]
    pub entry_id: String,
    #[serde(default)]
    pub page: i32,
    #[serde(rename = "pageSize", default)]
    pub page_size: i32,
    #[serde(default)]
    pub total: i32,
    #[serde(default)]
    pub items: Vec<MarketComment>,
    #[serde(rename = "generatedAt", default)]
    pub generated_at: Option<String>,
}

// ---- Session & Auth ----

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct MarketSessionResponse {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    session: String,
    #[serde(rename = "githubId", default)]
    github_id: i64,
    #[serde(default)]
    login: String,
    #[serde(rename = "avatarUrl", default)]
    avatar_url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Authenticated marketplace session metadata.
pub struct MarketAuthInfo {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub session: String,
    #[serde(rename = "githubId", default)]
    pub github_id: i64,
    #[serde(default)]
    pub login: String,
    #[serde(rename = "avatarUrl", default)]
    pub avatar_url: String,
}

// ---- Notifications ----

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Notification item returned by the marketplace API.
pub struct MarketNotification {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(rename = "entryId", default)]
    pub entry_id: Option<String>,
    #[serde(rename = "commentId", default)]
    pub comment_id: Option<String>,
    #[serde(rename = "actorId", default)]
    pub actor_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(rename = "createdAt", default)]
    pub created_at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Paginated notification response returned by the marketplace API.
pub struct MarketNotificationsResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub items: Vec<MarketNotification>,
}

// ---- My Entries ----

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Marketplace entry summary shown to the publishing user.
pub struct MarketPublisherEntrySummary {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub relation: String,
    #[serde(rename = "stateCode", default)]
    pub state_code: String,
    #[serde(rename = "categoryId", default)]
    pub category_id: Option<String>,
    #[serde(rename = "updatedAt", default)]
    pub updated_at: String,
    #[serde(rename = "reasonCodes", default)]
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Authenticated response containing entries owned by the user.
pub struct MarketMyEntriesResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub entries: Vec<MarketPublisherEntrySummary>,
    #[serde(rename = "generatedAt", default)]
    pub generated_at: Option<String>,
}

// ---- Publish ----

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Response returned after publishing an entry or version.
pub struct MarketPublishResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(rename = "entryId", default)]
    pub entry_id: String,
    #[serde(rename = "versionId", default)]
    pub version_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Response returned after updating a marketplace entry.
pub struct MarketEntryUpdateResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub item: MarketEntryUpdateItem,
    #[serde(default)]
    pub stats: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Updated marketplace entry item.
pub struct MarketEntryUpdateItem {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "stateCode", default)]
    pub state_code: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Paginated marketplace version response.
pub struct MarketVersionPage {
    #[serde(default)]
    pub ok: bool,
    #[serde(rename = "entryId", default)]
    pub entry_id: String,
    #[serde(rename = "generatedAt", default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub items: Vec<MarketEntryVersion>,
}

// ---- Manifest ----

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// Static marketplace manifest containing taxonomy and state metadata.
pub struct MarketManifest {
    #[serde(default)]
    pub ok: bool,
    #[serde(rename = "marketVersion", default)]
    pub market_version: i32,
    #[serde(rename = "generatedAt", default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub types: Vec<MarketTypeInfo>,
    #[serde(rename = "formatVersions", default)]
    pub format_versions: Vec<MarketFormatInfo>,
    #[serde(default)]
    pub categories: Vec<MarketCategoryInfo>,
    #[serde(default)]
    pub states: Vec<MarketStateInfo>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Marketplace type metadata.
pub struct MarketTypeInfo {
    #[serde(default, alias = "slug")]
    pub id: String,
    #[serde(default, alias = "label")]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Marketplace format metadata.
pub struct MarketFormatInfo {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub version: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Marketplace category metadata.
pub struct MarketCategoryInfo {
    #[serde(default, alias = "slug")]
    pub id: String,
    #[serde(default, alias = "label")]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Marketplace state metadata.
pub struct MarketStateInfo {
    #[serde(default)]
    pub code: String,
    #[serde(default, alias = "name")]
    pub label: String,
}

// ---- Type stats ----

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Download and update stats for one marketplace entry.
pub struct MarketStatsEntryResponse {
    #[serde(default)]
    pub downloads: i32,
    #[serde(rename = "lastDownloadAt", default)]
    pub last_download_at: Option<String>,
    #[serde(rename = "updatedAt", default)]
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// Marketplace stats response keyed by entry id.
pub struct MarketTypeStatsResponse {
    #[serde(rename = "updatedAt", default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub items: BTreeMap<String, MarketStatsEntryResponse>,
}

// ---- GitHub authentication ----

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
/// GitHub user profile returned during marketplace authentication.
pub struct GitHubUser {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub login: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(rename = "avatar_url", alias = "avatarUrl", default)]
    pub avatar_url: String,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(rename = "public_repos", alias = "publicRepos", default)]
    pub public_repos: Option<i32>,
    #[serde(default)]
    pub followers: Option<i32>,
    #[serde(default)]
    pub following: Option<i32>,
}

// ---- Service ----

#[derive(Clone)]
/// Client for static marketplace metadata and authenticated marketplace APIs.
pub struct MarketStatsApiService {
    http_host: Arc<dyn HttpHost>,
    github_token: Option<String>,
}

impl MarketStatsApiService {
    /// Creates a marketplace API client without a GitHub token.
    pub fn new() -> Self {
        Self::new_with_github_token(None)
    }

    /// Creates a marketplace API client with an optional GitHub token.
    pub fn new_with_github_token(github_token: Option<String>) -> Self {
        Self {
            http_host: operit_host_api::HostManager::defaultHttpHost(),
            github_token,
        }
    }

    // ── Read ───────────────────────────────────────────────

    /// Loads the marketplace manifest.
    pub fn get_manifest(&self) -> Result<MarketManifest, String> {
        self.request_static_json(&["manifest.json"])
    }

    /// Loads a paginated all-entry marketplace list.
    pub fn get_list_page(&self, sort: &str, page: i32) -> Result<MarketListPage, String> {
        self.request_static_json(&["lists", "all", sort, &format!("page-{page}.json")])
    }

    /// Loads a paginated marketplace list filtered by type.
    pub fn get_type_page(
        &self,
        r#type: &str,
        sort: &str,
        page: i32,
    ) -> Result<MarketListPage, String> {
        self.request_static_json(&["lists", "type", r#type, sort, &format!("page-{page}.json")])
    }

    /// Loads a paginated marketplace list filtered by category.
    pub fn get_category_page(
        &self,
        category_id: &str,
        sort: &str,
        page: i32,
    ) -> Result<MarketListPage, String> {
        self.request_static_json(&[
            "lists",
            "category",
            category_id,
            sort,
            &format!("page-{page}.json"),
        ])
    }

    /// Loads a paginated marketplace list filtered by type and category.
    pub fn get_type_category_page(
        &self,
        r#type: &str,
        category_id: &str,
        sort: &str,
        page: i32,
    ) -> Result<MarketListPage, String> {
        self.request_static_json(&[
            "lists",
            "type",
            r#type,
            "category",
            category_id,
            sort,
            &format!("page-{page}.json"),
        ])
    }

    /// Loads one static entries shard.
    pub fn get_entries_shard(&self, shard: &str) -> Result<MarketEntriesShard, String> {
        self.request_static_json(&["entries", &format!("{shard}.json")])
    }

    /// Loads one marketplace entry by id.
    pub fn get_entry_by_id(&self, entry_id: &str) -> Result<MarketEntrySummary, String> {
        let shard = entry_shard(entry_id);
        let shard_resp: MarketEntriesShard = self.get_entries_shard(&shard)?;
        shard_resp
            .entries_by_id
            .get(entry_id)
            .cloned()
            .ok_or_else(|| format!("market entry not found: {entry_id}"))
    }

    /// Loads a paginated comment page for an entry.
    pub fn get_comments_page(
        &self,
        entry_id: &str,
        page: i32,
    ) -> Result<MarketCommentPage, String> {
        match self.request_static_json(&["comments", entry_id, &format!("page-{page}.json")]) {
            Ok(page_data) => Ok(page_data),
            Err(error) if error.contains("HTTP 404") => Ok(MarketCommentPage {
                ok: true,
                entry_id: entry_id.to_string(),
                page,
                page_size: 50,
                total: 0,
                items: Vec::new(),
                generated_at: None,
            }),
            Err(error) => Err(error),
        }
    }

    /// Builds aggregate stats for marketplace entries of one type.
    pub fn get_stats(&self, r#type: &str) -> Result<MarketTypeStatsResponse, String> {
        let mut page = 1;
        let mut items = BTreeMap::new();
        loop {
            let list = self.get_type_page(r#type, "updated", page)?;
            for entry in &list.items {
                items.insert(
                    entry.id.clone(),
                    MarketStatsEntryResponse {
                        downloads: entry_downloads(entry),
                        last_download_at: None,
                        updated_at: Some(entry.updated_at.clone()),
                    },
                );
            }
            if page >= total_pages(list.total, list.page_size).max(1) {
                break;
            }
            page += 1;
        }
        Ok(MarketTypeStatsResponse {
            updated_at: Some(now_iso()),
            items,
        })
    }

    // ── Auth ───────────────────────────────────────────────

    /// Returns the GitHub user associated with the configured token.
    pub fn get_current_github_user(&self) -> Result<GitHubUser, String> {
        self.decode_git(
            "GET",
            &format!("{GITHUB_API_BASE_URL}/user"),
            vec![(
                "Accept".to_string(),
                "application/vnd.github+json".to_string(),
            )],
            Vec::new(),
        )
    }

    /// Exchanges the configured GitHub token for a marketplace session.
    pub fn exchange_github_token_for_market_session(&self) -> Result<MarketAuthInfo, String> {
        self.decode_v2("POST", &["auth", "github"], Vec::new(), Vec::new(), false)
    }

    // ── Comments ───────────────────────────────────────────

    /// Creates a comment on a marketplace entry.
    pub fn create_entry_comment(&self, entry_id: &str, body: &str) -> Result<String, String> {
        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "commentId")]
            comment_id: String,
        }
        let resp: Response = self.decode_v2(
            "POST",
            &["entries", entry_id, "comments"],
            vec![("Content-Type".to_string(), "application/json".to_string())],
            serde_json::to_vec(&serde_json::json!({"body": body})).map_err(|e| e.to_string())?,
            true,
        )?;
        Ok(resp.comment_id)
    }

    /// Edits an existing marketplace comment.
    pub fn edit_entry_comment(&self, comment_id: &str, body: &str) -> Result<(), String> {
        let _: serde_json::Value = self.decode_v2(
            "PATCH",
            &["comments", comment_id],
            vec![("Content-Type".to_string(), "application/json".to_string())],
            serde_json::to_vec(&serde_json::json!({"body": body})).map_err(|e| e.to_string())?,
            true,
        )?;
        Ok(())
    }

    /// Deletes an existing marketplace comment.
    pub fn delete_entry_comment(&self, comment_id: &str) -> Result<(), String> {
        let _: serde_json::Value = self.decode_v2(
            "DELETE",
            &["comments", comment_id],
            Vec::new(),
            Vec::new(),
            true,
        )?;
        Ok(())
    }

    // ── Reactions ──────────────────────────────────────────

    /// Adds a positive reaction to a marketplace entry.
    pub fn create_entry_reaction(&self, entry_id: &str) -> Result<(), String> {
        let _: serde_json::Value = self.decode_v2(
            "POST",
            &["entries", entry_id, "reactions"],
            vec![("Content-Type".to_string(), "application/json".to_string())],
            serde_json::to_vec(&serde_json::json!({"reaction": "+1"}))
                .map_err(|e| e.to_string())?,
            true,
        )?;
        Ok(())
    }

    // ── Notifications ──────────────────────────────────────

    /// Loads authenticated marketplace notifications.
    pub fn get_notifications(
        &self,
        limit: i32,
        offset: i32,
        since: Option<String>,
    ) -> Result<MarketNotificationsResponse, String> {
        let mut url = self.v2_url(&["notifications"])?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("limit", &limit.max(1).min(100).to_string());
            query.append_pair("offset", &offset.max(0).to_string());
            if let Some(since) = since.filter(|v| !v.trim().is_empty()) {
                query.append_pair("since", &since);
            }
        }
        self.decode_url("GET", url.as_str(), Vec::new(), Vec::new(), true)
    }

    // ── My entries ─────────────────────────────────────────

    /// Loads entries owned by the authenticated user.
    pub fn get_my_entries(&self) -> Result<MarketMyEntriesResponse, String> {
        self.decode_v2("GET", &["my", "entries"], Vec::new(), Vec::new(), true)
    }

    /// Loads authenticated user entries filtered by type.
    pub fn get_my_entries_by_type(&self, r#type: &str) -> Result<MarketMyEntriesResponse, String> {
        let mut url = self.v2_url(&["my", "entries"])?;
        url.query_pairs_mut().append_pair("type", r#type);
        self.decode_url("GET", url.as_str(), Vec::new(), Vec::new(), true)
    }

    // ── Publish ────────────────────────────────────────────

    /// Publishes a marketplace artifact entry.
    pub fn publish_artifact(
        &self,
        r#type: &str,
        title: &str,
        description: &str,
        detail: &str,
        category_id: &str,
        allow_public_updates: bool,
        version: &str,
        format_ver: &str,
        min_app_ver: &str,
        max_app_ver: Option<String>,
        changelog: Option<String>,
        project_id: &str,
        runtime_package_id: &str,
        asset_kind: &str,
        asset_url: &str,
        gh_owner: &str,
        gh_repo: &str,
        gh_release_tag: &str,
        asset_name: &str,
        sha256: &str,
    ) -> Result<MarketPublishResponse, String> {
        let version_json = market_artifact_version_json(
            version,
            format_ver,
            min_app_ver,
            max_app_ver,
            changelog,
            project_id,
            runtime_package_id,
        );
        self.decode_v2(
            "POST",
            &["publish"],
            vec![("Content-Type".to_string(), "application/json".to_string())],
            serde_json::to_vec(&serde_json::json!({
                "type": r#type,
                "title": title,
                "description": description,
                "detail": detail,
                "categoryId": category_id,
                "allowPublicUpdates": allow_public_updates,
                "version": version_json,
                "asset": {
                    "kind": asset_kind,
                    "url": asset_url,
                    "ghOwner": gh_owner,
                    "ghRepo": gh_repo,
                    "ghReleaseTag": gh_release_tag,
                    "assetName": asset_name,
                    "sha256": sha256,
                },
            }))
            .map_err(|e| e.to_string())?,
            true,
        )
    }

    /// Publishes a marketplace entry backed by a GitHub repository.
    pub fn publish_repo_entry(
        &self,
        r#type: &str,
        title: &str,
        description: &str,
        detail: &str,
        category_id: &str,
        allow_public_updates: bool,
        source_url: &str,
        ref_type: &str,
        ref_name: &str,
        install_config: &str,
        version: &str,
        format_ver: &str,
        min_app_ver: &str,
        max_app_ver: Option<String>,
        changelog: Option<String>,
    ) -> Result<MarketPublishResponse, String> {
        let version_json =
            market_base_version_json(version, format_ver, min_app_ver, max_app_ver, changelog);
        self.decode_v2(
            "POST",
            &["publish"],
            vec![("Content-Type".to_string(), "application/json".to_string())],
            serde_json::to_vec(&serde_json::json!({
                "type": r#type,
                "title": title,
                "description": description,
                "detail": detail,
                "categoryId": category_id,
                "allowPublicUpdates": allow_public_updates,
                "source": { "kind": "github_repo", "url": source_url },
                "repoVersion": {
                    "refType": ref_type,
                    "refName": ref_name,
                    "installConfig": install_config,
                },
                "version": version_json,
            }))
            .map_err(|e| e.to_string())?,
            true,
        )
    }

    /// Updates mutable metadata for a marketplace entry.
    pub fn update_entry(
        &self,
        entry_id: &str,
        title: Option<String>,
        description: Option<String>,
        detail: Option<String>,
        category_id: Option<String>,
        allow_public_updates: Option<bool>,
    ) -> Result<MarketEntryUpdateResponse, String> {
        let patch = market_entry_patch_json(
            title,
            description,
            detail,
            category_id,
            allow_public_updates,
        );
        self.decode_v2(
            "PATCH",
            &["entries", entry_id],
            vec![("Content-Type".to_string(), "application/json".to_string())],
            serde_json::to_vec(&patch).map_err(|e| e.to_string())?,
            true,
        )
    }

    /// Publishes a new artifact version for an existing marketplace entry.
    pub fn publish_artifact_version(
        &self,
        entry_id: &str,
        version: &str,
        format_ver: &str,
        min_app_ver: &str,
        max_app_ver: Option<String>,
        changelog: Option<String>,
        project_id: &str,
        runtime_package_id: &str,
        asset_kind: &str,
        asset_url: &str,
        gh_owner: &str,
        gh_repo: &str,
        gh_release_tag: &str,
        asset_name: &str,
        sha256: &str,
        entry_title: Option<String>,
        entry_description: Option<String>,
        entry_detail: Option<String>,
        entry_category_id: Option<String>,
        entry_allow_public_updates: Option<bool>,
    ) -> Result<MarketPublishResponse, String> {
        let body = market_new_version_body(
            Some(market_entry_patch_json(
                entry_title,
                entry_description,
                entry_detail,
                entry_category_id,
                entry_allow_public_updates,
            )),
            market_artifact_version_json(
                version,
                format_ver,
                min_app_ver,
                max_app_ver,
                changelog,
                project_id,
                runtime_package_id,
            ),
            None,
            Some(serde_json::json!({
                "kind": asset_kind,
                "url": asset_url,
                "ghOwner": gh_owner,
                "ghRepo": gh_repo,
                "ghReleaseTag": gh_release_tag,
                "assetName": asset_name,
                "sha256": sha256,
            })),
        );
        self.decode_v2(
            "POST",
            &["entries", entry_id, "versions"],
            vec![("Content-Type".to_string(), "application/json".to_string())],
            serde_json::to_vec(&body).map_err(|e| e.to_string())?,
            true,
        )
    }

    /// Publishes a new repository version for an existing marketplace entry.
    pub fn publish_repo_version(
        &self,
        entry_id: &str,
        version: &str,
        format_ver: &str,
        min_app_ver: &str,
        max_app_ver: Option<String>,
        changelog: Option<String>,
        ref_type: &str,
        ref_name: &str,
        install_config: &str,
        entry_title: Option<String>,
        entry_description: Option<String>,
        entry_detail: Option<String>,
        entry_category_id: Option<String>,
        entry_allow_public_updates: Option<bool>,
    ) -> Result<MarketPublishResponse, String> {
        let body = market_new_version_body(
            Some(market_entry_patch_json(
                entry_title,
                entry_description,
                entry_detail,
                entry_category_id,
                entry_allow_public_updates,
            )),
            market_base_version_json(version, format_ver, min_app_ver, max_app_ver, changelog),
            Some(serde_json::json!({
                "refType": ref_type,
                "refName": ref_name,
                "installConfig": install_config,
            })),
            None,
        );
        self.decode_v2(
            "POST",
            &["entries", entry_id, "versions"],
            vec![("Content-Type".to_string(), "application/json".to_string())],
            serde_json::to_vec(&body).map_err(|e| e.to_string())?,
            true,
        )
    }

    // ── Download ───────────────────────────────────────────

    /// Downloads a marketplace asset by id.
    pub fn download_asset(&self, asset_id: &str) -> Result<Vec<u8>, String> {
        let trimmed_asset_id = asset_id.trim();
        if trimmed_asset_id.is_empty() {
            return Err("asset id is empty".to_string());
        }
        let url = self.v2_url(&["assets", trimmed_asset_id, "download"])?;
        let resp = self.request("GET", url.as_str(), Vec::new(), Vec::new(), true)?;
        if is_success(resp.statusCode) {
            Ok(resp.body)
        } else {
            Err(format!(
                "HTTP {}: {}",
                resp.statusCode,
                summarize_body(&body_text(&resp.body))
            ))
        }
    }

    // ── Internal: HTTP helpers ─────────────────────────────

    fn request_v2_json<T: for<'de> Deserialize<'de>>(
        &self,
        path_segments: &[&str],
    ) -> Result<T, String> {
        let url = self.v2_url(path_segments)?;
        self.decode_url("GET", url.as_str(), Vec::new(), Vec::new(), false)
    }

    fn request_static_json<T: for<'de> Deserialize<'de>>(
        &self,
        path_segments: &[&str],
    ) -> Result<T, String> {
        let url = self.static_url(path_segments)?;
        self.decode_url("GET", url.as_str(), Vec::new(), Vec::new(), false)
    }

    fn static_url(&self, path_segments: &[&str]) -> Result<Url, String> {
        let mut url = Url::parse(MARKET_V2_STATIC_URL).map_err(|e| e.to_string())?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| "invalid market v2 static url".to_string())?;
            for seg in path_segments {
                segments.push(seg);
            }
        }
        Ok(url)
    }

    fn v2_url(&self, path_segments: &[&str]) -> Result<Url, String> {
        let mut url = Url::parse(MARKET_V2_BASE_URL).map_err(|e| e.to_string())?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| "invalid market v2 base url".to_string())?;
            for seg in path_segments {
                segments.push(seg);
            }
        }
        Ok(url)
    }

    fn decode_v2<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        path_segments: &[&str],
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        use_session: bool,
    ) -> Result<T, String> {
        let url = self.v2_url(path_segments)?;
        self.decode_url(method, url.as_str(), headers, body, use_session)
    }

    fn decode_git<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        url: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<T, String> {
        let resp = self.send_github(method, url, headers, body)?;
        let body = body_text(&resp.body);
        if !is_success(resp.statusCode) {
            return Err(format!(
                "HTTP {}: {}",
                resp.statusCode,
                summarize_body(&body)
            ));
        }
        serde_json::from_str::<T>(&body).map_err(|e| e.to_string())
    }

    fn send_github(
        &self,
        method: &str,
        url: &str,
        mut headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<HttpResponseData, String> {
        if let Some(token) = self
            .github_token
            .as_deref()
            .filter(|t| !t.trim().is_empty())
        {
            headers.push(("Authorization".to_string(), format!("Bearer {token}")));
        }
        self.request(method, url, headers, body, true)
    }

    fn decode_url<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        url: &str,
        mut headers: Vec<(String, String)>,
        body: Vec<u8>,
        use_session: bool,
    ) -> Result<T, String> {
        if use_session {
            let auth = self.exchange_github_token_for_market_session()?;
            headers.push((
                "Authorization".to_string(),
                format!("Bearer {}", auth.session),
            ));
        } else if url.ends_with("/auth/github") {
            if let Some(token) = self
                .github_token
                .as_deref()
                .filter(|t| !t.trim().is_empty())
            {
                headers.push(("Authorization".to_string(), format!("Bearer {token}")));
            }
        }
        let resp = self.request(method, url, headers, body, true)?;
        let body = body_text(&resp.body);
        if !is_success(resp.statusCode) {
            return Err(format!(
                "HTTP {}: {}",
                resp.statusCode,
                summarize_body(&body)
            ));
        }
        serde_json::from_str::<T>(&body).map_err(|e| e.to_string())
    }

    fn request(
        &self,
        method: &str,
        url: &str,
        mut headers: Vec<(String, String)>,
        body: Vec<u8>,
        follow_redirects: bool,
    ) -> Result<HttpResponseData, String> {
        headers.push(("User-Agent".to_string(), USER_AGENT.to_string()));
        self.http_host
            .executeHttpRequest(HttpRequestData {
                url: url.to_string(),
                method: method.to_string(),
                headers,
                body,
                formFields: Vec::new(),
                fileParts: Vec::new(),
                connectTimeoutSeconds: TIMEOUT_SECONDS,
                readTimeoutSeconds: TIMEOUT_SECONDS,
                followRedirects: follow_redirects,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            })
            .map_err(|e| e.to_string())
    }
}

// ── Artifact project models ────────────────────────────────

// ── Helpers ────────────────────────────────────────────────

fn entry_shard(id: &str) -> String {
    let mut hash = 2166136261u32;
    for byte in id.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:08x}")[0..2].to_string()
}

fn total_pages(total: i32, page_size: i32) -> i32 {
    let size = page_size.max(1);
    (total.max(0) + size - 1) / size
}

fn now_iso() -> String {
    chrono::DateTime::from_timestamp_millis(operit_host_api::TimeUtils::currentTimeMillis())
        .expect("current host time must be representable as a chrono timestamp")
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

fn entry_downloads(entry: &MarketEntrySummary) -> i32 {
    entry.download_count.max(entry.downloads)
}

fn market_base_version_json(
    version: &str,
    format_ver: &str,
    min_app_ver: &str,
    max_app_ver: Option<String>,
    changelog: Option<String>,
) -> serde_json::Value {
    let mut json = serde_json::json!({
        "version": version,
        "formatVer": format_ver,
        "minAppVer": min_app_ver,
    });
    if let Some(max) = max_app_ver.filter(|value| !value.trim().is_empty()) {
        json["maxAppVer"] = serde_json::Value::String(max);
    }
    if let Some(text) = changelog.filter(|value| !value.trim().is_empty()) {
        json["changelog"] = serde_json::Value::String(text);
    }
    json
}

fn market_artifact_version_json(
    version: &str,
    format_ver: &str,
    min_app_ver: &str,
    max_app_ver: Option<String>,
    changelog: Option<String>,
    project_id: &str,
    runtime_package_id: &str,
) -> serde_json::Value {
    let mut json =
        market_base_version_json(version, format_ver, min_app_ver, max_app_ver, changelog);
    json["projectId"] = serde_json::Value::String(project_id.to_string());
    json["runtimePackageId"] = serde_json::Value::String(runtime_package_id.to_string());
    json
}

fn market_entry_patch_json(
    title: Option<String>,
    description: Option<String>,
    detail: Option<String>,
    category_id: Option<String>,
    allow_public_updates: Option<bool>,
) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    if let Some(value) = title {
        map.insert("title".to_string(), serde_json::Value::String(value));
    }
    if let Some(value) = description {
        map.insert("description".to_string(), serde_json::Value::String(value));
    }
    if let Some(value) = detail {
        map.insert("detail".to_string(), serde_json::Value::String(value));
    }
    if let Some(value) = category_id {
        map.insert("categoryId".to_string(), serde_json::Value::String(value));
    }
    if let Some(value) = allow_public_updates {
        map.insert(
            "allowPublicUpdates".to_string(),
            serde_json::Value::Bool(value),
        );
    }
    serde_json::Value::Object(map)
}

fn market_new_version_body(
    entry_patch: Option<serde_json::Value>,
    version: serde_json::Value,
    repo_version: Option<serde_json::Value>,
    asset: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    if let Some(serde_json::Value::Object(entry)) = entry_patch {
        if !entry.is_empty() {
            map.insert("entry".to_string(), serde_json::Value::Object(entry));
        }
    }
    map.insert("version".to_string(), version);
    if let Some(repo_version) = repo_version {
        map.insert("repoVersion".to_string(), repo_version);
    }
    if let Some(asset) = asset {
        map.insert("asset".to_string(), asset);
    }
    serde_json::Value::Object(map)
}

fn is_success(status: i32) -> bool {
    (200..300).contains(&status)
}

fn is_redirection(status: i32) -> bool {
    (300..400).contains(&status)
}

fn body_text(body: &[u8]) -> String {
    String::from_utf8(body.to_vec()).unwrap_or_default()
}

fn summarize_body(body: &str) -> String {
    if body.trim().is_empty() {
        return String::new();
    }
    if body.contains("<html") || body.contains("<!DOCTYPE html") {
        return "[html body omitted]".to_string();
    }
    body.lines()
        .next()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(180)
        .collect()
}

// re-export snake-case aliases for callers using old naming
#[allow(non_snake_case)]
pub use MarketStatsApiService as MarketStatsApiServiceModule;

#[cfg(test)]
mod tests {
    use super::MarketStatsApiService;
    use operit_host_api::HostManager::setDefaultHttpHost;
    use operit_host_api::{HostError, HostResult, HttpHost, HttpRequestData, HttpResponseData};
    use std::sync::Arc;

    struct ReqwestTestHttpHost;

    impl HttpHost for ReqwestTestHttpHost {
        /// Executes one test HTTP request through reqwest.
        fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
            let method = reqwest::Method::from_bytes(request.method.as_bytes())
                .map_err(|e| HostError::new(e.to_string()))?;
            let client = reqwest::blocking::Client::builder()
                .redirect(if request.followRedirects {
                    reqwest::redirect::Policy::limited(10)
                } else {
                    reqwest::redirect::Policy::none()
                })
                .timeout(std::time::Duration::from_secs(
                    request.readTimeoutSeconds.max(1),
                ))
                .connect_timeout(std::time::Duration::from_secs(
                    request.connectTimeoutSeconds.max(1),
                ))
                .build()
                .map_err(|e| HostError::new(e.to_string()))?;
            let mut builder = client.request(method, &request.url);
            for (key, value) in request.headers {
                builder = builder.header(key, value);
            }
            if !request.body.is_empty() {
                builder = builder.body(request.body);
            }
            let response = builder.send().map_err(|e| HostError::new(e.to_string()))?;
            let status = response.status();
            let final_url = response.url().to_string();
            let headers = response
                .headers()
                .iter()
                .map(|(key, value)| {
                    (
                        key.to_string(),
                        value.to_str().unwrap_or_default().to_string(),
                    )
                })
                .collect::<Vec<_>>();
            let body = response
                .bytes()
                .map_err(|e| HostError::new(e.to_string()))?
                .to_vec();
            Ok(HttpResponseData {
                finalUrl: final_url,
                statusCode: status.as_u16() as i32,
                statusMessage: status.canonical_reason().unwrap_or_default().to_string(),
                headers,
                body,
            })
        }

        /// Rejects file downloads because market tests only exercise buffered requests.
        fn downloadFiles(
            &self,
            _request: operit_host_api::HttpDownloadRequest,
            _control: operit_host_api::HttpDownloadControl,
            _onProgress: operit_host_api::HttpDownloadProgressCallback,
        ) -> HostResult<operit_host_api::HttpDownloadResult> {
            Err(HostError::new(
                "market test HTTP downloads are not configured",
            ))
        }
    }

    #[test]
    #[ignore = "hits api.operit.app"]
    fn online_market_v2_manifest_list_and_shard_are_readable() {
        setDefaultHttpHost(Arc::new(ReqwestTestHttpHost));
        let api = MarketStatsApiService::new();
        let manifest = api.get_manifest().expect("manifest should load");
        assert_eq!(manifest.market_version, 2);

        let list = api
            .get_list_page("updated", 1)
            .expect("updated list page should load");
        assert!(list.total > 0, "updated list should not be empty");
        let first = list
            .items
            .first()
            .expect("updated list should contain items");

        let entry = api
            .get_entry_by_id(&first.id)
            .expect("entry shard lookup should load first list item");
        assert_eq!(entry.id, first.id);
    }
}
