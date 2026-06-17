use crate::commands::util::{parseCsvList, parse_i64_arg};
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::data::model::CharacterCard::CharacterSharedMemoryMount;
use operit_runtime::data::model::Memory::Memory;
use operit_runtime::data::preferences::CharacterCardManager::CharacterCardManager;
use operit_runtime::data::preferences::SharedMemoryStoreManager::SharedMemoryStoreManager;
use operit_runtime::data::repository::MemoryRepository::MemoryRepository;
use operit_runtime::data::repository::UserMarkdownRepository::UserMarkdownRepository;
use operit_runtime::util::OperitPaths::{characterMemoryOwnerKey, sharedMemoryOwnerKey};

pub fn run_memory_command(
    _context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_memory_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "character" => run_character_memory_command(&args[1..], output),
        "shared" => run_shared_memory_command(&args[1..], output),
        "mount" => run_mount_command(&args[1..], output),
        "unmount" => run_unmount_command(&args[1..], output),
        _ => {
            print_memory_usage(output);
            Ok(())
        }
    }
}

fn run_character_memory_command(
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
        "user" => run_user_command(&ownerKey, &args[2..], output),
        "item" => run_item_command(&ownerKey, &args[2..], output),
        "graph" => run_graph_command(&ownerKey, output),
        _ => {
            print_character_memory_usage(output);
            Ok(())
        }
    }
}

fn run_shared_memory_command(
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_shared_memory_usage(output);
        return Ok(());
    }
    match args[0].as_str() {
        "list" => {
            for store in SharedMemoryStoreManager::getInstance().getAllSharedMemoryStores()? {
                output.push_stdout_line(format!(
                    "{}\t{}\t{}\t{}",
                    store.id, store.name, store.createdAt, store.updatedAt
                ));
            }
            Ok(())
        }
        "create" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory shared create <name>".to_string())?
                .clone();
            let store = SharedMemoryStoreManager::getInstance().createSharedMemoryStore(name)?;
            output.push_stdout_line(format!("created={}", store.id));
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
            output.push_stdout_line(format!("renamed={}", store.id));
            Ok(())
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory shared delete <shared-id>".to_string())?;
            let deleted = SharedMemoryStoreManager::getInstance().deleteSharedMemoryStore(id)?;
            remove_shared_memory_mount_from_all_characters(id)?;
            output.push_stdout_line(format!("deleted={deleted}"));
            Ok(())
        }
        sharedId => {
            SharedMemoryStoreManager::getInstance()
                .getSharedMemoryStore(sharedId)
                .map_err(|error| error.to_string())?;
            let ownerKey = sharedMemoryOwnerKey(sharedId)?;
            match args.get(1).map(String::as_str) {
                Some("user") => run_user_command(&ownerKey, &args[2..], output),
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

fn run_mount_command(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    if args.len() < 2 {
        output.push_stdout_line(
            "operit2 memory mount <character-id> <shared-id> --read <true|false> --write <true|false>",
        );
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
    card.sharedMemoryMounts.push(CharacterSharedMemoryMount {
        sharedMemoryId: sharedId.clone(),
        readable,
        writable,
    });
    manager
        .updateCharacterCard(card)
        .map_err(|error| error.to_string())?;
    output.push_stdout_line(format!(
        "mounted={characterId}:{sharedId}:read={readable}:write={writable}"
    ));
    Ok(())
}

fn run_unmount_command(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    if args.len() < 2 {
        output.push_stdout_line("operit2 memory unmount <character-id> <shared-id>");
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
    manager
        .updateCharacterCard(card)
        .map_err(|error| error.to_string())?;
    output.push_stdout_line(format!(
        "unmounted={}",
        originalLen
            != CharacterCardManager::getInstance()
                .getCharacterCard(&characterId)
                .map_err(|error| error.to_string())?
                .sharedMemoryMounts
                .len()
    ));
    Ok(())
}

fn run_user_command(
    ownerKey: &str,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_user_usage(output);
        return Ok(());
    }
    let repository = UserMarkdownRepository::new(ownerKey);
    match args[0].as_str() {
        "show" => {
            output.push_stdout(repository.readUserMarkdown()?);
            Ok(())
        }
        "write" => {
            let content = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory <owner> user write <content>".to_string())?
                .clone();
            repository.writeUserMarkdown(content)?;
            output.push_stdout_line(format!("updated={ownerKey}/USER.md"));
            Ok(())
        }
        "path" => {
            output.push_stdout_line(repository.userMarkdownPath()?.display().to_string());
            Ok(())
        }
        _ => {
            print_user_usage(output);
            Ok(())
        }
    }
}

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
            for memory in memory_repository(ownerKey).searchMemories("*", None, 0.0, None, None)? {
                print_memory_item_line(&memory, output);
            }
            Ok(())
        }
        "search" => {
            let query = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory <owner> item search <query>".to_string())?;
            for memory in
                memory_repository(ownerKey).searchMemories(query, None, 0.0, None, None)?
            {
                print_memory_item_line(&memory, output);
            }
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
            output.push_stdout_line(format!("created={}", memory.id));
            Ok(())
        }
        "delete" => {
            let id = parse_i64_arg(
                args.get(1),
                "usage: operit2 memory <owner> item delete <id>",
            )?;
            output.push_stdout_line(format!(
                "deleted={}",
                memory_repository(ownerKey).deleteMemory(id)?
            ));
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
            output.push_stdout_line(format!(
                "moved={}",
                memory_repository(ownerKey).moveMemoriesToFolder(&ids, folder)?
            ));
            Ok(())
        }
        _ => {
            print_item_usage(output);
            Ok(())
        }
    }
}

