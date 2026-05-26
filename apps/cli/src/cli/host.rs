use super::core::CliCore;
use super::*;

pub(super) async fn run_host_command(core: &mut CliCore, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_host_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "show" => {
            println!("targetOs={}", std::env::consts::OS);
            println!("targetArch={}", std::env::consts::ARCH);
            println!(
                "coreVersion={}",
                core.application()
                    .coreVersion()
                    .await
                    .map_err(|error| error.to_string())?
            );
            Ok(())
        }
        "capabilities" => Err("host capabilities are not exposed by proxy core schema".to_string()),
        "paths" => Err("host paths are not exposed by proxy core schema".to_string()),
        _ => {
            print_host_usage();
            Ok(())
        }
    }
}
