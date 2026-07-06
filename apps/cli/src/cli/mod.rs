use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use flate2::read::GzDecoder;
use operit_link::CoreLinkError;
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::enhance::ToolExecutionManager::{AITool, ToolParameter};
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::tools::ToolPermissionSystem::{PermissionLevel, PermissionRequestResult};
use operit_runtime::data::api::MarketStatsApiService::{MarketEntrySummary, MarketListPage};
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
use operit_runtime::data::model::TtsConfig::TtsConfig;
use operit_runtime::data::preferences::ActivePromptManager::ActivePromptManager;
use operit_runtime::data::preferences::ApiPreferences::ApiPreferences;
use operit_runtime::data::preferences::CharacterCardManager::CharacterCardManager;
use operit_runtime::data::preferences::CharacterGroupCardManager::CharacterGroupCardManager;
use operit_runtime::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_runtime::data::preferences::PromptTagManager::PromptTagManager;
use operit_runtime::data::preferences::TtsConfigManager::TtsConfigManager;
use operit_runtime::data::repository::ChatHistoryManager::ChatHistoryManager;
use operit_runtime::services::core::MessageCoordinationDelegate::MessageCoordinationDelegate;
use operit_runtime::services::TtsSynthesisService::TtsSynthesisService;
use operit_runtime::util::stream::Stream::Stream;
use operit_runtime::util::GithubReleaseUtil::{
    FullUpdateProgressEvent, FullUpdateStatus, FullUpdateTarget, GithubReleaseUtil,
};
use sha2::{Digest, Sha256};
use tar::Archive;
use zip::ZipArchive;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

mod host_ops;
pub(crate) mod link;
mod transfer;
mod web_access;

use crate::chat_runtime::{run_chat_shell_command_with_core, run_shell_command};
use crate::core_proxy::{cli_core, local_cli_core};
use host_ops::{schedule_cli_uninstall, schedule_cli_update};
use link::{load_link_session, run_link_command};
use transfer::{run_backup_command, run_export_command, run_import_command};
use web_access::run_web_access_command;

const CLI_DEFAULT_TTS_PROVIDER_TYPE: &str = "OPENAI_COMPATIBLE";

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

    if args[0].as_str() == "install" {
        return run_install_cli_command(&args[1..]).await;
    }

    if args[0].as_str() == "uninstall" {
        return run_uninstall_cli_command(&args[1..]).await;
    }

    let mut core = local_cli_core()?;

    let result = match args[0].as_str() {
        "model" => run_core_command_and_print(&mut core, &args).await,
        "version" => run_version_core_command(&mut core).await,
        "prefs" => run_core_command_and_print(&mut core, &args).await,
        "host" => run_core_command_and_print(&mut core, &args).await,
        "log" => run_core_command_and_print(&mut core, &args).await,
        "memory" => run_core_command_and_print(&mut core, &args).await,
        "tts" => run_tts_cli_command(&args[1..]).await,
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

async fn run_tts_cli_command(args: &[String]) -> Result<(), String> {
    match args.get(0).map(String::as_str) {
        Some("config") => run_tts_config_cli_command(&args[1..]),
        Some("synthesize") => run_tts_synthesize_cli_command(&args[1..]),
        _ => {
            print_tts_usage();
            Ok(())
        }
    }
}

fn run_tts_config_cli_command(args: &[String]) -> Result<(), String> {
    let manager = TtsConfigManager::getInstance();
    match args.get(0).map(String::as_str) {
        Some("list") if args.len() == 1 => {
            let currentConfigId = manager.getCurrentTtsConfigId()?;
            for config in manager.getAllTtsConfigs()? {
                let currentMark = if config.id == currentConfigId {
                    " current=true"
                } else {
                    ""
                };
                println!(
                    "id={} name={} providerType={} model={} voice={}{}",
                    config.id,
                    config.name,
                    config.providerType,
                    config.model,
                    config.voice,
                    currentMark
                );
            }
            Ok(())
        }
        Some("show") if args.len() == 2 => {
            let config = manager.getTtsConfig(&args[1])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&config).expect("tts config must serialize")
            );
            Ok(())
        }
        Some("current") if args.len() == 1 => {
            let config = manager.getCurrentTtsConfig()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&config).expect("tts config must serialize")
            );
            Ok(())
        }
        Some("use") if args.len() == 2 => {
            let id = manager.setCurrentTtsConfigId(&args[1])?;
            println!("currentTtsConfigId={id}");
            Ok(())
        }
        Some("create") if args.len() == 8 => {
            let responseFormat = args[6].clone();
            let speed = args[7].parse::<f64>().map_err(|error| error.to_string())?;
            let config = manager.createTtsConfig(TtsConfig {
                id: String::new(),
                name: args[1].clone(),
                providerType: CLI_DEFAULT_TTS_PROVIDER_TYPE.to_string(),
                endpoint: args[2].clone(),
                apiKey: args[3].clone(),
                model: args[4].clone(),
                voice: args[5].clone(),
                responseFormat,
                speed,
                httpMethod: "POST".to_string(),
                requestBody: String::new(),
                contentType: "application/json".to_string(),
                headers: Vec::new(),
                responsePipeline: Vec::new(),
                createdAt: 0,
                updatedAt: 0,
            })?;
            println!("id={}", config.id);
            Ok(())
        }
        Some("update") if args.len() == 4 => {
            let mut config = manager.getTtsConfig(&args[1])?;
            match args[2].as_str() {
                "name" => config.name = args[3].clone(),
                "endpoint" => config.endpoint = args[3].clone(),
                "api-key" => config.apiKey = args[3].clone(),
                "model" => config.model = args[3].clone(),
                "voice" => config.voice = args[3].clone(),
                "response-format" => config.responseFormat = args[3].clone(),
                "speed" => {
                    config.speed = args[3].parse::<f64>().map_err(|error| error.to_string())?
                }
                "http-method" => config.httpMethod = args[3].clone(),
                "request-body" => config.requestBody = args[3].clone(),
                "content-type" => config.contentType = args[3].clone(),
                field => return Err(format!("unknown tts config field: {field}")),
            }
            let updated = manager.updateTtsConfig(config)?;
            println!("id={}", updated.id);
            Ok(())
        }
        Some("delete") if args.len() == 2 => {
            let deleted = manager.deleteTtsConfig(&args[1])?;
            println!("deleted={deleted}");
            Ok(())
        }
        _ => {
            print_tts_usage();
            Ok(())
        }
    }
}

