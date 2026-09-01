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
    let jsonMode = args.iter().any(|arg| arg == "--json");
    let commandArgs = args
        .iter()
        .filter(|arg| arg.as_str() != "--json")
        .cloned()
        .collect::<Vec<_>>();
    let mut output = CoreCommandOutput::new();
    output.setJsonMode(jsonMode);
    commands::run_core_command(application, &commandArgs, &mut output)?;
    output.finalizeJson()?;
    Ok(output)
}
