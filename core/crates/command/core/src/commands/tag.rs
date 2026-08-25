use crate::output::CoreCommandOutput;
use operit_host_api::HostManager::HostManager;
use operit_model::PromptTag::{PromptTag, TagType};
use operit_runtime::data::preferences::PromptTagManager::PromptTagManager;

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
            for tag in manager.getAllTags().map_err(|error| error.to_string())? {
                output.push_stdout_line(format!(
                    "{}\t{}\t{}\t{}\t{}",
                    tag.id,
                    tag.name,
                    tagTypeName(&tag.tagType),
                    tag.description,
                    tag.promptContent.replace('\n', "\\n")
                ));
            }
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
            print_tag(&tag, output);
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
            output.push_stdout_line(id);
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
            output.push_stdout_line(format!("updated: {id}"));
            Ok(())
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tag delete <id>".to_string())?;
            manager
                .deletePromptTag(id)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("deleted: {id}"));
            Ok(())
        }
        _ => {
            print_tag_usage(output);
            Ok(())
        }
    }
}

fn print_tag(tag: &PromptTag, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!("id={}", tag.id));
    output.push_stdout_line(format!("name={}", tag.name));
    output.push_stdout_line(format!("description={}", tag.description));
    output.push_stdout_line(format!("promptContent={}", tag.promptContent));
    output.push_stdout_line(format!("tagType={}", tagTypeName(&tag.tagType)));
    output.push_stdout_line(format!("createdAt={}", tag.createdAt));
    output.push_stdout_line(format!("updatedAt={}", tag.updatedAt));
}

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

fn tagTypeName(tagType: &TagType) -> &'static str {
    match tagType {
        TagType::TONE => "TONE",
        TagType::CHARACTER => "CHARACTER",
        TagType::FUNCTION => "FUNCTION",
        TagType::CUSTOM => "CUSTOM",
    }
}

fn print_tag_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 tag list");
    output.push_stdout_line("operit2 tag show <id>");
    output.push_stdout_line("operit2 tag create <name> [prompt-content] [description] [tag-type]");
    output.push_stdout_line("operit2 tag update <id> <field> <value>");
    output.push_stdout_line("operit2 tag delete <id>");
}
