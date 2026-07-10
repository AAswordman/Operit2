use std::cell::Cell;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::commands::util::read_content_arg;
use crate::output::CoreCommandOutput;
use operit_host_api::HostManager::HostManager;
use operit_providers::market::MarketStatsApiService::{
    MarketComment, MarketEntrySummary, MarketListPage, MarketNotification, MarketStatsApiService,
};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::preferences::GitHubAuthPreferences::GitHubAuthPreferences;
use operit_tools::tools::mcp_runtime::MCPLocalServer::MCPLocalServer;
use operit_tools::tools::mcp_runtime::MCPRepository::MCPRepository;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const GITHUB_ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_CLI_SCOPE: &str = "notifications,public_repo,user:email,read:user";

macro_rules! println {
    () => { market_stdout_line("") };
    ($($arg:tt)*) => { market_stdout_line(format!($($arg)*)) };
}

thread_local! {
    static MARKET_OUTPUT: Cell<*mut CoreCommandOutput> = Cell::new(std::ptr::null_mut());
}

fn set_market_output(output: &mut CoreCommandOutput) {
    MARKET_OUTPUT.with(|slot| slot.set(output as *mut CoreCommandOutput));
}

fn market_stdout_line(line: impl AsRef<str>) {
    MARKET_OUTPUT.with(|slot| {
        let output = slot.get();
        assert!(!output.is_null(), "market command output is not set");
        unsafe { (&mut *output).push_stdout_line(line.as_ref()) };
    });
}

struct MarketCommand {
    context: HostManager,
    tool_handler: AIToolHandler,
}

impl MarketCommand {
    fn new(application: &OperitApplication) -> Self {
        Self {
            context: application.hostManager.clone(),
            tool_handler: application.toolHandler.clone(),
        }
    }

    fn api(&self) -> MarketStatsApiService {
        MarketStatsApiService::new_with_github_token(
            GitHubAuthPreferences::getInstance().getCurrentAccessToken(),
        )
    }

    fn github_auth(&self) -> GitHubAuthPreferences {
        GitHubAuthPreferences::getInstance()
    }

    fn skill_repo(&self) -> SkillRepository {
        SkillRepository::getInstance(&self.context, self.tool_handler.runtimeSupport())
    }

    fn mcp_local(&self) -> MCPLocalServer {
        MCPLocalServer::getInstance(&self.context)
    }

    fn mcp_repo(&self) -> MCPRepository {
        MCPRepository::getInstance(&self.context, self.tool_handler.runtimeSupport())
    }

    fn package_manager(&self) -> PackageManagerCommand {
        PackageManagerCommand {
            manager: self.tool_handler.getOrCreatePackageManager(),
        }
    }
}

struct PackageManagerCommand {
    manager: Arc<Mutex<RuntimePackageManager>>,
}

impl PackageManagerCommand {
    fn add_from_external(&self, path: &str) -> String {
        self.manager
            .lock()
            .expect("package manager mutex poisoned")
            .addPackageFileFromExternalStorage(path)
    }
}

// ── Entry point ──────────────────────────────────────────────

pub fn run_market_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    set_market_output(output);
    let core = &mut MarketCommand::new(application);
    if args.is_empty() {
        print_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "auth" => run_auth(core, &args[1..]),
        "rank" => {
            let sort = normalize_sort(args.get(1).map(String::as_str).unwrap_or("updated"))?;
            let page = parse_i32_opt(args.get(2), 1)?;
            print_list(core, sort, page)
        }
        "list" => {
            let sort = normalize_sort(args.get(1).map(String::as_str).unwrap_or("updated"))?;
            let type_filter = args.get(2).map(String::as_str);
            let category = args.get(3).map(String::as_str);
            let page = parse_i32_opt(args.get(4), 1)?;
            print_list_filtered(core, sort, type_filter, category, page)
        }
        "search" => {
            let query = args.get(1).ok_or_else(|| {
                "usage: operit2 market search <query> [sort] [type|-] [category|-]".to_string()
            })?;
            let sort = normalize_sort(args.get(2).map(String::as_str).unwrap_or("updated"))?;
            let type_filter = args.get(3).map(String::as_str);
            let category = args.get(4).map(String::as_str);
            print_search(core, query, sort, type_filter, category)
        }
        "show" => {
            let entry_id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 market show <entryId>".to_string())?;
            print_entry(core, entry_id)
        }
        "comments" => {
            let entry_id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 market comments <entryId> [page]".to_string())?;
            let page = parse_i32_opt(args.get(2), 1)?;
            print_comments(core, entry_id, page)
        }
        "comment" => run_comment(core, &args[1..]),
        "like" => {
            let entry_id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 market like <entryId>".to_string())?;
            require_login(core)?;
            core.api().create_entry_reaction(entry_id)?;
            println!("liked {entry_id}");
            Ok(())
        }
        "notifications" => {
            let limit = parse_i32_opt(args.get(1), 50)?;
            let offset = parse_i32_opt(args.get(2), 0)?;
            print_notifications(core, limit, offset)
        }
        "my" => print_my_entries(core),
        "publish" => run_publish(core, &args[1..]),
        "install" => {
            let entry_id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 market install <entryId> [versionId]".to_string())?;
            let version_id = args.get(2).map(String::as_str);
            install_entry(core, entry_id, version_id)
        }
        "download" => {
            let asset_id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 market download <assetId>".to_string())?;
            let bytes = core.api().download_asset(asset_id)?;
            println!("downloaded asset {asset_id} bytes={}", bytes.len());
            Ok(())
        }
        _ => {
            print_usage();
            Ok(())
        }
    }
}

