use super::core::CliCore;
use super::*;

pub(super) async fn run_market_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_market_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "auth" => run_market_auth_command(core, &args[1..]).await,
        "stats" => {
            let marketType = args.get(1).ok_or_else(|| {
                "usage: operit2 cli market stats <skill|mcp|package|script>".to_string()
            })?;
            ensure_env_auth_token_saved(core).await?;
            let stats = core
                .api_market_stats_api_service()
                .getStats(marketType)
                .await
                .map_err(|error| error.to_string())?;
            println!("updatedAt={}", stats.updatedAt.unwrap_or_default());
            for (id, item) in stats.items {
                println!(
                    "{}\tdownloads={}\tlastDownloadAt={}\tupdatedAt={}",
                    id,
                    item.downloads,
                    item.lastDownloadAt.unwrap_or_default(),
                    item.updatedAt.unwrap_or_default()
                );
            }
            Ok(())
        }
        "rank" => {
            let marketType = args
                .get(1)
                .ok_or_else(|| "usage: operit2 cli market rank <skill|mcp|package|script> [updated|downloads|likes] [page]".to_string())?;
            let metric = args.get(2).map(String::as_str).unwrap_or("updated");
            let page = parse_optional_i32_arg(args.get(3), 1)?;
            print_market_rank(core, marketType, metric, page).await
        }
        "search" => {
            let marketType = args.get(1).ok_or_else(|| {
                "usage: operit2 cli market search <skill|mcp|package|script> <query> [page]"
                    .to_string()
            })?;
            let query = args.get(2).ok_or_else(|| {
                "usage: operit2 cli market search <skill|mcp|package|script> <query> [page]"
                    .to_string()
            })?;
            let page = parse_optional_i32_arg(args.get(3), 1)?;
            search_issue_market(core, marketType, query, page).await
        }
        "comments" => {
            let marketType = args.get(1).ok_or_else(|| {
                "usage: operit2 cli market comments <skill|mcp|package|script> <number> [page]"
                    .to_string()
            })?;
            let number = parse_i32_arg(
                args.get(2),
                "usage: operit2 cli market comments <skill|mcp|package|script> <number> [page]",
            )?;
            let page = parse_optional_i32_arg(args.get(3), 1)?;
            let (owner, repo, _) = issue_market_definition(marketType)?;
            ensure_env_auth_token_saved(core).await?;
            for comment in core
                .api_market_stats_api_service()
                .getIssueComments(owner, repo, number, page, 50)
                .await
                .map_err(|error| error.to_string())?
            {
                println!(
                    "#{}\t{}\t{}\t{}",
                    comment.id, comment.user.login, comment.updated_at, comment.html_url
                );
                println!("{}", comment.body);
                println!();
            }
            Ok(())
        }
        "comment" => {
            let marketType = args
                .get(1)
                .ok_or_else(|| "usage: operit2 cli market comment <skill|mcp|package|script> <number> <body-or-@file>".to_string())?;
            let number = parse_i32_arg(args.get(2), "usage: operit2 cli market comment <skill|mcp|package|script> <number> <body-or-@file>")?;
            let bodyArg = args
                .get(3)
                .ok_or_else(|| "usage: operit2 cli market comment <skill|mcp|package|script> <number> <body-or-@file>".to_string())?;
            require_github_login(core).await?;
            let body = read_skill_content_arg(bodyArg)?;
            let (owner, repo, _) = issue_market_definition(marketType)?;
            let comment = core
                .api_market_stats_api_service()
                .createIssueComment(owner, repo, number, &body)
                .await
                .map_err(|error| error.to_string())?;
            println!("created={}", comment.html_url);
            Ok(())
        }
        "reactions" => {
            let marketType = args.get(1).ok_or_else(|| {
                "usage: operit2 cli market reactions <skill|mcp|package|script> <number>"
                    .to_string()
            })?;
            let number = parse_i32_arg(
                args.get(2),
                "usage: operit2 cli market reactions <skill|mcp|package|script> <number>",
            )?;
            let (owner, repo, _) = issue_market_definition(marketType)?;
            ensure_env_auth_token_saved(core).await?;
            for reaction in core
                .api_market_stats_api_service()
                .getIssueReactions(owner, repo, number)
                .await
                .map_err(|error| error.to_string())?
            {
                println!(
                    "#{}\t{}\t{}\t{}",
                    reaction.id, reaction.content, reaction.user.login, reaction.created_at
                );
            }
            Ok(())
        }
        "react" => {
            let marketType = args
                .get(1)
                .ok_or_else(|| "usage: operit2 cli market react <skill|mcp|package|script> <number> <+1|heart|rocket|...>".to_string())?;
            let number = parse_i32_arg(args.get(2), "usage: operit2 cli market react <skill|mcp|package|script> <number> <+1|heart|rocket|...>")?;
            let content = args
                .get(3)
                .ok_or_else(|| "usage: operit2 cli market react <skill|mcp|package|script> <number> <+1|heart|rocket|...>".to_string())?;
            require_github_login(core).await?;
            let (owner, repo, _) = issue_market_definition(marketType)?;
            let reaction = core
                .api_market_stats_api_service()
                .createIssueReaction(owner, repo, number, content)
                .await
                .map_err(|error| error.to_string())?;
            println!("created={} content={}", reaction.id, reaction.content);
            Ok(())
        }
        "show" => {
            let marketType = args.get(1).ok_or_else(|| {
                "usage: operit2 cli market show <skill|mcp|package|script> <id-or-number>"
                    .to_string()
            })?;
            let target = args.get(2).ok_or_else(|| {
                "usage: operit2 cli market show <skill|mcp|package|script> <id-or-number>"
                    .to_string()
            })?;
            show_market_item(core, marketType, target).await
        }
        "install" => {
            let marketType = args
                .get(1)
                .ok_or_else(|| "usage: operit2 cli market install <skill|mcp|package|script> <id-or-url> [node-id]".to_string())?;
            let target = args
                .get(2)
                .ok_or_else(|| "usage: operit2 cli market install <skill|mcp|package|script> <id-or-url> [node-id]".to_string())?;
            install_market_item(core, marketType, target, args.get(3).map(String::as_str)).await
        }
        _ => {
            print_market_usage();
            Ok(())
        }
    }
}

