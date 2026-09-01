use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_tools::tools::packTool::RuntimePackageManager::BundledExternalPackageCandidate;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use std::collections::BTreeSet;

/// Runs ToolPkg plugin management commands.
pub fn run_plugin_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let tool_handler = application.toolHandler.clone();
    if args.is_empty() {
        print_plugin_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "help" | "-h" | "--help" => {
            print_plugin_usage(output);
            Ok(())
        }
        "list" => list_plugins(tool_handler, output),
        "more" => list_more_plugins(tool_handler, output),
        "show" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 plugin show <name>".to_string())?;
            show_plugin(tool_handler, name, output)
        }
        "import" => {
            let path = args
                .get(1)
                .ok_or_else(|| "usage: operit2 plugin import <toolpkg-path>".to_string())?;
            let package_manager = package_manager(&tool_handler);
            let mut guard = package_manager
                .lock()
                .expect("package manager mutex poisoned");
            let result = guard.addPackageFileFromExternalStorageResult(path)?;
            output.push_stdout_line(format!("Plugin imported: {}", result.packageName));
            output.push_stdout_line(format!("Format: {}", result.packageFormat));
            output.push_stdout_line(format!("Stored: {}", result.storedPath));
            output.setJsonStdout(serde_json::json!({ "path": path, "result": result }));
            Ok(())
        }
        "load" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 plugin load <name>".to_string())?;
            load_more_plugin(tool_handler, name, output)
        }
        "delete" | "remove" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 plugin delete <name>".to_string())?;
            delete_plugin(tool_handler, name, output)
        }
        "enable" => set_plugin_enabled(tool_handler, args.get(1), true, output),
        "disable" => set_plugin_enabled(tool_handler, args.get(1), false, output),
        _ => {
            print_plugin_usage(output);
            Ok(())
        }
    }
}

/// Lists loaded ToolPkg plugins.
fn list_plugins(tool_handler: AIToolHandler, output: &mut CoreCommandOutput) -> Result<(), String> {
    let package_manager = package_manager(&tool_handler);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let enabled = enabled_plugin_names_from_manager(&guard);
    let plugins = guard.getToolPkgContainerRuntimes();
    let mut items = Vec::new();
    output.push_stdout_line(format!("Plugins: {}", plugins.len()));
    for plugin in plugins {
        let isEnabled = enabled.contains(&plugin.packageName);
        let description = plugin.description.resolve(false);
        output.push_stdout_line(format!(
            "- {} — {} subpackages — {}{}",
            plugin.packageName,
            plugin.subpackages.len(),
            description,
            if isEnabled {
                " (enabled)"
            } else {
                " (disabled)"
            }
        ));
        items.push(serde_json::json!({
            "name": plugin.packageName,
            "displayName": plugin.displayName.resolve(false),
            "description": description,
            "version": plugin.version,
            "author": plugin.author,
            "enabled": isEnabled,
            "sourceType": format!("{:?}", plugin.sourceType),
            "sourcePath": plugin.sourcePath,
            "subpackages": plugin.subpackages.len(),
        }));
    }
    output.setJsonStdout(serde_json::Value::Array(items));
    Ok(())
}

/// Lists bundled ToolPkg plugins available to load.
fn list_more_plugins(
    tool_handler: AIToolHandler,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&tool_handler);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let candidates = guard.getBundledExternalPackageCandidates();
    let mut items = Vec::new();
    output.push_stdout_line(format!("Bundled plugins: {}", candidates.len()));
    for candidate in candidates {
        output.push_stdout_line(format_bundled_external_candidate(&candidate));
        items.push(bundled_external_candidate_json(&candidate));
    }
    output.setJsonStdout(serde_json::Value::Array(items));
    Ok(())
}

