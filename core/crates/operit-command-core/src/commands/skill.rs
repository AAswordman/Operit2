use std::path::{Path, PathBuf};

use crate::commands::util::{parse_bool_arg, read_content_arg};
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::data::skill::SkillRepository::SkillRepository;

pub fn run_skill_command(
    context: OperitApplicationContext,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_skill_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "dir" => {
            output.push_stdout_line(skill_repository(&context).getSkillsDirectoryPath());
            Ok(())
        }
        "list" => list_skills(context, output),
        "more" => list_more_skills(context, output),
        "load" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 skill load <name>".to_string())?;
            load_more_skill(context, name, output)
        }
        "show" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 skill show <name>".to_string())?;
            show_skill(context, name, output)
        }
        "create" => {
            let skillId = args.get(1).ok_or_else(|| {
                "usage: operit2 skill create <skill-id> <description> <content-or-@file> [attachment-path...]".to_string()
            })?;
            let description = args.get(2).ok_or_else(|| {
                "usage: operit2 skill create <skill-id> <description> <content-or-@file> [attachment-path...]".to_string()
            })?;
            let contentArg = args.get(3).ok_or_else(|| {
                "usage: operit2 skill create <skill-id> <description> <content-or-@file> [attachment-path...]".to_string()
            })?;
            let content = read_content_arg(contentArg)?;
            let attachmentPaths = args[4..].iter().map(PathBuf::from).collect::<Vec<_>>();
            output.push_stdout_line(skill_repository(&context).importSkillFromDirectInput(
                skillId,
                description,
                &content,
                &attachmentPaths,
            ));
            Ok(())
        }
        "import-zip" => {
            let zipPath = args.get(1).ok_or_else(|| {
                "usage: operit2 skill import-zip <zip-path> [sub-dir-in-zip]".to_string()
            })?;
            let repository = skill_repository(&context);
            let result = match args.get(2) {
                Some(subDir) => {
                    repository.importSkillFromZipWithSubDir(Path::new(zipPath), Some(subDir))
                }
                None => repository.importSkillFromZip(Path::new(zipPath)),
            };
            output.push_stdout_line(result);
            Ok(())
        }
        "delete" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 skill delete <name>".to_string())?;
            if skill_repository(&context).deleteSkill(name) {
                output.push_stdout_line(format!("deleted: {name}"));
                Ok(())
            } else {
                Err(format!("skill not found: {name}"))
            }
        }
        "visible" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 skill visible <name> [true|false]".to_string())?;
            let repository = skill_repository(&context);
            if args.len() == 2 {
                output.push_stdout_line(repository.isSkillVisibleToAi(name).to_string());
            } else {
                let visible = parse_bool_arg(
                    args.get(2),
                    "usage: operit2 skill visible <name> [true|false]",
                )?;
                repository
                    .setSkillVisibleToAi(name, visible)
                    .map_err(|error| error.to_string())?;
                output.push_stdout_line(format!("visible: {name}={visible}"));
            }
            Ok(())
        }
        "errors" => {
            for (name, error) in skill_repository(&context).getSkillLoadErrors() {
                output.push_stdout_line(format!("{name}\t{error}"));
            }
            Ok(())
        }
        _ => {
            print_skill_usage(output);
            Ok(())
        }
    }
}

fn list_more_skills(
    context: OperitApplicationContext,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let repository = skill_repository(&context);
    for candidate in repository.getBundledExternalSkillCandidates() {
        output.push_stdout_line(format!("{}\t{}", candidate.name, candidate.description));
    }
    Ok(())
}

fn load_more_skill(
    context: OperitApplicationContext,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let repository = skill_repository(&context);
    let skill = repository.importBundledExternalSkill(name)?;
    output.push_stdout_line(format!("loaded: {}", skill.name));
    Ok(())
}

fn list_skills(
    context: OperitApplicationContext,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let repository = skill_repository(&context);
    for (name, skill) in repository.getAvailableSkillPackages() {
        let visible = repository.isSkillVisibleToAi(&name);
        output.push_stdout_line(format!(
            "{}\tvisible={}\t{}\t{}",
            name,
            visible,
            skill.description,
            skill.directory.to_string_lossy()
        ));
    }
    let errors = repository.getSkillLoadErrors();
    if !errors.is_empty() {
        output.push_stderr_line(format!("loadErrors={}", errors.len()));
    }
    Ok(())
}

fn show_skill(
    context: OperitApplicationContext,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let repository = skill_repository(&context);
    let skills = repository.getAvailableSkillPackages();
    let skill = skills
        .get(name)
        .ok_or_else(|| format!("skill not found: {name}"))?;
    output.push_stdout_line(format!("name={}", skill.name));
    output.push_stdout_line(format!("description={}", skill.description));
    output.push_stdout_line(format!("directory={}", skill.directory.to_string_lossy()));
    output.push_stdout_line(format!("skillFile={}", skill.skillFile.to_string_lossy()));
    output.push_stdout_line(format!("visible={}", repository.isSkillVisibleToAi(name)));
    output.push_stdout_line("");
    if let Some(content) = repository.readSkillContent(name) {
        output.push_stdout(&content);
    }
    Ok(())
}

fn skill_repository(context: &OperitApplicationContext) -> SkillRepository {
    SkillRepository::getInstance(context)
}

fn print_skill_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 skill dir");
    output.push_stdout_line("operit2 skill list");
    output.push_stdout_line("operit2 skill more");
    output.push_stdout_line("operit2 skill load <name>");
    output.push_stdout_line("operit2 skill show <name>");
    output.push_stdout_line(
        "operit2 skill create <skill-id> <description> <content-or-@file> [attachment-path...]",
    );
    output.push_stdout_line("operit2 skill import-zip <zip-path> [sub-dir-in-zip]");
    output.push_stdout_line("operit2 skill delete <name>");
    output.push_stdout_line("operit2 skill visible <name> [true|false]");
    output.push_stdout_line("operit2 skill errors");
}
