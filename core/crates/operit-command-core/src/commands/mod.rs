mod approval;
mod chat;
mod host;
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
mod tag;
mod tool;
mod update;
mod util;
mod workspace;

use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;

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
        "tool" => {
            tool::run_tool_command(application.applicationContext.clone(), &args[1..], output)
        }
        "package" => {
            package::run_package_command(application.applicationContext.clone(), &args[1..], output)
        }
        "plugin" => {
            plugin::run_plugin_command(application.applicationContext.clone(), &args[1..], output)
        }
        "skill" => {
            skill::run_skill_command(application.applicationContext.clone(), &args[1..], output)
        }
        "mcp" => mcp::run_mcp_command(application.applicationContext.clone(), &args[1..], output),
        "market" => {
            market::run_market_command(application.applicationContext.clone(), &args[1..], output)
        }
        "host" => {
            host::run_host_command(application.applicationContext.clone(), &args[1..], output)
        }
        "log" => log::run_log_command(&args[1..], output),
        "prefs" => {
            prefs::run_prefs_command(application.applicationContext.clone(), &args[1..], output)
        }
        "approval" => approval::run_approval_command(
            application.applicationContext.clone(),
            &args[1..],
            output,
        ),
        "tag" => tag::run_tag_command(application.applicationContext.clone(), &args[1..], output),
        "memory" => {
            memory::run_memory_command(application.applicationContext.clone(), &args[1..], output)
        }
        "character" => people::run_character_command(application, &args[1..], output),
        "group" => people::run_group_command(application, &args[1..], output),
        "active-prompt" => people::run_active_prompt_command(application, &args[1..], output),
        "model" => {
            model::run_model_command(application.applicationContext.clone(), &args[1..], output)
        }
        "chat" => chat::run_chat_command(application, &args[1..], output),
        "workspace" => workspace::run_workspace_command(application, &args[1..], output),
        "update" => update::run_update_command(&args[1..], output),
        _ => {
            print_core_usage(output);
            Ok(())
        }
    }
}

fn print_core_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 <tool|package|plugin|skill|mcp|market|host|log|prefs|approval|tag|memory|character|group|active-prompt|model|chat|workspace|update>");
    output.push_stdout_line("operit2 tool <list|show|exec>");
    output.push_stdout_line(
        "operit2 package <help|dir|list|more|load|show|import|enable|disable|use|exec>",
    );
    output.push_stdout_line("operit2 plugin <help|list|more|load|show|import|enable|disable>");
    output
        .push_stdout_line("operit2 skill <dir|list|more|load|show|create|import-zip|delete|visible|errors>");
    output
        .push_stdout_line("operit2 mcp <dir|list|show|import|export|remove|enable|disable|start|tools|config|config-set|local-set|install-github|install-zip|meta|meta-set|describe>");
    output.push_stdout_line(
        "operit2 market <auth|stats|rank|search|show|install|comments|comment|reactions|react>",
    );
    output.push_stdout_line("operit2 host <show|capabilities|paths>");
    output.push_stdout_line("operit2 log <show|package|path|clear>");
    output.push_stdout_line(
        "operit2 prefs <show|thinking|thinking-quality|stream|media-history|mcp-timeout>",
    );
    output.push_stdout_line("operit2 approval <status|list|allow|ask|forbid|tool>");
    output.push_stdout_line("operit2 tag <list|show|create|update|delete>");
    output.push_stdout_line("operit2 memory <character|shared|mount|unmount>");
    output.push_stdout_line(
        "operit2 character <init|list|show|create|update|delete|set-active|combine|reset-default>",
    );
    output.push_stdout_line(
        "operit2 group <init|list|show|create|update|delete|set-active|duplicate>",
    );
    output.push_stdout_line(
        "operit2 active-prompt <show|set-card|set-group|activate-for-chat|resolved-card>",
    );
    output.push_stdout_line("operit2 model <init|list|show|set|set-key|api-settings-full|custom-headers|request-queue|api-key-pool|custom-parameters|parameters|tool-call|direct-image|direct-audio|direct-video|google-search|params|context-show|context-set|summary-show|summary-set|function-list|function-show|function-set|function-reset>");
    output.push_stdout_line("operit2 chat <new|list|show|current|switch|delete|delete-message|clear|rollback|branch|branches|lock|pin|stats|bind-character|bind-group|set-group|send>");
    output.push_stdout_line("operit2 workspace <default-path|create-default|bind-default|bind|unbind|list|chats|commands|commands-path|run|run-path>");
    output.push_stdout_line("operit2 update <run|check|target>");
}
