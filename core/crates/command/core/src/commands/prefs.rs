use crate::commands::util::{parse_i32_arg, parse_on_off_arg};
use crate::output::CoreCommandOutput;
use operit_host_api::HostManager::HostManager;
use operit_runtime::data::preferences::ApiPreferences::ApiPreferences;
use operit_runtime::data::preferences::PreferenceStorageManager::PreferenceStorageManager;
use operit_runtime::data::preferences::UserPreferencesManager::UserPreferencesManager;
use serde_json::json;
use std::collections::BTreeMap;

/// Runs preference display and update commands.
pub fn run_prefs_command(
    _context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_prefs_usage(output);
        return Ok(());
    }

    let preferences = ApiPreferences::getInstance();
    if let Some(result) = run_extended_preferences(&preferences, args, output) {
        return result;
    }
    match args[0].as_str() {
        "show" => print_api_preferences(&preferences, output),
        "stream" => {
            let enabled = parse_on_off_arg(args.get(1), "usage: operit2 prefs stream <on|off>")?;
            preferences
                .saveDisableStreamOutput(!enabled)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Stream output: {}", on_off(enabled)));
            output.setJsonStdout(serde_json::json!({
                "streamOutput": on_off(enabled),
                "streamOutputEnabled": enabled,
                "updated": true,
            }));
            Ok(())
        }
        "media-history" => {
            let maxImageHistoryUserTurns = parse_i32_arg(
                args.get(1),
                "usage: operit2 prefs media-history <image-user-turns> <media-user-turns>",
            )?;
            let maxMediaHistoryUserTurns = parse_i32_arg(
                args.get(2),
                "usage: operit2 prefs media-history <image-user-turns> <media-user-turns>",
            )?;
            preferences
                .updateMediaHistorySettings(maxImageHistoryUserTurns, maxMediaHistoryUserTurns)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Image history turns: {maxImageHistoryUserTurns}"));
            output.push_stdout_line(format!("Media history turns: {maxMediaHistoryUserTurns}"));
            output.setJsonStdout(serde_json::json!({
                "maxImageHistoryUserTurns": maxImageHistoryUserTurns,
                "maxMediaHistoryUserTurns": maxMediaHistoryUserTurns,
                "updated": true,
            }));
            Ok(())
        }
        "mcp-timeout" => {
            let seconds = parse_i32_arg(args.get(1), "usage: operit2 prefs mcp-timeout <seconds>")?;
            if seconds < 1 {
                return Err("mcp-timeout seconds must be at least 1".to_string());
            }
            preferences
                .saveMcpStartupTimeoutSeconds(seconds)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("MCP startup timeout: {seconds}s"));
            output.setJsonStdout(serde_json::json!({
                "mcpStartupTimeoutSeconds": seconds,
                "updated": true,
            }));
            Ok(())
        }
        _ => {
            print_prefs_usage(output);
            Ok(())
        }
    }
}

/// Prints all API preference values.
fn print_api_preferences(
    preferences: &ApiPreferences,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let disableStreamOutput = preferences
        .disableStreamOutputFlow()
        .first()
        .map_err(|error| error.to_string())?;
    let maxImageHistoryUserTurns = preferences
        .maxImageHistoryUserTurnsFlow()
        .first()
        .map_err(|error| error.to_string())?;
    let maxMediaHistoryUserTurns = preferences
        .maxMediaHistoryUserTurnsFlow()
        .first()
        .map_err(|error| error.to_string())?;
    let mcpStartupTimeoutSeconds = preferences
        .mcpStartupTimeoutSecondsFlow()
        .first()
        .map_err(|error| error.to_string())?;
    let streamOutputEnabled = !disableStreamOutput;
    let toolPkgPreHookTimeoutSeconds = preferences
        .getToolPkgPreHookTimeoutSeconds()
        .map_err(|error| error.to_string())?;
    output.push_stdout_line(format!("Stream output: {}", on_off(streamOutputEnabled)));
    output.push_stdout_line(format!("Image history turns: {maxImageHistoryUserTurns}"));
    output.push_stdout_line(format!("Media history turns: {maxMediaHistoryUserTurns}"));
    output.push_stdout_line(format!("MCP startup timeout: {mcpStartupTimeoutSeconds}s"));
    output.setJsonStdout(serde_json::json!({
        "streamOutput": on_off(streamOutputEnabled),
        "streamOutputEnabled": streamOutputEnabled,
        "maxImageHistoryUserTurns": maxImageHistoryUserTurns,
        "maxMediaHistoryUserTurns": maxMediaHistoryUserTurns,
        "mcpStartupTimeoutSeconds": mcpStartupTimeoutSeconds,
        "toolPkgPreHookTimeoutSeconds": toolPkgPreHookTimeoutSeconds,
        "enableTools": preferences.enableToolsFlow().first().map_err(|error| error.to_string())?,
        "disableUserPreferenceDescription": preferences.disableUserPreferenceDescriptionFlow().first().map_err(|error| error.to_string())?,
        "featureToggles": preferences.featureTogglesFlow().first().map_err(|error| error.to_string())?,
        "toolPromptVisibility": preferences.getToolPromptVisibilityMap().map_err(|error| error.to_string())?,
        "language": UserPreferencesManager::getInstance().getCurrentLanguage().map_err(|error| error.to_string())?,
    }));
    Ok(())
}