/// Shows one loaded ToolPkg plugin.
fn show_plugin(
    tool_handler: AIToolHandler,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&tool_handler);
    let guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let plugin = guard
        .getToolPkgContainerRuntime(name)
        .ok_or_else(|| format!("plugin not found: {name}"))?;
    let enabled = enabled_plugin_names_from_manager(&guard);
    let isEnabled = enabled.contains(&plugin.packageName);
    output.push_stdout_line(format!("Plugin: {}", plugin.packageName));
    output.push_stdout_line(format!(
        "Display name: {}",
        plugin.displayName.resolve(false)
    ));
    output.push_stdout_line(format!(
        "Description: {}",
        plugin.description.resolve(false)
    ));
    output.push_stdout_line(format!("Version: {}", plugin.version));
    output.push_stdout_line(format!("Author: {}", plugin.author.join(", ")));
    output.push_stdout_line(format!("Enabled: {isEnabled}"));
    output.push_stdout_line(format!("Source type: {:?}", plugin.sourceType));
    output.push_stdout_line(format!("Source path: {}", plugin.sourcePath));
    output.push_stdout_line(format!("Main entry: {}", plugin.mainEntry));
    output.push_stdout_line(format!("Subpackages: {}", plugin.subpackages.len()));
    for subpackage in &plugin.subpackages {
        output.push_stdout_line(format!("- {}", subpackage.packageName));
    }
    let counts = serde_json::json!({
        "resources": plugin.resources.len(),
        "uiModules": plugin.uiModules.len(),
        "uiRoutes": plugin.uiRoutes.len(),
        "navigationEntries": plugin.navigationEntries.len(),
        "desktopWidgets": plugin.desktopWidgets.len(),
        "appLifecycleHooks": plugin.appLifecycleHooks.len(),
        "messageProcessingPlugins": plugin.messageProcessingPlugins.len(),
        "xmlRenderPlugins": plugin.xmlRenderPlugins.len(),
        "inputMenuTogglePlugins": plugin.inputMenuTogglePlugins.len(),
        "chatInputHooks": plugin.chatInputHooks.len(),
        "chatViewHooks": plugin.chatViewHooks.len(),
        "toolLifecycleHooks": plugin.toolLifecycleHooks.len(),
        "promptInputHooks": plugin.promptInputHooks.len(),
        "promptHistoryHooks": plugin.promptHistoryHooks.len(),
        "promptEstimateHistoryHooks": plugin.promptEstimateHistoryHooks.len(),
        "systemPromptComposeHooks": plugin.systemPromptComposeHooks.len(),
        "toolPromptComposeHooks": plugin.toolPromptComposeHooks.len(),
        "promptFinalizeHooks": plugin.promptFinalizeHooks.len(),
        "promptEstimateFinalizeHooks": plugin.promptEstimateFinalizeHooks.len(),
        "summaryGenerateHooks": plugin.summaryGenerateHooks.len(),
        "aiProviders": plugin.aiProviders.len(),
    });
    output.push_stdout_line(format!("Resources: {}", plugin.resources.len()));
    output.push_stdout_line(format!("UI modules: {}", plugin.uiModules.len()));
    output.push_stdout_line(format!("UI routes: {}", plugin.uiRoutes.len()));
    output.push_stdout_line(format!(
        "Navigation entries: {}",
        plugin.navigationEntries.len()
    ));
    output.push_stdout_line(format!("Desktop widgets: {}", plugin.desktopWidgets.len()));
    output.push_stdout_line(format!(
        "Hooks — app: {}, message: {}, XML: {}, input menu: {}, chat input: {}, chat view: {}, tool lifecycle: {}, prompt input: {}, prompt history: {}, system prompt: {}, tool prompt: {}, prompt finalize: {}, summary: {}, AI providers: {}",
        plugin.appLifecycleHooks.len(),
        plugin.messageProcessingPlugins.len(),
        plugin.xmlRenderPlugins.len(),
        plugin.inputMenuTogglePlugins.len(),
        plugin.chatInputHooks.len(),
        plugin.chatViewHooks.len(),
        plugin.toolLifecycleHooks.len(),
        plugin.promptInputHooks.len(),
        plugin.promptHistoryHooks.len(),
        plugin.systemPromptComposeHooks.len(),
        plugin.toolPromptComposeHooks.len(),
        plugin.promptFinalizeHooks.len(),
        plugin.summaryGenerateHooks.len(),
        plugin.aiProviders.len()
    ));
    output.setJsonStdout(serde_json::json!({
        "name": plugin.packageName,
        "displayName": plugin.displayName.resolve(false),
        "description": plugin.description.resolve(false),
        "version": plugin.version,
        "author": plugin.author,
        "enabled": isEnabled,
        "sourceType": format!("{:?}", plugin.sourceType),
        "sourcePath": plugin.sourcePath,
        "mainEntry": plugin.mainEntry,
        "subpackages": plugin.subpackages,
        "counts": counts,
    }));
    Ok(())
}

