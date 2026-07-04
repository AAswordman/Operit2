use std::env;
use std::process::ExitCode;

use error::CliError;
use operit_runtime::util::AppLogger::AppLogger;

mod access;
mod bootstrap;
mod chat_runtime;
mod cli;
mod client_paths;
mod core_proxy;
mod error;
mod mdns;
mod tui;
mod web_access_assets;

pub(crate) use bootstrap::create_local_core;
pub(crate) use chat_runtime::{
    build_attachment_info, guess_mime_type, initialize_shell_chat, parse_shell_args, ChatSendArgs,
    ShellArgs,
};

#[tokio::main]
async fn main() -> ExitCode {
    error::install_panic_hook();
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), CliError> {
    AppLogger::set_enable_console_logging(false);
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        return tui::run_tui_command(&[]).await.map_err(CliError::internal);
    }

    match args[0].as_str() {
        "help" | "-h" | "--help" => {
            cli::print_root_usage();
            Ok(())
        }
        "cli" => cli::run_cli_root(&args[1..]).await.map_err(CliError::user),
        "tui" => tui::run_tui_command(&args[1..])
            .await
            .map_err(CliError::internal),
        "install" | "uninstall" => cli::run_cli_root(&args).await.map_err(CliError::user),
        value if value.starts_with('-') => tui::run_tui_command(&args)
            .await
            .map_err(CliError::internal),
        _ => {
            cli::print_root_usage();
            Ok(())
        }
    }
}
