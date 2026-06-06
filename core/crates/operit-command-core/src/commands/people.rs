use std::cell::Cell;

use crate::commands::util::parseCsvList;
use crate::output::CoreCommandOutput;
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::model::ActivePrompt::ActivePrompt;
use operit_runtime::data::model::CharacterCard::{
    CharacterCard, CharacterCardChatModelBindingMode, CharacterCardMemoryProfileBindingMode,
    CharacterCardToolAccessConfig,
};
use operit_runtime::data::model::CharacterGroupCard::{CharacterGroupCard, GroupMemberConfig};
use operit_runtime::data::model::PromptFunctionType::PromptFunctionType;
use operit_runtime::data::preferences::ActivePromptManager::ActivePromptManager;
use operit_runtime::data::preferences::CharacterCardManager::CharacterCardManager;
use operit_runtime::data::preferences::CharacterGroupCardManager::CharacterGroupCardManager;

macro_rules! println {
    () => {
        people_stdout_line("")
    };
    ($($arg:tt)*) => {
        people_stdout_line(format!($($arg)*))
    };
}

thread_local! {
    static PEOPLE_OUTPUT: Cell<*mut CoreCommandOutput> = Cell::new(std::ptr::null_mut());
}

fn set_people_output(output: &mut CoreCommandOutput) {
    PEOPLE_OUTPUT.with(|slot| slot.set(output as *mut CoreCommandOutput));
}

fn people_stdout_line(line: impl AsRef<str>) {
    PEOPLE_OUTPUT.with(|slot| {
        let output = slot.get();
        assert!(!output.is_null(), "people command output is not set");
        unsafe { (&mut *output).push_stdout_line(line.as_ref()) };
    });
}

struct PeopleCommand;

impl PeopleCommand {
    fn preferences_character_card_manager(&mut self) -> CharacterCardManager {
        CharacterCardManager::getInstance()
    }

    fn preferences_character_group_card_manager(&mut self) -> CharacterGroupCardManager {
        CharacterGroupCardManager::getInstance()
    }

    fn preferences_active_prompt_manager(&mut self) -> ActivePromptManager {
        ActivePromptManager::getInstance()
    }
}

pub fn run_character_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    set_people_output(output);
    let core = &mut PeopleCommand;
    if args.is_empty() {
        print_character_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "init" => {
            core.preferences_character_card_manager()
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            println!("initialized");
        }
        "list" => {
            core.preferences_character_card_manager()
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            for card in core
                .preferences_character_card_manager()
                .getAllCharacterCards()
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
                .map_err(|error| error.to_string())?;
            println!("updated: {id}");
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character delete <id>".to_string())?;
            core.preferences_character_card_manager()
                .deleteCharacterCard(id)
                .map_err(|error| error.to_string())?;
            println!("deleted: {id}");
        }
        "set-active" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 character set-active <id>".to_string())?;
            application
                .chatRuntimeHolder
                .getCore(ChatRuntimeSlot::MAIN)
                .switchActiveCharacterCardTarget(id.clone());
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
                .map_err(|error| error.to_string())?;
            println!("{prompt}");
        }
        "reset-default" => {
            core.preferences_character_card_manager()
                .resetDefaultCharacterCard()
                .map_err(|error| error.to_string())?;
            println!("default character reset");
        }
        _ => print_character_usage(),
    }
    Ok(())
}

pub fn run_group_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    set_people_output(output);
    let core = &mut PeopleCommand;
    if args.is_empty() {
        print_group_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "init" => {
            core.preferences_character_group_card_manager()
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            println!("initialized");
        }
        "list" => {
            core.preferences_character_group_card_manager()
                .initializeIfNeeded()
                .map_err(|error| error.to_string())?;
            for group in core
                .preferences_character_group_card_manager()
                .getAllCharacterGroupCards()
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
                .map_err(|error| error.to_string())?;
            println!("updated: {id}");
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group delete <id>".to_string())?;
            core.preferences_character_group_card_manager()
                .deleteCharacterGroupCard(id)
                .map_err(|error| error.to_string())?;
            println!("deleted: {id}");
        }
        "set-active" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 group set-active <id>".to_string())?;
            application
                .chatRuntimeHolder
                .getCore(ChatRuntimeSlot::MAIN)
                .switchActiveCharacterGroupTarget(id.clone());
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
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("group not found: {id}"))?;
            println!("{newId}");
        }
        _ => print_group_usage(),
    }
    Ok(())
}

