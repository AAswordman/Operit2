use super::core::CliCore;
use super::*;

async fn memory_profile_arg_with_core(
    core: &mut CliCore,
    value: Option<&String>,
) -> Result<String, String> {
    match value {
        Some(profileId) => Ok(profileId.clone()),
        None => core
            .preferences_user_preferences_manager()
            .activeProfileId()
            .await
            .map_err(|error| error.to_string()),
    }
}

pub(super) async fn run_memory_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_memory_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "profile" => run_memory_profile_command(core, &args[1..]).await,
        "kv" => run_memory_kv_command(core, &args[1..]).await,
        "item" => run_memory_item_command(core, &args[1..]).await,
        _ => {
            print_memory_usage();
            Ok(())
        }
    }
}

async fn run_memory_profile_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_memory_profile_usage();
        return Ok(());
    }
    core.preferences_user_preferences_manager()
        .initializeIfNeeded("Default")
        .await
        .map_err(|error| error.to_string())?;
    match args[0].as_str() {
        "list" => {
            let activeProfileId = core
                .preferences_user_preferences_manager()
                .activeProfileId()
                .await
                .map_err(|error| error.to_string())?;
            for profileId in core
                .preferences_user_preferences_manager()
                .profileList()
                .await
                .map_err(|error| error.to_string())?
            {
                let marker = if profileId == activeProfileId {
                    "*"
                } else {
                    " "
                };
                let profile = core
                    .preferences_user_preferences_manager()
                    .getProfile(&profileId)
                    .await
                    .map_err(|error| error.to_string())?;
                println!("{marker}\t{}\t{}", profile.id, profile.name);
            }
            Ok(())
        }
        "active" => {
            println!(
                "{}",
                core.preferences_user_preferences_manager()
                    .activeProfileId()
                    .await
                    .map_err(|error| error.to_string())?
            );
            Ok(())
        }
        "show" => {
            let profileId = memory_profile_arg_with_core(core, args.get(1)).await?;
            let profile = core
                .preferences_user_preferences_manager()
                .getProfile(&profileId)
                .await
                .map_err(|error| error.to_string())?;
            print_memory_profile(&profile);
            Ok(())
        }
        "create" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory profile create <name>".to_string())?
                .clone();
            let profile = core
                .preferences_user_preferences_manager()
                .createProfile(name, false)
                .await
                .map_err(|error| error.to_string())?;
            println!("created={}", profile.id);
            Ok(())
        }
        "switch" => {
            let profileId = args
                .get(1)
                .ok_or_else(|| "usage: operit2 memory profile switch <profile-id>".to_string())?
                .clone();
            core.preferences_user_preferences_manager()
                .setActiveProfile(profileId.clone())
                .await
                .map_err(|error| error.to_string())?;
            println!("active={profileId}");
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
            core.preferences_user_preferences_manager()
                .setCategoryLocked(category, locked)
                .await
                .map_err(|error| error.to_string())?;
            println!("{category}={locked}");
            Ok(())
        }
        _ => {
            print_memory_profile_usage();
            Ok(())
        }
    }
}

async fn run_memory_kv_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_memory_kv_usage();
        return Ok(());
    }
    core.preferences_user_preferences_manager()
        .initializeIfNeeded("Default")
        .await
        .map_err(|error| error.to_string())?;
    match args[0].as_str() {
        "show" => {
            let profileId = memory_profile_arg_with_core(core, args.get(1)).await?;
            let profile = core
                .preferences_user_preferences_manager()
                .getProfile(&profileId)
                .await
                .map_err(|error| error.to_string())?;
            print_memory_kv(&profile);
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
            let profileId = memory_profile_arg_with_core(core, args.get(3)).await?;
            let birthDate = if key == "birthDate" {
                Some(
                    value
                        .parse::<i64>()
                        .map_err(|error| format!("invalid birthDate millis: {error}"))?,
                )
            } else {
                None
            };
            core.preferences_user_preferences_manager()
                .updateProfileCategory(
                    profileId.clone(),
                    birthDate,
                    string_memory_kv_value(key, "gender", &value)?,
                    string_memory_kv_value(key, "personality", &value)?,
                    string_memory_kv_value(key, "identity", &value)?,
                    string_memory_kv_value(key, "occupation", &value)?,
                    string_memory_kv_value(key, "aiStyle", &value)?,
                )
                .await
                .map_err(|error| error.to_string())?;
            println!("{profileId}.{key}={value}");
            Ok(())
        }
        _ => {
            print_memory_kv_usage();
            Ok(())
        }
    }
}

async fn run_memory_item_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_memory_item_usage();
        return Ok(());
    }
    core.preferences_user_preferences_manager()
        .initializeIfNeeded("Default")
        .await
        .map_err(|error| error.to_string())?;
    match args[0].as_str() {
        "list" => {
            let profileId = memory_profile_arg_with_core(core, args.get(1)).await?;
            for memory in core
                .repository_memory_repository(&profileId)
                .searchMemories("*", None, 0.0, None, None)
                .await
                .map_err(|error| error.to_string())?
            {
                print_memory_item_line(&memory);
            }
            Ok(())
        }
        "search" => {
            let query = args.get(1).ok_or_else(|| {
                "usage: operit2 memory item search <query> [profile-id]".to_string()
            })?;
            let profileId = memory_profile_arg_with_core(core, args.get(2)).await?;
            for memory in core
                .repository_memory_repository(&profileId)
                .searchMemories(query, None, 0.0, None, None)
                .await
                .map_err(|error| error.to_string())?
            {
                print_memory_item_line(&memory);
            }
            Ok(())
        }
        "show" => {
            let title = args.get(1).ok_or_else(|| {
                "usage: operit2 memory item show <title> [profile-id]".to_string()
            })?;
            let profileId = memory_profile_arg_with_core(core, args.get(2)).await?;
            let memory = core
                .repository_memory_repository(&profileId)
                .findMemoryByTitle(title)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("memory item not found: {title}"))?;
            print_memory_item(&memory);
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
            let folder = args.get(3).cloned().unwrap_or_else(String::new);
            let tags = args.get(4).map(|value| parseCsvList(value));
            let profileId = memory_profile_arg_with_core(core, args.get(5)).await?;
            let memory = core
                .repository_memory_repository(&profileId)
                .createMemory(
                    title,
                    content,
                    "text".to_string(),
                    "cli".to_string(),
                    folder,
                    tags,
                )
                .await
                .map_err(|error| error.to_string())?;
            println!("created={}", memory.id);
            Ok(())
        }
        "delete" => {
            let id = parse_i64_arg(
                args.get(1),
                "usage: operit2 memory item delete <id> [profile-id]",
            )?;
            let profileId = memory_profile_arg_with_core(core, args.get(2)).await?;
            println!(
                "deleted={}",
                core.repository_memory_repository(&profileId)
                    .deleteMemory(id)
                    .await
                    .map_err(|error| error.to_string())?
            );
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
            let profileId = memory_profile_arg_with_core(core, args.get(3)).await?;
            println!(
                "moved={}",
                core.repository_memory_repository(&profileId)
                    .moveMemoriesToFolder(&ids, folder)
                    .await
                    .map_err(|error| error.to_string())?
            );
            Ok(())
        }
        _ => {
            print_memory_item_usage();
            Ok(())
        }
    }
}
