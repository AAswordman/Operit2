use crate::commands::util::{parseCsvList, parse_bool_arg, parse_i64_arg};
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::data::model::Memory::Memory;
use operit_runtime::data::model::PreferenceProfile::PreferenceProfile;
use operit_runtime::data::preferences::UserPreferencesManager::UserPreferencesManager;
use operit_runtime::data::repository::MemoryRepository::MemoryRepository;

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
        "profile" => run_memory_profile_command(&args[1..], output),
        "kv" => run_memory_kv_command(&args[1..], output),
        "item" => run_memory_item_command(&args[1..], output),
        _ => {
            print_memory_usage(output);
            Ok(())
        }
    }
}

fn run_memory_profile_command(
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_memory_profile_usage(output);
        return Ok(());
    }
    let manager = user_preferences_manager()?;
    match args[0].as_str() {
        "list" => {
            let activeProfileId = manager
                .activeProfileId()
                .map_err(|error| error.to_string())?;
            for profileId in manager
                .profileListFlow()
                .first()
                .map_err(|error| error.to_string())?
            {
                let marker = if profileId == activeProfileId {
                    "*"
                } else {
                    " "
                };
                let profile = manager
                    .getProfile(&profileId)
                    .map_err(|error| error.to_string())?;
                output.push_stdout_line(format!("{marker}\t{}\t{}", profile.id, profile.name));
            }
            Ok(())
        }
        "active" => {
            output.push_stdout_line(
                manager
                    .activeProfileId()
                    .map_err(|error| error.to_string())?,
            );
            Ok(())
        }
        "show" => {
            let profileId = memory_profile_arg(args.get(1), &manager)?;
            let profile = manager
                .getProfile(&profileId)
                .map_err(|error| error.to_string())?;
            print_memory_profile(&profile, output);
            Ok(())
        }
        "create" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory profile create <name>".to_string())?
                .clone();
            let profile = manager
                .createProfile(name, false)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("created={}", profile.id));
            Ok(())
        }
        "switch" => {
            let profileId = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory profile switch <profile-id>".to_string())?
                .clone();
            manager
                .setActiveProfile(profileId.clone())
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("active={profileId}"));
            Ok(())
        }
        "lock" => {
            let category = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory profile lock <birthDate|gender|personality|identity|occupation|aiStyle> <true|false>".to_string())?;
            let locked = parse_bool_arg(
                args.get(2),
                "usage: operit2 memory profile lock <birthDate|gender|personality|identity|occupation|aiStyle> <true|false>",
            )?;
            manager
                .setCategoryLocked(category, locked)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("{category}={locked}"));
            Ok(())
        }
        _ => {
            print_memory_profile_usage(output);
            Ok(())
        }
    }
}

fn run_memory_kv_command(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    if args.is_empty() {
        print_memory_kv_usage(output);
        return Ok(());
    }
    let manager = user_preferences_manager()?;
    match args[0].as_str() {
        "show" => {
            let profileId = memory_profile_arg(args.get(1), &manager)?;
            let profile = manager
                .getProfile(&profileId)
                .map_err(|error| error.to_string())?;
            print_memory_kv(&profile, output);
            Ok(())
        }
        "set" => {
            let key = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory kv set <birthDate|gender|personality|identity|occupation|aiStyle> <value> [profile-id]".to_string())?;
            let value = args
                .get(2)
                .ok_or_else(|| "usage: operit2 memory kv set <birthDate|gender|personality|identity|occupation|aiStyle> <value> [profile-id]".to_string())?
                .clone();
            let profileId = memory_profile_arg(args.get(3), &manager)?;
            let birthDate = if key == "birthDate" {
                Some(
                    value
                        .parse::<i64>()
                        .map_err(|error| format!("invalid birthDate millis: {error}"))?,
                )
            } else {
                None
            };
            manager
                .updateProfileCategory(
                    profileId.clone(),
                    birthDate,
                    string_memory_kv_value(key, "gender", &value)?,
                    string_memory_kv_value(key, "personality", &value)?,
                    string_memory_kv_value(key, "identity", &value)?,
                    string_memory_kv_value(key, "occupation", &value)?,
                    string_memory_kv_value(key, "aiStyle", &value)?,
                )
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("{profileId}.{key}={value}"));
            Ok(())
        }
        _ => {
            print_memory_kv_usage(output);
            Ok(())
        }
    }
}

