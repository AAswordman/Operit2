use crate::commands::util::parseCsvList;
use crate::output::CoreCommandOutput;
use operit_model::ActivePrompt::ActivePrompt;
use operit_model::CharacterCard::{
    CharacterCard, CharacterCardChatModelBindingMode, CharacterCardMemoryBindingMode,
    CharacterCardToolAccessConfig,
};
use operit_model::CharacterGroupCard::{CharacterGroupCard, GroupMemberConfig};
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::data::preferences::ActivePromptManager::ActivePromptManager;
use operit_runtime::data::preferences::CharacterCardManager::CharacterCardManager;
use operit_runtime::data::preferences::CharacterGroupCardManager::CharacterGroupCardManager;
use operit_runtime::services::ChatServiceCore::ChatServiceCore;
use serde_json::json;

/// Runs a synchronous action against the local main chat runtime core.
fn with_main_chat_core<R>(
    application: &OperitApplication,
    action: impl FnOnce(&mut ChatServiceCore) -> R,
) -> Result<R, String> {
    let mut holder = application
        .chatRuntimeHolder
        .try_lock()
        .map_err(|_| "Chat runtime holder is busy".to_string())?;
    Ok(action(holder.getCore(ChatRuntimeSlot::MAIN)))
}

struct PeopleCommand;

impl PeopleCommand {
    /// Returns the character card manager.
    fn preferences_character_card_manager(&mut self) -> CharacterCardManager {
        CharacterCardManager::getInstance()
    }

    /// Returns the character group card manager.
    fn preferences_character_group_card_manager(&mut self) -> CharacterGroupCardManager {
        CharacterGroupCardManager::getInstance()
    }

    /// Returns the active prompt manager.
    fn preferences_active_prompt_manager(&mut self) -> ActivePromptManager {
        ActivePromptManager::getInstance()
    }
}

