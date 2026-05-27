use std::collections::BTreeMap;
use std::sync::Arc;

use operit_host_api::{HttpHost, HttpRequestData, HttpResponseData};
use serde::{Deserialize, Serialize};
use url::Url;

const STATIC_BASE_URL: &str = "https://static.operit.app/market-stats";
const TRACK_BASE_URL: &str = "https://api.operit.app/market-stats";
const GITHUB_API_BASE_URL: &str = "https://api.github.com";
const USER_AGENT: &str = "Operit-Market-Stats";
const TIMEOUT_SECONDS: u64 = 15;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketStatsEntryResponse {
    #[serde(default)]
    pub downloads: i32,
    #[serde(rename = "lastDownloadAt", default)]
    pub lastDownloadAt: Option<String>,
    #[serde(rename = "updatedAt", default)]
    pub updatedAt: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketTypeStatsResponse {
    #[serde(rename = "updatedAt", default)]
    pub updatedAt: Option<String>,
    #[serde(default)]
    pub items: BTreeMap<String, MarketStatsEntryResponse>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarketRankIssueEntryResponse {
    pub id: String,
    #[serde(default)]
    pub downloads: i32,
    #[serde(rename = "lastDownloadAt", default)]
    pub lastDownloadAt: Option<String>,
    #[serde(rename = "updatedAt", default)]
    pub updatedAt: Option<String>,
    #[serde(rename = "statsUpdatedAt", default)]
    pub statsUpdatedAt: Option<String>,
    #[serde(rename = "displayTitle", default)]
    pub displayTitle: String,
    #[serde(rename = "summaryDescription", default)]
    pub summaryDescription: String,
    #[serde(rename = "authorLogin", default)]
    pub authorLogin: String,
    #[serde(rename = "authorAvatarUrl", default)]
    pub authorAvatarUrl: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    pub issue: GitHubIssue,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MarketRankPageResponse {
    #[serde(rename = "updatedAt", default)]
    pub updatedAt: Option<String>,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub metric: String,
    #[serde(default = "defaultPage")]
    pub page: i32,
    #[serde(rename = "pageSize", default)]
    pub pageSize: i32,
    #[serde(rename = "totalPages", default = "defaultPage")]
    pub totalPages: i32,
    #[serde(rename = "totalItems", default)]
    pub totalItems: i32,
    #[serde(default)]
    pub items: Vec<MarketRankIssueEntryResponse>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactProjectRankDefaultNodeResponse {
    #[serde(rename = "nodeId", default)]
    pub nodeId: String,
    #[serde(rename = "runtimePackageId", default)]
    pub runtimePackageId: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub version: String,
    #[serde(rename = "downloadUrl", default)]
    pub downloadUrl: String,
    #[serde(default = "openState")]
    pub state: String,
    #[serde(rename = "publishedAt", default)]
    pub publishedAt: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactProjectRankEntryResponse {
    #[serde(rename = "projectId", default)]
    pub projectId: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(rename = "projectDisplayName", default)]
    pub projectDisplayName: String,
    #[serde(rename = "projectDescription", default)]
    pub projectDescription: String,
    #[serde(rename = "rootPublisherLogin", default)]
    pub rootPublisherLogin: String,
    #[serde(rename = "rootPublisherAvatarUrl", default)]
    pub rootPublisherAvatarUrl: String,
    #[serde(rename = "contributorCount", default)]
    pub contributorCount: i32,
    #[serde(default)]
    pub downloads: i32,
    #[serde(default)]
    pub likes: i32,
    #[serde(rename = "latestNodeId", default)]
    pub latestNodeId: String,
    #[serde(rename = "latestOpenNodeId", default)]
    pub latestOpenNodeId: String,
    #[serde(rename = "defaultNodeId", default)]
    pub defaultNodeId: String,
    #[serde(rename = "latestPublishedAt", default)]
    pub latestPublishedAt: Option<String>,
    #[serde(rename = "defaultNode", default)]
    pub defaultNode: Option<ArtifactProjectRankDefaultNodeResponse>,
    #[serde(rename = "runtimePackageNodeSha256s", default)]
    pub runtimePackageNodeSha256s: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactProjectRankPageResponse {
    #[serde(rename = "updatedAt", default)]
    pub updatedAt: Option<String>,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub metric: String,
    #[serde(default = "defaultPage")]
    pub page: i32,
    #[serde(rename = "pageSize", default)]
    pub pageSize: i32,
    #[serde(rename = "totalPages", default = "defaultPage")]
    pub totalPages: i32,
    #[serde(rename = "totalItems", default)]
    pub totalItems: i32,
    #[serde(default)]
    pub items: Vec<ArtifactProjectRankEntryResponse>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ArtifactProjectEdgeResponse {
    #[serde(rename = "parentNodeId", default)]
    pub parentNodeId: String,
    #[serde(rename = "childNodeId", default)]
    pub childNodeId: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactProjectNodeResponse {
    #[serde(rename = "projectId", default)]
    pub projectId: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(rename = "projectDisplayName", default)]
    pub projectDisplayName: String,
    #[serde(rename = "projectDescription", default)]
    pub projectDescription: String,
    #[serde(rename = "runtimePackageId", default)]
    pub runtimePackageId: String,
    #[serde(rename = "nodeId", default)]
    pub nodeId: String,
    #[serde(rename = "rootNodeId", default)]
    pub rootNodeId: String,
    #[serde(rename = "parentNodeIds", default)]
    pub parentNodeIds: Vec<String>,
    #[serde(rename = "publisherLogin", default)]
    pub publisherLogin: String,
    #[serde(rename = "releaseTag", default)]
    pub releaseTag: String,
    #[serde(rename = "assetName", default)]
    pub assetName: String,
    #[serde(rename = "downloadUrl", default)]
    pub downloadUrl: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub version: String,
    #[serde(rename = "displayName", default)]
    pub displayName: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "sourceFileName", default)]
    pub sourceFileName: String,
    #[serde(rename = "minSupportedAppVersion", default)]
    pub minSupportedAppVersion: Option<String>,
    #[serde(rename = "maxSupportedAppVersion", default)]
    pub maxSupportedAppVersion: Option<String>,
    #[serde(rename = "publishedAt", default)]
    pub publishedAt: Option<String>,
    #[serde(default = "openState")]
    pub state: String,
    pub issue: GitHubIssue,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ArtifactProjectDetailResponse {
    #[serde(rename = "projectId", default)]
    pub projectId: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(rename = "projectDisplayName", default)]
    pub projectDisplayName: String,
    #[serde(rename = "projectDescription", default)]
    pub projectDescription: String,
    #[serde(rename = "rootNodeId", default)]
    pub rootNodeId: String,
    #[serde(rename = "rootPublisherLogin", default)]
    pub rootPublisherLogin: String,
    #[serde(rename = "rootPublisherAvatarUrl", default)]
    pub rootPublisherAvatarUrl: String,
    #[serde(rename = "contributorCount", default)]
    pub contributorCount: i32,
    #[serde(default)]
    pub downloads: i32,
    #[serde(default)]
    pub likes: i32,
    #[serde(rename = "latestNodeId", default)]
    pub latestNodeId: String,
    #[serde(rename = "latestOpenNodeId", default)]
    pub latestOpenNodeId: String,
    #[serde(rename = "defaultNodeId", default)]
    pub defaultNodeId: String,
    #[serde(rename = "latestPublishedAt", default)]
    pub latestPublishedAt: Option<String>,
    #[serde(default)]
    pub nodes: Vec<ArtifactProjectNodeResponse>,
    #[serde(default)]
    pub edges: Vec<ArtifactProjectEdgeResponse>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    pub avatarUrl: String,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(rename = "public_repos", alias = "publicRepos", default)]
    pub publicRepos: Option<i32>,
    #[serde(default)]
    pub followers: Option<i32>,
    #[serde(default)]
    pub following: Option<i32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubLabel {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubReactions {
    #[serde(rename = "total_count", alias = "totalCount", default)]
    pub totalCount: i32,
    #[serde(rename = "+1", default)]
    pub thumbsUp: i32,
    #[serde(rename = "-1", default)]
    pub thumbsDown: i32,
    #[serde(default)]
    pub laugh: i32,
    #[serde(default)]
    pub hooray: i32,
    #[serde(default)]
    pub confused: i32,
    #[serde(default)]
    pub heart: i32,
    #[serde(default)]
    pub rocket: i32,
    #[serde(default)]
    pub eyes: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GitHubIssue {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub number: i32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(rename = "html_url", default)]
    pub html_url: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub labels: Vec<GitHubLabel>,
    #[serde(default)]
    pub user: GitHubUser,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub reactions: Option<GitHubReactions>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GitHubIssueSearchResponse {
    #[serde(rename = "total_count", default)]
    pub total_count: i32,
    #[serde(default)]
    pub incomplete_results: bool,
    #[serde(default)]
    pub items: Vec<GitHubIssue>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GitHubComment {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub user: GitHubUser,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub html_url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GitHubReaction {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub user: GitHubUser,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMarketMetadata {
    #[serde(default)]
    pub description: String,
    #[serde(rename = "repositoryUrl", alias = "repoUrl", default)]
    pub repositoryUrl: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: String,
    #[serde(default)]
    pub version: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpMarketMetadata {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub repositoryUrl: String,
    #[serde(rename = "installConfig", alias = "installCommand", default)]
    pub installConfig: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: String,
    #[serde(default)]
    pub version: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactMarketMetadata {
    #[serde(default)]
    pub r#type: String,
    #[serde(rename = "projectId", default)]
    pub projectId: String,
    #[serde(rename = "projectDisplayName", default)]
    pub projectDisplayName: String,
    #[serde(rename = "projectDescription", default)]
    pub projectDescription: String,
    #[serde(rename = "runtimePackageId", default)]
    pub runtimePackageId: String,
    #[serde(rename = "nodeId", default)]
    pub nodeId: String,
    #[serde(rename = "rootNodeId", default)]
    pub rootNodeId: String,
    #[serde(rename = "parentNodeIds", default)]
    pub parentNodeIds: Vec<String>,
    #[serde(rename = "publisherLogin", default)]
    pub publisherLogin: String,
    #[serde(rename = "releaseTag", default)]
    pub releaseTag: String,
    #[serde(rename = "assetName", default)]
    pub assetName: String,
    #[serde(rename = "downloadUrl", default)]
    pub downloadUrl: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub version: String,
    #[serde(rename = "displayName", default)]
    pub displayName: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "sourceFileName", default)]
    pub sourceFileName: String,
    #[serde(rename = "minSupportedAppVersion", default)]
    pub minSupportedAppVersion: Option<String>,
    #[serde(rename = "maxSupportedAppVersion", default)]
    pub maxSupportedAppVersion: Option<String>,
    #[serde(rename = "normalizedId", default)]
    pub normalizedId: String,
    #[serde(rename = "forgeRepo", default)]
    pub forgeRepo: String,
}

#[derive(Clone)]
pub struct MarketStatsApiService {
    httpHost: Arc<dyn HttpHost>,
    githubToken: Option<String>,
}

impl MarketStatsApiService {
    pub fn new() -> Self {
        Self::newWithGitHubToken(None)
    }

    #[allow(non_snake_case)]
    pub fn newWithGitHubToken(githubToken: Option<String>) -> Self {
        Self {
            httpHost: crate::core::application::OperitApplicationContext::defaultHttpHost(),
            githubToken,
        }
    }

    #[allow(non_snake_case)]
    pub fn getStats(&self, r#type: &str) -> Result<MarketTypeStatsResponse, String> {
        self.requestStaticJson(&["stats", &format!("{type}.json")])
    }

    #[allow(non_snake_case)]
    pub fn getRankPage(
        &self,
        r#type: &str,
        metric: &str,
        page: i32,
    ) -> Result<MarketRankPageResponse, String> {
        self.requestStaticJson(&["rank", &format!("{type}-{metric}-page-{page}.json")])
    }

    #[allow(non_snake_case)]
    pub fn getArtifactRankPage(
        &self,
        r#type: &str,
        metric: &str,
        page: i32,
    ) -> Result<ArtifactProjectRankPageResponse, String> {
        self.requestStaticJson(&[
            "artifact-rank",
            &format!("{type}-{metric}-page-{page}.json"),
        ])
    }

    #[allow(non_snake_case)]
    pub fn getArtifactProject(
        &self,
        projectId: &str,
    ) -> Result<ArtifactProjectDetailResponse, String> {
        self.requestStaticJson(&["artifact-projects", &format!("{projectId}.json")])
    }

    #[allow(non_snake_case)]
    pub fn trackDownload(&self, r#type: &str, id: &str, targetUrl: &str) -> Result<(), String> {
        let url = Url::parse_with_params(
            &format!("{TRACK_BASE_URL}/download"),
            &[("type", r#type), ("id", id), ("target", targetUrl)],
        )
        .map_err(|error| error.to_string())?;
        let response = self.request("GET", url.as_str(), Vec::new(), Vec::new(), false)?;
        if isSuccess(response.statusCode) || isRedirection(response.statusCode) {
            Ok(())
        } else {
            Err(format!(
                "HTTP {}: {}",
                response.statusCode,
                summarizeBody(&bodyText(&response.body))
            ))
        }
    }

    #[allow(non_snake_case)]
    pub fn searchIssues(
        &self,
        owner: &str,
        repo: &str,
        label: &str,
        rawQuery: &str,
        page: i32,
        perPage: i32,
    ) -> Result<Vec<GitHubIssue>, String> {
        let query = buildQualifiedSearchQuery(owner, repo, label, rawQuery, true);
        let url = Url::parse_with_params(
            &format!("{GITHUB_API_BASE_URL}/search/issues"),
            &[
                ("q", query.as_str()),
                ("sort", "updated"),
                ("order", "desc"),
                ("page", &page.to_string()),
                ("per_page", &perPage.to_string()),
            ],
        )
        .map_err(|error| error.to_string())?;
        let response = self.sendGitHub(
            "GET",
            url.as_str(),
            vec![(
                "Accept".to_string(),
                "application/vnd.github+json, application/vnd.github.squirrel-girl-preview+json"
                    .to_string(),
            )],
            Vec::new(),
        )?;
        let body = bodyText(&response.body);
        if !isSuccess(response.statusCode) {
            return Err(format!(
                "HTTP {}: {}",
                response.statusCode,
                summarizeBody(&body)
            ));
        }
        serde_json::from_str::<GitHubIssueSearchResponse>(&body)
            .map(|response| response.items)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    pub fn getIssue(
        &self,
        owner: &str,
        repo: &str,
        issueNumber: i32,
    ) -> Result<GitHubIssue, String> {
        let url = format!("{GITHUB_API_BASE_URL}/repos/{owner}/{repo}/issues/{issueNumber}");
        self.decodeGitHub(
            "GET",
            &url,
            vec![(
                "Accept".to_string(),
                "application/vnd.github+json, application/vnd.github.squirrel-girl-preview+json"
                    .to_string(),
            )],
            Vec::new(),
        )
    }

    #[allow(non_snake_case)]
    pub fn getIssueComments(
        &self,
        owner: &str,
        repo: &str,
        issueNumber: i32,
        page: i32,
        perPage: i32,
    ) -> Result<Vec<GitHubComment>, String> {
        let url = Url::parse_with_params(
            &format!("{GITHUB_API_BASE_URL}/repos/{owner}/{repo}/issues/{issueNumber}/comments"),
            &[
                ("page", page.to_string()),
                ("per_page", perPage.to_string()),
            ],
        )
        .map_err(|error| error.to_string())?;
        self.decodeGitHub(
            "GET",
            url.as_str(),
            vec![(
                "Accept".to_string(),
                "application/vnd.github+json".to_string(),
            )],
            Vec::new(),
        )
    }

    #[allow(non_snake_case)]
    pub fn createIssueComment(
        &self,
        owner: &str,
        repo: &str,
        issueNumber: i32,
        body: &str,
    ) -> Result<GitHubComment, String> {
        let url =
            format!("{GITHUB_API_BASE_URL}/repos/{owner}/{repo}/issues/{issueNumber}/comments");
        self.decodeGitHub(
            "POST",
            &url,
            vec![
                (
                    "Accept".to_string(),
                    "application/vnd.github+json".to_string(),
                ),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            serde_json::to_vec(&serde_json::json!({ "body": body }))
                .map_err(|error| error.to_string())?,
        )
    }

    #[allow(non_snake_case)]
    pub fn getIssueReactions(
        &self,
        owner: &str,
        repo: &str,
        issueNumber: i32,
    ) -> Result<Vec<GitHubReaction>, String> {
        let url =
            format!("{GITHUB_API_BASE_URL}/repos/{owner}/{repo}/issues/{issueNumber}/reactions");
        self.decodeGitHub(
            "GET",
            &url,
            vec![(
                "Accept".to_string(),
                "application/vnd.github.squirrel-girl-preview+json".to_string(),
            )],
            Vec::new(),
        )
    }

    #[allow(non_snake_case)]
    pub fn createIssueReaction(
        &self,
        owner: &str,
        repo: &str,
        issueNumber: i32,
        content: &str,
    ) -> Result<GitHubReaction, String> {
        let url =
            format!("{GITHUB_API_BASE_URL}/repos/{owner}/{repo}/issues/{issueNumber}/reactions");
        self.decodeGitHub(
            "POST",
            &url,
            vec![
                (
                    "Accept".to_string(),
                    "application/vnd.github.squirrel-girl-preview+json".to_string(),
                ),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            serde_json::to_vec(&serde_json::json!({ "content": content }))
                .map_err(|error| error.to_string())?,
        )
    }

    #[allow(non_snake_case)]
    pub fn getCurrentUser(&self) -> Result<GitHubUser, String> {
        self.decodeGitHub(
            "GET",
            &format!("{GITHUB_API_BASE_URL}/user"),
            vec![(
                "Accept".to_string(),
                "application/vnd.github+json".to_string(),
            )],
            Vec::new(),
        )
    }

    #[allow(non_snake_case)]
    fn requestStaticJson<T: for<'de> Deserialize<'de>>(
        &self,
        pathSegments: &[&str],
    ) -> Result<T, String> {
        let mut url = Url::parse(STATIC_BASE_URL).map_err(|error| error.to_string())?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| "invalid static base url".to_string())?;
            for segment in pathSegments {
                segments.push(segment);
            }
        }
        let response = self.request("GET", url.as_str(), Vec::new(), Vec::new(), true)?;
        let body = bodyText(&response.body);
        if !isSuccess(response.statusCode) {
            return Err(format!(
                "HTTP {}: {}",
                response.statusCode,
                summarizeBody(&body)
            ));
        }
        serde_json::from_str::<T>(&body).map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn sendGitHub(
        &self,
        method: &str,
        url: &str,
        mut headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<HttpResponseData, String> {
        if let Some(token) = self
            .githubToken
            .as_ref()
            .filter(|token| !token.trim().is_empty())
        {
            headers.push(("Authorization".to_string(), format!("Bearer {token}")));
        }
        self.request(method, url, headers, body, true)
    }

    #[allow(non_snake_case)]
    fn decodeGitHub<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        url: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<T, String> {
        let response = self.sendGitHub(method, url, headers, body)?;
        let body = bodyText(&response.body);
        if !isSuccess(response.statusCode) {
            return Err(format!(
                "HTTP {}: {}",
                response.statusCode,
                summarizeBody(&body)
            ));
        }
        serde_json::from_str::<T>(&body).map_err(|error| error.to_string())
    }

    fn request(
        &self,
        method: &str,
        url: &str,
        mut headers: Vec<(String, String)>,
        body: Vec<u8>,
        followRedirects: bool,
    ) -> Result<HttpResponseData, String> {
        headers.push(("User-Agent".to_string(), USER_AGENT.to_string()));
        self.httpHost
            .executeHttpRequest(HttpRequestData {
                url: url.to_string(),
                method: method.to_string(),
                headers,
                body,
                formFields: Vec::new(),
                fileParts: Vec::new(),
                connectTimeoutSeconds: TIMEOUT_SECONDS,
                readTimeoutSeconds: TIMEOUT_SECONDS,
                followRedirects,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            })
            .map_err(|error| error.to_string())
    }
}

impl Default for MarketRankIssueEntryResponse {
    fn default() -> Self {
        Self {
            id: String::new(),
            downloads: 0,
            lastDownloadAt: None,
            updatedAt: None,
            statsUpdatedAt: None,
            displayTitle: String::new(),
            summaryDescription: String::new(),
            authorLogin: String::new(),
            authorAvatarUrl: String::new(),
            metadata: None,
            issue: GitHubIssue::default(),
        }
    }
}

#[allow(non_snake_case)]
fn isSuccess(statusCode: i32) -> bool {
    (200..300).contains(&statusCode)
}

#[allow(non_snake_case)]
fn isRedirection(statusCode: i32) -> bool {
    (300..400).contains(&statusCode)
}

#[allow(non_snake_case)]
fn bodyText(body: &[u8]) -> String {
    String::from_utf8(body.to_vec()).unwrap_or_else(|_| String::new())
}

impl Default for ArtifactProjectNodeResponse {
    fn default() -> Self {
        Self {
            projectId: String::new(),
            r#type: String::new(),
            projectDisplayName: String::new(),
            projectDescription: String::new(),
            runtimePackageId: String::new(),
            nodeId: String::new(),
            rootNodeId: String::new(),
            parentNodeIds: Vec::new(),
            publisherLogin: String::new(),
            releaseTag: String::new(),
            assetName: String::new(),
            downloadUrl: String::new(),
            sha256: String::new(),
            version: String::new(),
            displayName: String::new(),
            description: String::new(),
            sourceFileName: String::new(),
            minSupportedAppVersion: None,
            maxSupportedAppVersion: None,
            publishedAt: None,
            state: "open".to_string(),
            issue: GitHubIssue::default(),
        }
    }
}

#[allow(non_snake_case)]
pub fn parseSkillMarketMetadata(body: &str) -> Option<SkillMarketMetadata> {
    parseCommentJson(body, "<!-- operit-skill-json: ")
}

#[allow(non_snake_case)]
pub fn parseMcpMarketMetadata(body: &str) -> Option<McpMarketMetadata> {
    parseCommentJson(body, "<!-- operit-mcp-json: ")
}

#[allow(non_snake_case)]
pub fn parseArtifactMarketMetadata(body: &str) -> Option<ArtifactMarketMetadata> {
    parseCommentJson(body, "<!-- operit-market-json: ")
}

#[allow(non_snake_case)]
pub fn skillRepositoryUrlFromEntry(entry: &MarketRankIssueEntryResponse) -> String {
    if let Some(metadata) = entry
        .metadata
        .as_ref()
        .and_then(|value| serde_json::from_value::<SkillMarketMetadata>(value.clone()).ok())
    {
        if !metadata.repositoryUrl.trim().is_empty() {
            return metadata.repositoryUrl;
        }
    }
    entry
        .issue
        .body
        .as_deref()
        .and_then(parseSkillMarketMetadata)
        .map(|metadata| metadata.repositoryUrl)
        .unwrap_or_default()
}

#[allow(non_snake_case)]
pub fn mcpMetadataFromEntry(entry: &MarketRankIssueEntryResponse) -> McpMarketMetadata {
    if let Some(metadata) = entry
        .metadata
        .as_ref()
        .and_then(|value| serde_json::from_value::<McpMarketMetadata>(value.clone()).ok())
    {
        return metadata;
    }
    entry
        .issue
        .body
        .as_deref()
        .and_then(parseMcpMarketMetadata)
        .unwrap_or_default()
}

#[allow(non_snake_case)]
pub fn normalizeMarketArtifactId(raw: &str) -> String {
    let mut out = String::new();
    let mut previousDash = false;
    for ch in raw.trim().to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            previousDash = false;
        } else if !previousDash {
            out.push('-');
            previousDash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "untitled-artifact".to_string()
    } else {
        trimmed
    }
}

#[allow(non_snake_case)]
pub fn resolveMarketEntryId(preferredSource: &str, fallback: &str) -> String {
    let preferred = preferredSource.trim();
    let source = if preferred.is_empty() {
        fallback.to_string()
    } else {
        canonicalizeMarketSource(preferred)
    };
    normalizeMarketArtifactId(&source)
}

#[allow(non_snake_case)]
fn parseCommentJson<T: for<'de> Deserialize<'de>>(body: &str, prefix: &str) -> Option<T> {
    let start = body.find(prefix)? + prefix.len();
    let rest = &body[start..];
    let end = rest.find("-->")?;
    serde_json::from_str::<T>(rest[..end].trim()).ok()
}

#[allow(non_snake_case)]
fn canonicalizeMarketSource(raw: &str) -> String {
    if let Ok(url) = Url::parse(raw.trim()) {
        let host = url
            .host_str()
            .unwrap_or_default()
            .trim_start_matches("www.");
        let path = url.path().trim_matches('/').trim_end_matches(".git");
        return [host, path]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("/");
    }
    raw.trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.")
        .trim_end_matches(".git")
        .trim_matches('/')
        .to_string()
}

#[allow(non_snake_case)]
fn buildQualifiedSearchQuery(
    owner: &str,
    repo: &str,
    label: &str,
    rawQuery: &str,
    openOnly: bool,
) -> String {
    let mut query = String::new();
    query.push_str(rawQuery);
    query.push_str(" repo:");
    query.push_str(owner);
    query.push('/');
    query.push_str(repo);
    query.push_str(" is:issue");
    if openOnly {
        query.push_str(" is:open");
    }
    if !label.trim().is_empty() {
        query.push_str(" label:");
        query.push('"');
        query.push_str(&label.replace('"', "\\\""));
        query.push('"');
    }
    query
}

#[allow(non_snake_case)]
fn summarizeBody(body: &str) -> String {
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

#[allow(non_snake_case)]
fn defaultPage() -> i32 {
    1
}

#[allow(non_snake_case)]
fn openState() -> String {
    "open".to_string()
}
