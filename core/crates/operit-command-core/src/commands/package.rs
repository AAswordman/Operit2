use crate::commands::tool;
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_tools::tools::packTool::RuntimePackageManager::BundledExternalPackageCandidate;
use operit_tools::tools::AIToolHandler::AIToolHandler;

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
            output.push_stdout_line(guard.getExternalPackagesPath());
            Ok(())
        }
        "list" => list_packages(tool_handler, output),
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
            output.push_stdout_line(guard.addPackageFileFromExternalStorage(path));
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
            output.push_stdout_line(guard.usePackage(name));
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
    for (name, package) in packages {
        output.push_stdout_line(format!(
            "{}\tenabled={}\t{}\ttools={}",
            name,
            enabled.contains(&name),
            package.description.resolve(false),
            package.tools.len()
        ));
    }
    Ok(())
}

fn list_more_packages(
    tool_handler: AIToolHandler,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&tool_handler);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    for candidate in guard.getBundledExternalPackageCandidates() {
        output.push_stdout_line(format_bundled_external_candidate(&candidate));
    }
    Ok(())
}

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
    output.push_stdout_line(format!("name={}", package.name));
    output.push_stdout_line(format!(
        "displayName={}",
        package.display_name.resolve(false)
    ));
    output.push_stdout_line(format!(
        "description={}",
        package.description.resolve(false)
    ));
    output.push_stdout_line(format!("category={}", package.category));
    output.push_stdout_line(format!("enabledByDefault={}", package.enabled_by_default));
    output.push_stdout_line(format!("isBuiltIn={}", package.is_built_in));
    output.push_stdout_line(format!("tools={}", package.tools.len()));
    for tool in package.tools {
        output.push_stdout_line(format!(
            "- {}\tadvice={}\t{}",
            tool.name,
            tool.advice,
            tool.description.resolve(false)
        ));
        for parameter in tool.parameters {
            output.push_stdout_line(format!(
                "  - {}\t{}\trequired={}\t{}",
                parameter.name,
                parameter.parameter_type,
                parameter.required,
                parameter.description.resolve(false)
            ));
        }
    }
    Ok(())
}

fn load_more_package(
    tool_handler: AIToolHandler,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&tool_handler);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    output.push_stdout_line(guard.importBundledExternalPackage(name));
    Ok(())
}

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
    output.push_stdout_line(format!("deleted={name}"));
    Ok(())
}

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
    output.push_stdout_line(message);
    Ok(())
}

fn package_manager(
    tool_handler: &AIToolHandler,
) -> std::sync::Arc<
    std::sync::Mutex<operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager>,
> {
    tool_handler.getOrCreatePackageManager()
}

fn format_bundled_external_candidate(candidate: &BundledExternalPackageCandidate) -> String {
    format!(
        "{}\ttype={}\tloaded=false\t{}\ttools={}\tsubpackages={}",
        candidate.packageName,
        candidate.packageKind,
        candidate.description.resolve(false),
        candidate.toolCount,
        candidate.subpackageCount
    )
}

fn print_package_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 package help");
    output.push_stdout_line(
        "operit2 package dir                                  Show user package directory.",
    );
    output.push_stdout_line("operit2 package list                                 List loaded script packages and ToolPkg subpackages.");
    output.push_stdout_line("operit2 package more                                 List app-bundled official extras not loaded yet; type=script/toolpkg.");
    output.push_stdout_line("operit2 package load <name>                          Load one item from 'package more' into the user package directory.");
    output.push_stdout_line(
        "operit2 package show <name>                          Show a loaded package.",
    );
    output.push_stdout_line(
        "operit2 package import <js-ts-hjson-toolpkg-path>    Import a package file.",
    );
    output.push_stdout_line(
        "operit2 package delete <name>                        Delete an external package.",
    );
    output.push_stdout_line(
        "operit2 package enable <name>                        Enable a loaded package.",
    );
    output.push_stdout_line(
        "operit2 package disable <name>                       Disable a loaded package.",
    );
    output.push_stdout_line(
        "operit2 package use <name>                           Enable a package for execution.",
    );
    output.push_stdout_line(
        "operit2 package exec <package:tool> <params-json>    Execute one package tool.",
    );
}