/// Runs character card commands.
pub fn run_character_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let mut command = PeopleCommand;
    if args.is_empty() {
        print_character_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "list" => {
            let cards = command
                .preferences_character_card_manager()
                .getAllCharacterCards()
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Characters: {}", cards.len()));
            for card in &cards {
                output.push_stdout_line(format!(
                    "- {} | {} | default: {} | tags: {} | {}",
                    card.id,
                    card.name,
                    card.isDefault,
                    joined_values(&card.attachedTagIds),
                    card.description
                ));
            }
            output.setJsonStdout(serde_json::to_value(&cards).map_err(|error| error.to_string())?);
        }
        "show" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character show <id>".to_string())?;
            let card = command
                .preferences_character_card_manager()
                .getCharacterCard(id)
                .map_err(|error| error.to_string())?;
            print_character_card(&card, output);
            output.setJsonStdout(serde_json::to_value(&card).map_err(|error| error.to_string())?);
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
            let id = command
                .preferences_character_card_manager()
                .createCharacterCard(CharacterCard {
                    id: String::new(),
                    name: name.clone(),
                    description: String::new(),
                    characterSetting: characterSetting.clone(),
                    openingStatement: String::new(),
                    otherContentChat: String::new(),
                    otherContentVoice: String::new(),
                    avatarUri: None,
                    attachedTagIds: Vec::new(),
                    advancedCustomPrompt: String::new(),
                    marks: String::new(),
                    chatModelBindingMode: CharacterCardChatModelBindingMode::FOLLOW_GLOBAL
                        .to_string(),
                    chatModelId: None,
                    ttsConfigId: None,
                    memoryBindingMode: CharacterCardMemoryBindingMode::CHARACTER.to_string(),
                    sharedMemoryId: None,
                    sharedMemoryMounts: Vec::new(),
                    toolAccessConfig: CharacterCardToolAccessConfig::default(),
                    isDefault: false,
                    createdAt: now,
                    updatedAt: now,
                })
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Created character {id}"));
            output.setJsonStdout(json!({
                "id": id,
                "name": name,
                "characterSetting": characterSetting
            }));
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
            let mut card = command
                .preferences_character_card_manager()
                .getCharacterCard(id)
                .map_err(|error| error.to_string())?;
            match field.as_str() {
                "name" => card.name = value.clone(),
                "description" => card.description = value.clone(),
                "characterSetting" => card.characterSetting = value.clone(),
                "openingStatement" => card.openingStatement = value.clone(),
                "otherContentChat" => card.otherContentChat = value.clone(),
                "otherContentVoice" => card.otherContentVoice = value.clone(),
                "avatarUri" => card.avatarUri = nonBlankString(value.clone()),
                "advancedCustomPrompt" => card.advancedCustomPrompt = value.clone(),
                "marks" => card.marks = value.clone(),
                "attachedTagIds" => card.attachedTagIds = parseCsvList(&value),
                "chatModelBindingMode" => {
                    card.chatModelBindingMode =
                        CharacterCardChatModelBindingMode::normalize(Some(&value))
                }
                "chatModelId" => card.chatModelId = nonBlankString(value.clone()),
                _ => {
                    return Err("character fields: name | description | characterSetting | openingStatement | otherContentChat | otherContentVoice | avatarUri | attachedTagIds | advancedCustomPrompt | marks | chatModelBindingMode | chatModelId".to_string())
                }
            }
            let updated = card.clone();
            command
                .preferences_character_card_manager()
                .updateCharacterCard(card)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Updated character {id}"));
            output.push_stdout_line(format!("Field: {field}"));
            output.setJsonStdout(json!({
                "id": id,
                "field": field,
                "character": updated
            }));
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character delete <id>".to_string())?;
            command
                .preferences_character_card_manager()
                .deleteCharacterCard(id)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Deleted character {id}"));
            output.setJsonStdout(json!({ "id": id, "deleted": true }));
        }
        "set-active" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character set-active <id>".to_string())?;
            with_main_chat_core(application, |core| {
                core.switchActiveCharacterCardTarget(id.clone())
            })?;
            output.push_stdout_line(format!("Active character: {id}"));
            output.setJsonStdout(json!({
                "type": "character_card",
                "id": id,
                "active": true
            }));
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
            let prompt = command
                .preferences_character_card_manager()
                .combinePrompts(id, additionalTagIds.clone(), promptFunctionType.clone())
                .map_err(|error| error.to_string())?;
            output.push_stdout(&prompt);
            output.setJsonStdout(json!({
                "id": id,
                "promptFunctionType": promptFunctionType,
                "additionalTagIds": additionalTagIds,
                "prompt": prompt
            }));
        }
        "reset-default" => {
            command
                .preferences_character_card_manager()
                .resetDefaultCharacterCard()
                .map_err(|error| error.to_string())?;
            output.push_stdout_line("Default character reset");
            output.setJsonStdout(json!({ "defaultCharacterReset": true }));
        }
        _ => print_character_usage(output),
    }
    Ok(())
}