/// Formats a boolean switch as CLI text.
fn on_off(enabled: bool) -> &'static str {
    if enabled {
        "on"
    } else {
        "off"
    }
}

/// Prints preference command usage.
fn print_prefs_usage(output: &mut CoreCommandOutput) {
    let lines = vec![
        "operit2 prefs show",
        "operit2 prefs stream <on|off>",
        "operit2 prefs media-history <image-user-turns> <media-user-turns>",
        "operit2 prefs mcp-timeout <seconds>",
        "operit2 prefs pre-hook-timeout <seconds>",
        "operit2 prefs tools <on|off>",
        "operit2 prefs preference-description <on|off>",
        "operit2 prefs feature <key> <on|off>",
        "operit2 prefs tool-visible <name> <on|off>",
        "operit2 prefs language [system|zh-CN|en|ko|es|ms|id|pt-BR]",
        "operit2 prefs input [enter-send <on|off|auto>|long-paste <on|off> <1000..20000>]",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}

/// Routes missing preference controls to the same typed stores used by application settings.
fn run_extended_preferences(
    preferences: &ApiPreferences,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Option<Result<(), String>> {
    if !matches!(
        args[0].as_str(),
        "pre-hook-timeout"
            | "tools"
            | "preference-description"
            | "feature"
            | "tool-visible"
            | "language"
            | "input"
    ) {
        return None;
    }
    Some((|| {
        let store = PreferenceStorageManager::getInstance();
        let user_file = "user_preferences.preferences.json";
        let value = match args {
            [command, seconds] if command == "pre-hook-timeout" => {
                let seconds: i32 = seconds
                    .parse()
                    .map_err(|error: std::num::ParseIntError| error.to_string())?;
                if seconds < 1 {
                    return Err("pre-hook-timeout seconds must be at least 1".into());
                }
                preferences
                    .saveToolPkgPreHookTimeoutSeconds(seconds)
                    .map_err(|error| error.to_string())?;
                json!({"toolPkgPreHookTimeoutSeconds": seconds})
            }
            [command, enabled] if command == "tools" || command == "preference-description" => {
                let enabled = parse_on_off_arg(Some(enabled), "expected on | off")?;
                if command == "tools" {
                    preferences.saveEnableTools(enabled)
                } else {
                    preferences.saveDisableUserPreferenceDescription(!enabled)
                }
                .map_err(|error| error.to_string())?;
                json!({"setting": command, "enabled": enabled})
            }
            [command, key, enabled] if command == "feature" || command == "tool-visible" => {
                if key.trim().is_empty() {
                    return Err("Preference key must not be blank".into());
                }
                let enabled = parse_on_off_arg(Some(enabled), "expected on | off")?;
                if command == "feature" {
                    preferences.saveFeatureToggle(key, enabled)
                } else {
                    preferences.saveToolPromptVisibility(key, enabled)
                }
                .map_err(|error| error.to_string())?;
                json!({"key": key, "enabled": enabled})
            }
            [command] if command == "language" => {
                json!({"language": UserPreferencesManager::getInstance().getCurrentLanguage().map_err(|error| error.to_string())?})
            }
            [command, language] if command == "language" => {
                if !operit_util::LocaleUtils::LocaleUtils::get_supported_languages()
                    .iter()
                    .any(|entry| entry.code == *language)
                {
                    return Err("Unknown language code; use 'operit2 prefs'".into());
                }
                UserPreferencesManager::getInstance()
                    .saveAppLanguage(language.clone())
                    .map_err(|error| error.to_string())?;
                json!({"language": language})
            }
            [command] if command == "input" => json!(store
                .getPreferences(
                    user_file,
                    vec![
                        "enter_sends_message".into(),
                        "long_pasted_text_input_enabled".into(),
                        "long_pasted_text_input_threshold".into()
                    ]
                )
                .map_err(|error| error.to_string())?),
            [command, setting, enabled] if command == "input" && setting == "enter-send" => {
                if enabled == "auto" {
                    store.removePreference(user_file, "enter_sends_message")
                } else {
                    store.setPreference(
                        user_file,
                        "enter_sends_message",
                        json!(parse_on_off_arg(Some(enabled), "expected on | off | auto")?),
                    )
                }
                .map_err(|error| error.to_string())?;
                json!({"enterSend": enabled})
            }
            [command, setting, enabled, threshold]
                if command == "input" && setting == "long-paste" =>
            {
                let enabled = parse_on_off_arg(Some(enabled), "expected on | off")?;
                let threshold: i32 = threshold
                    .parse()
                    .map_err(|error: std::num::ParseIntError| error.to_string())?;
                if !(1000..=20000).contains(&threshold) {
                    return Err("Long-paste threshold must be between 1000 and 20000".into());
                }
                let values = BTreeMap::from([
                    ("long_pasted_text_input_enabled".into(), json!(enabled)),
                    ("long_pasted_text_input_threshold".into(), json!(threshold)),
                ]);
                store
                    .setPreferences(user_file, values.clone())
                    .map_err(|error| error.to_string())?;
                json!(values)
            }
            _ => return Err("Invalid preference arguments; use 'operit2 prefs'".into()),
        };
        output.push_stdout_line(
            serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?,
        );
        output.setJsonStdout(value);
        Ok(())
    })())
}
