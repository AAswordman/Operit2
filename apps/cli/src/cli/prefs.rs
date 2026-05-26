use super::core::CliCore;
use super::*;

pub(super) async fn run_prefs_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_prefs_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "show" => print_api_preferences(core).await,
        "thinking" => {
            let enabled = parse_on_off_arg(args.get(1), "usage: operit2 prefs thinking <on|off>")?;
            core.preferences_api_preferences()
                .saveEnableThinkingMode(enabled)
                .await
                .map_err(|error| error.to_string())?;
            println!("enableThinkingMode={enabled}");
            Ok(())
        }
        "thinking-quality" => {
            let level = parse_i32_arg(args.get(1), "usage: operit2 prefs thinking-quality <1-4>")?;
            core.preferences_api_preferences()
                .saveThinkingQualityLevel(level)
                .await
                .map_err(|error| error.to_string())?;
            println!("thinkingQualityLevel={}", level.clamp(1, 4));
            Ok(())
        }
        "stream" => {
            let enabled = parse_on_off_arg(args.get(1), "usage: operit2 prefs stream <on|off>")?;
            core.preferences_api_preferences()
                .saveDisableStreamOutput(!enabled)
                .await
                .map_err(|error| error.to_string())?;
            println!("streamOutput={}", if enabled { "on" } else { "off" });
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
            core.preferences_api_preferences()
                .updateMediaHistorySettings(maxImageHistoryUserTurns, maxMediaHistoryUserTurns)
                .await
                .map_err(|error| error.to_string())?;
            println!("maxImageHistoryUserTurns={maxImageHistoryUserTurns}");
            println!("maxMediaHistoryUserTurns={maxMediaHistoryUserTurns}");
            Ok(())
        }
        _ => {
            print_prefs_usage();
            Ok(())
        }
    }
}

async fn print_api_preferences(core: &mut CliCore) -> Result<(), String> {
    let enableThinkingMode = core
        .preferences_api_preferences()
        .enableThinkingMode()
        .await
        .map_err(|error| error.to_string())?;
    let thinkingQualityLevel = core
        .preferences_api_preferences()
        .thinkingQualityLevel()
        .await
        .map_err(|error| error.to_string())?;
    let disableStreamOutput = core
        .preferences_api_preferences()
        .disableStreamOutput()
        .await
        .map_err(|error| error.to_string())?;
    let maxImageHistoryUserTurns = core
        .preferences_api_preferences()
        .maxImageHistoryUserTurns()
        .await
        .map_err(|error| error.to_string())?;
    let maxMediaHistoryUserTurns = core
        .preferences_api_preferences()
        .maxMediaHistoryUserTurns()
        .await
        .map_err(|error| error.to_string())?;
    println!("enableThinkingMode={enableThinkingMode}");
    println!("thinkingQualityLevel={thinkingQualityLevel}");
    println!(
        "streamOutput={}",
        if disableStreamOutput { "off" } else { "on" }
    );
    println!("maxImageHistoryUserTurns={maxImageHistoryUserTurns}");
    println!("maxMediaHistoryUserTurns={maxMediaHistoryUserTurns}");
    Ok(())
}