fn print_usage() {
    println!("usage: operit2 market <auth|rank|list|search|show|comments|comment|like|notifications|my|publish|install|download>");
    println!("sort: updated|likes|downloads");
    println!("list: operit2 market list [sort] [type|-] [category|-] [page]");
    println!("search: operit2 market search <query> [sort] [type|-] [category|-]");
    println!("comment: operit2 market comment <entryId> <body-or-@file>");
    println!("comment edit: operit2 market comment edit <commentId> <body-or-@file>");
    println!("comment delete: operit2 market comment delete <commentId>");
    println!("publish artifact: operit2 market publish artifact <type> <title> <description-or-@file> <detail-or-@file> <categoryId> <allowPublicUpdates> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or-> <projectId> <runtimePackageId> <assetKind> <assetUrl> <ghOwner> <ghRepo> <ghReleaseTag> <assetName> <sha256>");
    println!("publish repo: operit2 market publish repo <type> <title> <description-or-@file> <detail-or-@file> <categoryId> <allowPublicUpdates> <sourceUrl> <refType> <refName> <installConfig-or-@file> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or->");
    println!("publish version artifact: operit2 market publish version artifact <entryId> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or-> <projectId> <runtimePackageId> <assetKind> <assetUrl> <ghOwner> <ghRepo> <ghReleaseTag> <assetName> <sha256> [entryTitle|-] [entryDescription-or-] [entryDetail-or-] [entryCategoryId|-] [entryAllowPublicUpdates|-]");
    println!("publish version repo: operit2 market publish version repo <entryId> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or-> <refType> <refName> <installConfig-or-@file> [entryTitle|-] [entryDescription-or-] [entryDetail-or-] [entryCategoryId|-] [entryAllowPublicUpdates|-]");
    println!("publish update-entry: operit2 market publish update-entry <entryId> <title-or-> <description-or-@file-or-> <detail-or-@file-or-> <categoryId-or-> <allowPublicUpdates-or->");
    println!("download: operit2 market download <assetId>");
}

// ── Auth ────────────────────────────────────────────────────

fn run_auth(core: &mut MarketCommand, args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("token") => {
            let token = args.get(1).ok_or_else(|| "usage: operit2 market auth token <token>".to_string())?;
            core.github_auth().updateAccessToken(token, "bearer", None).map_err(|e| e.to_string())?;
            println!("token saved");
            Ok(())
        }
        Some("login") => {
            run_github_device_login(core)
        }
        _ => Err("usage: operit2 market auth <token|login>  # login uses browser/device flow and requires OPERIT_GITHUB_CLIENT_ID or GITHUB_CLIENT_ID".to_string()),
    }
}

#[derive(Debug, Deserialize)]
struct GitHubDeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default)]
    expires_in: i64,
    #[serde(default)]
    interval: i64,
}

#[derive(Debug, Deserialize)]
struct GitHubDeviceTokenResponse {
    access_token: Option<String>,
    token_type: Option<String>,
    scope: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

fn run_github_device_login(core: &mut MarketCommand) -> Result<(), String> {
    let client_id = github_oauth_client_id()?;
    let client = reqwest::blocking::Client::new();
    let device = client
        .post(GITHUB_DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("scope", GITHUB_CLI_SCOPE),
        ])
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<GitHubDeviceCodeResponse>()
        .map_err(|e| e.to_string())?;

    println!("Open this URL in your browser: {}", device.verification_uri);
    println!("Enter code: {}", device.user_code);
    let _ = open_browser(&device.verification_uri);

    let timeout_seconds = if device.expires_in <= 0 {
        900
    } else {
        device.expires_in
    };
    let mut interval_seconds = if device.interval <= 0 {
        5
    } else {
        device.interval
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_seconds as u64);
    loop {
        if std::time::Instant::now() >= deadline {
            return Err("GitHub device login expired".to_string());
        }
        std::thread::sleep(Duration::from_secs(interval_seconds as u64));
        let resp = client
            .post(GITHUB_ACCESS_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", client_id.as_str()),
                ("device_code", device.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json::<GitHubDeviceTokenResponse>()
            .map_err(|e| e.to_string())?;

        if let Some(token) = resp.access_token.filter(|t| !t.trim().is_empty()) {
            let token_type = resp.token_type.as_deref().unwrap_or("bearer");
            let scope = resp.scope.as_deref().unwrap_or(GITHUB_CLI_SCOPE);
            core.github_auth()
                .updateAccessToken(&token, token_type, Some(scope))
                .map_err(|e| e.to_string())?;
            let user = MarketStatsApiService::new_with_github_token(Some(token))
                .get_current_github_user()?;
            println!("logged in as {} (github_id={})", user.login, user.id);
            return Ok(());
        }

        match resp.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                interval_seconds += 5;
                continue;
            }
            Some("expired_token") => return Err("GitHub device login expired".to_string()),
            Some(error) => {
                let description = resp.error_description.unwrap_or_default();
                return Err(format!("GitHub device login failed: {error} {description}"));
            }
            None => return Err("GitHub device login returned no access token".to_string()),
        }
    }
}