async fn run_market_auth_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("status") | None => {
            ensure_env_auth_token_saved(core).await?;
            println!(
                "loggedIn={}",
                core.preferences_git_hub_auth_preferences()
                    .isLoggedIn()
                    .await
                    .map_err(|error| error.to_string())?
            );
            if let Some(user) = core
                .preferences_git_hub_auth_preferences()
                .getCurrentUserInfo()
                .await
                .map_err(|error| error.to_string())?
            {
                println!("user={}", user.login);
            }
            Ok(())
        }
        Some("token") => {
            let token = args
                .get(1)
                .ok_or_else(|| "usage: operit2 cli market auth token <github-token>".to_string())?;
            core.preferences_git_hub_auth_preferences()
                .updateAccessToken(token, "bearer", None)
                .await
                .map_err(|error| error.to_string())?;
            println!("saved");
            Ok(())
        }
        Some("logout") => {
            core.preferences_git_hub_auth_preferences()
                .logout()
                .await
                .map_err(|error| error.to_string())?;
            println!("logged out");
            Ok(())
        }
        Some("whoami") => {
            require_github_login(core).await?;
            let user = core
                .api_market_stats_api_service()
                .getCurrentUser()
                .await
                .map_err(|error| error.to_string())?;
            println!("{}", user.login);
            Ok(())
        }
        _ => Err("usage: operit2 cli market auth <status|token|logout|whoami>".to_string()),
    }
}

async fn print_market_rank(
    core: &mut CliCore,
    marketType: &str,
    metric: &str,
    page: i32,
) -> Result<(), String> {
    if matches!(marketType, "package" | "script") {
        return print_artifact_rank(core, marketType, metric, page).await;
    }
    ensure_env_auth_token_saved(core).await?;
    let rank = core
        .api_market_stats_api_service()
        .getRankPage(marketType, metric, page)
        .await
        .map_err(|error| error.to_string())?;
    println!(
        "type={}\tmetric={}\tpage={}\ttotalPages={}\ttotalItems={}",
        rank.r#type, rank.metric, rank.page, rank.totalPages, rank.totalItems
    );
    for item in rank.items {
        print_issue_rank_entry(marketType, &item);
    }
    Ok(())
}

