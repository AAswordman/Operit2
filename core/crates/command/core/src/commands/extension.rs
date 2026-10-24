use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_store::ExtensionStore::{ExtensionRecord, ExtensionStore};
use serde_json::{json, Value};

const USAGE: &str = "usage: operit2 extension <list [package|plugin|skill|mcp] [--scope device|space|builtin]|show <kind> <id>|move <kind> <id> <device|space> --yes>";

#[derive(Debug, PartialEq)]
enum Command {
    Help,
    List {
        kind: Option<String>,
        scope: Option<String>,
    },
    Show {
        kind: String,
        id: String,
    },
    Move {
        kind: String,
        id: String,
        scope: String,
        confirmed: bool,
    },
}

fn kind(value: &str) -> Result<String, String> {
    match value {
        "plugin" | "package" => Ok("package".into()),
        "skill" | "mcp" => Ok(value.into()),
        _ => Err(format!(
            "Unknown extension kind: {value}; expected package, plugin, skill or mcp"
        )),
    }
}

fn scope(value: &str, allow_builtin: bool) -> Result<String, String> {
    match value {
        "device" | "space" => Ok(value.into()),
        "builtin" if allow_builtin => Ok(value.into()),
        _ => Err(format!(
            "Invalid extension scope: {value}; expected device or space{}",
            if allow_builtin { " or builtin" } else { "" }
        )),
    }
}

fn parse(args: &[String]) -> Result<Command, String> {
    match args.first().map(String::as_str) {
        None => Ok(Command::Help),
        Some("help" | "-h" | "--help") if args.len() == 1 => Ok(Command::Help),
        Some("list") => {
            let mut selected_kind = None;
            let mut selected_scope = None;
            let mut index = 1;
            while index < args.len() {
                if args[index] == "--scope" && selected_scope.is_none() {
                    index += 1;
                    selected_scope = Some(scope(args.get(index).ok_or(USAGE)?, true)?);
                } else if selected_kind.is_none() {
                    selected_kind = Some(kind(&args[index])?);
                } else {
                    return Err(USAGE.into());
                }
                index += 1;
            }
            Ok(Command::List {
                kind: selected_kind,
                scope: selected_scope,
            })
        }
        Some("show") if args.len() == 3 && !args[2].trim().is_empty() => Ok(Command::Show {
            kind: kind(&args[1])?,
            id: args[2].clone(),
        }),
        Some("move")
            if (args.len() == 4 || (args.len() == 5 && args[4] == "--yes"))
                && !args[2].trim().is_empty() =>
        {
            Ok(Command::Move {
                kind: kind(&args[1])?,
                id: args[2].clone(),
                scope: scope(&args[3], false)?,
                confirmed: args.len() == 5,
            })
        }
        _ => Err(USAGE.into()),
    }
}

/// Manages extension ownership through the same application facade used by Flutter.
pub fn run_extension_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let command = parse(args)?;
    if command == Command::Help {
        let lines = [USAGE, "plugin is an alias for package (ToolPkg containers and their subpackages share one scope).", "Moving to space shares content, settings and secrets with the device space.", "Moving to device removes the shared installation from the other devices.", "Changes require --yes; use --json for machine-readable output.", "Aliases: operit2 <plugin|package|skill|mcp> scope <id> [device|space --yes]"];
        for line in lines {
            output.push_stdout_line(line);
        }
        output.setJsonStdout(json!({"usage": lines}));
        return Ok(());
    }
    let storage = application
        .hostManager
        .runtimeStorageHost
        .clone()
        .ok_or("Runtime storage is unavailable")?;
    let store = ExtensionStore::new(storage);
    match command {
        Command::List { kind, scope } => {
            let kinds = kind
                .map(|kind| vec![kind])
                .unwrap_or_else(|| vec!["package".into(), "skill".into(), "mcp".into()]);
            let mut rows = Vec::new();
            for kind in kinds {
                // Scans Skills and validates unique ownership without inventing a CLI-only catalog.
                application.getExtensionScopes(kind.clone())?;
                for record in store.records(&kind)? {
                    let row = summary(&record);
                    if scope
                        .as_ref()
                        .is_none_or(|scope| row["scope"].as_str() == Some(scope))
                    {
                        rows.push(row);
                    }
                }
            }
            rows.sort_by_key(|row| {
                (
                    row["kind"].as_str().unwrap_or("").to_string(),
                    row["id"].as_str().unwrap_or("").to_string(),
                )
            });
            output.push_stdout_line(format!("Extensions: {}", rows.len()));
            for row in &rows {
                output.push_stdout_line(format!(
                    "{}\t{}\t{}",
                    row["kind"].as_str().unwrap_or(""),
                    row["id"].as_str().unwrap_or(""),
                    row["scope"].as_str().unwrap_or("")
                ));
            }
            output.setJsonStdout(json!({"extensions": rows}));
        }
        Command::Show { kind, id } => {
            let record = find_record(application, &store, &kind, &id)?;
            let row = summary(&record);
            output.push_stdout_line(format!(
                "{} {}: {}",
                kind,
                record.id,
                display_scope(&record)
            ));
            output.push_stdout_line(format!("Source: {}", record.sourceName));
            output.setJsonStdout(row);
        }
        Command::Move {
            kind,
            id,
            scope,
            confirmed,
        } => {
            let record = find_record(application, &store, &kind, &id)?;
            let previous = display_scope(&record).to_string();
            if previous == "builtin" {
                return Err("Built-in extensions cannot change scope".into());
            }
            if previous != scope && !confirmed {
                let warning = if scope == "space" {
                    "Content, settings and secrets will be shared with the device space."
                } else {
                    "The shared installation and its configuration will be removed from the other devices."
                };
                return Err(format!(
                    "{warning} Repeat the command with --yes to confirm."
                ));
            }
            application.setExtensionScope(kind.clone(), id, scope.clone())?;
            let updated = store.record(&kind, &record.id)?;
            output.push_stdout_line(format!(
                "{} {}: {} -> {}",
                kind, updated.id, previous, updated.scope
            ));
            output.setJsonStdout(json!({"kind": kind, "id": updated.id, "previousScope": previous, "scope": updated.scope, "changed": previous != scope}));
        }
        Command::Help => unreachable!(),
    }
    Ok(())
}

