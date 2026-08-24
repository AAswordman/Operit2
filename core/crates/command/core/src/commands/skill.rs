use std::path::{Path, PathBuf};

use crate::commands::util::{parse_bool_arg, read_content_arg};
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;

pub fn run_skill_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let repository = skill_repository(application);
    if args.is_empty() {
        print_skill_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "dir" => {
            output.push_stdout_line(repository.getSkillsDirectoryPath());
            Ok(())
        }
        "list" => list_skills(&repository, output),
        "more" => list_more_skills(&repository, output),
        "load" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 skill load <name>".to_string())?;
            load_more_skill(&repository, name, output)
        }
        "show" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 skill show <name>".to_string())?;
            show_skill(&repository, name, output)
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
            output.push_stdout_line(repository.importSkillFromDirectInput(
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
            if repository.deleteSkill(name) {
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
            for (name, error) in repository.getSkillLoadErrors() {
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
    repository: &SkillRepository,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    for candidate in repository.getBundledExternalSkillCandidates() {
        output.push_stdout_line(format!("{}\t{}", candidate.name, candidate.description));
    }
    Ok(())
}

fn load_more_skill(
    repository: &SkillRepository,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let skill = repository.importBundledExternalSkill(name)?;
    output.push_stdout_line(format!("loaded: {}", skill.name));
    Ok(())
}

fn list_skills(repository: &SkillRepository, output: &mut CoreCommandOutput) -> Result<(), String> {
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
    repository: &SkillRepository,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
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

fn skill_repository(application: &OperitApplication) -> SkillRepository {
    SkillRepository::getInstance(
        &application.hostManager,
        application.toolHandler.runtimeSupport(),
    )
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
