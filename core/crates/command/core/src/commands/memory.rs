use crate::commands::util::{parseCsvList, parse_i64_arg};
use crate::output::CoreCommandOutput;
use operit_host_api::HostManager::HostManager;
use operit_host_api::RuntimeStorageHost;
use operit_model::CharacterCard::CharacterSharedMemoryMount;
use operit_model::Memory::Memory;
use operit_runtime::data::preferences::CharacterCardManager::CharacterCardManager;
use operit_runtime::data::preferences::SharedMemoryStoreManager::SharedMemoryStoreManager;
use operit_store::repository::MemoryRepository::MemoryRepository;
use operit_store::repository::UserMarkdownRepository::UserMarkdownRepository;
use operit_util::OperitPaths::{characterMemoryOwnerKey, sharedMemoryOwnerKey};
use serde_json::json;

/// Runs memory commands for character stores, shared stores, and shared mounts.
pub fn run_memory_command(
    context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_memory_usage(output);
        return Ok(());
    }

    let storageHost = context
        .runtimeStorageHost
        .ok_or_else(|| "runtime storage host is unavailable".to_string())?;
    match args[0].as_str() {
        "character" => run_character_memory_command(&storageHost, &args[1..], output),
        "shared" => run_shared_memory_command(&storageHost, &args[1..], output),
        "mount" => run_mount_command(&args[1..], output),
        "unmount" => run_unmount_command(&args[1..], output),
        _ => {
            print_memory_usage(output);
            Ok(())
        }
    }
}

/// Runs memory commands scoped to one character owner.
fn run_character_memory_command(
    storageHost: &std::sync::Arc<dyn RuntimeStorageHost>,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.len() < 2 {
        print_character_memory_usage(output);
        return Ok(());
    }
    let characterId = args[0].clone();
    CharacterCardManager::getInstance()
        .getCharacterCard(&characterId)
        .map_err(|error| error.to_string())?;
    let ownerKey = characterMemoryOwnerKey(&characterId)?;
    match args[1].as_str() {
        "user" => run_user_command(storageHost, &ownerKey, &args[2..], output),
        "item" => run_item_command(&ownerKey, &args[2..], output),
        "graph" => run_graph_command(&ownerKey, output),
        _ => {
            print_character_memory_usage(output);
            Ok(())
        }
    }
}

/// Runs memory commands scoped to shared memory stores.
fn run_shared_memory_command(
    storageHost: &std::sync::Arc<dyn RuntimeStorageHost>,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_shared_memory_usage(output);
        return Ok(());
    }
    match args[0].as_str() {
        "list" => {
            let stores = SharedMemoryStoreManager::getInstance().getAllSharedMemoryStores()?;
            output.push_stdout_line(format!("Shared memory stores: {}", stores.len()));
            for store in &stores {
                output.push_stdout_line(format!(
                    "- {} | {} | created: {} | updated: {}",
                    store.id, store.name, store.createdAt, store.updatedAt
                ));
            }
            output.setJsonStdout(serde_json::to_value(&stores).map_err(|error| error.to_string())?);
            Ok(())
        }
        "create" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory shared create <name>".to_string())?
                .clone();
            let store = SharedMemoryStoreManager::getInstance().createSharedMemoryStore(name)?;
            output.push_stdout_line(format!("Created shared memory store {}", store.id));
            output.push_stdout_line(format!("Name: {}", store.name));
            output.setJsonStdout(serde_json::to_value(&store).map_err(|error| error.to_string())?);
            Ok(())
        }
        "rename" => {
            let id = args.get(1).ok_or_else(|| {
                "usage: operit2 memory shared rename <shared-id> <name>".to_string()
            })?;
            let name = args
                .get(2)
                .ok_or_else(|| {
                    "usage: operit2 memory shared rename <shared-id> <name>".to_string()
                })?
                .clone();
            let store =
                SharedMemoryStoreManager::getInstance().renameSharedMemoryStore(id, name)?;
            output.push_stdout_line(format!("Renamed shared memory store {}", store.id));
            output.push_stdout_line(format!("Name: {}", store.name));
            output.setJsonStdout(serde_json::to_value(&store).map_err(|error| error.to_string())?);
            Ok(())
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory shared delete <shared-id>".to_string())?;
            let deleted = SharedMemoryStoreManager::getInstance().deleteSharedMemoryStore(id)?;
            let cleanedCharacters = remove_shared_memory_mount_from_all_characters(id)?;
            output.push_stdout_line(format!("Deleted shared memory store {id}: {deleted}"));
            output.push_stdout_line(format!("Characters cleaned: {cleanedCharacters}"));
            output.setJsonStdout(json!({
                "sharedId": id,
                "deleted": deleted,
                "cleanedCharacters": cleanedCharacters
            }));
            Ok(())
        }
        sharedId => {
            SharedMemoryStoreManager::getInstance()
                .getSharedMemoryStore(sharedId)
                .map_err(|error| error.to_string())?;
            let ownerKey = sharedMemoryOwnerKey(sharedId)?;
            match args.get(1).map(String::as_str) {
                Some("user") => run_user_command(storageHost, &ownerKey, &args[2..], output),
                Some("item") => run_item_command(&ownerKey, &args[2..], output),
                Some("graph") => run_graph_command(&ownerKey, output),
                _ => {
                    print_shared_memory_usage(output);
                    Ok(())
                }
            }
        }
    }
}

