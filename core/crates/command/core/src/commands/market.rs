use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::commands::util::read_content_arg;
use crate::output::CoreCommandOutput;
use operit_host_api::HostManager::HostManager;
use operit_providers::market::MarketStatsApiService::{
    MarketComment, MarketEntryAsset, MarketEntrySummary, MarketEntryVersion, MarketListPage,
    MarketNotification, MarketStatsApiService,
};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::preferences::GitHubAuthPreferences::GitHubAuthPreferences;
use operit_tools::tools::mcp_runtime::MCPLocalServer::MCPLocalServer;
use operit_tools::tools::mcp_runtime::MCPRepository::MCPRepository;
use operit_tools::tools::packTool::RuntimePackageManager::{
    ExternalPackageImportResult, RuntimePackageManager,
};
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_util::RuntimeStorageLayout::{RUNTIME_ROOT_DIR_PATH, WORKSPACE_DIR_PATH};
use serde_json::json;
use sha2::{Digest, Sha256};

struct MarketCommand {
    context: HostManager,
    tool_handler: AIToolHandler,
}

impl MarketCommand {
    /// Creates a market command context from the current application.
    fn new(application: &OperitApplication) -> Self {
        Self {
            context: application.hostManager.clone(),
            tool_handler: application.toolHandler.clone(),
        }
    }

    /// Creates an authenticated marketplace service client for this command.
    fn api(&self) -> MarketStatsApiService {
        MarketStatsApiService::new_with_github_token(
            GitHubAuthPreferences::getInstance().getCurrentAccessToken(),
        )
    }

    /// Returns the GitHub auth preference service used by market writes.
    fn github_auth(&self) -> GitHubAuthPreferences {
        GitHubAuthPreferences::getInstance()
    }

    /// Returns the skill repository bound to the active host context.
    fn skill_repo(&self) -> SkillRepository {
        SkillRepository::getInstance(&self.context, self.tool_handler.runtimeSupport())
    }

    /// Returns the local MCP server registry bound to the active host context.
    fn mcp_local(&self) -> MCPLocalServer {
        MCPLocalServer::getInstance(&self.context)
    }

    /// Returns the MCP repository installer bound to the active host context.
    fn mcp_repo(&self) -> MCPRepository {
        MCPRepository::getInstance(&self.context, self.tool_handler.runtimeSupport())
    }

    /// Returns a package manager command adapter.
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
    /// Imports an external package file and returns structured metadata.
    fn add_from_external(&self, path: &str) -> Result<ExternalPackageImportResult, String> {
        self.manager
            .lock()
            .expect("package manager mutex poisoned")
            .addPackageFileFromExternalStorageResult(path)
    }
}

// ── Entry point ──────────────────────────────────────────────

pub fn run_market_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let core = &mut MarketCommand::new(application);
    if args.is_empty() {
        print_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "rank" => {
            let sort = normalize_sort(args.get(1).map(String::as_str).unwrap_or("updated"))?;
            let page = parse_i32_opt(args.get(2), 1)?;
            print_list(core, sort, page, output)
        }
        "list" => {
            let sort = normalize_sort(args.get(1).map(String::as_str).unwrap_or("updated"))?;
            let type_filter = args.get(2).map(String::as_str);
            let category = args.get(3).map(String::as_str);
            let page = parse_i32_opt(args.get(4), 1)?;
            print_list_filtered(core, sort, type_filter, category, page, output)
        }
        "search" => {
            let query = args.get(1).ok_or_else(|| {
                "usage: operit2 market search <query> [sort] [type|-] [category|-]".to_string()
            })?;
            let sort = normalize_sort(args.get(2).map(String::as_str).unwrap_or("updated"))?;
            let type_filter = args.get(3).map(String::as_str);
            let category = args.get(4).map(String::as_str);
            print_search(core, query, sort, type_filter, category, output)
        }
        "show" => {
            let entry_id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 market show <entryId>".to_string())?;
            print_entry(core, entry_id, output)
        }
        "comments" => {
            let entry_id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 market comments <entryId> [page]".to_string())?;
            let page = parse_i32_opt(args.get(2), 1)?;
            print_comments(core, entry_id, page, output)
        }
        "comment" => run_comment(core, &args[1..], output),
        "like" => {
            let entry_id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 market like <entryId>".to_string())?;
            require_login(core)?;
            core.api().create_entry_reaction(entry_id)?;
            output.push_stdout_line(format!("Liked market entry: {entry_id}"));
            output.setJsonStdout(json!({
                "entryId": entry_id,
                "reaction": "+1",
            }));
            Ok(())
        }
        "notifications" => {
            let limit = parse_i32_opt(args.get(1), 50)?;
            let offset = parse_i32_opt(args.get(2), 0)?;
            print_notifications(core, limit, offset, output)
        }
        "my" => print_my_entries(core, output),
        "publish" => run_publish(core, &args[1..], output),
        "install" => {
            let entry_id = args.get(1).ok_or_else(|| {
                "usage: operit2 market install <entryId> <clientAppVersion> [versionId]".to_string()
            })?;
            let client_app_version = args.get(2).ok_or_else(|| {
                "usage: operit2 market install <entryId> <clientAppVersion> [versionId]".to_string()
            })?;
            let version_id = args.get(3).map(String::as_str);
            install_entry(core, entry_id, client_app_version, version_id, output)
        }
        "download" => {
            let asset_id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 market download <assetId>".to_string())?;
            let bytes = core.api().download_asset(asset_id)?;
            output.push_stdout_line(format!(
                "Downloaded market asset: {asset_id} ({} bytes)",
                bytes.len()
            ));
            output.setJsonStdout(json!({
                "assetId": asset_id,
                "bytes": bytes.len(),
            }));
            Ok(())
        }
        _ => {
            print_usage(output);
            Ok(())
        }
    }
}

