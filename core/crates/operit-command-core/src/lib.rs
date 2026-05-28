#![allow(non_snake_case)]

mod commands;
mod output;

pub use output::CoreCommandOutput;

pub fn run_core_command_with_context(
    context: operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext,
    args: &[String],
) -> Result<CoreCommandOutput, String> {
    let mut application =
        operit_runtime::core::application::OperitApplication::OperitApplication::newWithContext(
            context,
        );
    application.onCreate()?;
    run_core_command(&mut application, args)
}

pub fn run_core_command(
    application: &mut operit_runtime::core::application::OperitApplication::OperitApplication,
    args: &[String],
) -> Result<CoreCommandOutput, String> {
    let mut output = CoreCommandOutput::new();
    commands::run_core_command(application, args, &mut output)?;
    Ok(output)
}
