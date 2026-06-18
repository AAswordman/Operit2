use std::env;
use std::process::ExitCode;

use operit_runtime::util::AppLogger::AppLogger;

mod access;
mod bootstrap;
mod chat_runtime;
mod cli;
mod client_paths;
mod core_proxy;
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
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    AppLogger::set_enable_console_logging(false);
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        return tui::run_tui_command(&[]).await;
    }

    match args[0].as_str() {
        "help" | "-h" | "--help" => {
            cli::print_root_usage();
            Ok(())
        }
        "cli" => cli::run_cli_root(&args[1..]).await,
        "tui" => tui::run_tui_command(&args[1..]).await,
        value if value.starts_with('-') => tui::run_tui_command(&args).await,
        _ => {
            cli::print_root_usage();
            Ok(())
        }
    }
}