fn find_record(
    application: &OperitApplication,
    store: &ExtensionStore,
    kind: &str,
    id: &str,
) -> Result<ExtensionRecord, String> {
    let scopes = application.getExtensionScopes(kind.into())?;
    if !scopes.contains_key(id) {
        return Err(format!("Extension not found: {kind}:{id}"));
    }
    store
        .records(kind)?
        .into_iter()
        .find(|record| {
            record.id == id
                || (kind == "package"
                    && record.settings["members"]
                        .as_array()
                        .is_some_and(|members| {
                            members.iter().any(|member| member.as_str() == Some(id))
                        }))
        })
        .ok_or_else(|| format!("Extension not found: {kind}:{id}"))
}

fn display_scope(record: &ExtensionRecord) -> &str {
    if record.settings["builtin"].as_bool() == Some(true) {
        "builtin"
    } else {
        &record.scope
    }
}

/// Do not print configuration values, credentials or embedded legacy content in a management list.
fn summary(record: &ExtensionRecord) -> Value {
    let mut value = json!({"kind": record.kind, "id": record.id, "scope": display_scope(record), "sourceName": record.sourceName});
    match record.kind.as_str() {
        "skill" => value["visible"] = record.settings["visible"].clone(),
        "package" => {
            for key in [
                "members",
                "enabledNames",
                "disabledNames",
                "subpackageStates",
            ] {
                value[key] = record.settings[key].clone();
            }
        }
        "mcp" => {
            value["enabled"] = json!(!record.settings["server"]["disabled"]
                .as_bool()
                .unwrap_or(false));
            value["deployment"] = json!(if record.settings["server"]["command"]
                .as_str()
                .is_some_and(|command| !command.trim().is_empty())
            {
                "local"
            } else {
                "remote"
            });
        }
        _ => {}
    }
    value
}

/// Adds a discoverable shortcut to the existing plugin/package/skill/MCP command families.
pub fn run_scope_command(
    application: &OperitApplication,
    family: &str,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let mut command = match args.len() {
        1 => vec!["show".into(), family.into()],
        2 | 3 => vec!["move".into(), family.into()],
        _ => {
            return Err(format!(
                "usage: operit2 {family} scope <id> [device|space --yes]"
            ))
        }
    };
    command.extend_from_slice(args);
    run_extension_command(application, &command, output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }
    #[test]
    fn parses_filters_and_plugin_alias() {
        assert_eq!(
            parse(&args(&["list", "plugin", "--scope", "builtin"])).unwrap(),
            Command::List {
                kind: Some("package".into()),
                scope: Some("builtin".into())
            }
        );
        assert_eq!(
            parse(&args(&["move", "skill", "demo", "space", "--yes"])).unwrap(),
            Command::Move {
                kind: "skill".into(),
                id: "demo".into(),
                scope: "space".into(),
                confirmed: true
            }
        );
    }
    #[test]
    fn rejects_invalid_or_extra_arguments_before_any_mutation() {
        for values in [
            vec!["move", "skill", "demo", "builtin", "--yes"],
            vec!["move", "unknown", "demo", "space", "--yes"],
            vec!["move", "skill", "demo", "space", "--force"],
            vec!["show", "skill", "demo", "extra"],
            vec!["list", "--scope"],
            vec!["list", "--scope", "space", "--scope", "device"],
        ] {
            assert!(parse(&args(&values)).is_err());
        }
    }
    #[test]
    fn summaries_never_expose_mcp_secrets_or_legacy_file_contents() {
        let record = ExtensionRecord {
            schemaVersion: 1,
            kind: "mcp".into(),
            id: "remote".into(),
            scope: "device".into(),
            sourceName: "".into(),
            settings: json!({"server": {"url": "https://secret@example.test", "headers": {"Authorization": "secret"}, "env": {"TOKEN": "secret"}}, "metadata": {"private": "secret"}}),
            files: std::collections::BTreeMap::from([("secret".into(), "secret".into())]),
        };
        let row = summary(&record);
        assert!(!row.to_string().contains("secret"));
        assert_eq!(row["scope"], "device");
    }
}