/// Runs character group commands.
pub fn run_group_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let mut command = PeopleCommand;
    if args.is_empty() {
        print_group_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "list" => {
            let groups = command
                .preferences_character_group_card_manager()
                .getAllCharacterGroupCards()
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Character groups: {}", groups.len()));
            for group in &groups {
                output.push_stdout_line(format!(
                    "- {} | {} | members: {} | {}",
                    group.id,
                    group.name,
                    group_members_summary(&group.members),
                    group.description
                ));
            }
            output.setJsonStdout(serde_json::to_value(&groups).map_err(|error| error.to_string())?);
        }
        "show" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group show <id>".to_string())?;
            let group = command
                .preferences_character_group_card_manager()
                .getCharacterGroupCard(id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("group not found: {id}"))?;
            print_character_group_card(&group, output);
            output.setJsonStdout(serde_json::to_value(&group).map_err(|error| error.to_string())?);
        }
        "create" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group create <name> [description]".to_string())?
                .clone();
            let description = args.get(2).cloned().unwrap_or_default();
            let id = command
                .preferences_character_group_card_manager()
                .createCharacterGroupCard(CharacterGroupCard {
                    id: String::new(),
                    name: name.clone(),
                    description: description.clone(),
                    members: Vec::new(),
                    createdAt: currentTimeMillis(),
                    updatedAt: currentTimeMillis(),
                })
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Created character group {id}"));
            output.setJsonStdout(json!({
                "id": id,
                "name": name,
                "description": description
            }));
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
            let mut group = command
                .preferences_character_group_card_manager()
                .getCharacterGroupCard(id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("group not found: {id}"))?;
            match field.as_str() {
                "name" => group.name = value.clone(),
                "description" => group.description = value.clone(),
                "members" => group.members = parse_group_members(&value),
                _ => return Err("group fields: name | description | members".to_string()),
            }
            group.updatedAt = currentTimeMillis();
            let updated = group.clone();
            command
                .preferences_character_group_card_manager()
                .updateCharacterGroupCard(group)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Updated character group {id}"));
            output.push_stdout_line(format!("Field: {field}"));
            output.setJsonStdout(json!({
                "id": id,
                "field": field,
                "group": updated
            }));
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group delete <id>".to_string())?;
            command
                .preferences_character_group_card_manager()
                .deleteCharacterGroupCard(id)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Deleted character group {id}"));
            output.setJsonStdout(json!({ "id": id, "deleted": true }));
        }
        "set-active" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group set-active <id>".to_string())?;
            with_main_chat_core(application, |core| {
                core.switchActiveCharacterGroupTarget(id.clone())
            })?;
            output.push_stdout_line(format!("Active character group: {id}"));
            output.setJsonStdout(json!({
                "type": "character_group",
                "id": id,
                "active": true
            }));
        }
        "duplicate" => {
            let id = args.get(1).ok_or_else(|| {
                "usage: operit2 group duplicate <source-id> [new-name]".to_string()
            })?;
            let newName = args.get(2).cloned();
            let newId = command
                .preferences_character_group_card_manager()
                .duplicateCharacterGroupCard(id, newName.clone())
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("group not found: {id}"))?;
            output.push_stdout_line(format!("Duplicated character group {id} -> {newId}"));
            output.setJsonStdout(json!({
                "sourceId": id,
                "newId": newId,
                "newName": newName
            }));
        }
        _ => print_group_usage(output),
    }
    Ok(())
}

/// Runs active prompt commands.
pub fn run_active_prompt_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let mut command = PeopleCommand;
    if args.is_empty() {
        print_active_prompt_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "show" => {
            let activePrompt = command
                .preferences_active_prompt_manager()
                .getActivePrompt()
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!(
                "Active prompt: {}",
                active_prompt_summary(&activePrompt)
            ));
            output.setJsonStdout(
                serde_json::to_value(&activePrompt).map_err(|error| error.to_string())?,
            );
        }
        "set-card" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 active-prompt set-card <id>".to_string())?;
            with_main_chat_core(application, |core| {
                core.switchActiveCharacterCardTarget(id.clone())
            })?;
            output.push_stdout_line(format!("Active character card: {id}"));
            output.setJsonStdout(json!({
                "type": "character_card",
                "id": id,
                "active": true
            }));
        }
        "set-group" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 active-prompt set-group <id>".to_string())?;
            with_main_chat_core(application, |core| {
                core.switchActiveCharacterGroupTarget(id.clone())
            })?;
            output.push_stdout_line(format!("Active character group: {id}"));
            output.setJsonStdout(json!({
                "type": "character_group",
                "id": id,
                "active": true
            }));
        }
        "activate-for-chat" => {
            let characterCardName = args.get(1).cloned().and_then(nonBlankString);
            let characterGroupId = args.get(2).cloned().and_then(nonBlankString);
            command
                .preferences_active_prompt_manager()
                .activateForChatBinding(characterCardName.clone(), characterGroupId.clone())
                .map_err(|error| error.to_string())?;
            output.push_stdout_line("Active prompt updated for chat binding");
            output.setJsonStdout(json!({
                "characterCardName": characterCardName,
                "characterGroupId": characterGroupId,
                "updated": true
            }));
        }
        "resolved-card" => {
            let id = command
                .preferences_active_prompt_manager()
                .resolveActiveCardIdForSend()
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Resolved character card: {id}"));
            output.setJsonStdout(json!({ "id": id }));
        }
        _ => print_active_prompt_usage(output),
    }
    Ok(())
}

