use crate::output::CoreCommandOutput;
use operit_util::AppLogger::AppLogger;

/// Runs log inspection and maintenance commands.
pub fn run_log_command(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    if args.is_empty() {
        print_log_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "show" => {
            let text = AppLogger::text()?;
            output.push_stdout(&text);
            output.setJsonStdout(serde_json::json!({"log": text}));
            Ok(())
        }
        "package" => {
            let text = AppLogger::package_text()?;
            output.push_stdout(&text);
            output.setJsonStdout(serde_json::json!({"packageLog": text}));
            Ok(())
        }
        "path" => {
            let log = AppLogger::get_log_file_path()?;
            let packageLog = AppLogger::get_package_log_file_path()?;
            output.push_stdout_line(format!("Log file: {log}"));
            output.push_stdout_line(format!("Package log file: {packageLog}"));
            output.setJsonStdout(serde_json::json!({
                "log": log,
                "packageLog": packageLog,
            }));
            Ok(())
        }
        "clear" => {
            AppLogger::reset_log_file();
            output.push_stdout_line("Logs cleared.");
            output.setJsonStdout(serde_json::json!({"cleared": true}));
            Ok(())
        }
        _ => {
            print_log_usage(output);
            Ok(())
        }
    }
}

/// Prints log command usage.
fn print_log_usage(output: &mut CoreCommandOutput) {
    let lines = vec!["operit2 log <show|package|path|clear>"];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}