async fn print_artifact_rank(
    core: &mut CliCore,
    marketType: &str,
    metric: &str,
    page: i32,
) -> Result<(), String> {
    ensure_env_auth_token_saved(core).await?;
    let rank = core
        .api_market_stats_api_service()
        .getArtifactRankPage(marketType, metric, page)
        .await
        .map_err(|error| error.to_string())?;
    println!(
        "{} market ({}) - page {}/{} - {} items",
        title_case_market_type(&rank.r#type),
        rank.metric,
        rank.page,
        rank.totalPages,
        rank.totalItems
    );
    println!();
    for (index, item) in rank.items.iter().enumerate() {
        let node = item.defaultNode.as_ref();
        let defaultNodeId = item
            .defaultNodeId
            .as_str()
            .if_empty_then(node.map(|node| node.nodeId.clone()).unwrap_or_default());
        let shortSha = node
            .map(|node| short_hash(&node.sha256))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{:>2}. {}",
            index + 1,
            item.projectDisplayName
                .as_str()
                .if_empty_then(item.projectId.clone())
        );
        println!("    id: {}", item.projectId);
        println!("    downloads: {}    likes: {}", item.downloads, item.likes);
        println!(
            "    default node: {}    sha256: {}",
            defaultNodeId, shortSha
        );
        if !item.projectDescription.trim().is_empty() {
            println!("    {}", single_line_summary(&item.projectDescription, 160));
        }
        println!(
            "    show: operit2 cli market show {} {}",
            marketType, item.projectId
        );
        println!(
            "    install: operit2 cli market install {} {}",
            marketType, item.projectId
        );
        if !defaultNodeId.trim().is_empty() {
            println!(
                "    install node: operit2 cli market install {} {} {}",
                marketType, item.projectId, defaultNodeId
            );
        }
        println!();
    }
    Ok(())
}

async fn search_issue_market(
    core: &mut CliCore,
    marketType: &str,
    query: &str,
    page: i32,
) -> Result<(), String> {
    let (owner, repo, label) = issue_market_definition(marketType)?;
    ensure_env_auth_token_saved(core).await?;
    let issues = core
        .api_market_stats_api_service()
        .searchIssues(owner, repo, label, query, page, 50)
        .await
        .map_err(|error| error.to_string())?;
    for issue in issues
        .into_iter()
        .filter(|issue| !has_review_blocking_label(issue))
    {
        print_github_issue_summary(marketType, &issue);
    }
    Ok(())
}

async fn show_market_item(
    core: &mut CliCore,
    marketType: &str,
    target: &str,
) -> Result<(), String> {
    match marketType {
        "package" | "script" => {
            ensure_env_auth_token_saved(core).await?;
            let detail = core
                .api_market_stats_api_service()
                .getArtifactProject(target)
                .await
                .map_err(|error| error.to_string())?;
            print_artifact_project(&detail);
            Ok(())
        }
        "skill" | "mcp" => {
            let number = match target.parse::<i32>() {
                Ok(number) => number,
                Err(_) => {
                    find_issue_rank_entry(core, marketType, target)
                        .await?
                        .issue
                        .number
                }
            };
            let (owner, repo, _) = issue_market_definition(marketType)?;
            ensure_env_auth_token_saved(core).await?;
            let issue = core
                .api_market_stats_api_service()
                .getIssue(owner, repo, number)
                .await
                .map_err(|error| error.to_string())?;
            print_github_issue(&issue);
            Ok(())
        }
        _ => Err("market type must be skill, mcp, package, or script".to_string()),
    }
}

async fn install_market_item(
    core: &mut CliCore,
    marketType: &str,
    target: &str,
    nodeId: Option<&str>,
) -> Result<(), String> {
    match marketType {
        "skill" => install_market_skill(core, target).await,
        "mcp" => install_market_mcp(core, target).await,
        "package" | "script" => install_market_artifact(core, marketType, target, nodeId).await,
        _ => Err("market type must be skill, mcp, package, or script".to_string()),
    }
}