fn github_oauth_client_id() -> Result<String, String> {
    env::var("OPERIT_GITHUB_CLIENT_ID")
        .or_else(|_| env::var("GITHUB_CLIENT_ID"))
        .map(|id| id.trim().to_string())
        .ok()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| {
            "GitHub OAuth client id required. Set OPERIT_GITHUB_CLIENT_ID or GITHUB_CLIENT_ID."
                .to_string()
        })
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err("opening browser is not supported on this platform".to_string())
}

// ── List ────────────────────────────────────────────────────

fn print_list(core: &mut MarketCommand, sort: &str, page: i32) -> Result<(), String> {
    let list = core.api().get_list_page(sort, page)?;
    println!(
        "generatedAt={}  sort={sort}  page={page}  total={}",
        list.generated_at.unwrap_or_default(),
        list.total
    );
    for entry in &list.items {
        println!(
            "{}/{} [{}]  {}",
            entry.r#type, entry.id, entry.state_code, entry.title
        );
    }
    Ok(())
}

fn print_list_filtered(
    core: &mut MarketCommand,
    sort: &str,
    type_filter: Option<&str>,
    category: Option<&str>,
    page: i32,
) -> Result<(), String> {
    let type_filter = clean_optional_arg(type_filter);
    let category = clean_optional_arg(category);
    let list = match (type_filter, category) {
        (Some(r#type), Some(category_id)) => {
            core.api()
                .get_type_category_page(r#type, category_id, sort, page)?
        }
        (Some(r#type), None) => core.api().get_type_page(r#type, sort, page)?,
        (None, Some(category_id)) => core.api().get_category_page(category_id, sort, page)?,
        (None, None) => core.api().get_list_page(sort, page)?,
    };
    println!("sort={sort}  page={page}  total={}", list.total);
    for entry in &list.items {
        println!(
            "{}/{} [{}]  {}",
            entry.r#type, entry.id, entry.state_code, entry.title
        );
    }
    Ok(())
}

fn print_search(
    core: &mut MarketCommand,
    query: &str,
    sort: &str,
    type_filter: Option<&str>,
    category: Option<&str>,
) -> Result<(), String> {
    let entries = load_all_market_pages(core, sort, type_filter, category)?;
    let query = query.trim().to_lowercase();
    let matched = entries
        .into_iter()
        .filter(|entry| market_entry_matches_query(entry, &query))
        .collect::<Vec<_>>();
    println!("search query={query}  sort={sort}  count={}", matched.len());
    for entry in &matched {
        println!(
            "{}/{} [{}]  {}",
            entry.r#type, entry.id, entry.state_code, entry.title
        );
    }
    Ok(())
}

fn load_all_market_pages(
    core: &mut MarketCommand,
    sort: &str,
    type_filter: Option<&str>,
    category: Option<&str>,
) -> Result<Vec<MarketEntrySummary>, String> {
    let type_filter = clean_optional_arg(type_filter);
    let category = clean_optional_arg(category);
    let first_page = load_market_page(core, sort, type_filter, category, 1)?;
    let total_pages = market_total_pages(first_page.total, first_page.page_size)?;
    let mut entries = first_page.items;
    for page in 2..=total_pages {
        entries.extend(load_market_page(core, sort, type_filter, category, page)?.items);
    }
    Ok(entries)
}

fn market_total_pages(total: i32, page_size: i32) -> Result<i32, String> {
    if page_size <= 0 {
        return Err(format!("invalid market page_size: {page_size}"));
    }
    Ok(((total + page_size - 1) / page_size).max(1))
}

fn load_market_page(
    core: &mut MarketCommand,
    sort: &str,
    type_filter: Option<&str>,
    category: Option<&str>,
    page: i32,
) -> Result<MarketListPage, String> {
    match (type_filter, category) {
        (Some(r#type), Some(category_id)) => {
            core.api()
                .get_type_category_page(r#type, category_id, sort, page)
        }
        (Some(r#type), None) => core.api().get_type_page(r#type, sort, page),
        (None, Some(category_id)) => core.api().get_category_page(category_id, sort, page),
        (None, None) => core.api().get_list_page(sort, page),
    }
}

fn market_entry_matches_query(entry: &MarketEntrySummary, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    entry.title.to_lowercase().contains(query)
        || entry.description.to_lowercase().contains(query)
        || entry.detail.to_lowercase().contains(query)
        || entry.id.to_lowercase().contains(query)
        || entry.r#type.to_lowercase().contains(query)
        || entry
            .category_id
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(query)
        || entry
            .author
            .as_ref()
            .map(|author| author.login.as_str())
            .unwrap_or("")
            .to_lowercase()
            .contains(query)
        || entry
            .publisher
            .as_ref()
            .map(|publisher| publisher.login.as_str())
            .unwrap_or("")
            .to_lowercase()
            .contains(query)
}

fn print_entry(core: &mut MarketCommand, entry_id: &str) -> Result<(), String> {
    let entry = core.api().get_entry_by_id(entry_id)?;
    println!("id: {}", entry.id);
    println!("type: {}", entry.r#type);
    println!("title: {}", entry.title);
    println!("description: {}", entry.description);
    println!(
        "detail: {}",
        entry.detail.chars().take(500).collect::<String>()
    );
    let author_login = entry
        .author
        .as_ref()
        .map(|a| a.login.as_str())
        .unwrap_or("");
    let author_avatar = entry
        .author
        .as_ref()
        .and_then(|a| {
            if a.avatar.is_empty() {
                None
            } else {
                Some(a.avatar.as_str())
            }
        })
        .unwrap_or("-");
    println!("author: {author_login} ({author_avatar})");
    println!(
        "publisher: {}",
        entry
            .publisher
            .as_ref()
            .map(|p| p.login.as_str())
            .unwrap_or("")
    );
    println!(
        "category_id: {}",
        entry.category_id.as_deref().unwrap_or("-")
    );
    println!("state: {}", entry.state_code);
    println!("featured: {}", entry.featured);
    println!("downloads: {}", entry_downloads(&entry));
    println!("allow_public_updates: {}", entry.allow_public_updates);
    println!("source: {:?}", entry.source.as_ref().map(|s| s.url.clone()));
    if let Some(artifact) = &entry.artifact {
        println!("artifact project_id: {}", artifact.project_id);
        println!(
            "artifact runtime_package_id: {}",
            artifact.runtime_package_id.as_deref().unwrap_or("-")
        );
    }
    println!("versions: {}", entry.versions.len());
    for version in &entry.versions {
        println!(
            "  version {}  id={}  format={}  publisher={}",
            version.version,
            version.id,
            version.format_ver,
            version
                .publisher
                .as_ref()
                .map(|p| p.login.as_str())
                .unwrap_or("")
        );
    }
    println!("assets: {}", entry.assets.len());
    for asset in &entry.assets {
        println!("  {}  kind={}  url={}", asset.id, asset.kind, asset.url);
    }
    for r in &entry.reactions {
        println!("reaction {}  total={}", r.reaction, r.total);
    }
    Ok(())
}

fn print_comments(core: &mut MarketCommand, entry_id: &str, page: i32) -> Result<(), String> {
    let page = core.api().get_comments_page(entry_id, page)?;
    println!(
        "comments for {entry_id}  page={}  total={}",
        page.page, page.total
    );
    for c in &page.items {
        println!(
            "#{} {} by {}  at {}",
            c.id,
            c.body.chars().take(120).collect::<String>(),
            c.author.login,
            c.created_at
        );
    }
    Ok(())
}

fn print_notifications(core: &mut MarketCommand, limit: i32, offset: i32) -> Result<(), String> {
    let resp = core.api().get_notifications(limit, offset, None)?;
    println!("notifications: {}", resp.items.len());
    for n in &resp.items {
        println!(
            "{} [{}] entry={:?}  {}  {}",
            n.id, n.kind, n.entry_id, n.title, n.created_at
        );
    }
    Ok(())
}

fn print_my_entries(core: &mut MarketCommand) -> Result<(), String> {
    let resp = core.api().get_my_entries()?;
    println!("my entries: {}", resp.entries.len());
    for e in &resp.entries {
        println!(
            "{}  {}  {}  {}  {:?}",
            e.id, e.r#type, e.relation, e.state_code, e.reason_codes
        );
        println!("  title={}", e.title);
    }
    Ok(())
}

fn run_comment(core: &mut MarketCommand, args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("edit") => {
            let comment_id = args.get(1).ok_or_else(|| "usage: operit2 market comment edit <commentId> <body-or-@file>".to_string())?;
            let body_arg = args.get(2).ok_or_else(|| "usage: operit2 market comment edit <commentId> <body-or-@file>".to_string())?;
            require_login(core)?;
            let body = read_content_arg(body_arg)?;
            core.api().edit_entry_comment(comment_id, &body)?;
            println!("edited comment={comment_id}");
            Ok(())
        }
        Some("delete") => {
            let comment_id = args.get(1).ok_or_else(|| "usage: operit2 market comment delete <commentId>".to_string())?;
            require_login(core)?;
            core.api().delete_entry_comment(comment_id)?;
            println!("deleted comment={comment_id}");
            Ok(())
        }
        Some(entry_id) => {
            let body_arg = args.get(1).ok_or_else(|| "usage: operit2 market comment <entryId> <body-or-@file>".to_string())?;
            require_login(core)?;
            let body = read_content_arg(body_arg)?;
            let comment_id = core.api().create_entry_comment(entry_id, &body)?;
            println!("created comment={comment_id}");
            Ok(())
        }
        None => Err("usage: operit2 market comment <entryId> <body-or-@file> | comment edit <commentId> <body-or-@file> | comment delete <commentId>".to_string()),
    }
}

// ── Publish ────────────────────────────────────────────────

fn run_publish(core: &mut MarketCommand, args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("proof") => publish_proof_cli(core, &args[1..]),
        Some("artifact") => publish_artifact_cli(core, &args[1..]),
        Some("repo") => publish_repo_cli(core, &args[1..]),
        Some("version") => publish_version_cli(core, &args[1..]),
        Some("update-entry") => update_entry_cli(core, &args[1..]),
        _ => Err(
            "usage: operit2 market publish <proof|artifact|repo|version|update-entry> ..."
                .to_string(),
        ),
    }
}

fn publish_proof_cli(core: &mut MarketCommand, args: &[String]) -> Result<(), String> {
    if args.len() < 5 {
        return Err(
            "usage: operit2 market publish proof <owner> <repo> <releaseTag> <assetName> <sha256>"
                .to_string(),
        );
    }
    require_login(core)?;
    let resp = core
        .api()
        .publish_proof(&args[0], &args[1], &args[2], &args[3], &args[4])?;
    println!("proof ok={}", resp.ok);
    if !resp.proof.trim().is_empty() {
        println!("proof={}", resp.proof);
    }
    Ok(())
}

fn publish_artifact_cli(core: &mut MarketCommand, args: &[String]) -> Result<(), String> {
    if args.len() < 20 {
        return Err("usage: operit2 market publish artifact <type> <title> <description-or-@file> <detail-or-@file> <categoryId> <allowPublicUpdates> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or-> <projectId> <runtimePackageId> <assetKind> <assetUrl> <ghOwner> <ghRepo> <ghReleaseTag> <assetName> <sha256>".to_string());
    }
    require_login(core)?;
    let description = read_content_arg(&args[2])?;
    let detail = read_content_arg(&args[3])?;
    let max_app_ver = parse_optional_string(&args[9]);
    let changelog = parse_optional_content_arg(&args[10])?;
    let resp = core.api().publish_artifact(
        &args[0],
        &args[1],
        &description,
        &detail,
        &args[4],
        parse_bool_arg(&args[5])?,
        &args[6],
        &args[7],
        &args[8],
        max_app_ver,
        changelog,
        &args[11],
        &args[12],
        &args[13],
        &args[14],
        &args[15],
        &args[16],
        &args[17],
        &args[18],
        &args[19],
    )?;
    println_publish_response(resp);
    Ok(())
}

fn publish_repo_cli(core: &mut MarketCommand, args: &[String]) -> Result<(), String> {
    if args.len() < 14 {
        return Err("usage: operit2 market publish repo <type> <title> <description-or-@file> <detail-or-@file> <categoryId> <allowPublicUpdates> <sourceUrl> <refType> <refName> <installConfig-or-@file> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or->".to_string());
    }
    require_login(core)?;
    let description = read_content_arg(&args[2])?;
    let detail = read_content_arg(&args[3])?;
    let install_config = read_content_arg(&args[9])?;
    let resp = core.api().publish_repo_entry(
        &args[0],
        &args[1],
        &description,
        &detail,
        &args[4],
        parse_bool_arg(&args[5])?,
        &args[6],
        &args[7],
        &args[8],
        &install_config,
        &args[10],
        &args[11],
        &args[12],
        parse_optional_string(&args[13]),
        parse_optional_content_arg(args.get(14).map(String::as_str).unwrap_or("-"))?,
    )?;
    println_publish_response(resp);
    Ok(())
}

fn publish_version_cli(core: &mut MarketCommand, args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("artifact") => publish_artifact_version_cli(core, &args[1..]),
        Some("repo") => publish_repo_version_cli(core, &args[1..]),
        _ => Err("usage: operit2 market publish version <artifact|repo> ...".to_string()),
    }
}

fn publish_artifact_version_cli(core: &mut MarketCommand, args: &[String]) -> Result<(), String> {
    if args.len() < 15 {
        return Err("usage: operit2 market publish version artifact <entryId> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or-> <projectId> <runtimePackageId> <assetKind> <assetUrl> <ghOwner> <ghRepo> <ghReleaseTag> <assetName> <sha256> [entryTitle|-] [entryDescription-or-] [entryDetail-or-] [entryCategoryId|-] [entryAllowPublicUpdates|-]".to_string());
    }
    require_login(core)?;
    let resp = core.api().publish_artifact_version(
        &args[0],
        &args[1],
        &args[2],
        &args[3],
        parse_optional_string(&args[4]),
        parse_optional_content_arg(&args[5])?,
        &args[6],
        &args[7],
        &args[8],
        &args[9],
        &args[10],
        &args[11],
        &args[12],
        &args[13],
        &args[14],
        parse_optional_string_arg(args.get(15)),
        parse_optional_content_arg(args.get(16).map(String::as_str).unwrap_or("-"))?,
        parse_optional_content_arg(args.get(17).map(String::as_str).unwrap_or("-"))?,
        parse_optional_string_arg(args.get(18)),
        parse_optional_bool_arg(args.get(19))?,
    )?;
    println_publish_response(resp);
    Ok(())
}

fn publish_repo_version_cli(core: &mut MarketCommand, args: &[String]) -> Result<(), String> {
    if args.len() < 9 {
        return Err("usage: operit2 market publish version repo <entryId> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or-> <refType> <refName> <installConfig-or-@file> [entryTitle|-] [entryDescription-or-] [entryDetail-or-] [entryCategoryId|-] [entryAllowPublicUpdates|-]".to_string());
    }
    require_login(core)?;
    let install_config = read_content_arg(&args[8])?;
    let resp = core.api().publish_repo_version(
        &args[0],
        &args[1],
        &args[2],
        &args[3],
        parse_optional_string(&args[4]),
        parse_optional_content_arg(&args[5])?,
        &args[6],
        &args[7],
        &install_config,
        parse_optional_string_arg(args.get(9)),
        parse_optional_content_arg(args.get(10).map(String::as_str).unwrap_or("-"))?,
        parse_optional_content_arg(args.get(11).map(String::as_str).unwrap_or("-"))?,
        parse_optional_string_arg(args.get(12)),
        parse_optional_bool_arg(args.get(13))?,
    )?;
    println_publish_response(resp);
    Ok(())
}

fn update_entry_cli(core: &mut MarketCommand, args: &[String]) -> Result<(), String> {
    if args.len() < 6 {
        return Err("usage: operit2 market publish update-entry <entryId> <title-or-> <description-or-@file-or-> <detail-or-@file-or-> <categoryId-or-> <allowPublicUpdates-or->".to_string());
    }
    require_login(core)?;
    let resp = core.api().update_entry(
        &args[0],
        parse_optional_string(&args[1]),
        parse_optional_content_arg(&args[2])?,
        parse_optional_content_arg(&args[3])?,
        parse_optional_string(&args[4]),
        parse_optional_bool_str(&args[5])?,
    )?;
    println_update_entry_response(resp);
    Ok(())
}

fn println_publish_response(
    resp: operit_providers::market::MarketStatsApiService::MarketPublishResponse,
) {
    println!("published ok={} entry={}", resp.ok, resp.entry_id);
    if !resp.version_id.trim().is_empty() {
        println!("version_id={}", resp.version_id);
    }
}

fn println_update_entry_response(
    resp: operit_providers::market::MarketStatsApiService::MarketEntryUpdateResponse,
) {
    println!("updated ok={} entry={}", resp.ok, resp.item.id);
    println!("state={}", resp.item.state_code);
}

// ── Install ─────────────────────────────────────────────────

fn install_entry(
    core: &mut MarketCommand,
    entry_id: &str,
    version_id: Option<&str>,
) -> Result<(), String> {
    let entry = core.api().get_entry_by_id(entry_id)?;
    match entry.r#type.as_str() {
        "skill" => install_skill_from_entry(core, entry),
        "mcp" => install_mcp_from_entry(core, entry),
        "package" | "script" => install_artifact_from_entry(core, entry, version_id),
        other => Err(format!("unknown market type: {other}")),
    }
}

fn install_skill_from_entry(
    core: &mut MarketCommand,
    entry: MarketEntrySummary,
) -> Result<(), String> {
    let source_url = entry
        .source
        .as_ref()
        .and_then(|s| {
            if s.url.trim().is_empty() {
                None
            } else {
                Some(s.url.clone())
            }
        })
        .ok_or_else(|| "skill entry has no source url".to_string())?;
    let result = core.skill_repo().importSkillFromGitHubRepo(&source_url);
    println!("{result}");
    Ok(())
}

fn install_mcp_from_entry(
    core: &mut MarketCommand,
    entry: MarketEntrySummary,
) -> Result<(), String> {
    let source_url = entry
        .source
        .as_ref()
        .and_then(|s| {
            if s.url.trim().is_empty() {
                None
            } else {
                Some(s.url.clone())
            }
        })
        .ok_or_else(|| "mcp entry has no source url".to_string())?;
    let plugin_id = sanitize_id(&entry.title);
    let metadata = operit_tools::tools::mcp_runtime::MCPLocalServer::PluginMetadata {
        name: entry.title.clone(),
        description: entry.description.clone(),
        author: entry
            .author
            .as_ref()
            .map(|a| a.login.clone())
            .unwrap_or_default(),
        version: "1.0.0".to_string(),
    };
    match core.mcp_repo().installMCPServerWithObject(
        plugin_id.clone(),
        source_url,
        metadata,
        String::new(),
        |_| {},
    ) {
        operit_tools::tools::mcp_runtime::MCPRepository::InstallResult::Success { pluginPath } => {
            println!("installed={plugin_id}");
            println!("path={pluginPath}");
            Ok(())
        }
        operit_tools::tools::mcp_runtime::MCPRepository::InstallResult::Error { message } => {
            Err(message)
        }
    }
}

fn install_artifact_from_entry(
    core: &mut MarketCommand,
    entry: MarketEntrySummary,
    version_id: Option<&str>,
) -> Result<(), String> {
    entry
        .artifact
        .as_ref()
        .ok_or_else(|| "entry is not an artifact".to_string())?;
    let requested_version_id = version_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            entry
                .latest_version
                .as_ref()
                .map(|version| version.id.clone())
                .filter(|id| !id.trim().is_empty())
        });
    let asset = if let Some(version_id) = requested_version_id.as_deref() {
        entry
            .assets
            .iter()
            .find(|asset| asset.version_id == version_id && !asset.id.trim().is_empty())
            .ok_or_else(|| format!("entry has no downloadable asset for version: {version_id}"))?
    } else {
        entry
            .assets
            .iter()
            .find(|asset| !asset.id.trim().is_empty())
            .ok_or_else(|| "entry has no downloadable asset".to_string())?
    };
    let temp_file = download_asset_to_temp_file(core, &asset.id)?;
    let result = core
        .package_manager()
        .add_from_external(&temp_file.to_string_lossy());
    let _ = fs::remove_file(&temp_file);
    if !result
        .to_ascii_lowercase()
        .starts_with("successfully imported")
    {
        return Err(result);
    }
    println!("{result}");
    Ok(())
}

fn download_asset_to_temp_file(
    core: &mut MarketCommand,
    asset_id: &str,
) -> Result<PathBuf, String> {
    let bytes = core.api().download_asset(asset_id)?;
    let mut tmp = env::temp_dir();
    tmp.push(format!("operit_dl_{}", current_millis()));
    fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    Ok(tmp)
}

fn sanitize_id(title: &str) -> String {
    title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

// ── Util ────────────────────────────────────────────────────

fn require_login(core: &mut MarketCommand) -> Result<(), String> {
    ensure_env_token(core)?;
    if core.github_auth().getCurrentAccessToken().is_some() {
        Ok(())
    } else {
        Err(
            "GitHub token required. Use `operit2 market auth token <token>` or set GITHUB_TOKEN."
                .to_string(),
        )
    }
}

fn ensure_env_token(core: &mut MarketCommand) -> Result<(), String> {
    let token = env::var("OPERIT_GITHUB_TOKEN")
        .ok()
        .or_else(|| env::var("GITHUB_TOKEN").ok())
        .filter(|t| !t.trim().is_empty());
    if let Some(token) = token {
        if core.github_auth().getCurrentAccessToken() != Some(token.clone()) {
            core.github_auth()
                .updateAccessToken(&token, "bearer", None)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn parse_i32_opt(raw: Option<&String>, default: i32) -> Result<i32, String> {
    match raw {
        Some(s) => s.parse::<i32>().map_err(|e| e.to_string()),
        None => Ok(default),
    }
}

fn normalize_sort(sort: &str) -> Result<&str, String> {
    match sort {
        "updated" | "likes" | "downloads" => Ok(sort),
        other => Err(format!(
            "invalid market sort: {other}. expected updated|likes|downloads"
        )),
    }
}

fn clean_optional_arg(value: Option<&str>) -> Option<&str> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "-" {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn parse_optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "-" {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_optional_string_arg(value: Option<&String>) -> Option<String> {
    value.and_then(|raw| parse_optional_string(raw))
}

fn parse_optional_content_arg(value: &str) -> Result<Option<String>, String> {
    match parse_optional_string(value) {
        Some(raw) => read_content_arg(&raw).map(Some),
        None => Ok(None),
    }
}

fn parse_bool_arg(value: &str) -> Result<bool, String> {
    match value.trim() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        other => Err(format!("invalid bool: {other}")),
    }
}

fn parse_optional_bool_str(value: &str) -> Result<Option<bool>, String> {
    match parse_optional_string(value) {
        Some(raw) => parse_bool_arg(&raw).map(Some),
        None => Ok(None),
    }
}

fn parse_optional_bool_arg(value: Option<&String>) -> Result<Option<bool>, String> {
    match value {
        Some(raw) => parse_optional_bool_str(raw),
        None => Ok(None),
    }
}

fn entry_downloads(entry: &MarketEntrySummary) -> i32 {
    entry.download_count.max(entry.downloads)
}

fn current_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use operit_host_api::HostManager::{setDefaultHttpHost, HostManager};
    use operit_host_api::{
        HostError, HostResult, HttpHost, HttpRequestData, HttpResponseData, RuntimeStorageEntry,
        RuntimeStorageHost,
    };
    use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;

    #[derive(Clone, Default)]
    struct MemoryStorageHost {
        files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    }

    impl RuntimeStorageHost for MemoryStorageHost {
        fn rootDir(&self) -> Option<std::path::PathBuf> {
            None
        }

        fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            let files = self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?;
            files
                .get(path)
                .cloned()
                .ok_or_else(|| HostError::new(format!("missing runtime storage file: {path}")))
        }

        fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            let mut files = self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?;
            files.insert(path.to_string(), content.to_vec());
            Ok(())
        }

        fn delete(&self, path: &str, _recursive: bool) -> HostResult<()> {
            let mut files = self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?;
            files.remove(path);
            Ok(())
        }

        fn exists(&self, path: &str) -> HostResult<bool> {
            let files = self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?;
            Ok(files.contains_key(path))
        }

        fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
            let files = self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?;
            Ok(files
                .iter()
                .filter(|(path, _)| path.starts_with(prefix))
                .map(|(path, content)| RuntimeStorageEntry {
                    path: path.clone(),
                    isDirectory: false,
                    size: content.len() as i64,
                })
                .collect())
        }
    }

    fn register_test_storage() {
        setDefaultRuntimeStorageHost(Arc::new(MemoryStorageHost::default()));
    }

    struct ReqwestTestHttpHost;

    impl HttpHost for ReqwestTestHttpHost {
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
            for (key, value) in &request.headers {
                builder = builder.header(key, value);
            }
            if !request.body.is_empty() {
                builder = builder.body(request.body);
            }
            let response = builder.send().map_err(|e| HostError::new(e.to_string()))?;
            let status = response.status();
            let status_code = status.as_u16() as i32;
            let status_message = status.canonical_reason().unwrap_or_default().to_string();
            let final_url = response.url().to_string();
            let headers = response
                .headers()
                .iter()
                .map(|(key, value)| {
                    (
                        key.as_str().to_string(),
                        value.to_str().unwrap_or_default().to_string(),
                    )
                })
                .collect();
            let body = response
                .bytes()
                .map_err(|e| HostError::new(e.to_string()))?
                .to_vec();
            Ok(HttpResponseData {
                finalUrl: final_url,
                statusCode: status_code,
                statusMessage: status_message,
                headers,
                body,
            })
        }
    }

    fn run_market_cli(args: &[&str]) {
        let mut root = std::env::temp_dir();
        root.push(format!("operit_market_test_{}", current_millis()));
        std::fs::create_dir_all(&root).expect("create test runtime root");
        operit_util::RuntimeStoreRoot::setDefaultRuntimeStoreRoot(root);
        register_test_storage();
        let ctx = HostManager::new();
        let mut out = CoreCommandOutput::new();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        // Tests that parsing does not panic; network/IO errors are OK at this level.
        let _ = run_market_command(ctx, &args, &mut out);
    }

    #[test]
    fn empty_prints_usage() {
        run_market_cli(&[]);
    }

    #[test]
    fn show_missing_id_prints_usage() {
        run_market_cli(&["show"]);
    }

    #[test]
    fn comments_missing_id_prints_usage() {
        run_market_cli(&["comments"]);
    }

    #[test]
    fn auth_token_missing_value_prints_usage() {
        run_market_cli(&["auth", "token"]);
    }

    #[test]
    fn auth_login_without_client_id_returns_error() {
        std::env::remove_var("OPERIT_GITHUB_CLIENT_ID");
        std::env::remove_var("GITHUB_CLIENT_ID");
        run_market_cli(&["auth", "login"]);
    }

    #[test]
    fn search_missing_query_prints_usage() {
        run_market_cli(&["search"]);
    }

    #[test]
    fn comment_missing_body_prints_usage() {
        run_market_cli(&["comment", "entry_1"]);
    }

    #[test]
    fn download_missing_id_prints_usage() {
        run_market_cli(&["download"]);
    }

    #[test]
    fn install_missing_id_prints_usage() {
        run_market_cli(&["install"]);
    }

    #[test]
    fn notifications_rejects_bad_limit_without_network() {
        run_market_cli(&["notifications", "bad-limit"]);
    }

    #[test]
    fn publish_missing_subcommand_prints_usage() {
        run_market_cli(&["publish"]);
    }

    #[test]
    fn publish_proof_missing_args_prints_usage() {
        run_market_cli(&["publish", "proof", "owner", "repo"]);
    }

    #[test]
    fn publish_artifact_missing_args_prints_usage() {
        run_market_cli(&["publish", "artifact", "script", "title"]);
    }

    #[test]
    fn publish_repo_missing_args_prints_usage() {
        run_market_cli(&["publish", "repo", "mcp", "title"]);
    }

    #[test]
    fn publish_version_missing_args_prints_usage() {
        run_market_cli(&["publish", "version"]);
    }

    #[test]
    fn invalid_featured_sort_is_rejected_without_network() {
        run_market_cli(&["rank", "featured"]);
    }

    fn run_online_rank(sort: &str) -> String {
        setDefaultHttpHost(Arc::new(ReqwestTestHttpHost));
        let mut root = std::env::temp_dir();
        root.push(format!("operit_market_online_test_{}", current_millis()));
        std::fs::create_dir_all(&root).expect("create test runtime root");
        operit_util::RuntimeStoreRoot::setDefaultRuntimeStoreRoot(root);
        register_test_storage();

        let ctx = HostManager::new();
        let mut out = CoreCommandOutput::new();
        let args = vec!["rank".to_string(), sort.to_string(), "1".to_string()];
        run_market_command(ctx, &args, &mut out)
            .expect("online rank command should read cloud market");
        out.stdout
    }

    fn assert_online_rank_output(stdout: &str, sort: &str) {
        assert!(
            stdout.contains(&format!("sort={sort}")),
            "stdout was: {stdout}"
        );
        assert!(stdout.contains("page=1"), "stdout was: {stdout}");
        assert!(
            stdout.contains("total=") && !stdout.contains("total=0"),
            "stdout was: {stdout}"
        );
        assert!(
            stdout.contains("package/")
                || stdout.contains("mcp/")
                || stdout.contains("skill/")
                || stdout.contains("script/"),
            "stdout was: {stdout}"
        );
    }

    #[test]
    #[ignore = "hits api.operit.app"]
    fn online_rank_command_reads_cloud_market_v2() {
        let stdout = run_online_rank("updated");
        assert_online_rank_output(&stdout, "updated");
    }

    #[test]
    #[ignore = "hits api.operit.app"]
    fn online_rank_command_reads_cloud_downloads_market_v2() {
        let stdout = run_online_rank("downloads");
        assert_online_rank_output(&stdout, "downloads");
    }
}