fn run_tts_synthesize_cli_command(args: &[String]) -> Result<(), String> {
    let mut characterId: Option<String> = None;
    let mut text: Option<String> = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--character" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--character requires a value".to_string())?;
                characterId = Some(value.clone());
            }
            "--text" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--text requires a value".to_string())?;
                text = Some(value.clone());
            }
            value => return Err(format!("unknown tts synthesize argument: {value}")),
        }
        index += 1;
    }
    let characterId = characterId.ok_or_else(|| "--character is required".to_string())?;
    let text = text.ok_or_else(|| "--text is required".to_string())?;
    let result =
        TtsSynthesisService::new(operit_store::RuntimeStorePaths::RuntimeStorePaths::default())
            .synthesizeForCharacter(&characterId, &text)?;
    for path in result.audioPaths {
        println!("audioPath={path}");
    }
    Ok(())
}

fn print_tts_usage() {
    println!("operit2 cli tts config list");
    println!("operit2 cli tts config show <id>");
    println!("operit2 cli tts config current");
    println!("operit2 cli tts config use <id>");
    println!("operit2 cli tts config create <name> <endpoint> <api-key> <model> <voice> <response-format> <speed>");
    println!("operit2 cli tts config update <id> <name|endpoint|api-key|model|voice|response-format|speed|http-method|request-body|content-type> <value>");
    println!("operit2 cli tts config delete <id>");
    println!("operit2 cli tts synthesize --character <id> --text <text>");
}

async fn run_update_cli_command(
    core: &mut crate::core_proxy::CliCore,
    args: &[String],
) -> Result<(), String> {
    if args.is_empty() {
        let target = FullUpdateTarget::cliForCurrentHost()?;
        return run_update_with_progress(
            env!("CARGO_PKG_VERSION"),
            target,
            UpdateApplyMode::InstallCurrentTarget,
        )
        .await;
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
            print_update_target(target)
        }
        "run" if args.len() == 2 => {
            let current_version = args.get(1).ok_or_else(|| cli_update_usage("run"))?;
            let target = FullUpdateTarget::cliForCurrentHost()?;
            run_update_with_progress(
                current_version,
                target,
                UpdateApplyMode::InstallCurrentTarget,
            )
            .await
        }
        "download" if args.len() == 2 => {
            let current_version = args.get(1).ok_or_else(|| cli_update_usage("download"))?;
            let target = FullUpdateTarget::cliForCurrentHost()?;
            run_update_with_progress(current_version, target, UpdateApplyMode::DownloadOnly).await
        }
        "check" if args.len() == 2 => {
            let current_version = args.get(1).ok_or_else(|| cli_update_usage("check"))?;
            let target = FullUpdateTarget::cliForCurrentHost()?;
            let command = vec![
                "update".to_string(),
                "check".to_string(),
                current_version.to_string(),
                target.product,
                target.platform,
                target.arch,
            ];
            run_core_command_and_print(core, &command).await
        }
        _ => {
            print_update_usage();
            Ok(())
        }
    }
}

