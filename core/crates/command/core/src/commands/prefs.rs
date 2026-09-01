use crate::commands::util::{parse_i32_arg, parse_on_off_arg};
use crate::output::CoreCommandOutput;
use operit_host_api::HostManager::HostManager;
use operit_runtime::data::preferences::ApiPreferences::ApiPreferences;

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
    match args[0].as_str() {
        "show" => print_api_preferences(&preferences, output),
        "thinking" => {
            let enabled = parse_on_off_arg(args.get(1), "usage: operit2 prefs thinking <on|off>")?;
            preferences
                .saveEnableThinkingMode(enabled)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("Thinking mode: {}", on_off(enabled)));
            output.setJsonStdout(serde_json::json!({
                "enableThinkingMode": enabled,
                "updated": true,
            }));
            Ok(())
        }
        "thinking-quality" => {
            let level = parse_i32_arg(args.get(1), "usage: operit2 prefs thinking-quality <1-4>")?;
            preferences
                .saveThinkingQualityLevel(level)
                .map_err(|error| error.to_string())?;
            let clampedLevel = level.clamp(1, 4);
            output.push_stdout_line(format!("Thinking quality: {clampedLevel}"));
            output.setJsonStdout(serde_json::json!({
                "thinkingQualityLevel": clampedLevel,
                "updated": true,
            }));
            Ok(())
        }
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
    let enableThinkingMode = preferences
        .enableThinkingModeFlow()
        .first()
        .map_err(|error| error.to_string())?;
    let thinkingQualityLevel = preferences
        .thinkingQualityLevelFlow()
        .first()
        .map_err(|error| error.to_string())?;
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
    output.push_stdout_line(format!("Thinking mode: {}", on_off(enableThinkingMode)));
    output.push_stdout_line(format!("Thinking quality: {thinkingQualityLevel}"));
    output.push_stdout_line(format!("Stream output: {}", on_off(streamOutputEnabled)));
    output.push_stdout_line(format!("Image history turns: {maxImageHistoryUserTurns}"));
    output.push_stdout_line(format!("Media history turns: {maxMediaHistoryUserTurns}"));
    output.push_stdout_line(format!("MCP startup timeout: {mcpStartupTimeoutSeconds}s"));
    output.setJsonStdout(serde_json::json!({
        "enableThinkingMode": enableThinkingMode,
        "thinkingQualityLevel": thinkingQualityLevel,
        "streamOutput": on_off(streamOutputEnabled),
        "streamOutputEnabled": streamOutputEnabled,
        "maxImageHistoryUserTurns": maxImageHistoryUserTurns,
        "maxMediaHistoryUserTurns": maxMediaHistoryUserTurns,
        "mcpStartupTimeoutSeconds": mcpStartupTimeoutSeconds,
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
        "operit2 prefs thinking <on|off>",
        "operit2 prefs thinking-quality <1-4>",
        "operit2 prefs stream <on|off>",
        "operit2 prefs media-history <image-user-turns> <media-user-turns>",
        "operit2 prefs mcp-timeout <seconds>",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}