async fn install_market_skill(core: &mut CliCore, target: &str) -> Result<(), String> {
    let repoUrl = if looks_like_url(target) {
        target.to_string()
    } else {
        let entry = find_issue_rank_entry(core, "skill", target).await?;
        let repoUrl = skillRepositoryUrlFromEntry(&entry);
        if repoUrl.trim().is_empty() {
            return Err(format!("skill entry has no repositoryUrl: {target}"));
        }
        let statsId = resolveMarketEntryId(&repoUrl, &entry.issue.title);
        let downloadTarget = if repoUrl.trim().is_empty() {
            entry.issue.html_url.as_str()
        } else {
            repoUrl.as_str()
        };
        let _ = core
            .api_market_stats_api_service()
            .trackDownload("skill", &statsId, downloadTarget)
            .await;
        repoUrl
    };
    println!(
        "{}",
        core.skill_skill_repository()
            .importSkillFromGitHubRepo(&repoUrl)
            .await
            .map_err(|error| error.to_string())?
    );
    Ok(())
}

async fn install_market_mcp(core: &mut CliCore, target: &str) -> Result<(), String> {
    if target.trim_start().starts_with('{') {
        let count = core
            .mcp_m_cplocal_server()
            .mergeConfigFromJson(target)
            .await
            .map_err(|error| error.to_string())?;
        println!("imported={count}");
        return Ok(());
    }

    let (metadata, issueUrl, statsId) = if looks_like_url(target) {
        let pluginId = mcp_id_from_title(target);
        (
            operit_runtime::data::mcp::MCPLocalServer::PluginMetadata {
                id: pluginId.clone(),
                name: pluginId,
                description: String::new(),
                logoUrl: None,
                author: "Unknown".to_string(),
                isInstalled: false,
                version: "1.0.0".to_string(),
                updatedAt: String::new(),
                longDescription: String::new(),
                repoUrl: target.to_string(),
                r#type: "local".to_string(),
                endpoint: None,
                connectionType: Some("httpStream".to_string()),
                disabled: false,
                bearerToken: None,
                headers: None,
                installedPath: None,
                installedTime: 0,
                marketConfig: None,
            },
            target.to_string(),
            normalizeMarketArtifactId(target),
        )
    } else {
        let entry = find_issue_rank_entry(core, "mcp", target).await?;
        let info = mcpMetadataFromEntry(&entry);
        let pluginId = mcp_id_from_title(&entry.issue.title);
        let statsId = resolveMarketEntryId(&info.repositoryUrl, &entry.issue.title);
        let metadata = operit_runtime::data::mcp::MCPLocalServer::PluginMetadata {
            id: pluginId,
            name: entry.issue.title.clone(),
            description: info
                .description
                .trim()
                .to_string()
                .if_empty_then(entry.summaryDescription.clone()),
            logoUrl: Some(
                entry
                    .authorAvatarUrl
                    .if_empty_then(entry.issue.user.avatarUrl.clone()),
            ),
            author: info
                .repositoryUrl
                .split("github.com/")
                .nth(1)
                .and_then(|value| value.split('/').next())
                .unwrap_or(&entry.issue.user.login)
                .to_string(),
            isInstalled: false,
            version: info.version.if_empty_then("1.0.0".to_string()),
            updatedAt: entry.issue.updated_at.clone(),
            longDescription: entry.issue.body.clone().unwrap_or_default(),
            repoUrl: info.repositoryUrl,
            r#type: "local".to_string(),
            endpoint: None,
            connectionType: Some("httpStream".to_string()),
            disabled: false,
            bearerToken: None,
            headers: None,
            installedPath: None,
            installedTime: 0,
            marketConfig: if info.installConfig.trim().is_empty() {
                None
            } else {
                Some(info.installConfig)
            },
        };
        (metadata, entry.issue.html_url, statsId)
    };

    if let Some(config) = metadata
        .marketConfig
        .as_ref()
        .filter(|config| !config.trim().is_empty())
    {
        let _ = core
            .api_market_stats_api_service()
            .trackDownload("mcp", &statsId, &issueUrl)
            .await;
        if !core
            .mcp_m_cprepository()
            .checkConfigNeedsPhysicalInstallation(config)
            .await
            .map_err(|error| error.to_string())?
        {
            let count = core
                .mcp_m_cplocal_server()
                .mergeConfigFromJson(config)
                .await
                .map_err(|error| error.to_string())?;
            println!("imported={count}");
            return Ok(());
        }
    }

    let pluginId = metadata.id.clone();
    let targetUrl = if metadata.repoUrl.trim().is_empty() {
        issueUrl.clone()
    } else {
        metadata.repoUrl.clone()
    };
    let _ = core
        .api_market_stats_api_service()
        .trackDownload("mcp", &statsId, &targetUrl)
        .await;
    core.mcp_m_cplocal_server()
        .addOrUpdatePluginMetadata(metadata)
        .await
        .map_err(|error| error.to_string())?;
    println!("registered={pluginId}");
    Ok(())
}