/// Prints one character card for a human reader.
fn print_character_card(card: &CharacterCard, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!("Character {}", card.id));
    output.push_stdout_line(format!("Name: {}", card.name));
    output.push_stdout_line(format!("Description: {}", card.description));
    output.push_stdout_line(format!("Character setting: {}", card.characterSetting));
    output.push_stdout_line(format!("Opening statement: {}", card.openingStatement));
    output.push_stdout_line(format!("Chat content: {}", card.otherContentChat));
    output.push_stdout_line(format!("Voice content: {}", card.otherContentVoice));
    output.push_stdout_line(format!("Tags: {}", joined_values(&card.attachedTagIds)));
    output.push_stdout_line(format!("Advanced prompt: {}", card.advancedCustomPrompt));
    output.push_stdout_line(format!("Marks: {}", card.marks));
    output.push_stdout_line(format!("Chat model binding: {}", card.chatModelBindingMode));
    output.push_stdout_line(format!(
        "Chat model id: {}",
        option_text(card.chatModelId.as_deref())
    ));
    output.push_stdout_line(format!(
        "Shared memory mounts: {}",
        card.sharedMemoryMounts.len()
    ));
    output.push_stdout_line(format!(
        "Tool access enabled: {}",
        card.toolAccessConfig.enabled
    ));
    output.push_stdout_line(format!("Default: {}", card.isDefault));
    output.push_stdout_line(format!("Created at: {}", card.createdAt));
    output.push_stdout_line(format!("Updated at: {}", card.updatedAt));
}

/// Prints one character group for a human reader.
fn print_character_group_card(group: &CharacterGroupCard, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!("Character group {}", group.id));
    output.push_stdout_line(format!("Name: {}", group.name));
    output.push_stdout_line(format!("Description: {}", group.description));
    output.push_stdout_line(format!(
        "Members: {}",
        group_members_summary(&group.members)
    ));
    output.push_stdout_line(format!("Created at: {}", group.createdAt));
    output.push_stdout_line(format!("Updated at: {}", group.updatedAt));
}

/// Parses a group member CSV into ordered member configs.
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

/// Parses a prompt function type argument.
fn parsePromptFunctionType(value: Option<&str>) -> Result<PromptFunctionType, String> {
    match value {
        Some("CHAT") | None => Ok(PromptFunctionType::CHAT),
        Some("VOICE") => Ok(PromptFunctionType::VOICE),
        Some(other) => Err(format!(
            "invalid promptFunctionType: {other}; expected CHAT | VOICE"
        )),
    }
}

/// Converts a string into an optional non-blank value.
fn nonBlankString(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Returns the current Unix timestamp in milliseconds.
#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after unix epoch")
        .as_millis() as i64
}

/// Joins text values for human-readable command output.
fn joined_values(values: &[String]) -> String {
    values.join(", ")
}

/// Formats optional text for human-readable command output.
fn option_text(value: Option<&str>) -> &str {
    match value {
        Some(text) => text,
        None => "-",
    }
}

/// Formats group members for human-readable command output.
fn group_members_summary(members: &[GroupMemberConfig]) -> String {
    members
        .iter()
        .map(|member| format!("{}:{}", member.characterCardId, member.orderIndex))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Formats an active prompt for human-readable command output.
fn active_prompt_summary(activePrompt: &ActivePrompt) -> String {
    match activePrompt {
        ActivePrompt::CharacterCard { id } => format!("character card {id}"),
        ActivePrompt::CharacterGroup { id } => format!("character group {id}"),
    }
}

/// Prints character command usage.
fn print_character_usage(output: &mut CoreCommandOutput) {
    let lines = [
        "operit2 character list",
        "operit2 character show <id>",
        "operit2 character create <name> [character-setting]",
        "operit2 character update <id> <field> <value>",
        "operit2 character delete <id>",
        "operit2 character set-active <id>",
        "operit2 character combine <id> [CHAT|VOICE] [tag-id-csv]",
        "operit2 character reset-default",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}

/// Prints group command usage.
fn print_group_usage(output: &mut CoreCommandOutput) {
    let lines = [
        "operit2 group list",
        "operit2 group show <id>",
        "operit2 group create <name> [description]",
        "operit2 group update <id> <field> <value>",
        "operit2 group delete <id>",
        "operit2 group set-active <id>",
        "operit2 group duplicate <source-id> [new-name]",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}

/// Prints active prompt command usage.
fn print_active_prompt_usage(output: &mut CoreCommandOutput) {
    let lines = [
        "operit2 active-prompt show",
        "operit2 active-prompt set-card <id>",
        "operit2 active-prompt set-group <id>",
        "operit2 active-prompt activate-for-chat [character-card-name] [character-group-id]",
        "operit2 active-prompt resolved-card",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}