fn run_memory_item_command(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    if args.is_empty() {
        print_memory_item_usage(output);
        return Ok(());
    }
    let manager = user_preferences_manager()?;
    match args[0].as_str() {
        "list" => {
            let profileId = memory_profile_arg(args.get(1), &manager)?;
            for memory in
                memory_repository(&profileId).searchMemories("*", None, 0.0, None, None)?
            {
                print_memory_item_line(&memory, output);
            }
            Ok(())
        }
        "search" => {
            let query = args.get(1).ok_or_else(|| {
                "usage: operit2 memory item search <query> [profile-id]".to_string()
            })?;
            let profileId = memory_profile_arg(args.get(2), &manager)?;
            for memory in
                memory_repository(&profileId).searchMemories(query, None, 0.0, None, None)?
            {
                print_memory_item_line(&memory, output);
            }
            Ok(())
        }
        "show" => {
            let title = args.get(1).ok_or_else(|| {
                "usage: operit2 memory item show <title> [profile-id]".to_string()
            })?;
            let profileId = memory_profile_arg(args.get(2), &manager)?;
            let memory = memory_repository(&profileId)
                .findMemoryByTitle(title)?
                .ok_or_else(|| format!("memory item not found: {title}"))?;
            print_memory_item(&memory, output);
            Ok(())
        }
        "create" => {
            let title = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory item create <title> <content> [folder] [tags-csv] [profile-id]".to_string())?
                .clone();
            let content = args
                .get(2)
                .ok_or_else(|| "usage: operit2 memory item create <title> <content> [folder] [tags-csv] [profile-id]".to_string())?
                .clone();
            let folder = match args.get(3) {
                Some(value) => value.clone(),
                None => String::new(),
            };
            let tags = args.get(4).map(|value| parseCsvList(value));
            let profileId = memory_profile_arg(args.get(5), &manager)?;
            let memory = memory_repository(&profileId).createMemory(
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
                "usage: operit2 memory item delete <id> [profile-id]",
            )?;
            let profileId = memory_profile_arg(args.get(2), &manager)?;
            output.push_stdout_line(format!(
                "deleted={}",
                memory_repository(&profileId).deleteMemory(id)?
            ));
            Ok(())
        }
        "move" => {
            let ids = args
                .get(1)
                .ok_or_else(|| {
                    "usage: operit2 memory item move <ids-csv> <folder> [profile-id]".to_string()
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
                "usage: operit2 memory item move <ids-csv> <folder> [profile-id]".to_string()
            })?;
            let profileId = memory_profile_arg(args.get(3), &manager)?;
            output.push_stdout_line(format!(
                "moved={}",
                memory_repository(&profileId).moveMemoriesToFolder(&ids, folder)?
            ));
            Ok(())
        }
        _ => {
            print_memory_item_usage(output);
            Ok(())
        }
    }
}

fn user_preferences_manager() -> Result<UserPreferencesManager, String> {
    let manager = UserPreferencesManager::getInstance();
    manager
        .initializeIfNeeded("Default")
        .map_err(|error| error.to_string())?;
    Ok(manager)
}

fn memory_repository(profileId: &str) -> MemoryRepository {
    MemoryRepository::new(profileId)
}

fn memory_profile_arg(
    value: Option<&String>,
    manager: &UserPreferencesManager,
) -> Result<String, String> {
    match value {
        Some(profileId) => Ok(profileId.clone()),
        None => manager.activeProfileId().map_err(|error| error.to_string()),
    }
}

fn string_memory_kv_value(key: &str, target: &str, value: &str) -> Result<Option<String>, String> {
    match key {
        "birthDate" => Ok(None),
        "gender" | "personality" | "identity" | "occupation" | "aiStyle" => {
            if key == target {
                Ok(Some(value.to_string()))
            } else {
                Ok(None)
            }
        }
        other => Err(format!("invalid memory kv key: {other}")),
    }
}

fn print_memory_profile(profile: &PreferenceProfile, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!("id={}", profile.id));
    output.push_stdout_line(format!("name={}", profile.name));
    output.push_stdout_line(format!("birthDate={}", profile.birthDate));
    output.push_stdout_line(format!("gender={}", profile.gender));
    output.push_stdout_line(format!("personality={}", profile.personality));
    output.push_stdout_line(format!("identity={}", profile.identity));
    output.push_stdout_line(format!("occupation={}", profile.occupation));
    output.push_stdout_line(format!("aiStyle={}", profile.aiStyle));
    output.push_stdout_line(format!("isInitialized={}", profile.isInitialized));
}

fn print_memory_kv(profile: &PreferenceProfile, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!("birthDate={}", profile.birthDate));
    output.push_stdout_line(format!("gender={}", profile.gender));
    output.push_stdout_line(format!("personality={}", profile.personality));
    output.push_stdout_line(format!("identity={}", profile.identity));
    output.push_stdout_line(format!("occupation={}", profile.occupation));
    output.push_stdout_line(format!("aiStyle={}", profile.aiStyle));
}

fn print_memory_item_line(memory: &Memory, output: &mut CoreCommandOutput) {
    let folderPath = match memory.folderPath.clone() {
        Some(value) => value,
        None => String::new(),
    };
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
    let folderPath = match memory.folderPath.clone() {
        Some(value) => value,
        None => String::new(),
    };
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
    output.push_stdout_line("operit2 memory <profile|kv|item>");
}

fn print_memory_profile_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 memory profile list");
    output.push_stdout_line("operit2 memory profile active");
    output.push_stdout_line("operit2 memory profile show [profile-id]");
    output.push_stdout_line("operit2 memory profile create <name>");
    output.push_stdout_line("operit2 memory profile switch <profile-id>");
    output.push_stdout_line("operit2 memory profile lock <birthDate|gender|personality|identity|occupation|aiStyle> <true|false>");
}

fn print_memory_kv_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 memory kv show [profile-id]");
    output.push_stdout_line("operit2 memory kv set <birthDate|gender|personality|identity|occupation|aiStyle> <value> [profile-id]");
}

fn print_memory_item_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 memory item list [profile-id]");
    output.push_stdout_line("operit2 memory item search <query> [profile-id]");
    output.push_stdout_line("operit2 memory item show <title> [profile-id]");
    output.push_stdout_line(
        "operit2 memory item create <title> <content> [folder] [tags-csv] [profile-id]",
    );
    output.push_stdout_line("operit2 memory item delete <id> [profile-id]");
    output.push_stdout_line("operit2 memory item move <ids-csv> <folder> [profile-id]");
}
