use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
use std::collections::BTreeSet;

pub fn run_plugin_command(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_plugin_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "list" => list_plugins(context, output),
        "show" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 plugin show <name>".to_string())?;
            show_plugin(context, name, output)
        }
        "import" => {
            let path = args
                .get(1)
                .ok_or_else(|| "usage: operit2 plugin import <toolpkg-path>".to_string())?;
            let package_manager = package_manager(&context);
            let mut guard = package_manager
                .lock()
                .expect("package manager mutex poisoned");
            output.push_stdout_line(guard.addPackageFileFromExternalStorage(path));
            Ok(())
        }
        "enable" => set_plugin_enabled(context, args.get(1), true, output),
        "disable" => set_plugin_enabled(context, args.get(1), false, output),
        _ => {
            print_plugin_usage(output);
            Ok(())
        }
    }
}

fn list_plugins(
    context: OperitApplicationContext,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&context);
    let guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let enabled = enabled_plugin_names_from_manager(&guard);
    for plugin in guard.getToolPkgContainerRuntimes() {
        output.push_stdout_line(format!(
            "{}\tenabled={}\t{}\tsubpackages={}",
            plugin.packageName,
            enabled.contains(&plugin.packageName),
            plugin.description.resolve(false),
            plugin.subpackages.len()
        ));
    }
    Ok(())
}

fn show_plugin(
    context: OperitApplicationContext,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let package_manager = package_manager(&context);
    let guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let plugin = guard
        .getToolPkgContainerRuntime(name)
        .ok_or_else(|| format!("plugin not found: {name}"))?;
    let enabled = enabled_plugin_names_from_manager(&guard);
    output.push_stdout_line(format!("name={}", plugin.packageName));
    output.push_stdout_line(format!("displayName={}", plugin.displayName.resolve(false)));
    output.push_stdout_line(format!("description={}", plugin.description.resolve(false)));
    output.push_stdout_line(format!("version={}", plugin.version));
    output.push_stdout_line(format!("author={}", plugin.author.join(",")));
    output.push_stdout_line(format!("enabled={}", enabled.contains(&plugin.packageName)));
    output.push_stdout_line(format!("sourceType={:?}", plugin.sourceType));
    output.push_stdout_line(format!("sourcePath={}", plugin.sourcePath));
    output.push_stdout_line(format!("mainEntry={}", plugin.mainEntry));
    output.push_stdout_line(format!("subpackages={}", plugin.subpackages.len()));
    for subpackage in plugin.subpackages {
        output.push_stdout_line(format!("- {}", subpackage.packageName));
    }
    output.push_stdout_line(format!("resources={}", plugin.resources.len()));
    output.push_stdout_line(format!("uiModules={}", plugin.uiModules.len()));
    output.push_stdout_line(format!("uiRoutes={}", plugin.uiRoutes.len()));
    output.push_stdout_line(format!(
        "navigationEntries={}",
        plugin.navigationEntries.len()
    ));
    output.push_stdout_line(format!("desktopWidgets={}", plugin.desktopWidgets.len()));
    output.push_stdout_line(format!(
        "appLifecycleHooks={}",
        plugin.appLifecycleHooks.len()
    ));
    output.push_stdout_line(format!(
        "messageProcessingPlugins={}",
        plugin.messageProcessingPlugins.len()
    ));
    output.push_stdout_line(format!(
        "xmlRenderPlugins={}",
        plugin.xmlRenderPlugins.len()
    ));
    output.push_stdout_line(format!(
        "inputMenuTogglePlugins={}",
        plugin.inputMenuTogglePlugins.len()
    ));
    output.push_stdout_line(format!("chatInputHooks={}", plugin.chatInputHooks.len()));
    output.push_stdout_line(format!("chatViewHooks={}", plugin.chatViewHooks.len()));
    output.push_stdout_line(format!(
        "toolLifecycleHooks={}",
        plugin.toolLifecycleHooks.len()
    ));
    output.push_stdout_line(format!(
        "promptInputHooks={}",
        plugin.promptInputHooks.len()
    ));
    output.push_stdout_line(format!(
        "promptHistoryHooks={}",
        plugin.promptHistoryHooks.len()
    ));
    output.push_stdout_line(format!(
        "promptEstimateHistoryHooks={}",
        plugin.promptEstimateHistoryHooks.len()
    ));
    output.push_stdout_line(format!(
        "systemPromptComposeHooks={}",
        plugin.systemPromptComposeHooks.len()
    ));
    output.push_stdout_line(format!(
        "toolPromptComposeHooks={}",
        plugin.toolPromptComposeHooks.len()
    ));
    output.push_stdout_line(format!(
        "promptFinalizeHooks={}",
        plugin.promptFinalizeHooks.len()
    ));
    output.push_stdout_line(format!(
        "promptEstimateFinalizeHooks={}",
        plugin.promptEstimateFinalizeHooks.len()
    ));
    output.push_stdout_line(format!(
        "summaryGenerateHooks={}",
        plugin.summaryGenerateHooks.len()
    ));
    output.push_stdout_line(format!("aiProviders={}", plugin.aiProviders.len()));
    Ok(())
}

fn set_plugin_enabled(
    context: OperitApplicationContext,
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
    let package_manager = package_manager(&context);
    let mut guard = package_manager
        .lock()
        .expect("package manager mutex poisoned");
    let message = if enabled {
        guard.enableToolPkgContainer(name)
    } else {
        guard.disableToolPkgContainer(name)
    };
    output.push_stdout_line(message);
    Ok(())
}

fn enabled_plugin_names_from_manager(
    manager: &operit_runtime::core::tools::packTool::PackageManager::PackageManager,
) -> BTreeSet<String> {
    manager
        .getEnabledToolPkgContainerRuntimes()
        .into_iter()
        .map(|plugin| plugin.packageName)
        .collect()
}

fn package_manager(
    context: &OperitApplicationContext,
) -> std::sync::Arc<
    std::sync::Mutex<operit_runtime::core::tools::packTool::PackageManager::PackageManager>,
> {
    AIToolHandler::getInstance(context.clone()).getOrCreatePackageManager()
}

fn print_plugin_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 plugin list");
    output.push_stdout_line("operit2 plugin show <name>");
    output.push_stdout_line("operit2 plugin import <toolpkg-path>");
    output.push_stdout_line("operit2 plugin enable <name>");
    output.push_stdout_line("operit2 plugin disable <name>");
}