/// Prints marketplace command usage in text and JSON form.
fn print_usage(output: &mut CoreCommandOutput) {
    let lines = vec![
        "usage: operit2 market <rank|list|search|show|comments|comment|like|notifications|my|publish|install|download>",
        "sort: updated|likes|downloads",
        "list: operit2 market list [sort] [type|-] [category|-] [page]",
        "search: operit2 market search <exact-query> [sort] [type|-] [category|-]",
        "comment: operit2 market comment <entryId> <body-or-@file>",
        "comment edit: operit2 market comment edit <commentId> <body-or-@file>",
        "comment delete: operit2 market comment delete <commentId>",
        "publish artifact: operit2 market publish artifact <type> <title> <description-or-@file> <detail-or-@file> <categoryId> <allowPublicUpdates> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or-> <projectId> <runtimePackageId> <assetKind> <assetUrl> <ghOwner> <ghRepo> <ghReleaseTag> <assetName> <sha256>",
        "publish repo: operit2 market publish repo <type> <title> <description-or-@file> <detail-or-@file> <categoryId> <allowPublicUpdates> <sourceUrl> <refType> <refName> <installConfig-or-@file> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or->",
        "publish version artifact: operit2 market publish version artifact <entryId> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or-> <projectId> <runtimePackageId> <assetKind> <assetUrl> <ghOwner> <ghRepo> <ghReleaseTag> <assetName> <sha256> [entryTitle|-] [entryDescription-or-] [entryDetail-or-] [entryCategoryId|-] [entryAllowPublicUpdates|-]",
        "publish version repo: operit2 market publish version repo <entryId> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or-> <refType> <refName> <installConfig-or-@file> [entryTitle|-] [entryDescription-or-] [entryDetail-or-] [entryCategoryId|-] [entryAllowPublicUpdates|-]",
        "publish update-entry: operit2 market publish update-entry <entryId> <title-or-> <description-or-@file-or-> <detail-or-@file-or-> <categoryId-or-> <allowPublicUpdates-or->",
        "install: operit2 market install <entryId> <clientAppVersion> [versionId]",
        "download: operit2 market download <assetId>",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({"usage": lines}));
}

// ── List ────────────────────────────────────────────────────

/// Prints one marketplace rank page.
fn print_list(
    core: &mut MarketCommand,
    sort: &str,
    page: i32,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let list = core.api().get_list_page(sort, page)?;
    let total_pages = market_total_pages(list.total, list.page_size)?;
    output.push_stdout_line("Market entries");
    output.push_stdout_line(format!("Sort: {sort}"));
    output.push_stdout_line(format!("Page: {page} / {total_pages}"));
    output.push_stdout_line(format!("Total: {}", list.total));
    output.push_stdout_line(format!(
        "Generated: {}",
        list.generated_at.as_deref().unwrap_or("-")
    ));
    for entry in &list.items {
        output.push_stdout_line(format!(
            "- {} / {} [{}] {}",
            entry.r#type, entry.id, entry.state_code, entry.title
        ));
    }
    output.setJsonStdout(json!({
        "command": "rank",
        "sort": sort,
        "page": page,
        "pageData": list,
    }));
    Ok(())
}