/// Loads one bundled ToolPkg plugin into user package storage.
fn load_more_plugin(
    tool_handler: AIToolHandler,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&tool_handler);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let message = guard.importBundledExternalPackage(name);
    output.push_stdout_line(format!("Plugin loaded: {name}"));
    output.push_stdout_line(message.clone());
    output.setJsonStdout(serde_json::json!({
        "name": name,
        "message": message,
        "loaded": true,
    }));
    Ok(())
}

/// Deletes one external ToolPkg plugin.
fn delete_plugin(
    tool_handler: AIToolHandler,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&tool_handler);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    if !guard.deletePackage(name) {
        return Err(format!("Failed to delete plugin: {name}"));
    }
    output.push_stdout_line(format!("Plugin deleted: {name}"));
    output.setJsonStdout(serde_json::json!({"name": name, "deleted": true}));
    Ok(())
}

/// Formats one bundled plugin candidate for CLI text.
fn format_bundled_external_candidate(candidate: &BundledExternalPackageCandidate) -> String {
    format!(
        "- {} — type: {} — loaded: false — tools: {} — subpackages: {} — {}",
        candidate.packageName,
        candidate.packageKind,
        candidate.toolCount,
        candidate.subpackageCount,
        candidate.description.resolve(false)
    )
}

/// Builds a JSON object for one bundled plugin candidate.
fn bundled_external_candidate_json(
    candidate: &BundledExternalPackageCandidate,
) -> serde_json::Value {
    serde_json::json!({
        "name": candidate.packageName,
        "type": candidate.packageKind,
        "loaded": false,
        "description": candidate.description.resolve(false),
        "tools": candidate.toolCount,
        "subpackages": candidate.subpackageCount,
    })
}

/// Sets the enabled state for one ToolPkg plugin.
fn set_plugin_enabled(
    tool_handler: AIToolHandler,
    name: Option<&String>,
    enabled: bool,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let name = name.ok_or_else(|| {
        if enabled {
            "usage: operit2 plugin enable <name>".to_string()
        } else {
            "usage: operit2 plugin disable <name>".to_string()
        }
    })?;
    let package_manager = package_manager(&tool_handler);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let message = if enabled {
        guard.enableToolPkgContainer(name)
    } else {
        guard.disableToolPkgContainer(name)
    };
    output.push_stdout_line(format!("Plugin {}: {name}", enabled_status(enabled)));
    output.push_stdout_line(message.clone());
    output.setJsonStdout(serde_json::json!({
        "name": name,
        "enabled": enabled,
        "message": message,
    }));
    Ok(())
}

/// Reads the enabled ToolPkg plugin names from the package manager.
fn enabled_plugin_names_from_manager(
    manager: &operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager,
) -> BTreeSet<String> {
    manager
        .getEnabledToolPkgContainerRuntimes()
        .into_iter()
        .map(|plugin| plugin.packageName)
        .collect()
}

/// Returns the runtime package manager from the tool handler.
fn package_manager(
    tool_handler: &AIToolHandler,
) -> std::sync::Arc<
    std::sync::Mutex<operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager>,
> {
    tool_handler.getOrCreatePackageManager()
}

/// Formats an enabled state as CLI text.
fn enabled_status(enabled: bool) -> &'static str {
    if enabled {
        "enabled"
    } else {
        "disabled"
    }
}

/// Prints ToolPkg plugin command usage.
fn print_plugin_usage(output: &mut CoreCommandOutput) {
    let lines = vec![
        "operit2 plugin help",
        "operit2 plugin list                         List loaded ToolPkg plugins.",
        "operit2 plugin more                         List app-bundled official extras not loaded yet; type=toolpkg/script.",
        "operit2 plugin load <name>                  Load one item from 'plugin more' into the user package directory.",
        "operit2 plugin show <name>                  Show a loaded ToolPkg plugin.",
        "operit2 plugin import <toolpkg-path>        Import a ToolPkg file.",
        "operit2 plugin delete <name>                Delete an external ToolPkg plugin.",
        "operit2 plugin enable <name>                Enable a loaded ToolPkg plugin.",
        "operit2 plugin disable <name>               Disable a loaded ToolPkg plugin.",
        "note: ordinary script packages shown by type=script can also be managed with 'operit2 package more/load'.",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}
