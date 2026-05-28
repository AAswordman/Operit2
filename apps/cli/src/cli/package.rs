use super::core::CliCore;
use super::*;

pub(super) async fn run_package_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_package_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "dir" => {
            println!(
                "{}",
                core.permissions_pack_tool_package_manager()
                    .getExternalPackagesPath()
                    .await
                    .map_err(|error| error.to_string())?
            );
        }
        "list" => {
            let enabled = core
                .permissions_pack_tool_package_manager()
                .getEnabledPackageNames()
                .await
                .map_err(|error| error.to_string())?;
            for (name, package) in core
                .permissions_pack_tool_package_manager()
                .getAvailablePackages()
                .await
                .map_err(|error| error.to_string())?
            {
                println!(
                    "{}\tenabled={}\t{}\ttools={}",
                    name,
                    enabled.contains(&name),
                    package.description.resolve(false),
                    package.tools.len()
                );
            }
        }
        "show" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 package show <name>".to_string())?;
            let package = core
                .permissions_pack_tool_package_manager()
                .getPackageTools(name)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("package not found: {name}"))?;
            println!("name={}", package.name);
            println!("displayName={}", package.display_name.resolve(false));
            println!("description={}", package.description.resolve(false));
            println!("category={}", package.category);
            println!("enabledByDefault={}", package.enabled_by_default);
            println!("isBuiltIn={}", package.is_built_in);
            println!("tools={}", package.tools.len());
            for tool in package.tools {
                println!(
                    "- {}\tadvice={}\t{}",
                    tool.name,
                    tool.advice,
                    tool.description.resolve(false)
                );
                for parameter in tool.parameters {
                    println!(
                        "  - {}\t{}\trequired={}\t{}",
                        parameter.name,
                        parameter.parameter_type,
                        parameter.required,
                        parameter.description.resolve(false)
                    );
                }
            }
        }
        "import" => {
            let path = args
                .get(1)
                .ok_or_else(|| "usage: operit2 package import <js-ts-hjson-path>".to_string())?;
            println!(
                "{}",
                core.permissions_pack_tool_package_manager()
                    .addPackageFileFromExternalStorage(path)
                    .await
                    .map_err(|error| error.to_string())?
            );
        }
        "enable" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 package enable <name>".to_string())?;
            println!(
                "{}",
                core.permissions_pack_tool_package_manager()
                    .enablePackage(name)
                    .await
                    .map_err(|error| error.to_string())?
            );
        }
        "disable" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 package disable <name>".to_string())?;
            println!(
                "{}",
                core.permissions_pack_tool_package_manager()
                    .disablePackage(name)
                    .await
                    .map_err(|error| error.to_string())?
            );
        }
        "use" => {
            let name = args
                .get(1)
                .ok_or_else(|| "usage: operit2 package use <name>".to_string())?;
            println!(
                "{}",
                core.permissions_pack_tool_package_manager()
                    .usePackage(name)
                    .await
                    .map_err(|error| error.to_string())?
            );
        }
        "exec" => {
            let toolName = args.get(1).ok_or_else(|| {
                "usage: operit2 package exec <package:tool> <params-json>".to_string()
            })?;
            let paramsJson = args.get(2).ok_or_else(|| {
                "usage: operit2 package exec <package:tool> <params-json>".to_string()
            })?;
            let packageName = toolName
                .split_once(':')
                .map(|(packageName, _)| packageName.to_string())
                .ok_or_else(|| "package exec tool name must use package:tool format".to_string())?;

            let _ = core
                .permissions_pack_tool_package_manager()
                .usePackage(&packageName)
                .await
                .map_err(|error| error.to_string())?;

            let parsedParams =
                serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(paramsJson)
                    .map_err(|error| error.to_string())?;
            let parameters = parsedParams
                .into_iter()
                .map(|(name, value)| {
                    let value = match value {
                        serde_json::Value::String(value) => value,
                        other => other.to_string(),
                    };
                    ToolParameter { name, value }
                })
                .collect::<Vec<_>>();
            let tool = AITool {
                name: toolName.clone(),
                parameters,
            };
            let result = core
                .permissions_ai_tool_handler()
                .executeTool(tool)
                .await
                .map_err(|error| error.to_string())?;
            if result.success {
                println!("{}", result.result);
            } else {
                return Err(result
                    .error
                    .unwrap_or_else(|| "package exec failed".to_string()));
            }
        }
        _ => print_package_usage(),
    }
    Ok(())
}