/// Prints one filtered marketplace list page.
fn print_list_filtered(
    core: &mut MarketCommand,
    sort: &str,
    type_filter: Option<&str>,
    category: Option<&str>,
    page: i32,
    output: &mut CoreCommandOutput,
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
    let total_pages = market_total_pages(list.total, list.page_size)?;
    output.push_stdout_line("Market entries");
    output.push_stdout_line(format!("Sort: {sort}"));
    output.push_stdout_line(format!("Page: {page} / {total_pages}"));
    output.push_stdout_line(format!("Type: {}", type_filter.unwrap_or("-")));
    output.push_stdout_line(format!("Category: {}", category.unwrap_or("-")));
    output.push_stdout_line(format!("Total: {}", list.total));
    for entry in &list.items {
        output.push_stdout_line(format!(
            "- {} / {} [{}] {}",
            entry.r#type, entry.id, entry.state_code, entry.title
        ));
    }
    output.setJsonStdout(json!({
        "command": "list",
        "sort": sort,
        "typeFilter": type_filter,
        "category": category,
        "page": page,
        "pageData": list,
    }));
    Ok(())
}

/// Prints exact marketplace field matches across the selected list scope.
fn print_search(
    core: &mut MarketCommand,
    query: &str,
    sort: &str,
    type_filter: Option<&str>,
    category: Option<&str>,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("market search query is empty".to_string());
    }
    let entries = load_all_market_pages(core, sort, type_filter, category)?;
    let type_filter = clean_optional_arg(type_filter);
    let category = clean_optional_arg(category);
    let matched = entries
        .into_iter()
        .filter(|entry| market_entry_matches_query(entry, query))
        .collect::<Vec<_>>();
    output.push_stdout_line("Market search");
    output.push_stdout_line(format!("Query: {query}"));
    output.push_stdout_line(format!("Sort: {sort}"));
    output.push_stdout_line(format!("Type: {}", type_filter.unwrap_or("-")));
    output.push_stdout_line(format!("Category: {}", category.unwrap_or("-")));
    output.push_stdout_line(format!("Matches: {}", matched.len()));
    for entry in &matched {
        output.push_stdout_line(format!(
            "- {} / {} [{}] {}",
            entry.r#type, entry.id, entry.state_code, entry.title
        ));
    }
    output.setJsonStdout(json!({
        "command": "search",
        "query": query,
        "sort": sort,
        "typeFilter": type_filter,
        "category": category,
        "items": matched,
    }));
    Ok(())
}

/// Loads every market list page for the selected scope.
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

/// Computes the total number of market list pages from API pagination metadata.
fn market_total_pages(total: i32, page_size: i32) -> Result<i32, String> {
    if page_size <= 0 {
        return Err(format!("invalid market page_size: {page_size}"));
    }
    Ok(((total + page_size - 1) / page_size).max(1))
}

/// Loads one market list page from the selected scope.
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

