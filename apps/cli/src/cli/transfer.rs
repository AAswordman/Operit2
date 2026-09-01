use std::fs;
use std::path::Path;

use operit_model::ImportStrategy;
use operit_runtime::services::ArchiveTransferManager::StagedArchive;
use operit_util::stream::ReverseStream::ReverseStream;
use tokio::io::AsyncReadExt;

use super::*;
use crate::core_proxy::CliCore;

const SNAPSHOT_IMPORT_CHUNK_BYTES: usize = 64 * 1024;

/// Runs export commands for memories, chats, and snapshots.
pub(super) async fn run_export_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_export_usage();
        return Ok(());
    }
    match args[0].as_str() {
        "memory" => {
            let path = args
                .get(1)
                .ok_or_else(|| "usage: operit2 export memory <path> <owner-key>".to_string())?;
            let ownerKey = memory_owner_key_arg_for_transfer(args.get(2))?;
            let content = core
                .repository_memory_repository(&ownerKey)
                .exportMemoriesToJson()
                .await
                .map_err(|error| error.to_string())?;
            write_text(path, &content)?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!({ "path": Path::new(path), "format": "memory" }));
            } else {
                println!("Exported memories to {}", Path::new(path).display());
            }
            Ok(())
        }
        "chat" => {
            let path = args
                .get(1)
                .ok_or_else(|| "usage: operit2 export chat <path>".to_string())?;
            let content = core
                .chat_runtime_holder_main()
                .exportChatHistoriesToJson()
                .await
                .map_err(|error| error.to_string())?;
            write_text(path, &content)?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!({ "path": Path::new(path), "format": "chat" }));
            } else {
                println!("Exported chats to {}", Path::new(path).display());
            }
            Ok(())
        }
        "snapshot" => export_snapshot(core, args.get(1)).await,
        _ => {
            print_export_usage();
            Ok(())
        }
    }
}

/// Runs import commands for memories, chats, and snapshots.
pub(super) async fn run_import_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_import_usage();
        return Ok(());
    }
    match args[0].as_str() {
        "memory" => {
            let path = args.get(1).ok_or_else(|| {
                "usage: operit2 import memory <path> <SKIP|UPDATE|CREATE_NEW> <owner-key>"
                    .to_string()
            })?;
            let strategy = parse_import_strategy(args.get(2))?;
            let ownerKey = memory_owner_key_arg_for_transfer(args.get(3))?;
            let content = read_text(path)?;
            let result = core
                .repository_memory_repository(&ownerKey)
                .importMemoriesFromJson(content, strategy)
                .await
                .map_err(|error| error.to_string())?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!(result));
            } else {
                println!("New memories: {}", result.newMemories);
                println!("Updated memories: {}", result.updatedMemories);
                println!("Skipped memories: {}", result.skippedMemories);
                println!("New links: {}", result.newLinks);
            }
            Ok(())
        }
        "chat" => {
            let path = args
                .get(1)
                .ok_or_else(|| "usage: operit2 import chat <path>".to_string())?;
            let content = read_text(path)?;
            let result = core
                .chat_runtime_holder_main()
                .importChatHistoriesFromJson(content)
                .await
                .map_err(|error| error.to_string())?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!(result));
            } else {
                println!("New chats: {}", result.new);
                println!("Updated chats: {}", result.updated);
                println!("Skipped chats: {}", result.skipped);
            }
            Ok(())
        }
        "snapshot" => import_snapshot(core, args.get(1)).await,
        "operit1-snapshot" => import_operit1_snapshot(core, args.get(1)).await,
        _ => {
            print_import_usage();
            Ok(())
        }
    }
}

/// Runs backup creation, restoration, and inspection commands.
pub(super) async fn run_backup_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_backup_usage();
        return Ok(());
    }
    match args[0].as_str() {
        "create" => export_snapshot(core, args.get(1)).await,
        "restore" => import_snapshot(core, args.get(1)).await,
        "inspect" => {
            let path = args
                .get(1)
                .ok_or_else(|| "usage: operit2 backup inspect <snapshot-zip-path>".to_string())?;
            let archive = stageArchiveUpload(core, path).await?;
            let result = core
                .services_snapshot_import_manager()
                .inspectRawSnapshot(archive.clone())
                .await
                .map_err(|error| error.to_string());
            let manifest = finishStagedArchive(core, &archive, result).await?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!(manifest));
            } else {
                println!("Snapshot format: {}", manifest.formatVersion);
                println!("Created: {}", manifest.createdAt);
                println!("Files ({}):", manifest.includes.len());
                for path in manifest.includes {
                    println!("  {path}");
                }
            }
            Ok(())
        }
        "inspect-operit1-snapshot" => {
            let path = args.get(1).ok_or_else(|| {
                "usage: operit2 backup inspect-operit1-snapshot <snapshot-zip-path>".to_string()
            })?;
            let archive = stageArchiveUpload(core, path).await?;
            let result = core
                .services_snapshot_import_manager()
                .inspectOperit1Snapshot(archive.clone())
                .await
                .map_err(|error| error.to_string());
            let preview = finishStagedArchive(core, &archive, result).await?;
            if cli_json_mode() {
                emit_cli_json(serde_json::json!(preview));
            } else {
                println!("Package: {}", preview.packageName);
                println!("Chats: {}", preview.chatCount);
                println!("Messages: {}", preview.messageCount);
                println!("Imported files: {}", preview.importedFileCount);
                println!("Imported external files: {}", preview.importedExternalFileCount);
                println!("Detected domains: {}", preview.detectedDomains.join(", "));
                println!("Model configurations: {}", preview.modelConfig.configs.len());
                for config in preview.modelConfig.configs {
                    println!("  {} ({})", config.name, config.configId);
                }
                for datastoreFile in preview.datastoreFiles {
                    println!("  datastore {}: {} keys", datastoreFile.fileName, datastoreFile.keyCount);
                }
            }
            Ok(())
        }
        _ => {
            print_backup_usage();
            Ok(())
        }
    }
}