async fn install_market_artifact(
    core: &mut CliCore,
    marketType: &str,
    projectId: &str,
    nodeId: Option<&str>,
) -> Result<(), String> {
    ensure_env_auth_token_saved(core).await?;
    let detail = core
        .api_market_stats_api_service()
        .getArtifactProject(projectId)
        .await
        .map_err(|error| error.to_string())?;
    let node = resolve_artifact_node(&detail.nodes, nodeId.or(Some(&detail.defaultNodeId)))
        .or_else(|| resolve_artifact_node(&detail.nodes, Some(&detail.latestOpenNodeId)))
        .or_else(|| resolve_artifact_node(&detail.nodes, Some(&detail.latestNodeId)))
        .ok_or_else(|| format!("artifact node not found for project: {projectId}"))?;
    let tempFile = download_artifact_node_to_temp_file(&node)?;
    let result = core
        .permissions_pack_tool_package_manager()
        .addPackageFileFromExternalStorage(&tempFile.to_string_lossy())
        .await
        .map_err(|error| error.to_string())?;
    let _ = fs::remove_file(&tempFile);
    if !result
        .to_ascii_lowercase()
        .starts_with("successfully imported")
    {
        return Err(result);
    }
    let _ = core
        .api_market_stats_api_service()
        .trackDownload(
            marketType,
            projectId,
            if node.downloadUrl.trim().is_empty() {
                node.issue.html_url.as_str()
            } else {
                node.downloadUrl.as_str()
            },
        )
        .await;
    println!("{result}");
    Ok(())
}

fn print_artifact_project(detail: &ArtifactProjectDetailResponse) {
    let marketType = if detail.r#type.trim().is_empty() {
        "package"
    } else {
        detail.r#type.as_str()
    };
    println!(
        "{} project: {}",
        title_case_market_type(marketType),
        detail
            .projectDisplayName
            .as_str()
            .if_empty_then(detail.projectId.clone())
    );
    println!("id: {}", detail.projectId);
    if !detail.projectDescription.trim().is_empty() {
        println!(
            "summary: {}",
            single_line_summary(&detail.projectDescription, 180)
        );
    }
    println!(
        "stats: downloads={}    likes={}    contributors={}",
        detail.downloads, detail.likes, detail.contributorCount
    );
    println!("default node: {}", value_or_dash(&detail.defaultNodeId));
    println!("latest open: {}", value_or_dash(&detail.latestOpenNodeId));
    println!("latest node: {}", value_or_dash(&detail.latestNodeId));
    println!();
    println!(
        "install default: operit2 cli market install {} {}",
        marketType, detail.projectId
    );
    if !detail.defaultNodeId.trim().is_empty() {
        println!(
            "install default node: operit2 cli market install {} {} {}",
            marketType, detail.projectId, detail.defaultNodeId
        );
    }
    println!();
    println!("Versions ({} nodes)", detail.nodes.len());
    print_artifact_node_tree(detail, marketType);
}

fn print_artifact_node_tree(detail: &ArtifactProjectDetailResponse, marketType: &str) {
    let mut childrenByParent = BTreeMap::<String, Vec<&ArtifactProjectNodeResponse>>::new();
    let mut childIds = BTreeSet::<String>::new();
    let nodesById = detail
        .nodes
        .iter()
        .map(|node| (node.nodeId.clone(), node))
        .collect::<BTreeMap<_, _>>();

    for node in &detail.nodes {
        for parentNodeId in node.parentNodeIds.iter().filter(|id| !id.trim().is_empty()) {
            if nodesById.contains_key(parentNodeId) {
                childrenByParent
                    .entry(parentNodeId.clone())
                    .or_default()
                    .push(node);
                childIds.insert(node.nodeId.clone());
            }
        }
    }

    for edge in &detail.edges {
        if childIds.contains(&edge.childNodeId) {
            continue;
        }
        if let (Some(parent), Some(child)) = (
            nodesById.get(&edge.parentNodeId),
            nodesById.get(&edge.childNodeId),
        ) {
            childrenByParent
                .entry(parent.nodeId.clone())
                .or_default()
                .push(*child);
            childIds.insert(child.nodeId.clone());
        }
    }

    let mut roots = detail
        .nodes
        .iter()
        .filter(|node| node.nodeId == detail.rootNodeId || !childIds.contains(&node.nodeId))
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| node_sort_key(left, detail).cmp(&node_sort_key(right, detail)));

    let mut printed = BTreeSet::<String>::new();
    for root in roots {
        print_artifact_node_branch(
            detail,
            marketType,
            root,
            "",
            true,
            true,
            &childrenByParent,
            &mut printed,
        );
    }

    for node in &detail.nodes {
        if !printed.contains(&node.nodeId) {
            print_artifact_node_branch(
                detail,
                marketType,
                node,
                "",
                true,
                true,
                &childrenByParent,
                &mut printed,
            );
        }
    }
}