/// Mounts one shared memory store to one character.
fn run_mount_command(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    if args.len() < 2 {
        print_mount_usage(output);
        return Ok(());
    }
    let characterId = args[0].clone();
    let sharedId = args[1].clone();
    SharedMemoryStoreManager::getInstance()
        .getSharedMemoryStore(&sharedId)
        .map_err(|error| error.to_string())?;
    let readable = parse_named_bool(args, "--read")?;
    let writable = parse_named_bool(args, "--write")?;
    let manager = CharacterCardManager::getInstance();
    let mut card = manager
        .getCharacterCard(&characterId)
        .map_err(|error| error.to_string())?;
    card.sharedMemoryMounts
        .retain(|mount| mount.sharedMemoryId != sharedId);
    let mount = CharacterSharedMemoryMount {
        sharedMemoryId: sharedId.clone(),
        readable,
        writable,
    };
    card.sharedMemoryMounts.push(mount.clone());
    manager
        .updateCharacterCard(card)
        .map_err(|error| error.to_string())?;
    output.push_stdout_line(format!("Mounted shared memory {sharedId} on {characterId}"));
    output.push_stdout_line(format!("Readable: {readable}"));
    output.push_stdout_line(format!("Writable: {writable}"));
    output.setJsonStdout(json!({
        "characterId": characterId,
        "sharedId": sharedId,
        "mount": mount,
        "mounted": true
    }));
    Ok(())
}

/// Removes one shared memory mount from one character.
fn run_unmount_command(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    if args.len() < 2 {
        print_unmount_usage(output);
        return Ok(());
    }
    let characterId = args[0].clone();
    let sharedId = args[1].clone();
    let manager = CharacterCardManager::getInstance();
    let mut card = manager
        .getCharacterCard(&characterId)
        .map_err(|error| error.to_string())?;
    let originalLen = card.sharedMemoryMounts.len();
    card.sharedMemoryMounts
        .retain(|mount| mount.sharedMemoryId != sharedId);
    let removed = originalLen != card.sharedMemoryMounts.len();
    manager
        .updateCharacterCard(card)
        .map_err(|error| error.to_string())?;
    output.push_stdout_line(format!(
        "Unmounted shared memory {sharedId} from {characterId}: {removed}"
    ));
    output.setJsonStdout(json!({
        "characterId": characterId,
        "sharedId": sharedId,
        "unmounted": removed
    }));
    Ok(())
}

/// Runs USER.md commands for one memory owner.
fn run_user_command(
    storageHost: &std::sync::Arc<dyn RuntimeStorageHost>,
    ownerKey: &str,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_user_usage(output);
        return Ok(());
    }
    let repository = UserMarkdownRepository::new(ownerKey, storageHost.clone());
    match args[0].as_str() {
        "show" => {
            let content = repository.readUserMarkdown()?;
            output.push_stdout(content.clone());
            output.setJsonStdout(json!({
                "ownerKey": ownerKey,
                "content": content
            }));
            Ok(())
        }
        "write" => {
            let content = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory <owner> user write <content>".to_string())?
                .clone();
            repository.writeUserMarkdown(content.clone())?;
            output.push_stdout_line(format!("Updated {ownerKey}/USER.md"));
            output.setJsonStdout(json!({
                "ownerKey": ownerKey,
                "contentLength": content.len(),
                "updated": true
            }));
            Ok(())
        }
        "path" => {
            let path = repository.userMarkdownPath()?.display().to_string();
            output.push_stdout_line(format!("USER.md path: {path}"));
            output.setJsonStdout(json!({
                "ownerKey": ownerKey,
                "path": path
            }));
            Ok(())
        }
        _ => {
            print_user_usage(output);
            Ok(())
        }
    }
}

