use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::enhance::ToolExecutionManager::{AITool, ToolParameter};
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::tools::ToolPermissionSystem::{PermissionLevel, PermissionRequestResult};
use operit_runtime::data::api::MarketStatsApiService::{
    mcpMetadataFromEntry, normalizeMarketArtifactId, parseArtifactMarketMetadata,
    parseMcpMarketMetadata, parseSkillMarketMetadata, resolveMarketEntryId,
    skillRepositoryUrlFromEntry, ArtifactProjectDetailResponse, ArtifactProjectNodeResponse,
    GitHubIssue, MarketRankIssueEntryResponse,
};
use operit_runtime::data::model::ActivePrompt::ActivePrompt;
use operit_runtime::data::model::ApiKeyInfo::ApiKeyInfo;
use operit_runtime::data::model::AttachmentInfo::AttachmentInfo;
use operit_runtime::data::model::CharacterCard::{
    CharacterCard, CharacterCardChatModelBindingMode, CharacterCardToolAccessConfig,
};
use operit_runtime::data::model::CharacterGroupCard::{CharacterGroupCard, GroupMemberConfig};
use operit_runtime::data::model::ChatMessage::ChatMessage;
use operit_runtime::data::model::ChatTurnOptions::ChatTurnOptions;
use operit_runtime::data::model::FunctionType::FunctionType;
use operit_runtime::data::model::InputProcessingState::InputProcessingState;
use operit_runtime::data::model::ModelConfigData::ApiProviderType;
use operit_runtime::data::model::ModelParameter::ModelParameter;
use operit_runtime::data::model::PromptFunctionType::PromptFunctionType;
use operit_runtime::data::model::PromptTag::TagType;
use operit_runtime::data::preferences::ActivePromptManager::ActivePromptManager;
use operit_runtime::data::preferences::ApiPreferences::ApiPreferences;
use operit_runtime::data::preferences::CharacterCardManager::CharacterCardManager;
use operit_runtime::data::preferences::CharacterGroupCardManager::CharacterGroupCardManager;
use operit_runtime::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_runtime::data::preferences::PromptTagManager::PromptTagManager;
use operit_runtime::data::repository::ChatHistoryManager::ChatHistoryManager;
use operit_runtime::services::core::MessageCoordinationDelegate::MessageCoordinationDelegate;
use operit_runtime::util::stream::Stream::Stream;
use operit_runtime::util::GithubReleaseUtil::FullUpdateTarget;
use sha2::{Digest, Sha256};

pub(crate) mod link;
mod transfer;
mod web_access;

use crate::chat_runtime::{run_chat_shell_command_with_core, run_shell_command};
use crate::core_proxy::{cli_core, local_cli_core};
use link::{load_link_session, run_link_command};
use transfer::{run_backup_command, run_export_command, run_import_command};
use web_access::run_web_access_command;

pub(crate) async fn run_cli_root(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_cli_usage();
        return Ok(());
    }

    if args[0].as_str() == "--link" {
        let session_name = args
            .get(1)
            .ok_or_else(|| "usage: operit2 cli --link <session> <command>".to_string())?;
        return run_cli_link_root(session_name, &args[2..]).await;
    }

    if args[0].as_str() == "link" {
        return run_link_command(&args[1..]).await;
    }

    if args[0].as_str() == "web" {
        return run_web_access_command(&args[1..]).await;
    }

    let mut core = local_cli_core()?;

    let result = match args[0].as_str() {
        "model" => run_core_command_and_print(&mut core, &args).await,
        "version" => run_version_core_command(&mut core).await,
        "prefs" => run_core_command_and_print(&mut core, &args).await,
        "host" => run_core_command_and_print(&mut core, &args).await,
        "log" => run_core_command_and_print(&mut core, &args).await,
        "memory" => run_core_command_and_print(&mut core, &args).await,
        "export" => run_export_command(&mut core, &args[1..]).await,
        "import" => run_import_command(&mut core, &args[1..]).await,
        "backup" => run_backup_command(&mut core, &args[1..]).await,
        "chat" if args.get(1).map(String::as_str) == Some("shell") => {
            run_chat_shell_command_with_core(&mut core, &args[2..]).await
        }
        "chat" => run_core_command_and_print(&mut core, &args).await,
        "workspace" => run_core_command_and_print(&mut core, &args).await,
        "shell" => run_shell_command(&args[1..]).await,
        "tag" => run_core_command_and_print(&mut core, &args).await,
        "character" => run_core_command_and_print(&mut core, &args).await,
        "group" => run_core_command_and_print(&mut core, &args).await,
        "active-prompt" => run_core_command_and_print(&mut core, &args).await,
        "approval" => run_core_command_and_print(&mut core, &args).await,
        "tool" => run_core_command_and_print(&mut core, &args).await,
        "market" => run_core_command_and_print(&mut core, &args).await,
        "update" => run_update_cli_command(&mut core, &args[1..]).await,
        "skill" => run_core_command_and_print(&mut core, &args).await,
        "package" => run_core_command_and_print(&mut core, &args).await,
        "plugin" => run_core_command_and_print(&mut core, &args).await,
        "mcp" => run_core_command_and_print(&mut core, &args).await,
        _ => {
            print_cli_usage();
            Ok(())
        }
    };
    result.map_err(rewrite_cli_usage_message)
}

