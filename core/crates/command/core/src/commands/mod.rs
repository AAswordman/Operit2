mod approval;
mod chat;
mod host;
mod local_models;
mod log;
mod market;
mod mcp;
mod memory;
mod model;
mod package;
mod people;
mod plugin;
mod prefs;
mod skill;
mod storage;
mod stt;
mod tag;
mod tool;
mod update;
mod usage;
mod util;
mod workspace;

use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;

/// Dispatches a top-level core command family into its command module.
pub fn run_core_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_core_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "tool" => tool::run_tool_command(application, &args[1..], output),
        "package" => package::run_package_command(application, &args[1..], output),
        "plugin" => plugin::run_plugin_command(application, &args[1..], output),
        "skill" => skill::run_skill_command(application, &args[1..], output),
        "mcp" => mcp::run_mcp_command(application, &args[1..], output),
        "market" => market::run_market_command(application, &args[1..], output),
        "host" => host::run_host_command(application.hostManager.clone(), &args[1..], output),
        "log" => log::run_log_command(&args[1..], output),
        "local-models" => local_models::run_local_models_command(application, &args[1..], output),
        "prefs" => prefs::run_prefs_command(application.hostManager.clone(), &args[1..], output),
        "approval" => {
            approval::run_approval_command(application.hostManager.clone(), &args[1..], output)
        }
        "tag" => tag::run_tag_command(application.hostManager.clone(), &args[1..], output),
        "memory" => memory::run_memory_command(application.hostManager.clone(), &args[1..], output),
        "character" => people::run_character_command(application, &args[1..], output),
        "group" => people::run_group_command(application, &args[1..], output),
        "active-prompt" => people::run_active_prompt_command(application, &args[1..], output),
        "model" => model::run_model_command(application.hostManager.clone(), &args[1..], output),
        "chat" => chat::run_chat_command(application, &args[1..], output),
        "workspace" => workspace::run_workspace_command(application, &args[1..], output),
        "storage" => storage::run_storage_command(application, &args[1..], output),
        "stt" => stt::run_stt_command(application, &args[1..], output),
        "update" => update::run_update_command(&args[1..], output),
        "usage" => usage::run_usage_command(application, &args[1..], output),
        _ => {
            print_core_usage(output);
            Ok(())
        }
    }
}

/// Prints top-level command usage.
fn print_core_usage(output: &mut CoreCommandOutput) {
    let lines = vec![
        "Global option: --json  Emit machine-readable JSON.",
        "operit2 <tool|package|plugin|skill|mcp|market|host|log|local-models|stt|prefs|approval|tag|memory|character|group|active-prompt|model|chat|workspace|storage|update|usage>",
        "operit2 tool <list|show|exec>",
        "operit2 package <help|dir|list|more|load|show|import|enable|disable|use|exec>",
        "operit2 plugin <help|list|more|load|show|import|enable|disable>",
        "operit2 skill <dir|list|more|load|show|create|import-zip|delete|visible|errors>",
        "operit2 mcp <dir|list|show|import|export|remove|enable|disable|start|kill|tools|config|config-set|local-set|install-github|install-zip|meta|meta-set|describe>",
        "operit2 market <rank|list|search|show|comments|comment|like|notifications|my|publish|install|download>",
        "operit2 host <show|capabilities|paths>",
        "operit2 log <show|package|path|clear>",
        "operit2 local-models <paths|catalog|show|installed|installed-show|install|verify|delete|engine-delete>",
        "operit2 stt <provider-list|provider-model-list|config|transcribe|transcribe-config>",
        "operit2 prefs <show|thinking|thinking-quality|stream|media-history|mcp-timeout>",
        "operit2 approval <status|read-only|workspace-write|full>",
        "operit2 tag <list|show|create|update|delete>",
        "operit2 memory <character|shared|mount|unmount>",
        "operit2 character <init|list|show|create|update|delete|set-active|combine|reset-default>",
        "operit2 group <init|list|show|create|update|delete|set-active|duplicate>",
        "operit2 active-prompt <show|set-card|set-group|activate-for-chat|resolved-card>",
        "operit2 model <provider-type-list|provider-list|provider-show|provider-create|provider-set-key|provider-set-endpoint|provider-model-available-list|provider-model-add|provider-model-create|list|show|use|params|parameters|context-show|context-set|summary-show|summary-set|function-list|function-show|function-set|function-reset>",
        "operit2 chat <new|list|show|current|switch|delete|delete-message|clear|rollback|branch|branches|lock|pin|stats|bind-character|bind-group|set-group|send>",
        "operit2 workspace <default-path|create-default|bind-default|bind|unbind|list|chats|commands|commands-path|run|run-path>",
        "operit2 storage <paths|migrate>",
        "operit2 update <run|check|target>",
        "operit2 usage <summary|records|models|clear>",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}