fn cli_update_usage(command: &str) -> String {
    format!("usage: operit2 cli update {command} <current-version>")
}

fn print_update_target(target: FullUpdateTarget) -> Result<(), String> {
    let package_name = target.assetName()?;
    println!("platform={}", target.platform);
    println!("arch={}", target.arch);
    println!("package={package_name}");
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateApplyMode {
    InstallCurrentTarget,
    DownloadOnly,
}

async fn run_update_with_progress(
    current_version: &str,
    target: FullUpdateTarget,
    apply_mode: UpdateApplyMode,
) -> Result<(), String> {
    let package_name = target.assetName()?;
    let target_for_install = target.clone();
    let channel = GithubReleaseUtil::fullUpdateChannelForVersion(current_version)?;
    let status = GithubReleaseUtil::checkForFullUpdateBlocking(current_version, target)?;
    match status {
        FullUpdateStatus::Available(info) => {
            println!("status=available");
            println!("currentVersion={current_version}");
            println!("channel={channel}");
            println!("latestVersion={}", info.version);
            println!("package={}", info.assetName);
            let work_dir = std::env::temp_dir().join("operit2").join("full_update");
            let last_line_len = Arc::new(Mutex::new(0usize));
            let progress_line_len = Arc::clone(&last_line_len);
            let package_path = GithubReleaseUtil::downloadAndPrepareFullUpdateBlocking(
                &info.downloadUrl,
                &info.assetName,
                &work_dir,
                move |event| match event {
                    FullUpdateProgressEvent::StageChanged { stage: _, message } => {
                        let mut last_line_len = progress_line_len
                            .lock()
                            .expect("progress line length mutex poisoned");
                        clear_progress_line(&mut *last_line_len);
                        println!("{message}");
                    }
                    FullUpdateProgressEvent::DownloadProgress {
                        readBytes,
                        totalBytes,
                        speedBytesPerSec,
                    } => {
                        let percent = readBytes as f64 * 100.0 / totalBytes as f64;
                        let line = format!(
                            "download={percent:.1}% bytes={}/{} speed={}/s",
                            format_bytes(readBytes),
                            format_bytes(totalBytes),
                            format_bytes(speedBytesPerSec),
                        );
                        let mut last_line_len = progress_line_len
                            .lock()
                            .expect("progress line length mutex poisoned");
                        print!("\r{line}");
                        if *last_line_len > line.len() {
                            print!("{}", " ".repeat(*last_line_len - line.len()));
                            print!("\r{line}");
                        }
                        io::stdout().flush().expect("stdout flush failed");
                        *last_line_len = line.len();
                    }
                },
            )?;
            let mut last_line_len = last_line_len
                .lock()
                .expect("progress line length mutex poisoned");
            clear_progress_line(&mut *last_line_len);
            println!("status=downloaded");
            println!("currentVersion={current_version}");
            println!("channel={channel}");
            println!("latestVersion={}", info.version);
            println!("package={}", info.assetName);
            println!("packagePath={}", package_path.display());
            println!("releasePageUrl={}", info.releasePageUrl);
            if apply_mode == UpdateApplyMode::InstallCurrentTarget {
                handle_downloaded_update_package(&target_for_install, &package_path)?;
            } else {
                println!("installStatus=download-only");
            }
        }
        FullUpdateStatus::UpToDate => {
            println!("status=up-to-date");
            println!("currentVersion={current_version}");
            println!("channel={channel}");
            println!("package={package_name}");
        }
    }
    Ok(())
}

fn clear_progress_line(last_line_len: &mut usize) {
    if *last_line_len == 0 {
        return;
    }
    print!("\r{}\r", " ".repeat(*last_line_len));
    io::stdout().flush().expect("stdout flush failed");
    *last_line_len = 0;
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

async fn run_install_cli_command(args: &[String]) -> Result<(), String> {
    if matches!(args, [command] if command == "status") {
        return print_cli_install_status();
    }
    let source = match args {
        [] => env::current_exe().map_err(|error| error.to_string())?,
        [flag, value] if flag == "--source" => PathBuf::from(value),
        _ => {
            print_install_usage();
            return Ok(());
        }
    };
    install_cli_from_source(&source, InstallMode::Direct, InstallOutput::Print, |_| {})
}

async fn run_uninstall_cli_command(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        print_uninstall_usage();
        return Ok(());
    }
    uninstall_cli()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DownloadedUpdateInstallStatus {
    Installed,
    Scheduled,
    NotInstalled,
    TargetMismatch,
}

pub(crate) fn install_downloaded_cli_update(
    target: &FullUpdateTarget,
    package_path: &Path,
    output: InstallOutput,
) -> Result<DownloadedUpdateInstallStatus, String> {
    if target.product != "cli" {
        return Ok(DownloadedUpdateInstallStatus::TargetMismatch);
    }
    let current_target = FullUpdateTarget::cliForCurrentHost()?;
    if target != &current_target {
        return Ok(DownloadedUpdateInstallStatus::TargetMismatch);
    }
    let source = extract_cli_binary_from_package(target, package_path)?;
    let install_state = current_cli_install_state()?;
    match install_state {
        CliInstallState::Installed => {
            install_cli_from_source(&source, InstallMode::Update, output, |_| {})?;
            Ok(DownloadedUpdateInstallStatus::Scheduled)
        }
        CliInstallState::NotInstalled => Ok(DownloadedUpdateInstallStatus::NotInstalled),
    }
}

fn handle_downloaded_update_package(
    target: &FullUpdateTarget,
    package_path: &Path,
) -> Result<(), String> {
    match install_downloaded_cli_update(target, package_path, InstallOutput::Print)? {
        DownloadedUpdateInstallStatus::Installed => {}
        DownloadedUpdateInstallStatus::Scheduled => {}
        DownloadedUpdateInstallStatus::NotInstalled => println!("installStatus=not-installed"),
        DownloadedUpdateInstallStatus::TargetMismatch => println!("installStatus=target-mismatch"),
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliInstallState {
    Installed,
    NotInstalled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallMode {
    Direct,
    Update,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallOutput {
    Print,
    Silent,
}

pub(crate) fn install_current_cli(output: InstallOutput) -> Result<(), String> {
    install_current_cli_with_progress(output, |_| {})
}

pub(crate) fn install_current_cli_with_progress<F>(
    output: InstallOutput,
    on_progress: F,
) -> Result<(), String>
where
    F: FnMut(CliInstallProgress),
{
    let source = env::current_exe().map_err(|error| error.to_string())?;
    install_cli_from_source(&source, InstallMode::Direct, output, on_progress)
}

pub(crate) fn cli_is_installed() -> Result<bool, String> {
    Ok(current_cli_install_state()? == CliInstallState::Installed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliInstallProgress {
    CopyOperit,
    CopyOperit2,
    UpdatePath,
    Complete,
}

fn install_cli_from_source(
    source: &Path,
    mode: InstallMode,
    output: InstallOutput,
    mut on_progress: impl FnMut(CliInstallProgress),
) -> Result<(), String> {
    if !source.is_file() {
        return Err(format!(
            "Operit2 CLI binary not found: {}",
            source.display()
        ));
    }

    let install_dir = cli_install_dir()?;
    let operit = cli_command_path(&install_dir, "operit");
    let operit2 = cli_command_path(&install_dir, "operit2");
    fs::create_dir_all(&install_dir).map_err(|error| error.to_string())?;

    if mode == InstallMode::Update {
        schedule_cli_update(source, &operit, &operit2, &install_dir)?;
        print_install_scheduled(&install_dir, output);
        return Ok(());
    }

    on_progress(CliInstallProgress::CopyOperit);
    copy_cli_binary(source, &operit)?;
    on_progress(CliInstallProgress::CopyOperit2);
    copy_cli_binary(source, &operit2)?;
    on_progress(CliInstallProgress::UpdatePath);
    add_cli_install_dir_to_path(&install_dir)?;
    on_progress(CliInstallProgress::Complete);

    print_install_installed(&install_dir, output);
    Ok(())
}

fn print_install_installed(install_dir: &Path, output: InstallOutput) {
    if output == InstallOutput::Silent {
        return;
    }
    println!("installStatus=installed");
    println!("installDir={}", install_dir.display());
    println!("command=operit");
    println!("command=operit2");
}

fn print_install_scheduled(install_dir: &Path, output: InstallOutput) {
    if output == InstallOutput::Silent {
        return;
    }
    println!("installStatus=scheduled");
    println!("installDir={}", install_dir.display());
    println!("message=restart-terminal-after-update");
}

fn uninstall_cli() -> Result<(), String> {
    let install_dir = cli_install_dir()?;
    let operit = cli_command_path(&install_dir, "operit");
    let operit2 = cli_command_path(&install_dir, "operit2");

    if current_exe_is_installed_cli()? {
        schedule_cli_uninstall(&operit, &operit2, &install_dir)?;
        println!("uninstallStatus=scheduled");
        println!("installDir={}", install_dir.display());
        println!("message=restart-terminal-after-uninstall");
        return Ok(());
    }

    remove_file_if_exists(&operit)?;
    remove_file_if_exists(&operit2)?;
    remove_cli_install_dir_from_path(&install_dir)?;

    println!("uninstallStatus=uninstalled");
    println!("installDir={}", install_dir.display());
    Ok(())
}

fn current_cli_install_state() -> Result<CliInstallState, String> {
    let install_dir = cli_install_dir()?;
    let operit = cli_command_path(&install_dir, "operit");
    let operit2 = cli_command_path(&install_dir, "operit2");
    if path_is_file(&operit)? || path_is_file(&operit2)? {
        Ok(CliInstallState::Installed)
    } else {
        Ok(CliInstallState::NotInstalled)
    }
}

fn current_exe_is_installed_cli() -> Result<bool, String> {
    let current = normalize_existing_path(&env::current_exe().map_err(|error| error.to_string())?)?;
    let install_dir = cli_install_dir()?;
    let operit = cli_command_path(&install_dir, "operit");
    let operit2 = cli_command_path(&install_dir, "operit2");
    Ok(existing_paths_equal(&current, &operit)? || existing_paths_equal(&current, &operit2)?)
}

fn print_cli_install_status() -> Result<(), String> {
    let install_dir = cli_install_dir()?;
    let operit = cli_command_path(&install_dir, "operit");
    let operit2 = cli_command_path(&install_dir, "operit2");
    let operit_exists = path_is_file(&operit)?;
    let operit2_exists = path_is_file(&operit2)?;
    println!("installDir={}", install_dir.display());
    println!("operitPath={}", operit.display());
    println!("operitExists={operit_exists}");
    println!("operit2Path={}", operit2.display());
    println!("operit2Exists={operit2_exists}");
    println!(
        "installed={}",
        current_cli_install_state()? == CliInstallState::Installed
    );
    println!(
        "pathContainsInstallDir={}",
        cli_install_dir_is_on_path(&install_dir)?
    );
    println!("currentExeIsInstalled={}", current_exe_is_installed_cli()?);
    println!(
        "currentExe={}",
        env::current_exe()
            .map_err(|error| error.to_string())?
            .display()
    );
    Ok(())
}

fn cli_command_path(install_dir: &Path, name: &str) -> PathBuf {
    install_dir.join(cli_command_file_name(name))
}

fn cli_command_file_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn cli_install_dir() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        let local_app_data =
            env::var_os("LOCALAPPDATA").ok_or_else(|| "LOCALAPPDATA is required".to_string())?;
        return Ok(PathBuf::from(local_app_data)
            .join("Programs")
            .join("Operit2")
            .join("bin"));
    }

    #[cfg(not(windows))]
    {
        let home = env::var_os("HOME").ok_or_else(|| "HOME is required".to_string())?;
        return Ok(PathBuf::from(home).join(".local").join("bin"));
    }
}

fn cli_profile_file() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or_else(|| "HOME is required".to_string())?;
    Ok(PathBuf::from(home).join(".profile"))
}

fn copy_cli_binary(source: &Path, destination: &Path) -> Result<(), String> {
    if existing_paths_equal(source, destination)? {
        set_cli_binary_permissions(destination)?;
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        return copy_cli_binary_atomic(source, destination);
    }

    #[cfg(windows)]
    {
        fs::copy(source, destination).map_err(|error| {
            format!(
                "Failed to copy {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
        set_cli_binary_permissions(destination)?;
        Ok(())
    }
}

#[cfg(not(windows))]
fn copy_cli_binary_atomic(source: &Path, destination: &Path) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| format!("invalid destination: {}", destination.display()))?;
    let file_name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("invalid destination file name: {}", destination.display()))?;
    let temp = parent.join(format!(".{file_name}.tmp-{}", unique_suffix()));

    fs::copy(source, &temp).map_err(|error| {
        format!(
            "Failed to copy {} to {}: {error}",
            source.display(),
            temp.display()
        )
    })?;
    set_cli_binary_permissions(&temp)?;
    match fs::rename(&temp, destination) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&temp);
            Err(format!(
                "Failed to replace {} with {}: {error}",
                destination.display(),
                source.display()
            ))
        }
    }
}

fn set_cli_binary_permissions(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path)
            .map_err(|error| error.to_string())?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to remove {}: {error}", path.display())),
    }
}

