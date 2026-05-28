use super::core::CliCore;
use super::*;

pub(super) async fn run_skill_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_skill_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "dir" => {
            println!(
                "{}",
                core.skill_repository()
                    .getSkillsDirectoryPath()
                    .await
                    .map_err(|error| error.to_string())?
            );
        }
        "list" => {
            let skills = core
                .skill_repository()
                .getAvailableSkillPackages()
                .await
                .map_err(|error| error.to_string())?;
            for (name, skill) in skills {
                let visible = core
                    .skill_repository()
                    .isSkillVisibleToAi(&name)
                    .await
                    .map_err(|error| error.to_string())?;
                println!(
                    "{}\tvisible={}\t{}\t{}",
                    name,
                    visible,
                    skill.description,
                    skill.directory.to_string_lossy()
                );
            }
            let errors = core
                .skill_repository()
                .getSkillLoadErrors()
                .await
                .map_err(|error| error.to_string())?;
            if !errors.is_empty() {
                eprintln!("loadErrors={}", errors.len());
            }
        }
        "show" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 skill show <name>".to_string())?;
            let skills = core
                .skill_repository()
                .getAvailableSkillPackages()
                .await
                .map_err(|error| error.to_string())?;
            let skill = skills
                .get(name)
                .ok_or_else(|| format!("skill not found: {name}"))?;
            println!("name={}", skill.name);
            println!("description={}", skill.description);
            println!("directory={}", skill.directory.to_string_lossy());
            println!("skillFile={}", skill.skillFile.to_string_lossy());
            println!(
                "visible={}",
                core.skill_repository()
                    .isSkillVisibleToAi(name)
                    .await
                    .map_err(|error| error.to_string())?
            );
            println!();
            if let Some(content) = core
                .skill_repository()
                .readSkillContent(name)
                .await
                .map_err(|error| error.to_string())?
            {
                print!("{content}");
            }
        }
        "create" => {
            let skillId = args
                .get(1)
                .ok_or_else(|| "usage: operit2 skill create <skill-id> <description> <content-or-@file> [attachment-path...]".to_string())?;
            let description = args
                .get(2)
                .ok_or_else(|| "usage: operit2 skill create <skill-id> <description> <content-or-@file> [attachment-path...]".to_string())?;
            let contentArg = args
                .get(3)
                .ok_or_else(|| "usage: operit2 skill create <skill-id> <description> <content-or-@file> [attachment-path...]".to_string())?;
            let content = read_skill_content_arg(contentArg)?;
            let attachmentPaths = args[4..].iter().map(PathBuf::from).collect::<Vec<_>>();
            println!(
                "{}",
                core.skill_repository()
                    .importSkillFromDirectInput(skillId, description, &content, &attachmentPaths)
                    .await
                    .map_err(|error| error.to_string())?
            );
        }
        "import-zip" => {
            let zipPath = args.get(1).ok_or_else(|| {
                "usage: operit2 skill import-zip <zip-path> [sub-dir-in-zip]".to_string()
            })?;
            let subDir = args.get(2).map(String::as_str);
            let result = match subDir {
                Some(subDir) => {
                    core.skill_repository()
                        .importSkillFromZipWithSubDir(Path::new(zipPath), Some(subDir))
                        .await
                }
                None => {
                    core.skill_repository()
                        .importSkillFromZip(Path::new(zipPath))
                        .await
                }
            }
            .map_err(|error| error.to_string())?;
            println!("{result}");
        }
        "delete" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 skill delete <name>".to_string())?;
            if core
                .skill_repository()
                .deleteSkill(name)
                .await
                .map_err(|error| error.to_string())?
            {
                println!("deleted: {name}");
            } else {
                return Err(format!("skill not found: {name}"));
            }
        }
        "visible" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 skill visible <name> [true|false]".to_string())?;
            if args.len() == 2 {
                println!(
                    "{}",
                    core.skill_repository()
                        .isSkillVisibleToAi(name)
                        .await
                        .map_err(|error| error.to_string())?
                );
            } else {
                let visible = parse_bool_arg(
                    args.get(2),
                    "usage: operit2 skill visible <name> [true|false]",
                )?;
                core.skill_repository()
                    .setSkillVisibleToAi(name, visible)
                    .await
                    .map_err(|error| error.to_string())?;
                println!("visible: {name}={visible}");
            }
        }
        "errors" => {
            for (name, error) in core
                .skill_repository()
                .getSkillLoadErrors()
                .await
                .map_err(|error| error.to_string())?
            {
                println!("{name}\t{error}");
            }
        }
        _ => print_skill_usage(),
    }
    Ok(())
}

pub(super) fn read_skill_content_arg(value: &str) -> Result<String, String> {
    if let Some(path) = value.strip_prefix('@') {
        return fs::read_to_string(path).map_err(|error| error.to_string());
    }
    Ok(value.to_string())
}
