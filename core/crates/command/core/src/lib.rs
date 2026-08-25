#![allow(non_snake_case)]

mod commands;
mod output;

pub use output::CoreCommandOutput;

/// Creates an application from the provided context and runs a core command.
pub fn run_core_command_with_context(
    context: operit_host_api::HostManager::HostManager,
    args: &[String],
) -> Result<CoreCommandOutput, String> {
    let mut application =
        operit_runtime::core::application::OperitApplication::OperitApplication::newWithContext(
            context,
        );
    application.onCreate()?;
    run_core_command(&mut application, args)
}

/// Runs a core command against an already initialized application.
pub fn run_core_command(
    application: &mut operit_runtime::core::application::OperitApplication::OperitApplication,
    args: &[String],
) -> Result<CoreCommandOutput, String> {
    let mut output = CoreCommandOutput::new();
    commands::run_core_command(application, args, &mut output)?;
    Ok(output)
}
