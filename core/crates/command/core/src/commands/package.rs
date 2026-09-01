use crate::commands::tool;
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_tools::tools::packTool::RuntimePackageManager::BundledExternalPackageCandidate;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Runs package management commands.
pub fn run_package_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let tool_handler = application.toolHandler.clone();
    if args.is_empty() {
        print_package_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "help" | "-h" | "--help" => {
            print_package_usage(output);
            Ok(())
        }
        "dir" => {
            let package_manager = package_manager(&tool_handler);
            let guard = package_manager
                .lock()
                .expect("package manager mutex poisoned");
            let path = guard.getExternalPackagesPath();
            output.push_stdout_line(format!("Package directory: {path}"));
            output.setJsonStdout(serde_json::json!({"packageDirectory": path}));
            Ok(())
        }
        "list" => list_packages(tool_handler, output),
        "menu" => list_input_menu_definitions(application, output),
        "more" => list_more_packages(tool_handler, output),
        "show" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 package show <name>".to_string())?;
            show_package(tool_handler, name, output)
        }
        "import" => {
            let path = args.get(1).ok_or_else(|| {
                "usage: operit2 package import <js-ts-hjson-toolpkg-path>".to_string()
            })?;
            let package_manager = package_manager(&tool_handler);
            let mut guard = package_manager
                .lock()
                .expect("package manager mutex poisoned");
            let result = guard.addPackageFileFromExternalStorageResult(path)?;
            output.push_stdout_line(format!("Package imported: {}", result.packageName));
            output.push_stdout_line(format!("Format: {}", result.packageFormat));
            output.push_stdout_line(format!("Stored: {}", result.storedPath));
            output.setJsonStdout(serde_json::json!({ "path": path, "result": result }));
            Ok(())
        }
        "load" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 package load <name>".to_string())?;
            load_more_package(tool_handler, name, output)
        }
        "delete" | "remove" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 package delete <name>".to_string())?;
            delete_package(tool_handler, name, output)
        }
        "enable" => set_package_enabled(tool_handler, args.get(1), true, output),
        "disable" => set_package_enabled(tool_handler, args.get(1), false, output),
        "use" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 package use <name>".to_string())?;
            let package_manager = package_manager(&tool_handler);
            let mut guard = package_manager
                .lock()
                .expect("package manager mutex poisoned");
            let message = guard.usePackage(name);
            output.push_stdout_line(format!("Package ready: {name}"));
            output.push_stdout_line(message.clone());
            output.setJsonStdout(serde_json::json!({
                "name": name,
                "message": message,
                "ready": true,
            }));
            Ok(())
        }
        "exec" => {
            let tool_name = args.get(1).ok_or_else(|| {
                "usage: operit2 package exec <package:tool> <params-json>".to_string()
            })?;
            let params_json = args.get(2).ok_or_else(|| {
                "usage: operit2 package exec <package:tool> <params-json>".to_string()
            })?;
            let package_name = tool_name
                .split_once(':')
                .map(|(package_name, _)| package_name.to_string())
                .ok_or_else(|| "package exec tool name must use package:tool format".to_string())?;
            {
                let package_manager = package_manager(&tool_handler);
                let mut guard = package_manager
                    .lock()
                    .expect("package manager mutex poisoned");
                guard.usePackage(&package_name);
            }
            tool::exec_tool(tool_handler, tool_name, params_json, output)
        }
        _ => {
            print_package_usage(output);
            Ok(())
        }
    }
}

/// Prints input-menu definitions produced by the registered ToolPkg bridge.
fn list_input_menu_definitions(
    application: &OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let bridge = application.inputMenuToggleBridge();
    let deadline = Instant::now() + Duration::from_secs(10);
    let definitions = loop {
        let definitions = bridge.createToggleDefinitionsForFlutter(None, BTreeMap::new(), None);
        let loading = definitions
            .iter()
            .any(|definition| definition.id == "toolpkg_input_menu_loading");
        if !loading {
            break definitions;
        }
        if Instant::now() >= deadline {
            return Err(
                "input-menu ToolPkg hooks did not finish loading within 10 seconds".to_string(),
            );
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let mut items = Vec::new();
    output.push_stdout_line(format!("Input menu toggles: {}", definitions.len()));
    for definition in definitions {
        let title = definition.title.clone().unwrap_or_default();
        let slot = definition.slot.clone().unwrap_or_default();
        output.push_stdout_line(format!(
            "- {} — {} — checked: {} — enabled: {} — slot: {}",
            definition.id, title, definition.isChecked, definition.isEnabled, slot,
        ));
        items.push(serde_json::to_value(definition).map_err(|error| error.to_string())?);
    }
    output.setJsonStdout(serde_json::Value::Array(items));
    Ok(())
}

/// Lists loaded packages as readable rows and JSON objects.
fn list_packages(
    tool_handler: AIToolHandler,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&tool_handler);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let enabled = guard.getEnabledPackageNames();
    let packages = guard.getAvailablePackages();
    let mut items = Vec::new();
    output.push_stdout_line(format!("Packages: {}", packages.len()));
    for (name, package) in packages {
        let isEnabled = enabled.contains(&name);
        let description = package.description.resolve(false);
        output.push_stdout_line(format!(
            "- {} — {} tools — {}{}",
            name,
            package.tools.len(),
            description,
            if isEnabled {
                " (enabled)"
            } else {
                " (disabled)"
            }
        ));
        items.push(serde_json::json!({
            "name": name,
            "displayName": package.display_name.resolve(false),
            "description": description,
            "category": package.category,
            "enabled": isEnabled,
            "enabledByDefault": package.enabled_by_default,
            "isBuiltIn": package.is_built_in,
            "tools": package.tools.len(),
        }));
    }
    output.setJsonStdout(serde_json::Value::Array(items));
    Ok(())
}

/// Lists bundled packages available to load.
fn list_more_packages(
    tool_handler: AIToolHandler,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&tool_handler);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let candidates = guard.getBundledExternalPackageCandidates();
    let mut items = Vec::new();
    output.push_stdout_line(format!("Bundled packages: {}", candidates.len()));
    for candidate in candidates {
        output.push_stdout_line(format_bundled_external_candidate(&candidate));
        items.push(bundled_external_candidate_json(&candidate));
    }
    output.setJsonStdout(serde_json::Value::Array(items));
    Ok(())
}