pub fn run_active_prompt_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    set_people_output(output);
    let core = &mut PeopleCommand;
    if args.is_empty() {
        print_active_prompt_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "show" => {
            match core
                .preferences_active_prompt_manager()
                .getActivePrompt()
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
            application
                .chatRuntimeHolder
                .getCore(ChatRuntimeSlot::MAIN)
                .switchActiveCharacterCardTarget(id.clone());
            println!("active character card: {id}");
        }
        "set-group" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 active-prompt set-group <id>".to_string())?;
            application
                .chatRuntimeHolder
                .getCore(ChatRuntimeSlot::MAIN)
                .switchActiveCharacterGroupTarget(id.clone());
            println!("active character group: {id}");
        }
        "activate-for-chat" => {
            let characterCardName = args.get(1).cloned().and_then(nonBlankString);
            let characterGroupId = args.get(2).cloned().and_then(nonBlankString);
            core.preferences_active_prompt_manager()
                .activateForChatBinding(characterCardName, characterGroupId)
                .map_err(|error| error.to_string())?;
            println!("active prompt updated");
        }
        "resolved-card" => {
            let id = core
                .preferences_active_prompt_manager()
                .resolveActiveCardIdForSend()
                .map_err(|error| error.to_string())?;
            println!("{id}");
        }
        _ => print_active_prompt_usage(),
    }
    Ok(())
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
    let chatModelConfigId = match card.chatModelConfigId.clone() {
        Some(value) => value,
        None => String::new(),
    };
    println!("chatModelConfigId={chatModelConfigId}");
    println!("chatModelIndex={}", card.chatModelIndex);
    println!("memoryProfileBindingMode={}", card.memoryProfileBindingMode);
    let memoryProfileId = match card.memoryProfileId.clone() {
        Some(value) => value,
        None => String::new(),
    };
    println!("memoryProfileId={memoryProfileId}");
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

fn parsePromptFunctionType(value: Option<&str>) -> Result<PromptFunctionType, String> {
    match value {
        Some("CHAT") | None => Ok(PromptFunctionType::CHAT),
        Some("VOICE") => Ok(PromptFunctionType::VOICE),
        Some(other) => Err(format!(
            "invalid promptFunctionType: {other}; expected CHAT | VOICE"
        )),
    }
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

fn print_character_usage() {
    println!("operit2 character init");
    println!("operit2 character list");
    println!("operit2 character show <id>");
    println!("operit2 character create <name> [character-setting]");
    println!("operit2 character update <id> <field> <value>");
    println!("operit2 character delete <id>");
    println!("operit2 character set-active <id>");
    println!("operit2 character combine <id> [CHAT|VOICE] [tag-id-csv]");
    println!("operit2 character reset-default");
}

fn print_group_usage() {
    println!("operit2 group init");
    println!("operit2 group list");
    println!("operit2 group show <id>");
    println!("operit2 group create <name> [description]");
    println!("operit2 group update <id> <field> <value>");
    println!("operit2 group delete <id>");
    println!("operit2 group set-active <id>");
    println!("operit2 group duplicate <source-id> [new-name]");
}

fn print_active_prompt_usage() {
    println!("operit2 active-prompt show");
    println!("operit2 active-prompt set-card <id>");
    println!("operit2 active-prompt set-group <id>");
    println!("operit2 active-prompt activate-for-chat [character-card-name] [character-group-id]");
    println!("operit2 active-prompt resolved-card");
}
