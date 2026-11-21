use std::collections::BTreeMap;

use crate::commands::util::read_content_arg;
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::preferences::PreferenceStorageManager::PreferenceStorageManager;
use operit_runtime::data::preferences::ThemeConfigManager::ThemeConfigManager;
use serde_json::{json, Value};

/// Manages saved appearance through its transactional owner, including active-theme updates.
pub fn run_theme_command(
    application: &OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        let usage = [
            "operit2 theme list",
            "operit2 theme show <id>",
            "operit2 theme active",
            "operit2 theme create <name> <snapshot-json|@vfs-file>",
            "operit2 theme update <id> <name> <snapshot-json|@vfs-file>",
            "operit2 theme rename <id> <name>",
            "operit2 theme delete <id>",
            "operit2 theme apply <id>",
            "operit2 theme appearance",
            "operit2 theme appearance-save <snapshot-json|@vfs-file>",
            "operit2 theme appearance-patch <initial-snapshot-json|@vfs-file> <preference-values-json|@vfs-file> [remove-keys-json]",
            "operit2 theme custom <snapshot-json|@vfs-file>",
        ];
        for line in usage {
            output.push_stdout_line(line);
        }
        output.setJsonStdout(json!({"usage": usage}));
        return Ok(());
    }
    let storage = application
        .hostManager
        .runtimeStorageHost
        .clone()
        .ok_or("Theme settings require runtime storage")?;
    let manager = ThemeConfigManager::new(storage);
    let value = match args {
        [command] if command == "list" => {
            json!(manager.list().map_err(|error| error.to_string())?)
        }
        [command] if command == "active" => {
            json!(manager.getActive().map_err(|error| error.to_string())?)
        }
        [command, id] if command == "show" => {
            json!(manager.get(id.clone()).map_err(|error| error.to_string())?)
        }
        [command, name, input] if command == "create" => json!(manager
            .create(name.clone(), snapshot(application, input)?)
            .map_err(|error| error.to_string())?),
        [command, id, name, input] if command == "update" => json!(manager
            .update(id.clone(), name.clone(), snapshot(application, input)?)
            .map_err(|error| error.to_string())?),
        [command, id, name] if command == "rename" => json!(manager
            .rename(id.clone(), name.clone())
            .map_err(|error| error.to_string())?),
        [command, id] if command == "delete" => {
            manager
                .delete(id.clone())
                .map_err(|error| error.to_string())?;
            json!({"id": id, "deleted": true})
        }
        [command, id] if command == "apply" => json!(manager
            .apply(id.clone())
            .map_err(|error| error.to_string())?),
        [command] if command == "appearance" => json!(PreferenceStorageManager::getInstance()
            .preferencesFlow("user_preferences.preferences.json")
            .map_err(|error| error.to_string())?
            .first()
            .map_err(|error| error.to_string())?),
        [command, input] if command == "appearance-save" => json!(manager
            .saveAppearance(snapshot(application, input)?)
            .map_err(|error| error.to_string())?),
        [command, initial, values, rest @ ..]
            if command == "appearance-patch" && rest.len() <= 1 =>
        {
            let remove = match rest.first() {
                Some(input) => {
                    serde_json::from_str(&read_content_arg(&application.hostManager, input)?)
                        .map_err(|error| error.to_string())?
                }
                None => Vec::new(),
            };
            json!(manager
                .patchAppearance(
                    snapshot(application, initial)?,
                    snapshot(application, values)?,
                    remove
                )
                .map_err(|error| error.to_string())?)
        }
        [command, input] if command == "custom" => {
            manager
                .useCustomAppearance(snapshot(application, input)?)
                .map_err(|error| error.to_string())?;
            json!({"customAppearance": true})
        }
        _ => return Err("Invalid theme arguments; use 'operit2 theme'".into()),
    };
    output
        .push_stdout_line(serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?);
    output.setJsonStdout(value);
    Ok(())
}

/// Decodes an explicit JSON object through the invocation's VFS file reader.
fn snapshot(
    application: &OperitApplication,
    input: &str,
) -> Result<BTreeMap<String, Value>, String> {
    serde_json::from_str(&read_content_arg(&application.hostManager, input)?)
        .map_err(|error| error.to_string())
}
