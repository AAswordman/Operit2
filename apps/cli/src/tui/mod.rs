mod app;
mod commands;
mod helpers;
mod input;
mod markdown;
mod render;

use app::OperitTui;

use crate::{create_cli_application, initialize_shell_chat, parse_shell_args};

pub(crate) async fn run_tui_command(args: &[String]) -> Result<(), String> {
    let shell_args = parse_shell_args(args)?;
    let mut application = create_cli_application();
    application.onCreate()?;
    let initial_chat_id = initialize_shell_chat(&mut application, &shell_args)?;
    let mut tui = OperitTui::new(application, shell_args, initial_chat_id)?;
    tui.run().await
}
