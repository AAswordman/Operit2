use std::collections::{BTreeMap, BTreeSet};

use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgSourceType;
use operit_store::ExtensionStore;
use operit_tools::tools::mcp_runtime::MCPLocalServer::MCPLocalServer;
use operit_tools::tools::packTool::RuntimePackageManager::ToolPkgLoadIssue;
use serde::{Deserialize, Serialize};

use super::MarketInstallSupport;
use super::OperitApplication::OperitApplication;

/// One normalized card row for the package manager's local catalog views.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionCatalogEntry {
    pub id: String,
    pub kind: String,
    pub source: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub authors: Vec<String>,
    pub category: String,
    pub scope: String,
    pub activation_state: String,
    pub runtime_state: String,
    pub update_state: String,
    pub is_customized: bool,
    pub market_entry_id: Option<String>,
    pub installed_market_version_id: Option<String>,
    pub logo_resource_key: Option<String>,
    pub logo_mime_type: Option<String>,
    pub tool_count: usize,
    pub subpackage_count: usize,
    pub ui_module_count: usize,
    pub has_ui: bool,
    pub chat_sidebar_entry_ids: Vec<String>,
    pub dependency_issue_count: usize,
}

/// An offline official extension that has not been installed yet.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionCatalogCandidate {
    pub id: String,
    pub kind: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub authors: Vec<String>,
    pub category: String,
    pub required_scope: Option<String>,
    pub tool_count: usize,
    pub subpackage_count: usize,
}

/// A package load or import failure represented independently of valid rows.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionCatalogDiagnostic {
    pub source_path: String,
    pub package_id: Option<String>,
    pub display_name: String,
    pub code: String,
    pub message: String,
    pub package_kind: String,
    pub can_delete: bool,
}

/// The complete local extension catalog used by the redesigned manager UI.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionCatalogSnapshot {
    pub revision: i64,
    pub installed: Vec<ExtensionCatalogEntry>,
    pub bundled_available: Vec<ExtensionCatalogCandidate>,
    pub diagnostics: Vec<ExtensionCatalogDiagnostic>,
}

/// The result of checking one installed marketplace entry against its latest version.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionCatalogUpdateCheck {
    pub market_entry_id: String,
    pub installed_version_id: String,
    pub latest_version_id: String,
    pub update_available: bool,
}

