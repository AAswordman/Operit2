use super::core::CliCore;
use super::*;

pub(super) async fn run_character_command(
    core: &mut CliCore,
    args: &[String],
) -> Result<(), String> {
    if args.is_empty() {
        print_character_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "init" => {
            core.preferences_character_card_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            println!("initialized");
        }
        "list" => {
            core.preferences_character_card_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            for card in core
                .preferences_character_card_manager()
                .getAllCharacterCards()
                .await
                .map_err(|error| error.to_string())?
            {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    card.id,
                    card.name,
                    card.isDefault,
                    card.attachedTagIds.join(","),
                    card.description
                );
            }
        }
        "show" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character show <id>".to_string())?;
            let card = core
                .preferences_character_card_manager()
                .getCharacterCard(id)
                .await
                .map_err(|error| error.to_string())?;
            print_character_card(&card);
        }
        "create" => {
            let name = args
                .get(1)
                .ok_or_else(|| {
                    "usage: operit2 character create <name> [character-setting]".to_string()
                })?
                .clone();
            let characterSetting = args.get(2).cloned().unwrap_or_default();
            let now = currentTimeMillis();
            let id = core
                .preferences_character_card_manager()
                .createCharacterCard(CharacterCard {
                    id: String::new(),
                    name,
                    description: String::new(),
                    characterSetting,
                    openingStatement: String::new(),
                    otherContentChat: String::new(),
                    otherContentVoice: String::new(),
                    attachedTagIds: Vec::new(),
                    advancedCustomPrompt: String::new(),
                    marks: String::new(),
                    chatModelBindingMode: CharacterCardChatModelBindingMode::FOLLOW_GLOBAL
                        .to_string(),
                    chatModelConfigId: None,
                    chatModelIndex: 0,
                    memoryProfileBindingMode: CharacterCardMemoryProfileBindingMode::FOLLOW_GLOBAL
                        .to_string(),
                    memoryProfileId: None,
                    toolAccessConfig: CharacterCardToolAccessConfig::default(),
                    isDefault: false,
                    createdAt: now,
                    updatedAt: now,
                })
                .await
                .map_err(|error| error.to_string())?;
            println!("{id}");
        }
        "update" => {
            let id = args.get(1).ok_or_else(|| {
                "usage: operit2 character update <id> <field> <value>".to_string()
            })?;
            let field = args.get(2).ok_or_else(|| {
                "usage: operit2 character update <id> <field> <value>".to_string()
            })?;
            let value = args
                .get(3)
                .ok_or_else(|| "usage: operit2 character update <id> <field> <value>".to_string())?
                .clone();
            let mut card = core
                .preferences_character_card_manager()
                .getCharacterCard(id)
                .await
                .map_err(|error| error.to_string())?;
            match field.as_str() {
                "name" => card.name = value,
                "description" => card.description = value,
                "characterSetting" => card.characterSetting = value,
                "openingStatement" => card.openingStatement = value,
                "otherContentChat" => card.otherContentChat = value,
                "otherContentVoice" => card.otherContentVoice = value,
                "advancedCustomPrompt" => card.advancedCustomPrompt = value,
                "marks" => card.marks = value,
                "attachedTagIds" => card.attachedTagIds = parseCsvList(&value),
                "chatModelBindingMode" => card.chatModelBindingMode = CharacterCardChatModelBindingMode::normalize(Some(&value)),
                "chatModelConfigId" => card.chatModelConfigId = nonBlankString(value),
                "chatModelIndex" => {
                    card.chatModelIndex = value
                        .parse::<i32>()
                        .map_err(|error| format!("invalid chatModelIndex: {error}"))?
                        .max(0)
                }
                "memoryProfileBindingMode" => {
                    card.memoryProfileBindingMode = CharacterCardMemoryProfileBindingMode::normalize(Some(&value))
                }
                "memoryProfileId" => card.memoryProfileId = nonBlankString(value),
                _ => {
                    return Err("character fields: name | description | characterSetting | openingStatement | otherContentChat | otherContentVoice | attachedTagIds | advancedCustomPrompt | marks | chatModelBindingMode | chatModelConfigId | chatModelIndex | memoryProfileBindingMode | memoryProfileId".to_string())
                }
            }
            core.preferences_character_card_manager()
                .updateCharacterCard(card)
                .await
                .map_err(|error| error.to_string())?;
            println!("updated: {id}");
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character delete <id>".to_string())?;
            core.preferences_character_card_manager()
                .deleteCharacterCard(id)
                .await
                .map_err(|error| error.to_string())?;
            println!("deleted: {id}");
        }
        "set-active" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character set-active <id>".to_string())?;
            core.preferences_active_prompt_manager()
                .setActivePrompt(ActivePrompt::CharacterCard { id: id.clone() })
                .await
                .map_err(|error| error.to_string())?;
            println!("active character: {id}");
        }
        "combine" => {
            let id = args.get(1).ok_or_else(|| {
                "usage: operit2 character combine <id> [CHAT|VOICE] [tag-id-csv]".to_string()
            })?;
            let promptFunctionType = parsePromptFunctionType(args.get(2).map(String::as_str))?;
            let additionalTagIds = args
                .get(3)
                .map(|value| parseCsvList(value))
                .unwrap_or_default();
            let prompt = core
                .preferences_character_card_manager()
                .combinePrompts(id, additionalTagIds, promptFunctionType)
                .await
                .map_err(|error| error.to_string())?;
            println!("{prompt}");
        }
        "reset-default" => {
            core.preferences_character_card_manager()
                .resetDefaultCharacterCard()
                .await
                .map_err(|error| error.to_string())?;
            println!("default character reset");
        }
        _ => print_character_usage(),
    }
    Ok(())
}