fn print_artifact_node_branch<'a>(
    detail: &ArtifactProjectDetailResponse,
    marketType: &str,
    node: &'a ArtifactProjectNodeResponse,
    prefix: &str,
    isLast: bool,
    isRoot: bool,
    childrenByParent: &BTreeMap<String, Vec<&'a ArtifactProjectNodeResponse>>,
    printed: &mut BTreeSet<String>,
) {
    let connector = if isRoot {
        ""
    } else if isLast {
        "`- "
    } else {
        "+- "
    };
    let badges = artifact_node_badges(detail, node);
    println!(
        "{}{}{}{}",
        prefix,
        connector,
        artifact_node_title(node),
        if badges.is_empty() {
            String::new()
        } else {
            format!(" {badges}")
        }
    );
    println!(
        "{}{}id={}  package={}  version={}  state={}  sha256={}",
        prefix,
        if isRoot { "   " } else { "   " },
        node.nodeId,
        value_or_dash(&node.runtimePackageId),
        value_or_dash(&node.version),
        value_or_dash(&node.state),
        short_hash(&node.sha256)
    );
    if !node.parentNodeIds.is_empty() {
        println!("{}   parents: {}", prefix, node.parentNodeIds.join(", "));
    }
    println!(
        "{}   install: operit2 cli market install {} {} {}",
        prefix, marketType, detail.projectId, node.nodeId
    );
    printed.insert(node.nodeId.clone());

    let mut children = childrenByParent
        .get(&node.nodeId)
        .cloned()
        .unwrap_or_default();
    children.sort_by(|left, right| node_sort_key(left, detail).cmp(&node_sort_key(right, detail)));
    let nextPrefix = if isRoot {
        prefix.to_string()
    } else if isLast {
        format!("{prefix}   ")
    } else {
        format!("{prefix}|  ")
    };
    let lastIndex = children.len().saturating_sub(1);
    for (index, child) in children.into_iter().enumerate() {
        if printed.contains(&child.nodeId) {
            continue;
        }
        print_artifact_node_branch(
            detail,
            marketType,
            child,
            &nextPrefix,
            index == lastIndex,
            false,
            childrenByParent,
            printed,
        );
    }
}

fn artifact_node_badges(
    detail: &ArtifactProjectDetailResponse,
    node: &ArtifactProjectNodeResponse,
) -> String {
    let mut badges = Vec::new();
    if node.nodeId == detail.defaultNodeId {
        badges.push("default");
    }
    if node.nodeId == detail.latestOpenNodeId {
        badges.push("latest-open");
    }
    if node.nodeId == detail.latestNodeId {
        badges.push("latest");
    }
    if node.state.eq_ignore_ascii_case("closed") {
        badges.push("closed");
    }
    if badges.is_empty() {
        String::new()
    } else {
        format!("[{}]", badges.join(", "))
    }
}

fn artifact_node_title(node: &ArtifactProjectNodeResponse) -> String {
    let title = node
        .displayName
        .as_str()
        .if_empty_then(node.runtimePackageId.clone())
        .if_empty_then(node.nodeId.clone());
    if node.version.trim().is_empty() || title.contains(&node.version) {
        title
    } else {
        format!("{title} v{}", node.version)
    }
}

