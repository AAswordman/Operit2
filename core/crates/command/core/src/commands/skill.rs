use std::path::{Path, PathBuf};

use crate::commands::util::{parse_bool_arg, read_content_arg};
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;
use serde_json::json;

/// Runs skill repository commands.
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
            let path = repository.getSkillsDirectoryPath();
            output.push_stdout_line("Skills directory");
            output.push_stdout_line(path.clone());
            output.setJsonStdout(json!({ "skillsDirectory": path }));
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
            let result = repository.importSkillFromDirectInput(
                skillId,
                description,
                &content,
                &attachmentPaths,
            );
            output.push_stdout_line("Skill create result");
            output.push_stdout_line(result.clone());
            output.setJsonStdout(json!({
                "skillId": skillId,
                "description": description,
                "attachmentPaths": attachmentPaths,
                "message": result
            }));
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
            output.push_stdout_line("Skill zip import result");
            output.push_stdout_line(result.clone());
            output.setJsonStdout(json!({
                "zipPath": zipPath,
                "subDirectory": args.get(2),
                "message": result
            }));
            Ok(())
        }
        "delete" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 skill delete <name>".to_string())?;
            if repository.deleteSkill(name) {
                output.push_stdout_line(format!("Deleted skill {name}"));
                output.setJsonStdout(json!({
                    "name": name,
                    "deleted": true
                }));
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
                let visible = repository.isSkillVisibleToAi(name);
                output.push_stdout_line(format!("Skill {name} visibility: {visible}"));
                output.setJsonStdout(json!({
                    "name": name,
                    "visible": visible
                }));
            } else {
                let visible = parse_bool_arg(
                    args.get(2),
                    "usage: operit2 skill visible <name> [true|false]",
                )?;
                repository
                    .setSkillVisibleToAi(name, visible)
                    .map_err(|error| error.to_string())?;
                output.push_stdout_line(format!("Skill {name} visibility updated: {visible}"));
                output.setJsonStdout(json!({
                    "name": name,
                    "visible": visible,
                    "updated": true
                }));
            }
            Ok(())
        }
        "errors" => {
            let errors = repository.getSkillLoadErrors();
            output.push_stdout_line(format!("Skill load errors: {}", errors.len()));
            for (name, error) in &errors {
                output.push_stdout_line(format!("- {name}: {error}"));
            }
            output.setJsonStdout(json!({ "errors": errors }));
            Ok(())
        }
        _ => {
            print_skill_usage(output);
            Ok(())
        }
    }
}

/// Lists bundled external skills available to import.
fn list_more_skills(
    repository: &SkillRepository,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let candidates = repository.getBundledExternalSkillCandidates();
    output.push_stdout_line(format!("Bundled skills available: {}", candidates.len()));
    for candidate in &candidates {
        output.push_stdout_line(format!("- {}: {}", candidate.name, candidate.description));
    }
    output.setJsonStdout(serde_json::to_value(&candidates).map_err(|error| error.to_string())?);
    Ok(())
}

/// Imports one bundled external skill by exact name.
fn load_more_skill(
    repository: &SkillRepository,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let skill = repository.importBundledExternalSkill(name)?;
    output.push_stdout_line(format!("Loaded skill {}", skill.name));
    output.push_stdout_line(format!("Description: {}", skill.description));
    output.push_stdout_line(format!("Directory: {}", skill.directory.to_string_lossy()));
    output.setJsonStdout(serde_json::to_value(&skill).map_err(|error| error.to_string())?);
    Ok(())
}

/// Lists installed skill packages and scan errors.
fn list_skills(repository: &SkillRepository, output: &mut CoreCommandOutput) -> Result<(), String> {
    let (skills, errors) = repository.getAvailableSkillPackagesSnapshot();
    let mut rows = Vec::new();
    output.push_stdout_line(format!("Installed skills: {}", skills.len()));
    for (name, skill) in skills {
        let visible = repository.isSkillVisibleToAi(&name);
        output.push_stdout_line(format!(
            "- {name}: {} | visible: {visible}",
            skill.description
        ));
        rows.push(json!({
            "name": name,
            "description": skill.description,
            "directory": skill.directory,
            "skillFile": skill.skillFile,
            "visible": visible
        }));
    }
    if !errors.is_empty() {
        output.push_stdout_line(format!("Load errors: {}", errors.len()));
    }
    output.setJsonStdout(json!({
        "skills": rows,
        "loadErrors": errors
    }));
    Ok(())
}

/// Shows one installed skill and its SKILL.md content.
fn show_skill(
    repository: &SkillRepository,
    name: &str,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let skills = repository.getAvailableSkillPackages();
    let skill = skills
        .get(name)
        .ok_or_else(|| format!("skill not found: {name}"))?;
    let visible = repository.isSkillVisibleToAi(name);
    let content = repository.readSkillContent(name);
    output.push_stdout_line(format!("Skill {}", skill.name));
    output.push_stdout_line(format!("Description: {}", skill.description));
    output.push_stdout_line(format!("Directory: {}", skill.directory.to_string_lossy()));
    output.push_stdout_line(format!("Skill file: {}", skill.skillFile.to_string_lossy()));
    output.push_stdout_line(format!("Visible to AI: {visible}"));
    if let Some(content) = content.as_ref() {
        output.push_stdout_line("");
        output.push_stdout_line("Content:");
        output.push_stdout(&content);
    }
    output.setJsonStdout(json!({
        "name": skill.name,
        "description": skill.description,
        "directory": skill.directory,
        "skillFile": skill.skillFile,
        "visible": visible,
        "content": content
    }));
    Ok(())
}

/// Returns the skill repository for the active application context.
fn skill_repository(application: &OperitApplication) -> SkillRepository {
    SkillRepository::getInstance(
        &application.hostManager,
        application.toolHandler.runtimeSupport(),
    )
}

/// Prints skill command usage.
fn print_skill_usage(output: &mut CoreCommandOutput) {
    let lines = [
        "operit2 skill dir",
        "operit2 skill list",
        "operit2 skill more",
        "operit2 skill load <name>",
        "operit2 skill show <name>",
        "operit2 skill create <skill-id> <description> <content-or-@file> [attachment-path...]",
        "operit2 skill import-zip <zip-path> [sub-dir-in-zip]",
        "operit2 skill delete <name>",
        "operit2 skill visible <name> [true|false]",
        "operit2 skill errors",
    ];
    for line in lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(json!({ "usage": lines }));
}