fn normalize_existing_path(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path).map_err(|error| format!("Failed to resolve {}: {error}", path.display()))
}

fn path_is_file(path: &Path) -> Result<bool, String> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("Failed to inspect {}: {error}", path.display())),
    }
}

fn existing_paths_equal(left: &Path, right: &Path) -> Result<bool, String> {
    let left = normalize_existing_path(left)?;
    let right = match fs::canonicalize(right) {
        Ok(path) => path,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("Failed to resolve {}: {error}", right.display())),
    };
    if cfg!(windows) {
        Ok(left
            .to_string_lossy()
            .eq_ignore_ascii_case(right.to_string_lossy().as_ref()))
    } else {
        Ok(left == right)
    }
}

fn add_cli_install_dir_to_path(install_dir: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        let current_path = read_windows_user_path()?;
        let mut parts = split_windows_path(&current_path);
        if !windows_path_parts_contain(&parts, install_dir) {
            parts.push(install_dir.display().to_string());
            write_windows_user_path(&parts.join(";"))?;
        }
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        let profile_file = cli_profile_file()?;
        let path_line = cli_unix_path_line();
        let current = read_text_file_when_present(&profile_file)?;
        let exists = current.lines().any(|line| line == path_line);
        if !exists {
            let mut next = current;
            if !next.is_empty() && !next.ends_with('\n') {
                next.push('\n');
            }
            next.push_str(path_line);
            next.push('\n');
            fs::write(&profile_file, next).map_err(|error| error.to_string())?;
        }
        return Ok(());
    }
}