pub(super) async fn run_group_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_group_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "init" => {
            core.preferences_character_group_card_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            println!("initialized");
        }
        "list" => {
            core.preferences_character_group_card_manager()
                .initializeIfNeeded()
                .await
                .map_err(|error| error.to_string())?;
            for group in core
                .preferences_character_group_card_manager()
                .getAllCharacterGroupCards()
                .await
                .map_err(|error| error.to_string())?
            {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    group.id,
                    group.name,
                    group.description,
                    group
                        .members
                        .iter()
                        .map(|member| format!("{}:{}", member.characterCardId, member.orderIndex))
                        .collect::<Vec<_>>()
                        .join(","),
                    group.createdAt
                );
            }
        }
        "show" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group show <id>".to_string())?;
            let group = core
                .preferences_character_group_card_manager()
                .getCharacterGroupCard(id)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("group not found: {id}"))?;
            print_character_group_card(&group);
        }
        "create" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group create <name> [description]".to_string())?
                .clone();
            let description = args.get(2).cloned().unwrap_or_default();
            let id = core
                .preferences_character_group_card_manager()
                .createCharacterGroupCard(CharacterGroupCard {
                    id: String::new(),
                    name,
                    description,
                    members: Vec::new(),
                    createdAt: currentTimeMillis(),
                    updatedAt: currentTimeMillis(),
                })
                .await
                .map_err(|error| error.to_string())?;
            println!("{id}");
        }
        "update" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group update <id> <field> <value>".to_string())?;
            let field = args
                .get(2)
                .ok_or_else(|| "usage: operit2 group update <id> <field> <value>".to_string())?;
            let value = args
                .get(3)
                .ok_or_else(|| "usage: operit2 group update <id> <field> <value>".to_string())?
                .clone();
            let mut group = core
                .preferences_character_group_card_manager()
                .getCharacterGroupCard(id)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("group not found: {id}"))?;
            match field.as_str() {
                "name" => group.name = value,
                "description" => group.description = value,
                "members" => group.members = parse_group_members(&value),
                _ => return Err("group fields: name | description | members".to_string()),
            }
            group.updatedAt = currentTimeMillis();
            core.preferences_character_group_card_manager()
                .updateCharacterGroupCard(group)
                .await
                .map_err(|error| error.to_string())?;
            println!("updated: {id}");
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group delete <id>".to_string())?;
            core.preferences_character_group_card_manager()
                .deleteCharacterGroupCard(id)
                .await
                .map_err(|error| error.to_string())?;
            println!("deleted: {id}");
        }
        "set-active" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group set-active <id>".to_string())?;
            core.preferences_active_prompt_manager()
                .setActivePrompt(ActivePrompt::CharacterGroup { id: id.clone() })
                .await
                .map_err(|error| error.to_string())?;
            println!("active group: {id}");
        }
        "duplicate" => {
            let id = args.get(1).ok_or_else(|| {
                "usage: operit2 group duplicate <source-id> [new-name]".to_string()
            })?;
            let newName = args.get(2).cloned();
            let newId = core
                .preferences_character_group_card_manager()
                .duplicateCharacterGroupCard(id, newName)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("group not found: {id}"))?;
            println!("{newId}");
        }
        _ => print_group_usage(),
    }
    Ok(())
}

pub(super) async fn run_active_prompt_command(
    core: &mut CliCore,
    args: &[String],
) -> Result<(), String> {
    if args.is_empty() {
        print_active_prompt_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "show" => {
            match core
                .preferences_active_prompt_manager()
                .getActivePrompt()
                .await
                .map_err(|error| error.to_string())?
            {
                ActivePrompt::CharacterCard { id } => println!("character_card\t{id}"),
                ActivePrompt::CharacterGroup { id } => println!("character_group\t{id}"),
            }
        }
        "set-card" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 active-prompt set-card <id>".to_string())?;
            core.preferences_active_prompt_manager()
                .setActivePrompt(ActivePrompt::CharacterCard { id: id.clone() })
                .await
                .map_err(|error| error.to_string())?;
            println!("active character card: {id}");
        }
        "set-group" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 active-prompt set-group <id>".to_string())?;
            core.preferences_active_prompt_manager()
                .setActivePrompt(ActivePrompt::CharacterGroup { id: id.clone() })
                .await
                .map_err(|error| error.to_string())?;
            println!("active character group: {id}");
        }
        "activate-for-chat" => {
            let characterCardName = args.get(1).cloned().and_then(nonBlankString);
            let characterGroupId = args.get(2).cloned().and_then(nonBlankString);
            core.preferences_active_prompt_manager()
                .activateForChatBinding(characterCardName, characterGroupId)
                .await
                .map_err(|error| error.to_string())?;
            println!("active prompt updated");
        }
        "resolved-card" => {
            let id = core
                .preferences_active_prompt_manager()
                .resolveActiveCardIdForSend()
                .await
                .map_err(|error| error.to_string())?;
            println!("{id}");
        }
        _ => print_active_prompt_usage(),
    }
    Ok(())
}