/// Shows a loaded package with its tools and parameters.
fn show_package(
    tool_handler: AIToolHandler,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&tool_handler);
    let guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let package = guard
        .getPackageTools(name)
        .ok_or_else(|| format!("package not found: {name}"))?;
    output.push_stdout_line(format!("Package: {}", package.name));
    output.push_stdout_line(format!(
        "Display name: {}",
        package.display_name.resolve(false)
    ));
    output.push_stdout_line(format!(
        "Description: {}",
        package.description.resolve(false)
    ));
    output.push_stdout_line(format!("Category: {}", package.category));
    output.push_stdout_line(format!(
        "Enabled by default: {}",
        package.enabled_by_default
    ));
    output.push_stdout_line(format!("Built in: {}", package.is_built_in));
    output.push_stdout_line(format!("Tools: {}", package.tools.len()));
    let mut tools = Vec::new();
    for tool in &package.tools {
        output.push_stdout_line(format!(
            "- {} — advice: {} — {}",
            tool.name,
            tool.advice,
            tool.description.resolve(false)
        ));
        let mut parameters = Vec::new();
        for parameter in &tool.parameters {
            output.push_stdout_line(format!(
                "  - {} — type: {} — required: {} — {}",
                parameter.name,
                parameter.parameter_type,
                parameter.required,
                parameter.description.resolve(false)
            ));
            parameters.push(serde_json::json!({
                "name": parameter.name,
                "description": parameter.description.resolve(false),
                "parameterType": parameter.parameter_type,
                "required": parameter.required,
            }));
        }
        tools.push(serde_json::json!({
            "name": tool.name,
            "description": tool.description.resolve(false),
            "advice": tool.advice,
            "parameters": parameters,
        }));
    }
    output.setJsonStdout(serde_json::json!({
        "name": package.name,
        "displayName": package.display_name.resolve(false),
        "description": package.description.resolve(false),
        "category": package.category,
        "enabledByDefault": package.enabled_by_default,
        "isBuiltIn": package.is_built_in,
        "tools": tools,
    }));
    Ok(())
}

/// Loads one bundled package into user package storage.
fn load_more_package(
    tool_handler: AIToolHandler,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&tool_handler);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let message = guard.importBundledExternalPackage(name);
    output.push_stdout_line(format!("Package loaded: {name}"));
    output.push_stdout_line(message.clone());
    output.setJsonStdout(serde_json::json!({
        "name": name,
        "message": message,
        "loaded": true,
    }));
    Ok(())
}

/// Deletes one package from external package storage.
fn delete_package(
    tool_handler: AIToolHandler,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&tool_handler);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    if !guard.deletePackage(name) {
        return Err(format!("Failed to delete package: {name}"));
    }
    output.push_stdout_line(format!("Package deleted: {name}"));
    output.setJsonStdout(serde_json::json!({"name": name, "deleted": true}));
    Ok(())
}

/// Sets the enabled state for one package.
fn set_package_enabled(
    tool_handler: AIToolHandler,
    name: Option<&String>,
    enabled: bool,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let name = name.ok_or_else(|| {
        if enabled {
            "usage: operit2 package enable <name>".to_string()
        } else {
            "usage: operit2 package disable <name>".to_string()
        }
    })?;
    let package_manager = package_manager(&tool_handler);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let message = if enabled {
        guard.enablePackage(name)
    } else {
        guard.disablePackage(name)
    };
    output.push_stdout_line(format!("Package {}: {name}", enabled_status(enabled)));
    output.push_stdout_line(message.clone());
    output.setJsonStdout(serde_json::json!({
        "name": name,
        "enabled": enabled,
        "message": message,
    }));
    Ok(())
}

/// Returns the runtime package manager from the tool handler.
fn package_manager(
    tool_handler: &AIToolHandler,
) -> std::sync::Arc<
    std::sync::Mutex<operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager>,
> {
    tool_handler.getOrCreatePackageManager()
}

/// Formats one bundled package candidate for CLI text.
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

/// Builds a JSON object for one bundled package candidate.
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

/// Formats an enabled state as CLI text.
fn enabled_status(enabled: bool) -> &'static str {
    if enabled {
        "enabled"
    } else {
        "disabled"
    }
}

/// Prints package command usage.
fn print_package_usage(output: &mut CoreCommandOutput) {
    let lines = vec![
        "operit2 package help",
        "operit2 package dir                                  Show user package directory.",
        "operit2 package list                                 List loaded script packages and ToolPkg subpackages.",
        "operit2 package more                                 List app-bundled official extras not loaded yet; type=script/toolpkg.",
        "operit2 package load <name>                          Load one item from 'package more' into the user package directory.",
        "operit2 package show <name>                          Show a loaded package.",
        "operit2 package import <js-ts-hjson-toolpkg-path>    Import a package file.",
        "operit2 package delete <name>                        Delete an external package.",
        "operit2 package enable <name>                        Enable a loaded package.",
        "operit2 package disable <name>                       Disable a loaded package.",
        "operit2 package use <name>                           Enable a package for execution.",
        "operit2 package exec <package:tool> <params-json>    Execute one package tool.",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}
