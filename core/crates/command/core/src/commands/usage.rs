use crate::output::CoreCommandOutput;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_store::repository::UsageStatisticsStore::{
    TokenStatsModel, TokenUsageRecord, UsageStatisticsStore,
};

/// Runs token usage commands.
pub fn run_usage_command(
    _application: &mut OperitApplication,
    args: &[String],
    output: &mut CoreCommandOutput,
) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("summary") => print_summary(output),
        Some("records") => print_records(output),
        Some("models") => print_models(output),
        Some("clear") => clear_records(output),
        _ => {
            print_usage_usage(output);
            Ok(())
        }
    }
}

/// Prints aggregate token usage.
fn print_summary(output: &mut CoreCommandOutput) -> Result<(), String> {
    let records = UsageStatisticsStore::new()
        .getAllTokenUsageRecords()
        .map_err(|e| e.to_string())?;
    let requests: i64 = records.iter().map(|r| r.requestCount).sum();
    let uncached: i64 = records.iter().filter_map(|r| r.uncachedInputTokens).sum();
    let cached: i64 = records.iter().filter_map(|r| r.cachedInputTokens).sum();
    let cacheWrite: i64 = records.iter().filter_map(|r| r.cacheWriteTokens).sum();
    let totalInput: i64 = records.iter().filter_map(|r| r.totalInputTokens).sum();
    let outputTokens: i64 = records.iter().filter_map(|r| r.outputTokens).sum();
    output.push_stdout_line(format!("Records: {}", records.len()));
    output.push_stdout_line(format!("Requests: {requests}"));
    output.push_stdout_line(format!("Uncached input tokens: {uncached}"));
    output.push_stdout_line(format!("Cached input tokens: {cached}"));
    output.push_stdout_line(format!("Cache write tokens: {cacheWrite}"));
    output.push_stdout_line(format!("Total input tokens: {totalInput}"));
    output.push_stdout_line(format!("Output tokens: {outputTokens}"));
    output.setJsonStdout(serde_json::json!({
        "records": records.len(),
        "requests": requests,
        "uncachedInputTokens": uncached,
        "cachedInputTokens": cached,
        "cacheWriteTokens": cacheWrite,
        "totalInputTokens": totalInput,
        "outputTokens": outputTokens,
    }));
    Ok(())
}

/// Prints token usage records.
fn print_records(output: &mut CoreCommandOutput) -> Result<(), String> {
    let records = UsageStatisticsStore::new()
        .getAllTokenUsageRecords()
        .map_err(|e| e.to_string())?;
    let values = records
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    output.push_stdout_line(format!("Token usage records: {}", records.len()));
    for record in &records {
        print_usage_record(record, output);
    }
    output.setJsonStdout(serde_json::Value::Array(values));
    Ok(())
}

/// Prints token pricing models.
fn print_models(output: &mut CoreCommandOutput) -> Result<(), String> {
    let models = UsageStatisticsStore::new()
        .getAllTokenStatsModels()
        .map_err(|e| e.to_string())?;
    let values = models
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    output.push_stdout_line(format!("Token pricing models: {}", models.len()));
    for model in &models {
        print_stats_model(model, output);
    }
    output.setJsonStdout(serde_json::Value::Array(values));
    Ok(())
}

/// Clears token usage records.
fn clear_records(output: &mut CoreCommandOutput) -> Result<(), String> {
    UsageStatisticsStore::new()
        .clearAllTokenUsageRecords()
        .map_err(|e| e.to_string())?;
    output.push_stdout_line("Token usage records cleared.");
    output.setJsonStdout(serde_json::json!({"clearedTokenUsageRecords": true}));
    Ok(())
}

/// Prints one token usage record as a compact ledger line.
fn print_usage_record(record: &TokenUsageRecord, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!(
        "#{} {} / {} — requests: {} — input: {} — cached: {} — output: {}",
        record.id,
        record.provider,
        record.model,
        record.requestCount,
        format_optional_i64(record.totalInputTokens),
        format_optional_i64(record.cachedInputTokens),
        format_optional_i64(record.outputTokens)
    ));
}

/// Prints one token pricing model as a compact ledger line.
fn print_stats_model(model: &TokenStatsModel, output: &mut CoreCommandOutput) {
    output.push_stdout_line(format!(
        "{} / {} — config: {} — billing: {} — currency: {} — input: {} — cached: {} — write: {} — output: {} — request: {}",
        model.provider,
        model.model,
        model.configId,
        format_optional_string(model.billingMode.as_ref()),
        format_optional_string(model.currency.as_ref()),
        format_optional_f64(model.inputPricePerMillion),
        format_optional_f64(model.cachedInputPricePerMillion),
        format_optional_f64(model.cacheWritePricePerMillion),
        format_optional_f64(model.outputPricePerMillion),
        format_optional_f64(model.pricePerRequest)
    ));
}

/// Formats an optional integer for CLI text.
fn format_optional_i64(value: Option<i64>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "-".to_string(),
    }
}

/// Formats an optional float for CLI text.
fn format_optional_f64(value: Option<f64>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "-".to_string(),
    }
}

/// Formats an optional string for CLI text.
fn format_optional_string(value: Option<&String>) -> String {
    match value {
        Some(value) => value.clone(),
        None => "-".to_string(),
    }
}

/// Prints token usage command usage.
fn print_usage_usage(output: &mut CoreCommandOutput) {
    let lines = vec!["operit2 usage <summary|records|models|clear>"];
    for line in &lines {
        output.push_stdout_line(line);
    }
    output.setJsonStdout(serde_json::json!({"usage": lines}));
}
