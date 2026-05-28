use crate::commands::tool;
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;

pub fn run_package_command(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_package_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "dir" => {
            let package_manager = package_manager(&context);
            let guard = package_manager
                .lock()
                .expect("package manager mutex poisoned");
            output.push_stdout_line(guard.getExternalPackagesPath());
            Ok(())
        }
        "list" => list_packages(context, output),
        "show" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 package show <name>".to_string())?;
            show_package(context, name, output)
        }
        "import" => {
            let path = args.get(1).ok_or_else(|| {
                "usage: operit2 package import <js-ts-hjson-toolpkg-path>".to_string()
            })?;
            let package_manager = package_manager(&context);
            let mut guard = package_manager
                .lock()
                .expect("package manager mutex poisoned");
            output.push_stdout_line(guard.addPackageFileFromExternalStorage(path));
            Ok(())
        }
        "enable" => set_package_enabled(context, args.get(1), true, output),
        "disable" => set_package_enabled(context, args.get(1), false, output),
        "use" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 package use <name>".to_string())?;
            let package_manager = package_manager(&context);
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
                let package_manager = package_manager(&context);
                let mut guard = package_manager
                    .lock()
                    .expect("package manager mutex poisoned");
                guard.usePackage(&package_name);
            }
            tool::exec_tool(context, tool_name, params_json, output)
        }
        _ => {
            print_package_usage(output);
            Ok(())
        }
    }
}

fn list_packages(
    context: OperitApplicationContext,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&context);
    let guard = package_manager
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

fn show_package(
    context: OperitApplicationContext,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&context);
    let guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let package = guard
        .getPackageTools(name)
        .ok_or_else(|| format!("package not found: {name}"))?;
    output.push_stdout_line(format!("name={}", package.name));
    output.push_stdout_line(format!("displayName={}", package.display_name.resolve(false)));
    output.push_stdout_line(format!("description={}", package.description.resolve(false)));
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

fn set_package_enabled(
    context: OperitApplicationContext,
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
    let package_manager = package_manager(&context);
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
    context: &OperitApplicationContext,
) -> std::sync::Arc<std::sync::Mutex<operit_runtime::core::tools::packTool::PackageManager::PackageManager>>
{
    AIToolHandler::getInstance(context.clone()).getOrCreatePackageManager()
}

fn print_package_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 package dir");
    output.push_stdout_line("operit2 package list");
    output.push_stdout_line("operit2 package show <name>");
    output.push_stdout_line("operit2 package import <js-ts-hjson-toolpkg-path>");
    output.push_stdout_line("operit2 package enable <name>");
    output.push_stdout_line("operit2 package disable <name>");
    output.push_stdout_line("operit2 package use <name>");
    output.push_stdout_line("operit2 package exec <package:tool> <params-json>");
}
