use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;

/// Runs persistent-synchronization recovery commands.
pub fn run_sync_command(
    application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("reset-transactions") if args.len() == 1 => reset_transactions(application, output),
        _ => {
            print_sync_usage(output);
            Ok(())
        }
    }
}

/// Replaces synchronization history with the current materialized business data.
/// Recovery for stuck immutable-operation conflicts; retained data never moves.
fn reset_transactions(
    application: &OperitApplication,
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    let result = application.syncResetTransactions()?;
    let owners = result["owners"].as_u64().unwrap_or(0);
    let files = result["files"].as_u64().unwrap_or(0);
    output.push_stdout_line(format!(
        "Transaction history reset: {owners} data owners and {files} file owners rebuilt from current data; earlier history retired."
    ));
    output.setJsonStdout(result);
    Ok(())
}

/// Prints sync command usage.
fn print_sync_usage(output: &mut CoreCommandOutput) {
    let lines = vec![
        "operit2 sync reset-transactions  Rebuild local synchronization history from current data (recovery for stuck immutable-operation conflicts).",
    ];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}