/// Exports a raw runtime snapshot to a local archive file.
async fn export_snapshot(core: &mut CliCore, path: Option<&String>) -> Result<(), String> {
    let path = path.ok_or_else(|| "usage: operit2 export snapshot <path>".to_string())?;
    let bytes = core
        .services_snapshot_import_manager()
        .exportRawSnapshot()
        .await
        .map_err(|error| error.to_string())?;
    write_bytes(path, &bytes)?;
    if cli_json_mode() {
        emit_cli_json(serde_json::json!({ "path": Path::new(path), "bytes": bytes.len(), "format": "snapshot" }));
    } else {
        println!("Exported snapshot to {} ({} bytes)", Path::new(path).display(), bytes.len());
    }
    Ok(())
}

/// Restores a raw runtime snapshot from a local archive file.
async fn import_snapshot(core: &mut CliCore, path: Option<&String>) -> Result<(), String> {
    let path = path.ok_or_else(|| "usage: operit2 import snapshot <path>".to_string())?;
    let archive = stageArchiveUpload(core, path).await?;
    let result = async {
        core.services_snapshot_import_manager()
            .inspectRawSnapshot(archive.clone())
            .await
            .map_err(|error| error.to_string())?;
        core.services_snapshot_import_manager()
            .restoreRawSnapshot(archive.clone())
            .await
            .map_err(|error| error.to_string())
    }
    .await;
    finishStagedArchive(core, &archive, result).await?;
    if cli_json_mode() {
        emit_cli_json(serde_json::json!({ "path": Path::new(path), "format": "snapshot" }));
    } else {
        println!("Imported snapshot from {}", Path::new(path).display());
    }
    Ok(())
}

/// Imports an Operit1 snapshot and reports the structured migration result.
async fn import_operit1_snapshot(core: &mut CliCore, path: Option<&String>) -> Result<(), String> {
    let path = path
        .ok_or_else(|| "usage: operit2 import operit1-snapshot <snapshot-zip-path>".to_string())?;
    let archive = stageArchiveUpload(core, path).await?;
    let importResult = async {
        core.services_snapshot_import_manager()
            .inspectOperit1Snapshot(archive.clone())
            .await
            .map_err(|error| error.to_string())?;
        core.services_snapshot_import_manager()
            .importOperit1Snapshot(archive.clone())
            .await
            .map_err(|error| error.to_string())
    }
    .await;
    let result = finishStagedArchive(core, &archive, importResult).await?;
    if cli_json_mode() {
        emit_cli_json(serde_json::json!(result));
    } else {
        println!("Imported chats: {}", result.importedChats);
        println!("Imported messages: {}", result.importedMessages);
        println!("Imported memories: {}", result.importedMemories);
        println!("Imported files: {}", result.importedFiles);
        println!("Imported external files: {}", result.importedExternalFiles);
        println!("Imported workspaces: {}", result.importedWorkspaces);
        if !result.modelConfig.skippedFields.is_empty() {
            println!("Skipped fields: {}", result.modelConfig.skippedFields.join(", "));
        }
    }
    Ok(())
}