pub(crate) async fn run_cli_link_root(session_name: &str, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_cli_link_usage();
        return Ok(());
    }
    let session = load_link_session(session_name)?;
    let mut core = cli_core(session);
    let result = match args[0].as_str() {
        "model" => run_core_command_and_print(&mut core, &args).await,
        "version" => run_version_core_command(&mut core).await,
        "prefs" => run_core_command_and_print(&mut core, &args).await,
        "host" => run_core_command_and_print(&mut core, &args).await,
        "log" => run_core_command_and_print(&mut core, &args).await,
        "memory" => run_core_command_and_print(&mut core, &args).await,
        "export" => run_export_command(&mut core, &args[1..]).await,
        "import" => run_import_command(&mut core, &args[1..]).await,
        "backup" => run_backup_command(&mut core, &args[1..]).await,
        "chat" if args.get(1).map(String::as_str) == Some("shell") => {
            run_chat_shell_command_with_core(&mut core, &args[2..]).await
        }
        "chat" => run_core_command_and_print(&mut core, &args).await,
        "workspace" => run_core_command_and_print(&mut core, &args).await,
        "shell" => {
            let mut shell_args = vec!["shell".to_string()];
            shell_args.extend_from_slice(&args[1..]);
            run_chat_shell_command_with_core(&mut core, &shell_args[1..]).await
        }
        "tag" => run_core_command_and_print(&mut core, &args).await,
        "character" => run_core_command_and_print(&mut core, &args).await,
        "group" => run_core_command_and_print(&mut core, &args).await,
        "active-prompt" => run_core_command_and_print(&mut core, &args).await,
        "approval" => run_core_command_and_print(&mut core, &args).await,
        "tool" => run_core_command_and_print(&mut core, &args).await,
        "market" => run_core_command_and_print(&mut core, &args).await,
        "update" => run_core_command_and_print(&mut core, &args).await,
        "skill" => run_core_command_and_print(&mut core, &args).await,
        "package" => run_core_command_and_print(&mut core, &args).await,
        "plugin" => run_core_command_and_print(&mut core, &args).await,
        "mcp" => run_core_command_and_print(&mut core, &args).await,
        _ => {
            print_cli_link_usage();
            Ok(())
        }
    };
    result.map_err(rewrite_cli_usage_message)
}

async fn run_update_cli_command(
    core: &mut crate::core_proxy::CliCore,
    args: &[String],
) -> Result<(), String> {
    if args.is_empty() {
        let target = FullUpdateTarget::cliForCurrentHost()?;
        let command = vec![
            "update".to_string(),
            "run".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            target.product,
            target.platform,
            target.arch,
        ];
        return run_core_command_and_print(core, &command).await;
    }

    match args[0].as_str() {
        "check" if args.len() == 1 => {
            let target = FullUpdateTarget::cliForCurrentHost()?;
            let command = vec![
                "update".to_string(),
                "check".to_string(),
                env!("CARGO_PKG_VERSION").to_string(),
                target.product,
                target.platform,
                target.arch,
            ];
            run_core_command_and_print(core, &command).await
        }
        "target" if args.len() == 1 => {
            let target = FullUpdateTarget::cliForCurrentHost()?;
            let package_name = target.assetName()?;
            println!("product={}", target.product);
            println!("platform={}", target.platform);
            println!("arch={}", target.arch);
            println!("package={package_name}");
            Ok(())
        }
        "run" if args.len() == 5 => {
            let mut command = vec!["update".to_string()];
            command.extend_from_slice(args);
            run_core_command_and_print(core, &command).await
        }
        "check" if args.len() == 5 => {
            let mut command = vec!["update".to_string()];
            command.extend_from_slice(args);
            run_core_command_and_print(core, &command).await
        }
        "target" if args.len() == 4 => {
            let mut command = vec!["update".to_string()];
            command.extend_from_slice(args);
            run_core_command_and_print(core, &command).await
        }
        _ => {
            print_update_usage();
            Ok(())
        }
    }
}

