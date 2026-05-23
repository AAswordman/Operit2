mod approval;
mod app;
mod commands;
mod empty_state;
mod helpers;
mod input;
mod link_proxy_rs;
mod markdown;
mod render;
mod theme;
mod typewriter;

use app::OperitTui;
use link_proxy_rs::TuiLocalCoreBorrowExt;

use crate::{create_local_core, initialize_shell_chat, parse_shell_args};

pub(crate) async fn run_tui_command(args: &[String]) -> Result<(), String> {
    let shell_args = parse_shell_args(args)?;
    let mut core = create_local_core();
    core.callValue("application", "onCreate", serde_json::Value::Null)
        .map_err(|error| error.message)?;
    let initial_chat_id = core.withApplication(|application| initialize_shell_chat(application, &shell_args))?;
    let mut tui = OperitTui::new(core, shell_args, initial_chat_id)?;
    tui.run().await
}