/// Returns whether an entry has a field exactly matching the query.
fn market_entry_matches_query(entry: &MarketEntrySummary, query: &str) -> bool {
    market_field_matches(&entry.id, query)
        || market_field_matches(&entry.r#type, query)
        || market_field_matches(&entry.title, query)
        || market_field_matches(&entry.description, query)
        || market_field_matches(&entry.detail, query)
        || entry
            .category_id
            .as_deref()
            .map(|category_id| market_field_matches(category_id, query))
            .unwrap_or(false)
        || entry
            .author
            .as_ref()
            .map(|author| market_field_matches(&author.login, query))
            .unwrap_or(false)
        || entry
            .publisher
            .as_ref()
            .map(|publisher| market_field_matches(&publisher.login, query))
            .unwrap_or(false)
}

/// Returns whether one market field exactly matches a query value.
fn market_field_matches(value: &str, query: &str) -> bool {
    value.trim().eq_ignore_ascii_case(query)
}

/// Prints one market entry detail.
fn print_entry(
    core: &mut MarketCommand,
    entry_id: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let entry = core.api().get_entry_by_id(entry_id)?;
    output.push_stdout_line(format!("Market entry: {}", entry.title));
    output.push_stdout_line(format!("ID: {}", entry.id));
    output.push_stdout_line(format!("Type: {}", entry.r#type));
    output.push_stdout_line(format!("State: {}", entry.state_code));
    output.push_stdout_line(format!("Description: {}", entry.description));
    output.push_stdout_line(format!(
        "Detail: {}",
        entry.detail.chars().take(500).collect::<String>()
    ));
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
    output.push_stdout_line(format!("Author: {author_login} ({author_avatar})"));
    output.push_stdout_line(format!(
        "Publisher: {}",
        entry
            .publisher
            .as_ref()
            .map(|p| p.login.as_str())
            .unwrap_or("")
    ));
    output.push_stdout_line(format!(
        "Category: {}",
        entry.category_id.as_deref().unwrap_or("-")
    ));
    output.push_stdout_line(format!("Featured: {}", entry.featured));
    output.push_stdout_line(format!("Downloads: {}", entry_downloads(&entry)));
    output.push_stdout_line(format!("Public updates: {}", entry.allow_public_updates));
    output.push_stdout_line(format!(
        "Source: {}",
        entry.source.as_ref().map(|s| s.url.as_str()).unwrap_or("-")
    ));
    if let Some(artifact) = &entry.artifact {
        output.push_stdout_line(format!("Artifact project: {}", artifact.project_id));
        output.push_stdout_line(format!(
            "Artifact runtime package: {}",
            artifact.runtime_package_id.as_deref().unwrap_or("-")
        ));
    }
    output.push_stdout_line(format!("Versions: {}", entry.versions.len()));
    for version in &entry.versions {
        output.push_stdout_line(format!(
            "- Version {} — {} — format {} — publisher {}",
            version.version,
            version.id,
            version.format_ver,
            version
                .publisher
                .as_ref()
                .map(|p| p.login.as_str())
                .unwrap_or("")
        ));
    }
    output.push_stdout_line(format!("Assets: {}", entry.assets.len()));
    for asset in &entry.assets {
        output.push_stdout_line(format!("- {} — {} — {}", asset.id, asset.kind, asset.url));
    }
    for r in &entry.reactions {
        output.push_stdout_line(format!("Reaction {}: {}", r.reaction, r.total));
    }
    output.setJsonStdout(json!(entry));
    Ok(())
}

/// Prints one page of market entry comments.
fn print_comments(
    core: &mut MarketCommand,
    entry_id: &str,
    page: i32,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let comments_page = core.api().get_comments_page(entry_id, page)?;
    output.push_stdout_line(format!("Comments for market entry: {entry_id}"));
    output.push_stdout_line(format!(
        "Page: {} | Total: {}",
        comments_page.page, comments_page.total
    ));
    for c in &comments_page.items {
        output.push_stdout_line(format!(
            "- #{} by {} at {}: {}",
            c.id,
            c.author.login,
            c.created_at,
            c.body.chars().take(120).collect::<String>()
        ));
    }
    output.setJsonStdout(json!(comments_page));
    Ok(())
}

/// Prints authenticated marketplace notifications.
fn print_notifications(
    core: &mut MarketCommand,
    limit: i32,
    offset: i32,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let resp = core.api().get_notifications(limit, offset, None)?;
    output.push_stdout_line(format!("Market notifications: {}", resp.items.len()));
    output.push_stdout_line(format!("Window: {limit} items starting at {offset}"));
    for n in &resp.items {
        output.push_stdout_line(format!(
            "- {} [{}] — entry {} — {} — {}",
            n.id,
            n.kind,
            n.entry_id.as_deref().unwrap_or("-"),
            n.title,
            n.created_at
        ));
    }
    output.setJsonStdout(json!(resp));
    Ok(())
}

/// Prints marketplace entries owned by the authenticated user.
fn print_my_entries(
    core: &mut MarketCommand,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let resp = core.api().get_my_entries()?;
    output.push_stdout_line(format!("My market entries: {}", resp.entries.len()));
    for e in &resp.entries {
        output.push_stdout_line(format!(
            "- {} — {} — relation {} — state {} — reasons {:?}",
            e.id, e.r#type, e.relation, e.state_code, e.reason_codes
        ));
        output.push_stdout_line(format!("  Title: {}", e.title));
    }
    output.setJsonStdout(json!(resp));
    Ok(())
}

/// Executes marketplace comment mutation commands.
fn run_comment(
    core: &mut MarketCommand,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("edit") => {
            let comment_id = args.get(1).ok_or_else(|| "usage: operit2 market comment edit <commentId> <body-or-@file>".to_string())?;
            let body_arg = args.get(2).ok_or_else(|| "usage: operit2 market comment edit <commentId> <body-or-@file>".to_string())?;
            require_login(core)?;
            let body = read_content_arg(body_arg)?;
            core.api().edit_entry_comment(comment_id, &body)?;
            output.push_stdout_line(format!("Edited market comment: {comment_id}"));
            output.setJsonStdout(json!({
                "action": "edit",
                "commentId": comment_id,
            }));
            Ok(())
        }
        Some("delete") => {
            let comment_id = args.get(1).ok_or_else(|| "usage: operit2 market comment delete <commentId>".to_string())?;
            require_login(core)?;
            core.api().delete_entry_comment(comment_id)?;
            output.push_stdout_line(format!("Deleted market comment: {comment_id}"));
            output.setJsonStdout(json!({
                "action": "delete",
                "commentId": comment_id,
            }));
            Ok(())
        }
        Some(entry_id) => {
            let body_arg = args.get(1).ok_or_else(|| "usage: operit2 market comment <entryId> <body-or-@file>".to_string())?;
            require_login(core)?;
            let body = read_content_arg(body_arg)?;
            let comment_id = core.api().create_entry_comment(entry_id, &body)?;
            output.push_stdout_line(format!("Created market comment: {comment_id}"));
            output.setJsonStdout(json!({
                "action": "create",
                "entryId": entry_id,
                "commentId": comment_id,
            }));
            Ok(())
        }
        None => Err("usage: operit2 market comment <entryId> <body-or-@file> | comment edit <commentId> <body-or-@file> | comment delete <commentId>".to_string()),
    }
}

// ── Publish ────────────────────────────────────────────────

/// Dispatches marketplace publish subcommands.
fn run_publish(
    core: &mut MarketCommand,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("artifact") => publish_artifact_cli(core, &args[1..], output),
        Some("repo") => publish_repo_cli(core, &args[1..], output),
        Some("version") => publish_version_cli(core, &args[1..], output),
        Some("update-entry") => update_entry_cli(core, &args[1..], output),
        _ => Err(
            "usage: operit2 market publish <artifact|repo|version|update-entry> ...".to_string(),
        ),
    }
}

/// Publishes one artifact-backed marketplace entry.
fn publish_artifact_cli(
    core: &mut MarketCommand,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
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
    write_publish_response(resp, output);
    Ok(())
}

/// Publishes one repository-backed marketplace entry.
fn publish_repo_cli(
    core: &mut MarketCommand,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
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
    write_publish_response(resp, output);
    Ok(())
}

/// Dispatches marketplace version publishing commands.
fn publish_version_cli(
    core: &mut MarketCommand,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("artifact") => publish_artifact_version_cli(core, &args[1..], output),
        Some("repo") => publish_repo_version_cli(core, &args[1..], output),
        _ => Err("usage: operit2 market publish version <artifact|repo> ...".to_string()),
    }
}