fn rewrite_cli_usage_message(message: String) -> String {
    if message.contains("operit2 cli ") {
        return message;
    }
    message.replace("operit2 ", "operit2 cli ")
}

async fn run_core_command_and_print(
    core: &mut crate::core_proxy::CliCore,
    args: &[String],
) -> Result<(), String> {
    let output = core
        .runCoreCommand(args)
        .await
        .map_err(|error| rewrite_core_command_usage_message(error.message))?;
    if !output.stdout.is_empty() {
        print!("{}", rewrite_core_command_usage_message(output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", rewrite_core_command_usage_message(output.stderr));
    }
    Ok(())
}

fn rewrite_core_command_usage_message(message: String) -> String {
    rewrite_cli_usage_message(message)
}

async fn run_version_core_command(core: &mut crate::core_proxy::CliCore) -> Result<(), String> {
    println!("cliVersion={}", env!("CARGO_PKG_VERSION"));
    println!(
        "coreVersion={}",
        core.application()
            .coreVersion()
            .await
            .map_err(|error| error.to_string())?
    );
    println!("linkVersion={}", operit_link::LINK_VERSION);
    println!("targetOs={}", std::env::consts::OS);
    println!("targetArch={}", std::env::consts::ARCH);
    Ok(())
}

pub(crate) fn print_root_usage() {
    println!("operit2");
    println!("operit2 [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 tui [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 cli <version|prefs|host|log|memory|export|import|backup|model|chat|workspace|tag|character|group|active-prompt|approval|tool|market|update|skill|package|plugin|mcp|link|web|shell>");
    println!("operit2 cli --link <session> <version|prefs|host|log|memory|export|import|backup|model|chat|workspace|tag|character|group|active-prompt|approval|tool|market|update|skill|package|plugin|mcp|shell>");
    println!();
    print_cli_usage();
}

fn print_cli_usage() {
    println!("operit2 cli --link <session> <version|chat|workspace>");
    println!("operit2 cli version");
    println!("operit2 cli prefs <show|thinking|thinking-quality|stream|media-history|mcp-timeout>");
    println!("operit2 cli host <show|capabilities|paths>");
    println!("operit2 cli log <show|package|path|clear>");
    println!("operit2 cli memory <character|shared|mount|unmount>");
    println!("operit2 cli export <memory|chat|snapshot>");
    println!("operit2 cli import <memory|chat|snapshot>");
    println!("operit2 cli backup <create|restore|inspect>");
    println!("operit2 cli model <init|list|show|set|set-key|api-settings-full|custom-headers|request-queue|api-key-pool|custom-parameters|parameters|tool-call|direct-image|direct-audio|direct-video|google-search|params|context-show|context-set|summary-show|summary-set|function-list|function-show|function-set|function-reset>");
    println!("operit2 cli tag <list|show|create|update|delete>");
    println!("operit2 cli character <init|list|show|create|update|delete|set-active|combine|reset-default>");
    println!("operit2 cli group <init|list|show|create|update|delete|set-active|duplicate>");
    println!("operit2 cli active-prompt <show|set-card|set-group|activate-for-chat|resolved-card>");
    println!("operit2 cli approval <status|list|allow|ask|forbid|tool>");
    println!("operit2 cli tool <list|show|exec>");
    println!(
        "operit2 cli market <auth|stats|rank|search|show|install|comments|comment|reactions|react>"
    );
    println!("operit2 cli update [check|target]");
    println!("operit2 cli skill <dir|list|more|load|show|create|import-zip|delete|visible|errors>");
    println!("operit2 cli package <help|dir|list|more|load|show|import|enable|disable|use|exec>");
    println!("operit2 cli plugin <help|list|more|load|show|import|enable|disable>");
    println!("operit2 cli mcp <dir|list|show|import|export|remove|enable|disable|start|tools|config|config-set|local-set|meta|meta-set|describe>");
    println!(
        "operit2 cli link <serve|discover|hello|connect|sessions|session-delete|accepted-sessions|accepted-session-delete|ping|sync|sync-status|call|watch|tui|run>"
    );
    println!("operit2 cli web <open|close|status|token>");
    println!("operit2 cli shell [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 cli chat <new|list|show|current|switch|delete|delete-message|clear|rollback|branch|branches|lock|pin|stats|bind-character|bind-group|set-group|shell|send>");
    println!("operit2 cli chat new [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 cli chat list");
    println!("operit2 cli chat show <chat-id> [--runtime]");
    println!("operit2 cli chat current");
    println!("operit2 cli chat switch <chat-id>");
    println!("operit2 cli chat delete <chat-id>");
    println!("operit2 cli chat delete-message <index>");
    println!("operit2 cli chat clear");
    println!("operit2 cli chat rollback <message-index>");
    println!("operit2 cli chat branch [--up-to <message-timestamp>]");
    println!("operit2 cli chat branches [parent-chat-id]");
    println!("operit2 cli chat lock <chat-id> <true|false>");
    println!("operit2 cli chat pin <chat-id> <true|false>");
    println!("operit2 cli chat stats");
    println!("operit2 cli chat bind-character <chat-id> <character-card-name>");
    println!("operit2 cli chat bind-group <chat-id> <character-group-id>");
    println!("operit2 cli chat set-group <chat-id> <group-name>");
    println!("operit2 cli chat shell [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 cli chat send [--chat <chat-id>] <message>");
    println!(
        "operit2 cli workspace <default-path|create-default|bind-default|bind|unbind|list|chats|commands|commands-path|run|run-path>"
    );
    println!("operit2 cli workspace default-path <chat-id>");
    println!("operit2 cli workspace create-default <chat-id> [project-type]");
    println!("operit2 cli workspace bind-default <chat-id> [project-type]");
    println!("operit2 cli workspace bind <chat-id> <workspace>");
    println!("operit2 cli workspace unbind <chat-id>");
    println!("operit2 cli workspace list");
    println!("operit2 cli workspace chats <workspace>");
    println!("operit2 cli workspace commands <chat-id>");
    println!("operit2 cli workspace commands-path <workspace>");
    println!("operit2 cli workspace run <chat-id> <command-id>");
    println!("operit2 cli workspace run-path <workspace> <command-id>");
}

fn print_cli_link_usage() {
    println!("operit2 cli --link <session> <version|prefs|host|log|memory|export|import|backup|model|chat|workspace|tag|character|group|active-prompt|approval|tool|market|update|skill|package|plugin|mcp|shell>");
    println!("operit2 cli link run <session> <version|chat>");
}

fn print_model_usage() {
    println!("operit2 cli model init");
    println!("operit2 cli model provider-type-list");
    println!("operit2 cli model provider-list");
    println!("operit2 cli model provider-show <provider-id>");
    println!("operit2 cli model provider-create <name> <provider-type-id> <endpoint>");
    println!("operit2 cli model provider-set-key <provider-id> <api-key>");
    println!("operit2 cli model provider-set-endpoint <provider-id> <endpoint>");
    println!("operit2 cli model provider-model-available-list <provider-id>");
    println!("operit2 cli model provider-model-add <provider-id> <provider-model-id>");
    println!("operit2 cli model provider-model-create <provider-id> <provider-model-id>");
    println!("operit2 cli model list");
    println!("operit2 cli model show [model-id]");
    println!("operit2 cli model use <model-id>");
    println!("operit2 cli model params [model-id]");
    println!("operit2 cli model parameters <model-id> <parameters-json>");
    println!("operit2 cli model context-show [model-id]");
    println!("operit2 cli model context-set <model-id> <context-length> <max-context-length> <enable-max-context-mode>");
    println!("operit2 cli model summary-show [model-id]");
    println!("operit2 cli model summary-set <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>");
    println!("operit2 cli model function-list");
    println!("operit2 cli model function-show <function-type>");
    println!("operit2 cli model function-set <function-type> <model-id>");
    println!("operit2 cli model function-reset [function-type]");
}

fn print_prefs_usage() {
    println!("operit2 cli prefs show");
    println!("operit2 cli prefs thinking <on|off>");
    println!("operit2 cli prefs thinking-quality <1-4>");
    println!("operit2 cli prefs stream <on|off>");
    println!("operit2 cli prefs media-history <image-user-turns> <media-user-turns>");
    println!("operit2 cli prefs mcp-timeout <seconds>");
}

fn print_host_usage() {
    println!("operit2 cli host show");
    println!("operit2 cli host capabilities");
    println!("operit2 cli host paths");
}

fn print_memory_usage() {
    println!("operit2 cli memory character <character-id> user <show|write|path>");
    println!(
        "operit2 cli memory character <character-id> item <list|search|show|create|delete|move>"
    );
    println!("operit2 cli memory character <character-id> graph");
    println!("operit2 cli memory shared <list|create|rename|delete>");
    println!("operit2 cli memory shared <shared-id> user <show|write|path>");
    println!("operit2 cli memory shared <shared-id> item <list|search|show|create|delete|move>");
    println!("operit2 cli memory shared <shared-id> graph");
    println!("operit2 cli memory mount <character-id> <shared-id> --read <true|false> --write <true|false>");
    println!("operit2 cli memory unmount <character-id> <shared-id>");
}

fn print_chat_usage() {
    println!("operit2 cli chat new [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 cli chat list");
    println!("operit2 cli chat show <chat-id> [--runtime]");
    println!("operit2 cli chat current");
    println!("operit2 cli chat switch <chat-id>");
    println!("operit2 cli chat delete <chat-id>");
    println!("operit2 cli chat delete-message <index>");
    println!("operit2 cli chat clear");
    println!("operit2 cli chat rollback <message-index>");
    println!("operit2 cli chat branch [--up-to <message-timestamp>]");
    println!("operit2 cli chat branches [parent-chat-id]");
    println!("operit2 cli chat lock <chat-id> <true|false>");
    println!("operit2 cli chat pin <chat-id> <true|false>");
    println!("operit2 cli chat stats");
    println!("operit2 cli chat bind-character <chat-id> <character-card-name>");
    println!("operit2 cli chat bind-group <chat-id> <character-group-id>");
    println!("operit2 cli chat set-group <chat-id> <group-name>");
    println!("operit2 cli chat shell [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>]");
    println!("operit2 cli chat send [--chat <chat-id>] <message>");
}

fn print_tag_usage() {
    println!("operit2 cli tag list");
    println!("operit2 cli tag show <id>");
    println!("operit2 cli tag create <name> [prompt-content] [description] [tag-type]");
    println!("operit2 cli tag update <id> <field> <value>");
    println!("operit2 cli tag delete <id>");
}

fn print_character_usage() {
    println!("operit2 cli character init");
    println!("operit2 cli character list");
    println!("operit2 cli character show <id>");
    println!("operit2 cli character create <name> [character-setting]");
    println!("operit2 cli character update <id> <field> <value>");
    println!("operit2 cli character delete <id>");
    println!("operit2 cli character set-active <id>");
    println!("operit2 cli character combine <id> [CHAT|VOICE] [tag-id-csv]");
    println!("operit2 cli character reset-default");
}

fn print_group_usage() {
    println!("operit2 cli group init");
    println!("operit2 cli group list");
    println!("operit2 cli group show <id>");
    println!("operit2 cli group create <name> [description]");
    println!("operit2 cli group update <id> <field> <value>");
    println!("operit2 cli group delete <id>");
    println!("operit2 cli group set-active <id>");
    println!("operit2 cli group duplicate <source-id> [new-name]");
}

fn print_active_prompt_usage() {
    println!("operit2 cli active-prompt show");
    println!("operit2 cli active-prompt set-card <id>");
    println!("operit2 cli active-prompt set-group <id>");
    println!(
        "operit2 cli active-prompt activate-for-chat [character-card-name] [character-group-id]"
    );
    println!("operit2 cli active-prompt resolved-card");
}

fn print_approval_usage() {
    println!("operit2 cli approval status");
    println!("operit2 cli approval list");
    println!("operit2 cli approval allow");
    println!("operit2 cli approval ask");
    println!("operit2 cli approval forbid");
    println!("operit2 cli approval tool <tool-name> <allow|ask|forbid|clear>");
}

fn print_tool_usage() {
    println!("operit2 cli tool list <public|internal|all>");
    println!("operit2 cli tool show <tool-name>");
    println!("operit2 cli tool exec <tool-name> <params-json>");
}

fn print_market_usage() {
    println!("operit2 cli market auth <status|token|logout|whoami>");
    println!("operit2 cli market stats <skill|mcp|package|script>");
    println!("operit2 cli market rank <skill|mcp|package|script> [updated|downloads|likes] [page]");
    println!("operit2 cli market search <skill|mcp|package|script> <query> [page]");
    println!("operit2 cli market show <skill|mcp|package|script> <id-or-number>");
    println!("operit2 cli market install <skill|mcp|package|script> <id-or-url> [node-id]");
    println!("operit2 cli market comments <skill|mcp|package|script> <number> [page]");
    println!("operit2 cli market comment <skill|mcp|package|script> <number> <body-or-@file>");
    println!("operit2 cli market reactions <skill|mcp|package|script> <number>");
    println!("operit2 cli market react <skill|mcp|package|script> <number> <+1|heart|rocket|...>");
}

fn print_update_usage() {
    println!("operit2 cli update");
    println!("operit2 cli update check");
    println!("operit2 cli update target");
    println!(
        "operit2 cli update run <current-version> <app|cli> <windows|linux|macos|android> <arch>"
    );
    println!(
        "operit2 cli update check <current-version> <app|cli> <windows|linux|macos|android> <arch>"
    );
    println!("operit2 cli update target <app|cli> <windows|linux|macos|android> <arch>");
}

fn print_skill_usage() {
    println!("operit2 cli skill dir");
    println!("operit2 cli skill list");
    println!("operit2 cli skill more");
    println!("operit2 cli skill load <name>");
    println!("operit2 cli skill show <name>");
    println!(
        "operit2 cli skill create <skill-id> <description> <content-or-@file> [attachment-path...]"
    );
    println!("operit2 cli skill import-zip <zip-path> [sub-dir-in-zip]");
    println!("operit2 cli skill delete <name>");
    println!("operit2 cli skill visible <name> [true|false]");
    println!("operit2 cli skill errors");
}

fn print_package_usage() {
    println!("operit2 cli package help");
    println!("operit2 cli package dir");
    println!("operit2 cli package list");
    println!("operit2 cli package more");
    println!("operit2 cli package load <name>");
    println!("operit2 cli package show <name>");
    println!("operit2 cli package import <js-ts-hjson-toolpkg-path>");
    println!("operit2 cli package enable <name>");
    println!("operit2 cli package disable <name>");
    println!("operit2 cli package use <name>");
    println!("operit2 cli package exec <package:tool> <params-json>");
}

fn print_plugin_usage() {
    println!("operit2 cli plugin help");
    println!("operit2 cli plugin list");
    println!("operit2 cli plugin more");
    println!("operit2 cli plugin load <name>");
    println!("operit2 cli plugin show <name>");
    println!("operit2 cli plugin import <toolpkg-path>");
    println!("operit2 cli plugin enable <name>");
    println!("operit2 cli plugin disable <name>");
}

fn print_mcp_usage() {
    println!("operit2 cli mcp dir");
    println!("operit2 cli mcp list");
    println!("operit2 cli mcp show <id>");
    println!("operit2 cli mcp import <json-or-@file>");
    println!("operit2 cli mcp export");
    println!("operit2 cli mcp remove <id>");
    println!("operit2 cli mcp enable <id>");
    println!("operit2 cli mcp disable <id>");
    println!("operit2 cli mcp start <id>");
    println!("operit2 cli mcp tools <id>");
    println!("operit2 cli mcp config <id>");
    println!("operit2 cli mcp config-set <id> <json-or-@file>");
    println!("operit2 cli mcp local-set <id> [--disabled true|false] [--env KEY=VALUE] [--approve TOOL] -- <command> [args...]");
    println!("operit2 cli mcp meta <id>");
    println!("operit2 cli mcp meta-set <id> <name> <description-or-@file> <author> <version>");
    println!("operit2 cli mcp describe <id>");
}

fn print_chat_history_header(chat: &operit_runtime::data::model::ChatHistory::ChatHistory) {
    println!("id={}", chat.id);
    println!("title={}", chat.title);
    println!("createdAt={}", chat.createdAt);
    println!("updatedAt={}", chat.updatedAt);
    println!("inputTokens={}", chat.inputTokens);
    println!("outputTokens={}", chat.outputTokens);
    println!("currentWindowSize={}", chat.currentWindowSize);
    println!("group={}", chat.group.clone().unwrap_or_default());
    println!("displayOrder={}", chat.displayOrder);
    println!("workspace={}", chat.workspace.clone().unwrap_or_default());
    println!(
        "parentChatId={}",
        chat.parentChatId.clone().unwrap_or_default()
    );
    println!(
        "characterCardName={}",
        chat.characterCardName.clone().unwrap_or_default()
    );
    println!(
        "characterGroupId={}",
        chat.characterGroupId.clone().unwrap_or_default()
    );
    println!("locked={}", chat.locked);
    println!("pinned={}", chat.pinned);
}

fn print_chat_message(message: &operit_runtime::data::model::ChatMessage::ChatMessage) {
    println!("--- message ---");
    println!("sender={}", message.sender);
    println!("timestamp={}", message.timestamp);
    println!("roleName={}", message.roleName);
    println!("selectedVariantIndex={}", message.selectedVariantIndex);
    println!("variantCount={}", message.variantCount);
    println!("provider={}", message.provider);
    println!("modelName={}", message.modelName);
    println!("inputTokens={}", message.inputTokens);
    println!("cachedInputTokens={}", message.cachedInputTokens);
    println!("outputTokens={}", message.outputTokens);
    println!("sentAt={}", message.sentAt);
    println!("waitDurationMs={}", message.waitDurationMs);
    println!("outputDurationMs={}", message.outputDurationMs);
    println!("completedAt={}", message.completedAt);
    println!("displayMode={:?}", message.displayMode);
    println!("isFavorite={}", message.isFavorite);
    println!("content={}", message.content);
}

fn print_tag(tag: &operit_runtime::data::model::PromptTag::PromptTag) {
    println!("id={}", tag.id);
    println!("name={}", tag.name);
    println!("description={}", tag.description);
    println!("promptContent={}", tag.promptContent);
    println!("tagType={}", tagTypeName(&tag.tagType));
    println!("createdAt={}", tag.createdAt);
    println!("updatedAt={}", tag.updatedAt);
}

fn print_character_card(card: &CharacterCard) {
    println!("id={}", card.id);
    println!("name={}", card.name);
    println!("description={}", card.description);
    println!("characterSetting={}", card.characterSetting);
    println!("openingStatement={}", card.openingStatement);
    println!("otherContentChat={}", card.otherContentChat);
    println!("otherContentVoice={}", card.otherContentVoice);
    println!("attachedTagIds={}", card.attachedTagIds.join(","));
    println!("advancedCustomPrompt={}", card.advancedCustomPrompt);
    println!("marks={}", card.marks);
    println!("chatModelBindingMode={}", card.chatModelBindingMode);
    println!(
        "chatModelId={}",
        card.chatModelId.clone().unwrap_or_default()
    );
    println!(
        "sharedMemoryMounts={}",
        serde_json::to_string(&card.sharedMemoryMounts).expect("sharedMemoryMounts must serialize")
    );
    println!(
        "toolAccessConfig={}",
        serde_json::to_string(&card.toolAccessConfig).expect("toolAccessConfig must serialize")
    );
    println!("isDefault={}", card.isDefault);
    println!("createdAt={}", card.createdAt);
    println!("updatedAt={}", card.updatedAt);
}

fn print_character_group_card(group: &CharacterGroupCard) {
    println!("id={}", group.id);
    println!("name={}", group.name);
    println!("description={}", group.description);
    println!(
        "members={}",
        group
            .members
            .iter()
            .map(|member| format!("{}:{}", member.characterCardId, member.orderIndex))
            .collect::<Vec<_>>()
            .join(",")
    );
    println!("createdAt={}", group.createdAt);
    println!("updatedAt={}", group.updatedAt);
}

fn parse_group_members(value: &str) -> Vec<GroupMemberConfig> {
    let mut result = Vec::new();
    for (index, item) in value.split(',').enumerate() {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        result.push(GroupMemberConfig {
            characterCardId: trimmed.to_string(),
            orderIndex: index as i32,
        });
    }
    result
}

fn parseTagType(value: Option<&str>) -> Result<TagType, String> {
    match value.unwrap_or("CUSTOM") {
        "TONE" => Ok(TagType::TONE),
        "CHARACTER" => Ok(TagType::CHARACTER),
        "FUNCTION" => Ok(TagType::FUNCTION),
        "CUSTOM" => Ok(TagType::CUSTOM),
        other => Err(format!(
            "invalid tagType: {other}; expected TONE | CHARACTER | FUNCTION | CUSTOM"
        )),
    }
}

fn parse_permission_level_arg(value: Option<&str>) -> Result<PermissionLevel, String> {
    match value {
        Some("allow") | Some("ALLOW") => Ok(PermissionLevel::ALLOW),
        Some("ask") | Some("ASK") => Ok(PermissionLevel::ASK),
        Some("forbid") | Some("FORBID") => Ok(PermissionLevel::FORBID),
        _ => Err("expected allow, ask, or forbid".to_string()),
    }
}

fn tagTypeName(tagType: &TagType) -> &'static str {
    match tagType {
        TagType::TONE => "TONE",
        TagType::CHARACTER => "CHARACTER",
        TagType::FUNCTION => "FUNCTION",
        TagType::CUSTOM => "CUSTOM",
    }
}

fn parsePromptFunctionType(value: Option<&str>) -> Result<PromptFunctionType, String> {
    match value.unwrap_or("CHAT") {
        "CHAT" => Ok(PromptFunctionType::CHAT),
        "VOICE" => Ok(PromptFunctionType::VOICE),
        other => Err(format!(
            "invalid promptFunctionType: {other}; expected CHAT | VOICE"
        )),
    }
}

fn parseFunctionType(value: &str) -> Result<FunctionType, String> {
    match value {
        "CHAT" => Ok(FunctionType::CHAT),
        "SUMMARY" => Ok(FunctionType::SUMMARY),
        "MEMORY" => Ok(FunctionType::MEMORY),
        "UI_CONTROLLER" => Ok(FunctionType::UI_CONTROLLER),
        "TRANSLATION" => Ok(FunctionType::TRANSLATION),
        "GREP" => Ok(FunctionType::GREP),
        "ROLE_RESPONSE_PLANNER" => Ok(FunctionType::ROLE_RESPONSE_PLANNER),
        "IMAGE_RECOGNITION" => Ok(FunctionType::IMAGE_RECOGNITION),
        "AUDIO_RECOGNITION" => Ok(FunctionType::AUDIO_RECOGNITION),
        "VIDEO_RECOGNITION" => Ok(FunctionType::VIDEO_RECOGNITION),
        other => Err(format!("invalid FunctionType: {other}")),
    }
}

fn parse_f32_arg(value: Option<&String>, usage: &str) -> Result<f32, String> {
    value
        .ok_or_else(|| usage.to_string())?
        .parse::<f32>()
        .map_err(|error| error.to_string())
}

fn parse_i32_arg(value: Option<&String>, usage: &str) -> Result<i32, String> {
    value
        .ok_or_else(|| usage.to_string())?
        .parse::<i32>()
        .map_err(|error| error.to_string())
}

fn parse_optional_i32_arg(value: Option<&String>, defaultValue: i32) -> Result<i32, String> {
    match value {
        Some(value) => value.parse::<i32>().map_err(|error| error.to_string()),
        None => Ok(defaultValue),
    }
}

fn parse_i64_arg(value: Option<&String>, usage: &str) -> Result<i64, String> {
    value
        .ok_or_else(|| usage.to_string())?
        .parse::<i64>()
        .map_err(|error| error.to_string())
}

fn parse_bool_arg(value: Option<&String>, usage: &str) -> Result<bool, String> {
    match value.ok_or_else(|| usage.to_string())?.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("invalid bool: {other}; expected true | false")),
    }
}