fn node_sort_key(
    node: &ArtifactProjectNodeResponse,
    detail: &ArtifactProjectDetailResponse,
) -> (i32, String) {
    let rank = if node.nodeId == detail.defaultNodeId {
        0
    } else if node.nodeId == detail.latestOpenNodeId {
        1
    } else if node.nodeId == detail.latestNodeId {
        2
    } else {
        3
    };
    (rank, node.publishedAt.clone().unwrap_or_default())
}

fn print_issue_rank_entry(marketType: &str, entry: &MarketRankIssueEntryResponse) {
    let repositoryUrl = if marketType == "skill" {
        skillRepositoryUrlFromEntry(entry)
    } else {
        mcpMetadataFromEntry(entry).repositoryUrl
    };
    println!(
        "{}\t{}\tdownloads={}\tlikes={}\tupdatedAt={}\trepo={}",
        entry.id,
        entry
            .displayTitle
            .clone()
            .if_empty_then(entry.issue.title.clone()),
        entry.downloads,
        entry
            .issue
            .reactions
            .as_ref()
            .map(|item| item.thumbsUp)
            .unwrap_or(0),
        entry.issue.updated_at,
        repositoryUrl
    );
    println!("  {}", entry.summaryDescription);
}

fn print_github_issue_summary(marketType: &str, issue: &GitHubIssue) {
    let marketTarget = issue
        .body
        .as_deref()
        .and_then(|body| match marketType {
            "skill" => parseSkillMarketMetadata(body).map(|metadata| metadata.repositoryUrl),
            "mcp" => parseMcpMarketMetadata(body).map(|metadata| metadata.repositoryUrl),
            "package" | "script" => parseArtifactMarketMetadata(body)
                .map(|metadata| metadata.projectId.if_empty_then(metadata.downloadUrl)),
            _ => None,
        })
        .unwrap_or_default();
    println!(
        "#{}\t{}\t{}\tlikes={}\ttarget={}",
        issue.number,
        issue.title,
        issue.updated_at,
        issue
            .reactions
            .as_ref()
            .map(|item| item.thumbsUp)
            .unwrap_or(0),
        marketTarget
    );
}

fn print_github_issue(issue: &GitHubIssue) {
    println!("number={}", issue.number);
    println!("title={}", issue.title);
    println!("state={}", issue.state);
    println!("url={}", issue.html_url);
    println!("user={}", issue.user.login);
    println!("createdAt={}", issue.created_at);
    println!("updatedAt={}", issue.updated_at);
    println!(
        "labels={}",
        issue
            .labels
            .iter()
            .map(|label| label.name.clone())
            .collect::<Vec<_>>()
            .join(",")
    );
    println!();
    println!("{}", issue.body.clone().unwrap_or_default());
}

async fn find_issue_rank_entry(
    core: &mut CliCore,
    marketType: &str,
    target: &str,
) -> Result<MarketRankIssueEntryResponse, String> {
    let normalized = normalizeMarketArtifactId(target);
    ensure_env_auth_token_saved(core).await?;
    let mut page = 1;
    loop {
        let rank = core
            .api_market_stats_api_service()
            .getRankPage(marketType, "updated", page)
            .await
            .map_err(|error| error.to_string())?;
        if let Some(entry) = rank.items.into_iter().find(|entry| {
            entry.id == target
                || normalizeMarketArtifactId(&entry.id) == normalized
                || entry.issue.number.to_string() == target
                || normalizeMarketArtifactId(&entry.issue.title) == normalized
        }) {
            return Ok(entry);
        }
        if page >= rank.totalPages.max(1) {
            break;
        }
        page += 1;
    }
    Err(format!("market entry not found: {marketType}/{target}"))
}

fn resolve_artifact_node(
    nodes: &[ArtifactProjectNodeResponse],
    nodeId: Option<&str>,
) -> Option<ArtifactProjectNodeResponse> {
    let nodeId = nodeId?.trim();
    if nodeId.is_empty() {
        return None;
    }
    nodes.iter().find(|node| node.nodeId == nodeId).cloned()
}

