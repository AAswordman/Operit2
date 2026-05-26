use super::core::CliCore;
use super::*;

pub(super) async fn run_tag_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_tag_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "list" => {
            for tag in core
                .preferences_prompt_tag_manager()
                .getAllTags()
                .await
                .map_err(|error| error.to_string())?
            {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    tag.id,
                    tag.name,
                    tagTypeName(&tag.tagType),
                    tag.description,
                    tag.promptContent.replace('\n', "\\n")
                );
            }
        }
        "show" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tag show <id>".to_string())?;
            let tag = core
                .preferences_prompt_tag_manager()
                .getAllTags()
                .await
                .map_err(|error| error.to_string())?;
            let tag = tag
                .into_iter()
                .find(|tag| tag.id == *id)
                .ok_or_else(|| format!("tag not found: {id}"))?;
            print_tag(&tag);
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
            let id = core
                .preferences_prompt_tag_manager()
                .createPromptTag(name, description, promptContent, tagType)
                .await
                .map_err(|error| error.to_string())?;
            println!("{id}");
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
            core.preferences_prompt_tag_manager()
                .updatePromptTag(id, name, description, promptContent, tagType)
                .await
                .map_err(|error| error.to_string())?;
            println!("updated: {id}");
        }
        "delete" => {
            let id = args
                .get(1)
                .ok_or_else(|| "usage: operit2 tag delete <id>".to_string())?;
            core.preferences_prompt_tag_manager()
                .deletePromptTag(id)
                .await
                .map_err(|error| error.to_string())?;
            println!("deleted: {id}");
        }
        _ => print_tag_usage(),
    }
    Ok(())
}
