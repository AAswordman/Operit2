use crate::commands::util::{parse_i32_arg, parse_on_off_arg};
use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::data::preferences::ApiPreferences::ApiPreferences;

pub fn run_prefs_command(
    _context: OperitApplicationContext,
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
            output.push_stdout_line(format!("enableThinkingMode={enabled}"));
            Ok(())
        }
        "thinking-quality" => {
            let level = parse_i32_arg(args.get(1), "usage: operit2 prefs thinking-quality <1-4>")?;
            preferences
                .saveThinkingQualityLevel(level)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!("thinkingQualityLevel={}", level.clamp(1, 4)));
            Ok(())
        }
        "stream" => {
            let enabled = parse_on_off_arg(args.get(1), "usage: operit2 prefs stream <on|off>")?;
            preferences
                .saveDisableStreamOutput(!enabled)
                .map_err(|error| error.to_string())?;
            output.push_stdout_line(format!(
                "streamOutput={}",
                if enabled { "on" } else { "off" }
            ));
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
            output.push_stdout_line(format!(
                "maxImageHistoryUserTurns={maxImageHistoryUserTurns}"
            ));
            output.push_stdout_line(format!(
                "maxMediaHistoryUserTurns={maxMediaHistoryUserTurns}"
            ));
            Ok(())
        }
        _ => {
            print_prefs_usage(output);
            Ok(())
        }
    }
}

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
    output.push_stdout_line(format!("enableThinkingMode={enableThinkingMode}"));
    output.push_stdout_line(format!("thinkingQualityLevel={thinkingQualityLevel}"));
    output.push_stdout_line(format!(
        "streamOutput={}",
        if disableStreamOutput { "off" } else { "on" }
    ));
    output.push_stdout_line(format!(
        "maxImageHistoryUserTurns={maxImageHistoryUserTurns}"
    ));
    output.push_stdout_line(format!(
        "maxMediaHistoryUserTurns={maxMediaHistoryUserTurns}"
    ));
    Ok(())
}

fn print_prefs_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 prefs show");
    output.push_stdout_line("operit2 prefs thinking <on|off>");
    output.push_stdout_line("operit2 prefs thinking-quality <1-4>");
    output.push_stdout_line("operit2 prefs stream <on|off>");
    output.push_stdout_line("operit2 prefs media-history <image-user-turns> <media-user-turns>");
}
