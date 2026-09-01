use crate::output::CoreCommandOutput;
use operit_host_api::HostManager::HostManager;
use operit_model::PromptTag::{PromptTag, TagType};
use operit_runtime::data::preferences::PromptTagManager::PromptTagManager;

/// Runs prompt tag management commands.
pub fn run_tag_command(
    _context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_tag_usage(output);
        return Ok(());
    }
    let manager = PromptTagManager::getInstance();
    match args[0].as_str() {
        "list" => {
            let tags = manager.getAllTags().map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Prompt tags: {}", tags.len()));
            for tag in &tags {
                output.push_stdout_line(format!(
                    "- {} ({}) [{}] {}",
                    tag.id,
                    tag.name,
                    tagTypeName(&tag.tagType),
                    tag.description
                ));
            }
            output.setJsonStdout(serde_json::to_value(tags).map_err(|error| error.to_string())?);
            Ok(())
        }
        "show" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tag show <id>".to_string())?;
            let tag = manager
                .getAllTags()
                .map_err(|error| error.to_string())?
                .into_iter()
                .find(|tag| tag.id == *id)
                .ok_or_else(|| format!("tag not found: {id}"))?;
            print_tag(&tag, output)?;
            Ok(())
        }
        "create" => {
            let name = args
                .get(1)
                .ok_or_else(|| {
                    "usage: operit2 tag create <name> [prompt-content] [description] [tag-type]"
                        .to_string()
                })?
                .clone();
            let promptContent = args.get(2).cloned().unwrap_or_default();
            let description = args.get(3).cloned().unwrap_or_default();
            let tagType = parseTagType(args.get(4).map(String::as_str))?;
            let id = manager
                .createPromptTag(name, description, promptContent, tagType)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Prompt tag created: {id}"));
            output.setJsonStdout(serde_json::json!({"id": id, "created": true}));
            Ok(())
        }
        "update" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tag update <id> <field> <value>".to_string())?;
            let field = args
                .get(2)
                .ok_or_else(|| "usage: operit2 tag update <id> <field> <value>".to_string())?;
            let value = args
                .get(3)
                .ok_or_else(|| "usage: operit2 tag update <id> <field> <value>".to_string())?
                .clone();
            let (name, description, promptContent, tagType) = match field.as_str() {
                "name" => (Some(value), None, None, None),
                "description" => (None, Some(value), None, None),
                "promptContent" => (None, None, Some(value), None),
                "tagType" => (None, None, None, Some(parseTagType(Some(&value))?)),
                _ => {
                    return Err(
                        "tag fields: name | description | promptContent | tagType".to_string()
                    )
                }
            };
            manager
                .updatePromptTag(id, name, description, promptContent, tagType)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Prompt tag updated: {id}"));
            output.setJsonStdout(serde_json::json!({"id": id, "updated": true}));
            Ok(())
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tag delete <id>".to_string())?;
            manager
                .deletePromptTag(id)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Prompt tag deleted: {id}"));
            output.setJsonStdout(serde_json::json!({"id": id, "deleted": true}));
            Ok(())
        }
        _ => {
            print_tag_usage(output);
            Ok(())
        }
    }
}

/// Prints one prompt tag and records its JSON object.
fn print_tag(tag: &PromptTag, output: &mut CoreCommandOutput) -> Result<(), String> {
    output.push_stdout_line(format!("Prompt tag: {}", tag.name));
    output.push_stdout_line(format!("ID: {}", tag.id));
    output.push_stdout_line(format!("Type: {}", tagTypeName(&tag.tagType)));
    output.push_stdout_line(format!("Description: {}", tag.description));
    output.push_stdout_line(format!("Prompt content: {}", tag.promptContent));
    output.push_stdout_line(format!("Created at: {}", tag.createdAt));
    output.push_stdout_line(format!("Updated at: {}", tag.updatedAt));
    output.setJsonStdout(serde_json::to_value(tag).map_err(|error| error.to_string())?);
    Ok(())
}

/// Parses a prompt tag type from a command argument.
fn parseTagType(value: Option<&str>) -> Result<TagType, String> {
    match value {
        Some("TONE") => Ok(TagType::TONE),
        Some("CHARACTER") => Ok(TagType::CHARACTER),
        Some("FUNCTION") => Ok(TagType::FUNCTION),
        Some("CUSTOM") | None => Ok(TagType::CUSTOM),
        Some(other) => Err(format!(
            "invalid tagType: {other}; expected TONE | CHARACTER | FUNCTION | CUSTOM"
        )),
    }
}

/// Formats a prompt tag type for command output.
fn tagTypeName(tagType: &TagType) -> &'static str {
    match tagType {
        TagType::TONE => "TONE",
        TagType::CHARACTER => "CHARACTER",
        TagType::FUNCTION => "FUNCTION",
        TagType::CUSTOM => "CUSTOM",
    }
}

/// Prints prompt tag command usage.
fn print_tag_usage(output: &mut CoreCommandOutput) {
    let lines = vec![
        "operit2 tag list",
        "operit2 tag show <id>",
        "operit2 tag create <name> [prompt-content] [description] [tag-type]",
        "operit2 tag update <id> <field> <value>",
        "operit2 tag delete <id>",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}
