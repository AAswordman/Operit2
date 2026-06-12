use crate::output::CoreCommandOutput;
use operit_runtime::util::AppLogger::AppLogger;

pub fn run_log_command(args: &[String], output: &mut CoreCommandOutput) -> Result<(), String> {
    if args.is_empty() {
        print_log_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "show" => {
            output.push_stdout(AppLogger::text()?);
            Ok(())
        }
        "package" => {
            output.push_stdout(AppLogger::package_text()?);
            Ok(())
        }
        "path" => {
            output.push_stdout_line(format!("log={}", AppLogger::get_log_file_path()?));
            output.push_stdout_line(format!(
                "packageLog={}",
                AppLogger::get_package_log_file_path()?
            ));
            Ok(())
        }
        "clear" => {
            AppLogger::reset_log_file();
            output.push_stdout_line("logs cleared");
            Ok(())
        }
        _ => {
            print_log_usage(output);
            Ok(())
        }
    }
}

fn print_log_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 log <show|package|path|clear>");
}