/// Streams one local archive file through the generated reverse-stream API.
async fn stageArchiveUpload(core: &mut CliCore, path: &str) -> Result<StagedArchive, String> {
    let byteLength = i64::try_from(fs::metadata(path).map_err(|error| error.to_string())?.len())
        .map_err(|_| format!("archive is too large: {}", Path::new(path).display()))?;
    let archiveId = core
        .services_archive_transfer_manager()
        .beginArchiveUpload(byteLength)
        .await
        .map_err(|error| error.to_string())?;
    let (mut sender, bytes) = ReverseStream::channel();
    let uploadPath = path.to_string();
    let producer = tokio::spawn(async move {
        let mut file = tokio::fs::File::open(&uploadPath)
            .await
            .map_err(|error| error.to_string())?;
        let mut buffer = vec![0u8; SNAPSHOT_IMPORT_CHUNK_BYTES];
        loop {
            let read = file
                .read(&mut buffer)
                .await
                .map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            sender.send(buffer[..read].to_vec()).await?;
        }
        sender.close();
        Ok::<(), String>(())
    });
    let result = async {
        core.services_archive_transfer_manager()
            .writeArchiveUpload(archiveId.clone(), bytes)
            .await
            .map_err(|error| error.to_string())?;
        producer
            .await
            .map_err(|error| format!("archive upload producer failed: {error}"))??;
        core.services_archive_transfer_manager()
            .completeArchiveUpload(archiveId.clone(), byteLength)
            .await
            .map_err(|error| error.to_string())
    }
    .await;
    match result {
        Ok(archive) => Ok(archive),
        Err(error) => {
            let mut messages = vec![error];
            if let Err(discardError) = core
                .services_archive_transfer_manager()
                .discardArchiveUpload(archiveId)
                .await
            {
                messages.push(format!("failed to discard archive upload: {discardError}"));
            }
            Err(messages.join("; "))
        }
    }
}

/// Discards one staged archive after its terminal consumer operation completes or fails.
async fn finishStagedArchive<T>(
    core: &mut CliCore,
    archive: &StagedArchive,
    result: Result<T, String>,
) -> Result<T, String> {
    let discardResult = core
        .services_archive_transfer_manager()
        .discardArchiveUpload(archive.archiveId.clone())
        .await
        .map_err(|error| error.to_string());
    match (result, discardResult) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(discardError)) => {
            Err(format!("failed to discard staged archive: {discardError}"))
        }
        (Err(operationError), Ok(())) => Err(operationError),
        (Err(operationError), Err(discardError)) => Err(format!(
            "{operationError}; failed to discard staged archive: {discardError}"
        )),
    }
}

fn memory_owner_key_arg_for_transfer(value: Option<&String>) -> Result<String, String> {
    value
        .map(|ownerKey| ownerKey.trim().to_string())
        .filter(|ownerKey| !ownerKey.is_empty())
        .ok_or_else(|| "owner-key is required, use character:<id> or shared:<id>".to_string())
}

fn parse_import_strategy(value: Option<&String>) -> Result<ImportStrategy, String> {
    match value
        .ok_or_else(|| {
            "usage: operit2 import memory <path> <SKIP|UPDATE|CREATE_NEW> <owner-key>".to_string()
        })?
        .as_str()
    {
        "SKIP" => Ok(ImportStrategy::SKIP),
        "UPDATE" => Ok(ImportStrategy::UPDATE),
        "CREATE_NEW" => Ok(ImportStrategy::CREATE_NEW),
        other => Err(format!(
            "invalid import strategy: {other}; expected SKIP | UPDATE | CREATE_NEW"
        )),
    }
}

fn read_text(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| error.to_string())
}

fn write_text(path: &str, content: &str) -> Result<(), String> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
    }
    fs::write(path, content).map_err(|error| error.to_string())
}

fn write_bytes(path: &str, content: &[u8]) -> Result<(), String> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
    }
    fs::write(path, content).map_err(|error| error.to_string())
}

/// Prints export command usage in the selected output format.
fn print_export_usage() {
    if cli_json_mode() {
        emit_cli_json(serde_json::json!({ "usage": "operit2 cli export <memory|chat|snapshot>" }));
        return;
    }
    println!("operit2 cli export memory <path> <owner-key>");
    println!("operit2 cli export chat <path>");
    println!("operit2 cli export snapshot <path>");
}

/// Prints import command usage in the selected output format.
fn print_import_usage() {
    if cli_json_mode() {
        emit_cli_json(serde_json::json!({ "usage": "operit2 cli import <memory|chat|snapshot|operit1-snapshot>" }));
        return;
    }
    println!("operit2 cli import memory <path> <SKIP|UPDATE|CREATE_NEW> <owner-key>");
    println!("operit2 cli import chat <path>");
    println!("operit2 cli import snapshot <path>");
    println!("operit2 cli import operit1-snapshot <snapshot-zip-path>");
}

/// Prints backup command usage in the selected output format.
fn print_backup_usage() {
    if cli_json_mode() {
        emit_cli_json(serde_json::json!({ "usage": "operit2 cli backup <create|restore|inspect|inspect-operit1-snapshot>" }));
        return;
    }
    println!("operit2 cli backup create <snapshot-zip-path>");
    println!("operit2 cli backup restore <snapshot-zip-path>");
    println!("operit2 cli backup inspect <snapshot-zip-path>");
    println!("operit2 cli backup inspect-operit1-snapshot <snapshot-zip-path>");
}