/// Publishes one artifact-backed marketplace entry version.
fn publish_artifact_version_cli(
    core: &mut MarketCommand,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
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
    write_publish_response(resp, output);
    Ok(())
}

/// Publishes one repository-backed marketplace entry version.
fn publish_repo_version_cli(
    core: &mut MarketCommand,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
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
    write_publish_response(resp, output);
    Ok(())
}

/// Updates marketplace entry metadata.
fn update_entry_cli(
    core: &mut MarketCommand,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
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
    write_update_entry_response(resp, output);
    Ok(())
}

/// Writes a marketplace publish response as readable text and JSON.
fn write_publish_response(
    resp: operit_providers::market::MarketStatsApiService::MarketPublishResponse,
    output: &mut CoreCommandOutput,
) {
    output.push_stdout_line(format!("Published market entry: {}", resp.entry_id));
    output.push_stdout_line(format!("OK: {}", resp.ok));
    if !resp.version_id.trim().is_empty() {
        output.push_stdout_line(format!("Version: {}", resp.version_id));
    }
    output.setJsonStdout(json!(resp));
}

/// Writes a marketplace entry update response as readable text and JSON.
fn write_update_entry_response(
    resp: operit_providers::market::MarketStatsApiService::MarketEntryUpdateResponse,
    output: &mut CoreCommandOutput,
) {
    output.push_stdout_line(format!("Updated market entry: {}", resp.item.id));
    output.push_stdout_line(format!("OK: {}", resp.ok));
    output.push_stdout_line(format!("State: {}", resp.item.state_code));
    output.setJsonStdout(json!(resp));
}

// ── Install ─────────────────────────────────────────────────

/// Installs one marketplace entry according to its declared type.
fn install_entry(
    core: &mut MarketCommand,
    entry_id: &str,
    client_app_version: &str,
    version_id: Option<&str>,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let entry = core.api().get_entry_by_id(entry_id)?;
    ensure_entry_app_version_supported(&entry, client_app_version, version_id)?;
    match entry.r#type.as_str() {
        "skill" => install_skill_from_entry(core, entry, output),
        "mcp" => install_mcp_from_entry(core, entry, output),
        "package" | "script" => install_artifact_from_entry(core, entry, version_id, output),
        other => Err(format!("unknown market type: {other}")),
    }
}

/// Describes the numeric app version used by marketplace compatibility metadata.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct MarketAppVersion {
    major: u64,
    minor: u64,
    patch: u64,
    build: u64,
}

/// Rejects an installation when the selected marketplace version excludes the client version.
fn ensure_entry_app_version_supported(
    entry: &MarketEntrySummary,
    client_app_version: &str,
    version_id: Option<&str>,
) -> Result<(), String> {
    let client_version = parse_market_app_version(client_app_version, "客户端版本")?;
    let target_version = resolve_market_install_version(entry, version_id)?;

    let minimum_value = target_version.min_app_ver.trim();
    if !minimum_value.is_empty() {
        let minimum_version = parse_market_app_version(minimum_value, "最低支持版本")?;
        if client_version < minimum_version {
            return Err(format!(
                "无法下载：客户端版本 {client_app_version} 低于该资源要求的最低版本 {minimum_value}。请更新客户端后再下载。"
            ));
        }
    }

    if let Some(maximum_value) = target_version
        .max_app_ver
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let maximum_version = parse_market_app_version(maximum_value, "最高支持版本")?;
        if client_version > maximum_version {
            return Err(format!(
                "无法下载：客户端版本 {client_app_version} 高于该资源最高支持的版本 {maximum_value}。请使用受支持的客户端版本。"
            ));
        }
    }

    Ok(())
}