fn download_artifact_node_to_temp_file(
    node: &ArtifactProjectNodeResponse,
) -> Result<PathBuf, String> {
    if node.downloadUrl.trim().is_empty() {
        return Err("artifact node has empty downloadUrl".to_string());
    }
    let extension = Path::new(&node.assetName)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bin");
    let tempFile = std::env::temp_dir().join(format!(
        "operit_market_{}_{}.{}",
        sanitize_cli_temp_part(&node.runtimePackageId),
        current_time_millis(),
        extension
    ));
    let mut response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("Operit-Market")
        .build()
        .map_err(|error| error.to_string())?
        .get(&node.downloadUrl)
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Download failed: HTTP {}",
            response.status().as_u16()
        ));
    }
    let mut out = fs::File::create(&tempFile).map_err(|error| error.to_string())?;
    std::io::copy(&mut response, &mut out).map_err(|error| error.to_string())?;

    let actualSha256 = sha256_file(&tempFile)?;
    if !node.sha256.trim().is_empty() && !actualSha256.eq_ignore_ascii_case(&node.sha256) {
        let _ = fs::remove_file(&tempFile);
        return Err("Downloaded file sha256 mismatch".to_string());
    }
    Ok(tempFile)
}

fn env_auth_token() -> Option<String> {
    env::var("OPERIT_GITHUB_TOKEN")
        .ok()
        .or_else(|| env::var("GITHUB_TOKEN").ok())
        .filter(|token| !token.trim().is_empty())
}

async fn ensure_env_auth_token_saved(core: &mut CliCore) -> Result<(), String> {
    if let Some(token) = env_auth_token() {
        if core
            .preferences_git_hub_auth_preferences()
            .getCurrentAccessToken()
            .await
            .map_err(|error| error.to_string())?
            .as_deref()
            != Some(token.as_str())
        {
            core.preferences_git_hub_auth_preferences()
                .updateAccessToken(&token, "bearer", None)
                .await
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

async fn require_github_login(core: &mut CliCore) -> Result<(), String> {
    ensure_env_auth_token_saved(core).await?;
    if core
        .preferences_git_hub_auth_preferences()
        .getCurrentAccessToken()
        .await
        .map_err(|error| error.to_string())?
        .is_some()
    {
        Ok(())
    } else {
        Err(
            "GitHub token required. Use `operit2 cli market auth token <token>` or GITHUB_TOKEN."
                .to_string(),
        )
    }
}

fn issue_market_definition(
    marketType: &str,
) -> Result<(&'static str, &'static str, &'static str), String> {
    match marketType {
        "skill" => Ok(("AAswordman", "OperitSkillMarket", "skill-plugin")),
        "mcp" => Ok(("AAswordman", "OperitMCPMarket", "mcp-plugin")),
        "package" => Ok(("AAswordman", "OperitPackageMarket", "package-artifact")),
        "script" => Ok(("AAswordman", "OperitScriptMarket", "script-artifact")),
        _ => Err("issue market type must be skill, mcp, package, or script".to_string()),
    }
}

fn has_review_blocking_label(issue: &GitHubIssue) -> bool {
    issue.labels.iter().any(|label| {
        label.name.eq_ignore_ascii_case("review:changes-requested")
            || label.name.eq_ignore_ascii_case("review:rejected")
    })
}

fn looks_like_url(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("github.com/")
}

fn mcp_id_from_title(title: &str) -> String {
    let mut out = String::new();
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out.trim_matches('_')
        .if_empty_then("mcp_plugin".to_string())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read =
            std::io::Read::read(&mut file, &mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn sanitize_cli_temp_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn current_time_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis() as i64
}

fn title_case_market_type(value: &str) -> String {
    match value {
        "mcp" => "MCP".to_string(),
        "skill" => "Skill".to_string(),
        "package" => "Package".to_string(),
        "script" => "Script".to_string(),
        other => other.to_string(),
    }
}

fn single_line_summary(value: &str, maxChars: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    for ch in compact.chars() {
        if out.chars().count() >= maxChars {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

fn short_hash(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "-".to_string()
    } else {
        trimmed.chars().take(12).collect()
    }
}

fn value_or_dash(value: &str) -> String {
    if value.trim().is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

trait CliStringExt {
    fn if_empty_then(self, value: String) -> String;
}

impl CliStringExt for String {
    fn if_empty_then(self, value: String) -> String {
        if self.trim().is_empty() {
            value
        } else {
            self
        }
    }
}

impl CliStringExt for &str {
    fn if_empty_then(self, value: String) -> String {
        if self.trim().is_empty() {
            value
        } else {
            self.to_string()
        }
    }
}