fn cli_install_dir_is_on_path(install_dir: &Path) -> Result<bool, String> {
    #[cfg(windows)]
    {
        let current_path = read_windows_user_path()?;
        let parts = split_windows_path(&current_path);
        return Ok(windows_path_parts_contain(&parts, install_dir));
    }

    #[cfg(not(windows))]
    {
        let profile_file = cli_profile_file()?;
        let current = read_text_file_when_present(&profile_file)?;
        return Ok(current.lines().any(|line| line == cli_unix_path_line()));
    }
}

fn remove_cli_install_dir_from_path(install_dir: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        let current_path = read_windows_user_path()?;
        let parts = split_windows_path(&current_path)
            .into_iter()
            .filter(|part| !windows_path_part_matches(part, install_dir))
            .collect::<Vec<_>>();
        write_windows_user_path(&parts.join(";"))?;
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        let profile_file = cli_profile_file()?;
        let current = match fs::read_to_string(&profile_file) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.to_string()),
        };
        let path_line = cli_unix_path_line();
        let next = current
            .lines()
            .filter(|line| *line != path_line)
            .collect::<Vec<_>>()
            .join("\n");
        let content = if next.is_empty() {
            String::new()
        } else {
            format!("{next}\n")
        };
        fs::write(&profile_file, content).map_err(|error| error.to_string())?;
        return Ok(());
    }
}

