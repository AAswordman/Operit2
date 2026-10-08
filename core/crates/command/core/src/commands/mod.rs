mod approval;
mod catalog;
mod chat;
mod delegated;
mod extension;
mod host;
mod local_models;
mod log;
mod market;
mod mcp;
mod model;
mod package;
mod plugin;
mod prefs;
mod skill;
mod storage;
mod stt;
mod tool;
mod update;
mod usage;
mod util;
mod workspace;

use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;

use catalog::{BuiltinRoot, RootCommandOwner, BUILTIN_ROOTS};
use delegated::ReadyCommandRegistry;

/// Resolves one root owner from a ready directory before executing its selected handler once.
pub async fn run_core_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let registry = ReadyCommandRegistry::load(&application.toolHandler).await?;
    if args.is_empty() {
        return print_core_usage(&registry, output);
    }

    match registry.resolve(&args[0])? {
        RootCommandOwner::Plugin(selected) => registry.execute(&selected, &args[1..], output).await,
        RootCommandOwner::Core(root) => {
            run_core_owned_command(application, &registry, root, &args[1..], output).await
        }
    }
}

/// Dispatches only inherent Core handlers and their original extension scope aliases.
async fn run_core_owned_command(
    application: &mut OperitApplication,
    registry: &ReadyCommandRegistry,
    root: BuiltinRoot,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.first().map(String::as_str) == Some("scope") {
        if let Some(kind) = root.scope_alias_kind() {
            return extension::run_scope_command(application, kind, &args[1..], output);
        }
    }

    match root {
        BuiltinRoot::Extension => extension::run_extension_command(application, args, output),
        BuiltinRoot::Tool => tool::run_tool_command(application, args, output).await,
        BuiltinRoot::Package => package::run_package_command(application, args, output).await,
        BuiltinRoot::Plugin => {
            plugin::run_plugin_command(application, registry, args, output).await
        }
        BuiltinRoot::Skill => skill::run_skill_command(application, args, output),
        BuiltinRoot::Mcp => mcp::run_mcp_command(application, args, output).await,
        BuiltinRoot::Market => market::run_market_command(application, args, output),
        BuiltinRoot::Host => host::run_host_command(application.hostManager.clone(), args, output),
        BuiltinRoot::Log => log::run_log_command(args, output),
        BuiltinRoot::LocalModels => {
            local_models::run_local_models_command(application, args, output)
        }
        BuiltinRoot::Prefs => {
            prefs::run_prefs_command(application.hostManager.clone(), args, output)
        }
        BuiltinRoot::Approval => {
            approval::run_approval_command(application.hostManager.clone(), args, output)
        }
        BuiltinRoot::Model => {
            model::run_model_command(application.hostManager.clone(), args, output)
        }
        BuiltinRoot::Chat => chat::run_chat_command(application, args, output).await,
        BuiltinRoot::Workspace => workspace::run_workspace_command(application, args, output).await,
        BuiltinRoot::Storage => storage::run_storage_command(application, args, output),
        BuiltinRoot::Stt => stt::run_stt_command(application, args, output),
        BuiltinRoot::Update => update::run_update_command(args, output),
        BuiltinRoot::Usage => usage::run_usage_command(application, args, output),
    }
}

/// Discovers registered plugin roots and their metadata from the invocation's exact ready catalog.
fn print_core_usage(
    registry: &ReadyCommandRegistry,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let commands = registry.plugin_commands()?;
    let builtin_names = BUILTIN_ROOTS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    let root_names = builtin_names
        .iter()
        .copied()
        .chain(commands.iter().map(|command| command.name.as_str()))
        .collect::<Vec<_>>();
    let mut lines = vec![
        "Global option: --json  Emit machine-readable JSON.".to_string(),
        format!("operit2 <{}>", root_names.join("|")),
        "Plugin root commands are discovered from enabled ToolPkg registrations.".to_string(),
    ];
    lines.extend([
        "operit2 extension <list [kind] [--scope scope]|show <kind> <id>|move <kind> <id> <device|space> --yes>",
        "operit2 <plugin|package|skill|mcp> scope <id> [device|space --yes]",
        "operit2 tool <list|show|exec>",
        "operit2 package <help|dir|list|more|load|show|import|enable|disable|use|exec>",
        "operit2 plugin <help|list|commands|exec|more|load|show|import|enable|disable>",
        "operit2 skill <dir|list|more|load|show|create|import-zip|delete|visible|errors>",
        "operit2 mcp <dir|list|show|import|export|remove|enable|disable|start|kill|tools|config|config-set|local-set|install-github|install-zip|meta|meta-set|describe>",
        "operit2 market <rank|list|search|show|comments|comment|like|notifications|my|publish|install|download>",
        "operit2 host <show>",
        "operit2 log <show|package|path|clear>",
        "operit2 local-models <paths|catalog|show|installed|installed-show|install|verify|delete|engine-delete>",
        "operit2 stt <provider-list|provider-model-list|config|transcribe|transcribe-config>",
        "operit2 prefs <show|thinking|thinking-quality|stream|media-history|mcp-timeout>",
        "operit2 approval <status|read-only|workspace-write|full>",
        "operit2 plugin commands  Show registered plugin command usage and descriptions.",
        "operit2 plugin exec <registered-command> [args...]  Explicitly invoke a registered plugin command.",
        "operit2 model <codex-login|provider-type-list|provider-list|provider-show|provider-create|provider-set-key|provider-set-endpoint|provider-model-available-list|provider-model-add|provider-model-create|list|show|use|params|parameters|context-show|context-set|summary-show|summary-set|function-list|function-show|function-set|function-reset>",
        "operit2 chat <new|list|show|current|switch|delete|delete-message|clear|rollback|branch|branches|lock|pin|stats|send>",
        "operit2 workspace <default-path|create-default|bind-default|bind|unbind|list|chats|commands|commands-path|run|run-path>",
        "operit2 storage <paths|migrate>",
        "operit2 update <run|check|target>",
        "operit2 usage <summary|records|models|clear>",
    ].into_iter().map(str::to_string));
    for command in &commands {
        lines.push(format!(
            "{}  {} ({})",
            command.usage, command.description, command.containerPackageName
        ));
    }
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({
        "usage": lines,
        "builtinCommands": builtin_names,
        "registeredCommands": commands,
    }));
    Ok(())
}