fn parse_on_off_arg(value: Option<&String>, usage: &str) -> Result<bool, String> {
    match value.ok_or_else(|| usage.to_string())?.as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        other => Err(format!("invalid switch: {other}; expected on | off")),
    }
}

fn parseApiProviderType(value: &str) -> Result<ApiProviderType, String> {
    ApiProviderType::fromProviderTypeId(value)
        .ok_or_else(|| format!("invalid ApiProviderType: {value}"))
}

fn functionTypeName(functionType: &FunctionType) -> &'static str {
    match functionType {
        FunctionType::CHAT => "CHAT",
        FunctionType::SUMMARY => "SUMMARY",
        FunctionType::MEMORY => "MEMORY",
        FunctionType::UI_CONTROLLER => "UI_CONTROLLER",
        FunctionType::TRANSLATION => "TRANSLATION",
        FunctionType::GREP => "GREP",
        FunctionType::ROLE_RESPONSE_PLANNER => "ROLE_RESPONSE_PLANNER",
        FunctionType::IMAGE_RECOGNITION => "IMAGE_RECOGNITION",
        FunctionType::AUDIO_RECOGNITION => "AUDIO_RECOGNITION",
        FunctionType::VIDEO_RECOGNITION => "VIDEO_RECOGNITION",
    }
}