pub(super) fn snapshot(app: &OperitApplication) -> Result<ExtensionCatalogSnapshot, String> {
    let package_scopes = app.getExtensionScopes("package".into())?;
    let skill_scopes = app.getExtensionScopes("skill".into())?;
    let mcp_scopes = app.getExtensionScopes("mcp".into())?;
    let market_origins = MarketInstallSupport::installed_marker_origins(app)?;

    let mut bundled_updates = BTreeSet::<String>::new();
    let mut bundled_available = Vec::<ExtensionCatalogCandidate>::new();
    let mut package_versions = BTreeMap::<String, String>::new();

    let packageManager = app.packageManager();
    let mut manager = packageManager
        .lock()
        .map_err(|error| error.to_string())?;
    let plugin_runtimes = manager.getToolPkgContainerRuntimes();
    let packages = manager.getExecutableAvailablePackages();
    let enabled_names = BTreeSet::from_iter(manager.getEnabledPackageNames());
    let customized_names = BTreeSet::from_iter(manager.getCustomizedPackageNames()?);
    let bundled_candidates = manager.getBundledExternalPackageCandidates();
    let load_issues = manager
        .getToolPkgLoadIssues()
        .into_iter()
        .map(|issue| {
            let can_delete = manager.canDeleteToolPkgLoadIssue(&issue)?;
            Ok((issue, can_delete))
        })
        .collect::<Result<Vec<_>, String>>()?;
    for source in manager.getPublishablePackageSources() {
        if let Some(version) = source.inferredVersion {
            package_versions.insert(source.packageName, version);
        }
    }
    drop(manager);

    let plugin_owned_names = plugin_runtimes
        .iter()
        .flat_map(|runtime| runtime.subpackages.iter().map(|child| child.packageName.clone()))
        .collect::<BTreeSet<_>>();

    for candidate in &bundled_candidates {
        if package_scopes.contains_key(&candidate.packageName) {
            bundled_updates.insert(candidate.packageName.clone());
            continue;
        }
        bundled_available.push(ExtensionCatalogCandidate {
            id: candidate.packageName.clone(),
            kind: if candidate.isToolPkg { "toolpkg" } else { "script" }.into(),
            display_name: candidate.displayName.resolve(false),
            description: candidate.description.resolve(false),
            version: candidate.version.clone(),
            authors: candidate.author.clone(),
            category: candidate.category.clone(),
            required_scope: candidate.requiredInstallationScope.clone(),
            tool_count: candidate.toolCount,
            subpackage_count: candidate.subpackageCount,
        });
    }

    let mut installed = Vec::<ExtensionCatalogEntry>::new();
    for runtime in &plugin_runtimes {
        let market_marker = market_origins
            .get(&("package".into(), runtime.packageName.clone()));
        let source = match &runtime.sourceType {
            ToolPkgSourceType::ASSET => "builtin",
            ToolPkgSourceType::MARKET => "market",
            ToolPkgSourceType::EXTERNAL => {
                if customized_names.contains(&runtime.packageName) {
                    "builtin"
                } else if bundled_updates.contains(&runtime.packageName) {
                    "bundled"
                } else {
                    "manual"
                }
            }
        };
        let enabled = enabled_names.contains(&runtime.packageName);
        installed.push(ExtensionCatalogEntry {
            id: runtime.packageName.clone(),
            kind: "toolpkg".into(),
            source: source.into(),
            display_name: runtime.displayName.resolve(false),
            description: runtime.description.resolve(false),
            version: runtime.version.clone(),
            authors: runtime.author.clone(),
            category: "ToolPkg".into(),
            scope: package_scopes
                .get(&runtime.packageName)
                .cloned()
                .unwrap_or_else(|| "device".into()),
            activation_state: if enabled { "enabled" } else { "disabled" }.into(),
            runtime_state: if runtime.dependencyIssues.is_empty() {
                "ready".into()
            } else {
                "error".into()
            },
            update_state: if bundled_updates.contains(&runtime.packageName) {
                "available".into()
            } else if market_marker.is_some() {
                "unknown".into()
            } else {
                "noSource".into()
            },
            is_customized: customized_names.contains(&runtime.packageName),
            market_entry_id: market_marker.map(|marker| marker.entry_id.clone()),
            installed_market_version_id: market_marker.map(|marker| marker.version_id.clone()),
            logo_resource_key: runtime.logoResource.as_ref().map(|resource| resource.key.clone()),
            logo_mime_type: runtime
                .logoResource
                .as_ref()
                .map(|resource| resource.mime.clone()),
            tool_count: runtime.subpackages.len(),
            subpackage_count: runtime.subpackages.len(),
            ui_module_count: runtime.uiModules.len(),
            has_ui: !runtime.uiModules.is_empty() || !runtime.navigationEntries.is_empty(),
            chat_sidebar_entry_ids: runtime
                .navigationEntries
                .iter()
                .filter(|entry| entry.surface == "chat_sidebar_tabs")
                .map(|entry| entry.id.clone())
                .collect(),
            dependency_issue_count: runtime.dependencyIssues.len(),
        });
    }

    for package in packages.values() {
        if plugin_owned_names.contains(&package.name) {
            continue;
        }
        let market_marker = market_origins.get(&("package".into(), package.name.clone()));
        let source = if customized_names.contains(&package.name) {
            "builtin"
        } else if package.is_built_in {
            "builtin"
        } else if market_marker.is_some() {
            "market"
        } else if bundled_updates.contains(&package.name) {
            "bundled"
        } else {
            "manual"
        };
        let enabled = enabled_names.contains(&package.name);
        installed.push(ExtensionCatalogEntry {
            id: package.name.clone(),
            kind: "script".into(),
            source: source.into(),
            display_name: non_empty_or(
                package.display_name.resolve(false),
                &package.name,
            ),
            description: package.description.resolve(false),
            version: package_versions
                .get(&package.name)
                .cloned()
                .unwrap_or_default(),
            authors: package.author.clone(),
            category: package.category.clone(),
            scope: package_scopes
                .get(&package.name)
                .cloned()
                .unwrap_or_else(|| "device".into()),
            activation_state: if enabled { "enabled" } else { "disabled" }.into(),
            runtime_state: "ready".into(),
            update_state: if bundled_updates.contains(&package.name) {
                "available".into()
            } else if market_marker.is_some() {
                "unknown".into()
            } else {
                "noSource".into()
            },
            is_customized: customized_names.contains(&package.name),
            market_entry_id: market_marker.map(|marker| marker.entry_id.clone()),
            installed_market_version_id: market_marker.map(|marker| marker.version_id.clone()),
            logo_resource_key: None,
            logo_mime_type: None,
            tool_count: package.tools.len(),
            subpackage_count: 0,
            ui_module_count: 0,
            has_ui: false,
            chat_sidebar_entry_ids: Vec::new(),
            dependency_issue_count: 0,
        });
    }

    let skill_repository = app.skillRepository();
    let (skills, skill_errors) = skill_repository.getAvailableSkillPackagesSnapshot();
    let visible_skills = skill_repository
        .getAiVisibleSkillPackages()
        .into_keys()
        .collect::<BTreeSet<_>>();
    for skill in skills.values() {
        let market_marker = market_origins.get(&("skill".into(), skill.name.clone()));
        installed.push(ExtensionCatalogEntry {
            id: skill.name.clone(),
            kind: "skill".into(),
            source: if market_marker.is_some() { "market" } else { "manual" }.into(),
            display_name: skill.name.clone(),
            description: skill.description.clone(),
            version: String::new(),
            authors: Vec::new(),
            category: "Skill".into(),
            scope: skill_scopes
                .get(&skill.name)
                .cloned()
                .unwrap_or_else(|| "device".into()),
            activation_state: if visible_skills.contains(&skill.name) {
                "aiVisible".into()
            } else {
                "aiHidden".into()
            },
            runtime_state: "ready".into(),
            update_state: if market_marker.is_some() {
                "unknown".into()
            } else {
                "noSource".into()
            },
            is_customized: false,
            market_entry_id: market_marker.map(|marker| marker.entry_id.clone()),
            installed_market_version_id: market_marker.map(|marker| marker.version_id.clone()),
            logo_resource_key: None,
            logo_mime_type: None,
            tool_count: 0,
            subpackage_count: 0,
            ui_module_count: 0,
            has_ui: false,
            chat_sidebar_entry_ids: Vec::new(),
            dependency_issue_count: 0,
        });
    }

    let mcp = MCPLocalServer::getInstance(&app.hostManager);
    let servers = mcp.getAllMCPServersChecked()?;
    let metadata = mcp.getAllPluginMetadata();
    let statuses = mcp.getAllServerStatus();
    for (server_id, server) in servers {
        let display_metadata = metadata.get(&server_id);
        let status = statuses.get(&server_id);
        let market_marker = market_origins
            .get(&("mcp".into(), server_id.clone()))
            .or_else(|| market_origins.get(&("mcp_plugin".into(), server_id.clone())));
        installed.push(ExtensionCatalogEntry {
            id: server_id.clone(),
            kind: "mcp".into(),
            source: if market_marker.is_some() { "market" } else { "manual" }.into(),
            display_name: display_metadata
                .map(|value| value.name.clone())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| server_id.clone()),
            description: display_metadata
                .map(|value| value.description.clone())
                .unwrap_or_default(),
            version: display_metadata
                .map(|value| value.version.clone())
                .unwrap_or_default(),
            authors: display_metadata
                .map(|value| vec![value.author.clone()])
                .unwrap_or_default(),
            category: "MCP".into(),
            scope: if server.command.trim().is_empty() {
                mcp_scopes
                    .get(&server_id)
                    .cloned()
                    .unwrap_or_else(|| "device".into())
            } else {
                "localOnly".into()
            },
            activation_state: if server.disabled { "disabled" } else { "enabled" }.into(),
            runtime_state: if let Some(error) = status
                .and_then(|value| value.errorMessage.as_ref())
                .filter(|value| !value.trim().is_empty())
            {
                format!("error:{error}")
            } else if status
                .map(|value| value.lastStartTime > 0 && value.lastStartTime >= value.lastStopTime)
                .unwrap_or(false)
            {
                "running".into()
            } else {
                "stopped".into()
            },
            update_state: if market_marker.is_some() {
                "unknown".into()
            } else {
                "noSource".into()
            },
            is_customized: false,
            market_entry_id: market_marker.map(|marker| marker.entry_id.clone()),
            installed_market_version_id: market_marker.map(|marker| marker.version_id.clone()),
            logo_resource_key: None,
            logo_mime_type: None,
            tool_count: status
                .and_then(|value| value.cachedTools.as_ref())
                .map(|tools| tools.len())
                .unwrap_or_default(),
            subpackage_count: 0,
            ui_module_count: 0,
            has_ui: false,
            chat_sidebar_entry_ids: Vec::new(),
            dependency_issue_count: 0,
        });
    }

    installed.sort_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    bundled_available.sort_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut diagnostics = load_issues
        .into_iter()
        .map(|(issue, can_delete)| ExtensionCatalogDiagnostic {
            source_path: issue.sourcePath,
            package_id: issue.packageName,
            display_name: issue.displayName,
            code: issue.code,
            message: issue.message,
            package_kind: issue.packageKind,
            can_delete,
        })
        .collect::<Vec<_>>();
    diagnostics.extend(skill_errors.into_iter().map(|(id, message)| {
        ExtensionCatalogDiagnostic {
            source_path: id.clone(),
            package_id: Some(id),
            display_name: "Skill".into(),
            code: "skill_load_failed".into(),
            message,
            package_kind: "skill".into(),
            can_delete: false,
        }
    }));
    diagnostics.sort_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
            .then_with(|| left.source_path.cmp(&right.source_path))
    });

    Ok(ExtensionCatalogSnapshot {
        revision: ExtensionStore::catalogRevisionFlow().value(),
        installed,
        bundled_available,
        diagnostics,
    })
}