fn run_graph_command(ownerKey: &str, output: &mut CoreCommandOutput) -> Result<(), String> {
    let graph = memory_repository(ownerKey).getMemoryGraph()?;
    output
        .push_stdout_line(serde_json::to_string_pretty(&graph).map_err(|error| error.to_string())?);
    Ok(())
}

fn memory_repository(ownerKey: &str) -> MemoryRepository {
    MemoryRepository::new(ownerKey)
}

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

fn remove_shared_memory_mount_from_all_characters(sharedId: &str) -> Result<(), String> {
    let manager = CharacterCardManager::getInstance();
    for mut card in manager
        .getAllCharacterCards()
        .map_err(|error| error.to_string())?
    {
        let originalLen = card.sharedMemoryMounts.len();
        card.sharedMemoryMounts
            .retain(|mount| mount.sharedMemoryId != sharedId);
        if originalLen != card.sharedMemoryMounts.len() {
            manager
                .updateCharacterCard(card)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn print_memory_item_line(memory: &Memory, output: &mut CoreCommandOutput) {
    let folderPath = memory.folderPath.clone().unwrap_or_default();
    output.push_stdout_line(format!(
        "{}\t{}\t{}\t{}",
        memory.id,
        memory.title,
        folderPath,
        memory
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>()
            .join(",")
    ));
}

fn print_memory_item(memory: &Memory, output: &mut CoreCommandOutput) {
    let folderPath = memory.folderPath.clone().unwrap_or_default();
    output.push_stdout_line(format!("id={}", memory.id));
    output.push_stdout_line(format!("uuid={}", memory.uuid));
    output.push_stdout_line(format!("title={}", memory.title));
    output.push_stdout_line(format!("content={}", memory.content));
    output.push_stdout_line(format!("contentType={}", memory.contentType));
    output.push_stdout_line(format!("source={}", memory.source));
    output.push_stdout_line(format!("credibility={}", memory.credibility));
    output.push_stdout_line(format!("importance={}", memory.importance));
    output.push_stdout_line(format!("folderPath={folderPath}"));
    output.push_stdout_line(format!("createdAt={}", memory.createdAt));
    output.push_stdout_line(format!("updatedAt={}", memory.updatedAt));
    output.push_stdout_line(format!("lastAccessedAt={}", memory.lastAccessedAt));
    output.push_stdout_line(format!(
        "tags={}",
        memory
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>()
            .join(",")
    ));
}

fn print_memory_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 memory <character|shared|mount|unmount>");
}

fn print_character_memory_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 memory character <character-id> user <show|write|path>");
    output.push_stdout_line(
        "operit2 memory character <character-id> item <list|search|show|create|delete|move>",
    );
    output.push_stdout_line("operit2 memory character <character-id> graph");
}

fn print_shared_memory_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 memory shared <list|create|rename|delete>");
    output.push_stdout_line("operit2 memory shared <shared-id> user <show|write|path>");
    output.push_stdout_line(
        "operit2 memory shared <shared-id> item <list|search|show|create|delete|move>",
    );
    output.push_stdout_line("operit2 memory shared <shared-id> graph");
}

fn print_user_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 memory <owner> user show");
    output.push_stdout_line("operit2 memory <owner> user write <content>");
    output.push_stdout_line("operit2 memory <owner> user path");
}

fn print_item_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 memory <owner> item list");
    output.push_stdout_line("operit2 memory <owner> item search <query>");
    output.push_stdout_line("operit2 memory <owner> item show <title>");
    output.push_stdout_line(
        "operit2 memory <owner> item create <title> <content> [folder] [tags-csv]",
    );
    output.push_stdout_line("operit2 memory <owner> item delete <id>");
    output.push_stdout_line("operit2 memory <owner> item move <ids-csv> <folder>");
}