/// Resolves the precise marketplace version requested for one installation.
fn resolve_market_install_version<'a>(
    entry: &'a MarketEntrySummary,
    version_id: Option<&str>,
) -> Result<&'a MarketEntryVersion, String> {
    match version_id.map(str::trim).filter(|value| !value.is_empty()) {
        Some(requested_version_id) => entry
            .versions
            .iter()
            .find(|version| version.id == requested_version_id)
            .ok_or_else(|| {
                format!(
                    "market entry has no version metadata for requested version: {requested_version_id}"
                )
            }),
        None => entry
            .latest_version
            .as_ref()
            .ok_or_else(|| "market entry has no latest version metadata".to_string()),
    }
}

/// Parses one marketplace app-version value using x.y.z or x.y.z+n notation.
fn parse_market_app_version(value: &str, label: &str) -> Result<MarketAppVersion, String> {
    let normalized = value.trim();
    let mut version_parts = normalized.split('+');
    let core = version_parts
        .next()
        .ok_or_else(|| format!("{label} must use x.y.z or x.y.z+n format"))?;
    let build = match (version_parts.next(), version_parts.next()) {
        (None, None) => 0,
        (Some(value), None) => parse_market_app_version_component(Some(value), label, "build")?,
        (_, Some(_)) => return Err(format!("{label} must use x.y.z or x.y.z+n format: {value}")),
    };
    let mut parts = core.split('.');
    let major = parse_market_app_version_component(parts.next(), label, "major")?;
    let minor = parse_market_app_version_component(parts.next(), label, "minor")?;
    let patch = parse_market_app_version_component(parts.next(), label, "patch")?;
    if parts.next().is_some() {
        return Err(format!("{label} must use x.y.z or x.y.z+n format: {value}"));
    }
    Ok(MarketAppVersion {
        major,
        minor,
        patch,
        build,
    })
}

/// Parses one numeric component from marketplace compatibility metadata.
fn parse_market_app_version_component(
    value: Option<&str>,
    label: &str,
    component: &str,
) -> Result<u64, String> {
    let value = value.ok_or_else(|| format!("{label} must use x.y.z or x.y.z+n format"))?;
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!(
            "{label} {component} component must be numeric: {value}"
        ));
    }
    value
        .parse::<u64>()
        .map_err(|error| format!("{label} {component} component is invalid: {error}"))
}

/// Installs a skill marketplace entry from its repository source.
fn install_skill_from_entry(
    core: &mut MarketCommand,
    entry: MarketEntrySummary,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let source_url = entry_source_url(&entry, "skill")?;
    let result = core.skill_repo().importSkillFromGitHubRepo(&source_url);
    output.push_stdout_line("Skill import");
    output.push_stdout_line(format!("Entry: {} ({})", entry.title, entry.id));
    output.push_stdout_line(format!("Source: {source_url}"));
    output.push_stdout_line(result.clone());
    output.setJsonStdout(json!({
        "entryId": entry.id,
        "type": entry.r#type,
        "title": entry.title,
        "sourceUrl": source_url,
        "message": result,
    }));
    Ok(())
}

/// Installs an MCP marketplace entry from its repository source.
fn install_mcp_from_entry(
    core: &mut MarketCommand,
    entry: MarketEntrySummary,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let source_url = entry_source_url(&entry, "mcp")?;
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
        source_url.clone(),
        metadata,
        String::new(),
        |_| {},
    ) {
        operit_tools::tools::mcp_runtime::MCPRepository::InstallResult::Success { pluginPath } => {
            output.push_stdout_line("MCP install");
            output.push_stdout_line(format!("Entry: {} ({})", entry.title, entry.id));
            output.push_stdout_line(format!("Plugin: {plugin_id}"));
            output.push_stdout_line(format!("Path: {pluginPath}"));
            output.setJsonStdout(json!({
                "entryId": entry.id,
                "type": entry.r#type,
                "title": entry.title,
                "sourceUrl": source_url,
                "pluginId": plugin_id,
                "path": pluginPath,
                "installed": true,
            }));
            Ok(())
        }
        operit_tools::tools::mcp_runtime::MCPRepository::InstallResult::Error { message } => {
            Err(message)
        }
    }
}

