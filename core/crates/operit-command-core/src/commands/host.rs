use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_host_api::HostManager::HostManager;

pub fn run_host_command(
    context: HostManager,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    if args.is_empty() {
        print_host_usage(output);
        return Ok(());
    }

    match args[0].as_str() {
        "show" => {
            output.push_stdout_line(format!("targetOs={}", std::env::consts::OS));
            output.push_stdout_line(format!("targetArch={}", std::env::consts::ARCH));
            output.push_stdout_line(format!(
                "coreVersion={}",
                OperitApplication::newWithContext(context).coreVersion()
            ));
            Ok(())
        }
        "capabilities" => Err("host capabilities are not exposed by core command".to_string()),
        "paths" => Err("host paths are not exposed by core command".to_string()),
        _ => {
            print_host_usage(output);
            Ok(())
        }
    }
}

fn print_host_usage(output: &mut CoreCommandOutput) {
    output.push_stdout_line("operit2 host show");
    output.push_stdout_line("operit2 host capabilities");
    output.push_stdout_line("operit2 host paths");
}