pub(super) fn check_updates(
    app: &OperitApplication,
) -> Result<Vec<ExtensionCatalogUpdateCheck>, String> {
    let origins = MarketInstallSupport::installed_marker_origins(app)?;
    let mut results = Vec::new();
    let mut checked_entry_ids = BTreeSet::new();
    for (_, marker) in origins {
        if !checked_entry_ids.insert(marker.entry_id.clone()) {
            continue;
        }
        let entry = operit_providers::market::MarketStatsApiService::MarketStatsApiService::new()
            .get_entry_by_id(&marker.entry_id)?;
        let latest_version_id = entry
            .latest_version
            .map(|version| version.id.trim().to_string())
            .unwrap_or_default();
        let update_available = !latest_version_id.is_empty()
            && latest_version_id != marker.version_id;
        results.push(ExtensionCatalogUpdateCheck {
            market_entry_id: marker.entry_id,
            installed_version_id: marker.version_id,
            update_available,
            latest_version_id,
        });
    }
    results.sort_by(|left, right| left.market_entry_id.cmp(&right.market_entry_id));
    Ok(results)
}

pub(super) fn delete_diagnostic(
    app: &OperitApplication,
    diagnostic: ExtensionCatalogDiagnostic,
) -> Result<bool, String> {
    let issue = ToolPkgLoadIssue {
        sourcePath: diagnostic.source_path,
        packageName: diagnostic.package_id,
        displayName: diagnostic.display_name,
        code: diagnostic.code,
        message: diagnostic.message,
        packageKind: diagnostic.package_kind,
    };
    let packageManager = app.packageManager();
    let mut manager = packageManager
        .lock()
        .map_err(|error| error.to_string())?;
    manager.deleteToolPkgLoadIssue(issue)?;
    Ok(true)
}

fn non_empty_or(value: String, replacement: &str) -> String {
    if value.trim().is_empty() {
        replacement.to_string()
    } else {
        value
    }
}