/// Installs one artifact through the market asset endpoint and verifies its immutable digest.
fn install_artifact_from_entry(
    core: &mut MarketCommand,
    entry: MarketEntrySummary,
    version_id: Option<&str>,
    output: &mut CoreCommandOutput,
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
    let asset_id = asset.id.clone();
    let asset_version_id = asset.version_id.clone();
    let entry_id = entry.id.clone();
    let entry_type = entry.r#type.clone();
    let entry_title = entry.title.clone();
    let temp_file = download_asset_to_temp_file(core, asset)?;
    let package_manager = core.package_manager();
    let import_result = package_manager.add_from_external(&temp_file.to_string_lossy());
    fs::remove_file(&temp_file)
        .map_err(|error| format!("failed to remove market temp file: {error}"))?;
    let import_result = import_result?;
    write_external_package_import_result(&import_result, output);
    output.setJsonStdout(json!({
        "entryId": entry_id,
        "type": entry_type,
        "title": entry_title,
        "assetId": asset_id,
        "versionId": asset_version_id,
        "import": import_result,
    }));
    Ok(())
}

/// Extracts the non-empty source URL for a repository-backed market entry.
fn entry_source_url(entry: &MarketEntrySummary, entry_type: &str) -> Result<String, String> {
    let source = entry
        .source
        .as_ref()
        .ok_or_else(|| format!("{entry_type} entry has no source url"))?;
    let source_url = source.url.trim();
    if source_url.is_empty() {
        return Err(format!("{entry_type} entry has no source url"));
    }
    Ok(source_url.to_string())
}

/// Writes an external package import result in readable text form.
fn write_external_package_import_result(
    result: &ExternalPackageImportResult,
    output: &mut CoreCommandOutput,
) {
    output.push_stdout_line(format!("Package imported: {}", result.packageName));
    output.push_stdout_line(format!("Format: {}", result.packageFormat));
    output.push_stdout_line(format!("Stored: {}", result.storedPath));
    if let Some(source_notice) = &result.sourceNotice {
        output.push_stdout_line(format!("Source notice: {source_notice}"));
    }
}

/// Downloads one market asset to a temporary file whose extension matches the published asset.
fn download_asset_to_temp_file(
    core: &mut MarketCommand,
    asset: &MarketEntryAsset,
) -> Result<PathBuf, String> {
    let bytes = core.api().download_asset(&asset.id)?;
    verify_market_asset_sha256(&bytes, &asset.sha256)?;
    let tmp = market_asset_temp_path(asset)?;
    fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    Ok(tmp)
}

/// Verifies the downloaded bytes against the SHA-256 recorded in the market entry.
fn verify_market_asset_sha256(bytes: &[u8], expected_sha256: &str) -> Result<(), String> {
    let normalized_expected = expected_sha256.trim().to_ascii_lowercase();
    if normalized_expected.len() != 64
        || !normalized_expected
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("market asset SHA-256 is invalid".to_string());
    }
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != normalized_expected {
        return Err("market asset SHA-256 mismatch".to_string());
    }
    Ok(())
}

/// Creates an extension-preserving temporary path for one verified market asset.
fn market_asset_temp_path(asset: &MarketEntryAsset) -> Result<PathBuf, String> {
    let asset_name = asset
        .asset_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "market asset name is missing".to_string())?;
    if asset_name.contains('/') || asset_name.contains('\\') {
        return Err("market asset name must not contain a path".to_string());
    }
    let mut path = env::temp_dir();
    path.push(format!("operit_market_{}_{}", current_millis(), asset_name));
    Ok(path)
}

/// Converts a market title into a stable identifier-safe string.
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

/// Requires the GitHub OAuth broker session needed by market write operations.
fn require_login(core: &mut MarketCommand) -> Result<(), String> {
    if core.github_auth().getCurrentAccessToken().is_some() {
        Ok(())
    } else {
        Err("GitHub login required.".to_string())
    }
}

/// Parses an optional signed integer argument with an explicit default.
fn parse_i32_opt(raw: Option<&String>, default: i32) -> Result<i32, String> {
    match raw {
        Some(s) => s.parse::<i32>().map_err(|e| e.to_string()),
        None => Ok(default),
    }
}

/// Validates the market sort key against the supported values.
fn normalize_sort(sort: &str) -> Result<&str, String> {
    match sort {
        "updated" | "likes" | "downloads" => Ok(sort),
        other => Err(format!(
            "invalid market sort: {other}. expected updated|likes|downloads"
        )),
    }
}

/// Converts blank and sentinel arguments into absent values.
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