fn read_text_file_when_present(path: &Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.to_string()),
    }
}

fn cli_unix_path_line() -> &'static str {
    r#"export PATH="$HOME/.local/bin:$PATH""#
}

#[cfg(windows)]
fn read_windows_user_path() -> Result<String, String> {
    read_windows_user_environment_value("Path")
}

#[cfg(windows)]
fn write_windows_user_path(value: &str) -> Result<(), String> {
    let escaped = value.replace('\'', "''");
    let script = format!("[Environment]::SetEnvironmentVariable('Path', '{escaped}', 'User')");
    run_powershell_script(&script)
}

#[cfg(windows)]
fn read_windows_user_environment_value(name: &str) -> Result<String, String> {
    let escaped = name.replace('\'', "''");
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &format!("[Environment]::GetEnvironmentVariable('{escaped}', 'User')"),
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string())
}

#[cfg(windows)]
fn split_windows_path(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(windows)]
fn windows_path_parts_contain(parts: &[String], target: &Path) -> bool {
    parts
        .iter()
        .any(|part| windows_path_part_matches(part, target))
}

#[cfg(windows)]
fn windows_path_part_matches(part: &str, target: &Path) -> bool {
    let part = part.trim_end_matches(['\\', '/']);
    let target = target.display().to_string();
    let target = target.trim_end_matches(['\\', '/']);
    part.eq_ignore_ascii_case(target)
}

fn extract_cli_binary_from_package(
    target: &FullUpdateTarget,
    package_path: &Path,
) -> Result<PathBuf, String> {
    let extract_dir = std::env::temp_dir()
        .join("operit2")
        .join("cli_update_extract")
        .join(unique_suffix());
    fs::create_dir_all(&extract_dir).map_err(|error| error.to_string())?;
    match target.platform.as_str() {
        "windows" => extract_cli_binary_from_zip(package_path, &extract_dir),
        "linux" | "macos" => extract_cli_binary_from_tar_gz(package_path, &extract_dir),
        other => Err(format!("Unsupported CLI update package platform: {other}")),
    }
}