fn parseCsvList(value: &str) -> Vec<String> {
    let mut result = Vec::new();
    for item in value.split(',') {
        let trimmed = item.trim();
        if !trimmed.is_empty() && !result.iter().any(|entry| entry == trimmed) {
            result.push(trimmed.to_string());
        }
    }
    result
}

fn print_memory_item_line(memory: &operit_runtime::data::model::Memory::Memory) {
    println!(
        "{}\t{}\t{}\t{}",
        memory.id,
        memory.title,
        memory.folderPath.clone().unwrap_or_else(String::new),
        memory
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn print_memory_item(memory: &operit_runtime::data::model::Memory::Memory) {
    println!("id={}", memory.id);
    println!("uuid={}", memory.uuid);
    println!("title={}", memory.title);
    println!("content={}", memory.content);
    println!("contentType={}", memory.contentType);
    println!("source={}", memory.source);
    println!("credibility={}", memory.credibility);
    println!("importance={}", memory.importance);
    println!(
        "folderPath={}",
        memory.folderPath.clone().unwrap_or_else(String::new)
    );
    println!("createdAt={}", memory.createdAt);
    println!("updatedAt={}", memory.updatedAt);
    println!("lastAccessedAt={}", memory.lastAccessedAt);
    println!(
        "tags={}",
        memory
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn nonBlankString(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after unix epoch")
        .as_millis() as i64
}