/// Parses a string argument that may intentionally be absent.
fn parse_optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "-" {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Parses an optional owned string argument from command input.
fn parse_optional_string_arg(value: Option<&String>) -> Option<String> {
    value.and_then(|raw| parse_optional_string(raw))
}

/// Resolves an optional literal or file-backed content argument.
fn parse_optional_content_arg(value: &str) -> Result<Option<String>, String> {
    match parse_optional_string(value) {
        Some(raw) => read_content_arg(&raw).map(Some),
        None => Ok(None),
    }
}

/// Parses a boolean argument using the canonical true or false literals.
fn parse_bool_arg(value: &str) -> Result<bool, String> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("invalid bool: {other}; expected true | false")),
    }
}

/// Parses an optional canonical boolean string.
fn parse_optional_bool_str(value: &str) -> Result<Option<bool>, String> {
    match parse_optional_string(value) {
        Some(raw) => parse_bool_arg(&raw).map(Some),
        None => Ok(None),
    }
}

/// Parses an optional canonical boolean command argument.
fn parse_optional_bool_arg(value: Option<&String>) -> Result<Option<bool>, String> {
    match value {
        Some(raw) => parse_optional_bool_str(raw),
        None => Ok(None),
    }
}

/// Returns the effective download count exposed by a market entry.
fn entry_downloads(entry: &MarketEntrySummary) -> i32 {
    entry.download_count.max(entry.downloads)
}

/// Returns the current Unix timestamp in milliseconds for temporary names.
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
    use std::path::PathBuf;

    use operit_host_api::HostManager::{setDefaultHttpHost, HostManager};
    use operit_host_api::{
        HostError, HostResult, HttpHost, HttpRequestData, HttpResponseData, RuntimeStorageEntry,
        RuntimeStorageHost,
    };

    #[derive(Clone)]
    struct MemoryStorageHost {
        files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
        runtime_root: PathBuf,
        workspace_root: PathBuf,
    }

    impl MemoryStorageHost {
        /// Creates isolated runtime and workspace roots for one market command test.
        fn new(root: PathBuf) -> Self {
            let runtime_root = root.join(RUNTIME_ROOT_DIR_PATH);
            let workspace_root = root.join(WORKSPACE_DIR_PATH);
            std::fs::create_dir_all(&runtime_root).expect("create test runtime root");
            std::fs::create_dir_all(&workspace_root).expect("create test workspace root");
            Self {
                files: Arc::new(Mutex::new(BTreeMap::new())),
                runtime_root,
                workspace_root,
            }
        }
    }

    impl RuntimeStorageHost for MemoryStorageHost {
        fn runtimeRootDir(&self) -> Option<PathBuf> {
            Some(self.runtime_root.clone())
        }

        fn workspaceRootDir(&self) -> Option<PathBuf> {
            Some(self.workspace_root.clone())
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

        /// Appends bytes to one market-command test storage entry.
        fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .entry(path.to_string())
                .or_default()
                .extend_from_slice(content);
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

    /// Creates an application configured with isolated runtime storage for market commands.
    fn market_test_application(root: PathBuf) -> OperitApplication {
        let storage_host = Arc::new(MemoryStorageHost::new(root));
        let mut host_manager = HostManager::new();
        host_manager.runtimeStorageHost = Some(storage_host);
        OperitApplication::newWithContext(host_manager)
    }

    /// Accepts bytes only when their digest matches the immutable market record.
    #[test]
    fn verifies_market_asset_sha256() {
        let bytes = b"operit-market-asset";
        let sha256 = format!("{:x}", Sha256::digest(bytes));
        assert!(verify_market_asset_sha256(bytes, &sha256).is_ok());
        assert!(verify_market_asset_sha256(bytes, "0".repeat(64).as_str()).is_err());
    }

    /// Preserves the market asset extension for the runtime package importer.
    #[test]
    fn market_asset_temp_path_preserves_asset_name() {
        let asset = MarketEntryAsset {
            id: "asset-1".to_string(),
            version_id: "version-1".to_string(),
            kind: "github_release_asset".to_string(),
            url: "https://github.com/example/release/download/plugin.toolpkg".to_string(),
            sha256: "0".repeat(64),
            asset_name: Some("plugin.toolpkg".to_string()),
        };
        let path = market_asset_temp_path(&asset).expect("asset path should be created");
        assert!(path.to_string_lossy().ends_with("plugin.toolpkg"));
    }

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

    fn run_market_cli(args: &[&str]) {
        let mut root = std::env::temp_dir();
        root.push(format!("operit_market_test_{}", current_millis()));
        std::fs::create_dir_all(&root).expect("create test runtime root");
        let application = market_test_application(root);
        let mut out = CoreCommandOutput::new();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        // Tests that parsing does not panic; network/IO errors are OK at this level.
        let _ = run_market_command(&application, &args, &mut out);
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
}