fn extract_cli_binary_from_zip(package_path: &Path, extract_dir: &Path) -> Result<PathBuf, String> {
    let file = fs::File::open(package_path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut entry = archive.by_name("operit2.exe").map_err(|error| {
        format!(
            "operit2.exe not found in {}: {error}",
            package_path.display()
        )
    })?;
    let destination = extract_dir.join("operit2.exe");
    let mut output = fs::File::create(&destination).map_err(|error| error.to_string())?;
    io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
    Ok(destination)
}

fn extract_cli_binary_from_tar_gz(
    package_path: &Path,
    extract_dir: &Path,
) -> Result<PathBuf, String> {
    let file = fs::File::open(package_path).map_err(|error| error.to_string())?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    for entry in archive.entries().map_err(|error| error.to_string())? {
        let mut entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path().map_err(|error| error.to_string())?;
        if archive_path_is_exact_file(&path, "operit2") {
            let destination = extract_dir.join("operit2");
            entry
                .unpack(&destination)
                .map_err(|error| error.to_string())?;
            #[cfg(unix)]
            {
                let mut permissions = fs::metadata(&destination)
                    .map_err(|error| error.to_string())?
                    .permissions();
                permissions.set_mode(0o755);
                fs::set_permissions(&destination, permissions)
                    .map_err(|error| error.to_string())?;
            }
            return Ok(destination);
        }
    }
    Err(format!("operit2 not found in {}", package_path.display()))
}

fn archive_path_is_exact_file(path: &Path, file_name: &str) -> bool {
    let mut components = path.components().filter_map(|component| match component {
        Component::CurDir => None,
        Component::Normal(value) => Some(value),
        _ => Some(OsStr::new("")),
    });
    let first = components.next();
    let second = components.next();
    first == Some(OsStr::new(file_name)) && second.is_none()
}

fn unique_suffix() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_millis();
    format!("{}-{millis}", std::process::id())
}

#[cfg(windows)]
fn run_powershell_script(script: &str) -> Result<(), String> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn rewrite_cli_usage_message(message: String) -> String {
    const ROOT_USAGE_PREFIX: &str = "usage: operit2 ";
    const CLI_USAGE_PREFIX: &str = "usage: operit2 cli ";
    if message.starts_with(CLI_USAGE_PREFIX) {
        return message;
    }
    match message.strip_prefix(ROOT_USAGE_PREFIX) {
        Some(rest) => format!("{CLI_USAGE_PREFIX}{rest}"),
        None => message,
    }
}