/// Runs memory item commands for one memory owner.
fn run_item_command(
    ownerKey: &str,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_item_usage(output);
        return Ok(());
    }
    match args[0].as_str() {
        "list" => {
            let memories =
                memory_repository(ownerKey).searchMemories("*", None, 0.0, None, None)?;
            output.push_stdout_line(format!("Memory items: {}", memories.len()));
            for memory in &memories {
                print_memory_item_line(memory, output);
            }
            output.setJsonStdout(json!({
                "ownerKey": ownerKey,
                "items": memories
            }));
            Ok(())
        }
        "search" => {
            let query = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory <owner> item search <query>".to_string())?;
            let memories =
                memory_repository(ownerKey).searchMemories(query, None, 0.0, None, None)?;
            output.push_stdout_line(format!("Memory search results: {}", memories.len()));
            output.push_stdout_line(format!("Query: {query}"));
            for memory in &memories {
                print_memory_item_line(memory, output);
            }
            output.setJsonStdout(json!({
                "ownerKey": ownerKey,
                "query": query,
                "items": memories
            }));
            Ok(())
        }
        "show" => {
            let title = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory <owner> item show <title>".to_string())?;
            let memory = memory_repository(ownerKey)
                .findMemoryByTitle(title)?
                .ok_or_else(|| format!("memory item not found: {title}"))?;
            print_memory_item(&memory, output);
            output.setJsonStdout(json!({
                "ownerKey": ownerKey,
                "item": memory
            }));
            Ok(())
        }
        "create" => {
            let title = args
                .get(1)
                .ok_or_else(|| {
                    "usage: operit2 memory <owner> item create <title> <content> [folder] [tags-csv]"
                        .to_string()
                })?
                .clone();
            let content = args
                .get(2)
                .ok_or_else(|| {
                    "usage: operit2 memory <owner> item create <title> <content> [folder] [tags-csv]"
                        .to_string()
                })?
                .clone();
            let folder = args.get(3).cloned().unwrap_or_default();
            let tags = args.get(4).map(|value| parseCsvList(value));
            let memory = memory_repository(ownerKey).createMemory(
                title,
                content,
                "text".to_string(),
                "cli".to_string(),
                folder,
                tags,
            )?;
            output.push_stdout_line(format!("Created memory item {}", memory.id));
            output.push_stdout_line(format!("Title: {}", memory.title));
            output.setJsonStdout(json!({
                "ownerKey": ownerKey,
                "item": memory,
                "created": true
            }));
            Ok(())
        }
        "delete" => {
            let id = parse_i64_arg(
                args.get(1),
                "usage: operit2 memory <owner> item delete <id>",
            )?;
            let deleted = memory_repository(ownerKey).deleteMemory(id)?;
            output.push_stdout_line(format!("Deleted memory item {id}: {deleted}"));
            output.setJsonStdout(json!({
                "ownerKey": ownerKey,
                "id": id,
                "deleted": deleted
            }));
            Ok(())
        }
        "move" => {
            let ids = args
                .get(1)
                .ok_or_else(|| {
                    "usage: operit2 memory <owner> item move <ids-csv> <folder>".to_string()
                })?
                .split(',')
                .map(|value| {
                    value
                        .trim()
                        .parse::<i64>()
                        .map_err(|error| error.to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let folder = args.get(2).ok_or_else(|| {
                "usage: operit2 memory <owner> item move <ids-csv> <folder>".to_string()
            })?;
            let moved = memory_repository(ownerKey).moveMemoriesToFolder(&ids, folder)?;
            output.push_stdout_line(format!("Moved memory items: {moved}"));
            output.push_stdout_line(format!("Folder: {folder}"));
            output.setJsonStdout(json!({
                "ownerKey": ownerKey,
                "ids": ids,
                "folder": folder,
                "moved": moved
            }));
            Ok(())
        }
        _ => {
            print_item_usage(output);
            Ok(())
        }
    }
}

/// Prints the memory graph for one memory owner.
fn run_graph_command(ownerKey: &str, output: &mut CoreCommandOutput) -> Result<(), String> {
    let graph = memory_repository(ownerKey).getMemoryGraph()?;
    output.push_stdout_line(format!("Memory graph for {ownerKey}"));
    output.push_stdout_line(format!("Nodes: {}", graph.nodes.len()));
    output.push_stdout_line(format!("Edges: {}", graph.edges.len()));
    output.setJsonStdout(json!({
        "ownerKey": ownerKey,
        "graph": graph
    }));
    Ok(())
}

/// Creates a memory repository for one owner key.
fn memory_repository(ownerKey: &str) -> MemoryRepository {
    MemoryRepository::new(ownerKey)
}

/// Parses a named boolean flag from command arguments.
fn parse_named_bool(args: &[String], name: &str) -> Result<bool, String> {
    let index = args
        .iter()
        .position(|value| value == name)
        .ok_or_else(|| format!("missing argument: {name}"))?;
    let raw = args
        .get(index + 1)
        .ok_or_else(|| format!("missing value for argument: {name}"))?;
    match raw.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid bool for {name}: {raw}")),
    }
}

/// Removes one shared memory id from every character card mount list.
fn remove_shared_memory_mount_from_all_characters(sharedId: &str) -> Result<usize, String> {
    let manager = CharacterCardManager::getInstance();
    let mut changedCards = 0usize;
    for mut card in manager
        .getAllCharacterCards()
        .map_err(|error| error.to_string())?
    {
        let originalLen = card.sharedMemoryMounts.len();
        card.sharedMemoryMounts
            .retain(|mount| mount.sharedMemoryId != sharedId);
        if originalLen != card.sharedMemoryMounts.len() {
            changedCards += 1;
            manager
                .updateCharacterCard(card)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(changedCards)
}

/// Prints one memory item as a compact list row.
fn print_memory_item_line(memory: &Memory, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!(
        "- {} | {} | folder: {} | tags: {}",
        memory.id,
        memory.title,
        optional_text(memory.folderPath.as_deref()),
        memory_tag_names(memory)
    ));
}

/// Prints one memory item in detail.
fn print_memory_item(memory: &Memory, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!("Memory item {}", memory.id));
    output.push_stdout_line(format!("UUID: {}", memory.uuid));
    output.push_stdout_line(format!("Title: {}", memory.title));
    output.push_stdout_line(format!("Content: {}", memory.content));
    output.push_stdout_line(format!("Content type: {}", memory.contentType));
    output.push_stdout_line(format!("Source: {}", memory.source));
    output.push_stdout_line(format!("Credibility: {}", memory.credibility));
    output.push_stdout_line(format!("Importance: {}", memory.importance));
    output.push_stdout_line(format!(
        "Folder: {}",
        optional_text(memory.folderPath.as_deref())
    ));
    output.push_stdout_line(format!("Created at: {}", memory.createdAt));
    output.push_stdout_line(format!("Updated at: {}", memory.updatedAt));
    output.push_stdout_line(format!("Last accessed at: {}", memory.lastAccessedAt));
    output.push_stdout_line(format!("Tags: {}", memory_tag_names(memory)));
}

/// Formats memory tag names for readable command output.
fn memory_tag_names(memory: &Memory) -> String {
    memory
        .tags
        .iter()
        .map(|tag| tag.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Formats optional text for readable command output.
fn optional_text(value: Option<&str>) -> &str {
    match value {
        Some(text) => text,
        None => "-",
    }
}

/// Prints top-level memory command usage.
fn print_memory_usage(output: &mut CoreCommandOutput) {
    let lines = ["operit2 memory <character|shared|mount|unmount>"];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}

/// Prints character memory command usage.
fn print_character_memory_usage(output: &mut CoreCommandOutput) {
    let lines = [
        "operit2 memory character <character-id> user <show|write|path>",
        "operit2 memory character <character-id> item <list|search|show|create|delete|move>",
        "operit2 memory character <character-id> graph",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}

/// Prints shared memory command usage.
fn print_shared_memory_usage(output: &mut CoreCommandOutput) {
    let lines = [
        "operit2 memory shared <list|create|rename|delete>",
        "operit2 memory shared <shared-id> user <show|write|path>",
        "operit2 memory shared <shared-id> item <list|search|show|create|delete|move>",
        "operit2 memory shared <shared-id> graph",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}

/// Prints shared memory mount command usage.
fn print_mount_usage(output: &mut CoreCommandOutput) {
    let lines = [
        "operit2 memory mount <character-id> <shared-id> --read <true|false> --write <true|false>",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}

/// Prints shared memory unmount command usage.
fn print_unmount_usage(output: &mut CoreCommandOutput) {
    let lines = ["operit2 memory unmount <character-id> <shared-id>"];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}

/// Prints USER.md command usage.
fn print_user_usage(output: &mut CoreCommandOutput) {
    let lines = [
        "operit2 memory <owner> user show",
        "operit2 memory <owner> user write <content>",
        "operit2 memory <owner> user path",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}

/// Prints memory item command usage.
fn print_item_usage(output: &mut CoreCommandOutput) {
    let lines = [
        "operit2 memory <owner> item list",
        "operit2 memory <owner> item search <query>",
        "operit2 memory <owner> item show <title>",
        "operit2 memory <owner> item create <title> <content> [folder] [tags-csv]",
        "operit2 memory <owner> item delete <id>",
        "operit2 memory <owner> item move <ids-csv> <folder>",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}