async fn run_core_command_and_print(
    core: &mut crate::core_proxy::CliCore,
    args: &[String],
) -> Result<(), String> {
    let output = core
        .runCoreCommand(args)
        .await
        .map_err(core_command_error_message)?;
    if !output.stdout.is_empty() {
        print!("{}", rewrite_core_command_usage_message(output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", rewrite_core_command_usage_message(output.stderr));
    }
    Ok(())
}

fn rewrite_core_command_usage_message(message: String) -> String {
    let ends_with_newline = message.ends_with('\n');
    let lines = message
        .lines()
        .map(rewrite_core_command_usage_line)
        .collect::<Vec<_>>();
    let mut rewritten = lines.join("\n");
    if ends_with_newline {
        rewritten.push('\n');
    }
    rewritten
}

fn rewrite_core_command_usage_line(line: &str) -> String {
    const ROOT_COMMAND_PREFIX: &str = "operit2 ";
    const CLI_COMMAND_PREFIX: &str = "operit2 cli ";
    if line.starts_with(CLI_COMMAND_PREFIX) {
        return line.to_string();
    }
    match line.strip_prefix(ROOT_COMMAND_PREFIX) {
        Some(rest) => format!("{CLI_COMMAND_PREFIX}{rest}"),
        None => rewrite_cli_usage_message(line.to_string()),
    }
}

fn core_command_error_message(error: CoreLinkError) -> String {
    if error.isCommandError() {
        rewrite_core_command_usage_message(error.message)
    } else {
        rewrite_core_command_usage_message(error.to_string())
    }
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
    println!("operit2 install [--source <path>]");
    println!("operit2 uninstall");
    println!("operit2 [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>] [--update-current-version <version>]");
    println!("operit2 tui [--chat <chat-id>] [--character <character-card-name>] [--group-card <character-group-id>] [--group <group-name>] [--update-current-version <version>]");
    println!("operit2 cli <version|prefs|host|log|memory|tts|export|import|backup|model|chat|workspace|tag|character|group|active-prompt|approval|tool|market|update|install|uninstall|skill|package|plugin|mcp|link|web|shell>");
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
    println!("operit2 cli tts config <list|show|current|use|create|update|delete>");
    println!("operit2 cli tts synthesize --character <id> --text <text>");
    println!("operit2 cli export <memory|chat|snapshot>");
    println!("operit2 cli import <memory|chat|snapshot|operit1-model-config>");
    println!("operit2 cli backup <create|restore|inspect|inspect-operit1-model-config>");
    println!("operit2 cli model <init|list|show|set|set-key|api-settings-full|custom-headers|request-queue|api-key-pool|custom-parameters|parameters|tool-call|direct-image|direct-audio|direct-video|google-search|params|context-show|context-set|summary-show|summary-set|function-list|function-show|function-set|function-reset>");
    println!("operit2 cli tag <list|show|create|update|delete>");
    println!("operit2 cli character <init|list|show|create|update|delete|set-active|combine|reset-default>");
    println!("operit2 cli group <init|list|show|create|update|delete|set-active|duplicate>");
    println!("operit2 cli active-prompt <show|set-card|set-group|activate-for-chat|resolved-card>");
    println!("operit2 cli approval <status|list|allow|ask|forbid|tool>");
    println!("operit2 cli tool <list|show|exec>");
    println!(
        "operit2 cli market <auth|rank|list|search|show|comments|comment|like|notifications|my|publish|install|download>"
    );
    println!("operit2 cli update [check|target]");
    println!("operit2 cli install [--source <path>]");
    println!("operit2 cli uninstall");
    println!("operit2 cli skill <dir|list|more|load|show|create|import-zip|delete|visible|errors>");
    println!("operit2 cli package <help|dir|list|more|load|show|import|enable|disable|use|exec>");
    println!("operit2 cli plugin <help|list|more|load|show|import|enable|disable>");
    println!("operit2 cli mcp <dir|list|show|import|export|remove|enable|disable|start|kill|tools|config|config-set|local-set|meta|meta-set|describe>");
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
    println!("operit2 cli model use <provider-id> <model-id>");
    println!("operit2 cli model params [model-id]");
    println!("operit2 cli model parameters <provider-id> <model-id> <parameters-json>");
    println!("operit2 cli model context-show [model-id]");
    println!("operit2 cli model context-set <provider-id> <model-id> <max-context-length> <enable-max-context-mode>");
    println!("operit2 cli model summary-show [model-id]");
    println!("operit2 cli model summary-set <provider-id> <model-id> <enable-summary> <summary-token-threshold> <enable-summary-by-message-count> <summary-message-count-threshold>");
    println!("operit2 cli model function-list");
    println!("operit2 cli model function-show <function-type>");
    println!("operit2 cli model function-set <function-type> <provider-id> <model-id>");
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
    println!("operit2 cli market auth <token|login>");
    println!("operit2 cli market rank [updated|likes|downloads] [page]");
    println!("operit2 cli market list [updated|likes|downloads] [type|-] [category|-] [page]");
    println!("operit2 cli market search <query> [updated|likes|downloads] [type|-] [category|-]");
    println!("operit2 cli market show <entryId>");
    println!("operit2 cli market install <entryId> [versionId]");
    println!("operit2 cli market comments <entryId> [page]");
    println!("operit2 cli market comment <entryId> <body-or-@file>");
    println!("operit2 cli market comment edit <commentId> <body-or-@file>");
    println!("operit2 cli market comment delete <commentId>");
    println!("operit2 cli market like <entryId>");
    println!("operit2 cli market notifications [limit] [offset]");
    println!("operit2 cli market my");
    println!("operit2 cli market publish artifact <type> <title> <description-or-@file> <detail-or-@file> <categoryId> <allowPublicUpdates> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or-> <projectId> <runtimePackageId> <assetKind> <assetUrl> <ghOwner> <ghRepo> <ghReleaseTag> <assetName> <sha256>");
    println!("operit2 cli market publish repo <type> <title> <description-or-@file> <detail-or-@file> <categoryId> <allowPublicUpdates> <sourceUrl> <refType> <refName> <installConfig-or-@file> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or->");
    println!("operit2 cli market publish version artifact <entryId> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or-> <projectId> <runtimePackageId> <assetKind> <assetUrl> <ghOwner> <ghRepo> <ghReleaseTag> <assetName> <sha256> [entryTitle|-] [entryDescription-or-] [entryDetail-or-] [entryCategoryId|-] [entryAllowPublicUpdates|-]");
    println!("operit2 cli market publish version repo <entryId> <version> <formatVer> <minAppVer> <maxAppVer-or-> <changelog-or-> <refType> <refName> <installConfig-or-@file> [entryTitle|-] [entryDescription-or-] [entryDetail-or-] [entryCategoryId|-] [entryAllowPublicUpdates|-]");
    println!("operit2 cli market publish update-entry <entryId> <title-or-> <description-or-@file-or-> <detail-or-@file-or-> <categoryId-or-> <allowPublicUpdates-or->");
    println!("operit2 cli market download <assetId>");
}

fn print_update_usage() {
    println!("operit2 cli update");
    println!("operit2 cli update check");
    println!("operit2 cli update target");
    println!("operit2 cli update run <current-version>");
    println!("operit2 cli update download <current-version>");
    println!("operit2 cli update check <current-version>");
}

fn print_install_usage() {
    println!("operit2 install [--source <path>]");
    println!("operit2 cli install [--source <path>]");
    println!("operit2 cli install status");
}

fn print_uninstall_usage() {
    println!("operit2 uninstall");
    println!("operit2 cli uninstall");
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
    println!("operit2 cli mcp kill <id>");
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
        "ttsConfigId={}",
        card.ttsConfigId.clone().unwrap_or_default()
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
