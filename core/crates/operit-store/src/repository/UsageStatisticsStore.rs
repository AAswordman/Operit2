use crate::sqliteParams;
use crate::SqliteStore::{SqliteRow, SqliteRowGet, SqliteStoreError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::AppDatabase::AppDatabase;
use operit_model::FunctionType::FunctionType;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum UsageRequestSource {
    CHAT_RESPONSE,
    TOOL_RESULT_RESPONSE,
    SUMMARY_GENERATION,
    MEMORY_ANALYSIS,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct UsageRequestRecord {
    pub id: String,
    pub createdAtMs: i64,
    pub providerModel: String,
    pub provider: String,
    pub modelName: String,
    pub functionType: FunctionType,
    pub source: UsageRequestSource,
    pub chatId: Option<String>,
    pub inputTokens: i32,
    pub outputTokens: i32,
    pub cachedInputTokens: i32,
}

pub struct UsageStatisticsStore;

impl UsageStatisticsStore {
    pub fn new() -> Self {
        Self
    }

    #[allow(non_snake_case)]
    pub fn getAllRequestRecords(&self) -> Result<Vec<UsageRequestRecord>, String> {
        let database = AppDatabase::default().map_err(|error| error.to_string())?;
        database
            .store()
            .queryRows(
                r#"
                SELECT id, createdAtMs, providerModel, provider, modelName,
                    functionType, source, chatId, inputTokens, outputTokens,
                    cachedInputTokens
                FROM usage_request_records
                ORDER BY createdAtMs ASC, id ASC
                "#,
                sqliteParams![],
            )
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|row| mapUsageRequestRecord(&row).map_err(|error| error.to_string()))
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn clearAllRequestRecords(&self) -> Result<(), String> {
        let database = AppDatabase::default().map_err(|error| error.to_string())?;
        database
            .store()
            .execute("DELETE FROM usage_request_records", sqliteParams![])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn recordProviderModelRequest(
        &self,
        providerModel: String,
        functionType: FunctionType,
        source: UsageRequestSource,
        chatId: Option<String>,
        inputTokens: i32,
        outputTokens: i32,
        cachedInputTokens: i32,
    ) -> Result<UsageRequestRecord, String> {
        let (provider, modelName) = splitProviderModel(&providerModel)?;
        let record = UsageRequestRecord {
            id: Uuid::new_v4().to_string(),
            createdAtMs: currentTimeMillis(),
            providerModel,
            provider,
            modelName,
            functionType,
            source,
            chatId,
            inputTokens,
            outputTokens,
            cachedInputTokens,
        };
        let database = AppDatabase::default().map_err(|error| error.to_string())?;
        database
            .store()
            .execute(
                r#"
                INSERT INTO usage_request_records (
                    id, createdAtMs, providerModel, provider, modelName,
                    functionType, source, chatId, inputTokens, outputTokens,
                    cachedInputTokens
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                sqliteParams![
                    record.id,
                    record.createdAtMs,
                    record.providerModel,
                    record.provider,
                    record.modelName,
                    functionTypeName(&record.functionType),
                    usageRequestSourceName(&record.source),
                    record.chatId,
                    record.inputTokens,
                    record.outputTokens,
                    record.cachedInputTokens,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(record)
    }
}

#[allow(non_snake_case)]
fn mapUsageRequestRecord(row: &SqliteRow) -> Result<UsageRequestRecord, SqliteStoreError> {
    let functionTypeName: String = row.get("functionType")?;
    let sourceName: String = row.get("source")?;
    Ok(UsageRequestRecord {
        id: row.get("id")?,
        createdAtMs: row.get("createdAtMs")?,
        providerModel: row.get("providerModel")?,
        provider: row.get("provider")?,
        modelName: row.get("modelName")?,
        functionType: parseFunctionType(&functionTypeName)?,
        source: parseUsageRequestSource(&sourceName)?,
        chatId: row.get("chatId")?,
        inputTokens: row.get("inputTokens")?,
        outputTokens: row.get("outputTokens")?,
        cachedInputTokens: row.get("cachedInputTokens")?,
    })
}

#[allow(non_snake_case)]
fn splitProviderModel(providerModel: &str) -> Result<(String, String), String> {
    let trimmed = providerModel.trim();
    let colonIndex = trimmed
        .find(':')
        .ok_or_else(|| format!("providerModel must contain ':': {providerModel}"))?;
    let provider = trimmed[..colonIndex].trim().to_string();
    let modelName = trimmed[colonIndex + 1..].trim().to_string();
    if provider.is_empty() || modelName.is_empty() {
        return Err(format!(
            "providerModel must contain non-empty provider and model: {providerModel}"
        ));
    }
    Ok((provider, modelName))
}

#[allow(non_snake_case)]
fn functionTypeName(functionType: &FunctionType) -> &'static str {
    match functionType {
        FunctionType::CHAT => "CHAT",
        FunctionType::SUMMARY => "SUMMARY",
        FunctionType::MEMORY => "MEMORY",
        FunctionType::UI_CONTROLLER => "UI_CONTROLLER",
        FunctionType::TRANSLATION => "TRANSLATION",
        FunctionType::GREP => "GREP",
        FunctionType::ROLE_RESPONSE_PLANNER => "ROLE_RESPONSE_PLANNER",
        FunctionType::IMAGE_RECOGNITION => "IMAGE_RECOGNITION",
        FunctionType::AUDIO_RECOGNITION => "AUDIO_RECOGNITION",
        FunctionType::VIDEO_RECOGNITION => "VIDEO_RECOGNITION",
    }
}

#[allow(non_snake_case)]
fn parseFunctionType(value: &str) -> Result<FunctionType, SqliteStoreError> {
    match value {
        "CHAT" => Ok(FunctionType::CHAT),
        "SUMMARY" => Ok(FunctionType::SUMMARY),
        "MEMORY" => Ok(FunctionType::MEMORY),
        "UI_CONTROLLER" => Ok(FunctionType::UI_CONTROLLER),
        "TRANSLATION" => Ok(FunctionType::TRANSLATION),
        "GREP" => Ok(FunctionType::GREP),
        "ROLE_RESPONSE_PLANNER" => Ok(FunctionType::ROLE_RESPONSE_PLANNER),
        "IMAGE_RECOGNITION" => Ok(FunctionType::IMAGE_RECOGNITION),
        "AUDIO_RECOGNITION" => Ok(FunctionType::AUDIO_RECOGNITION),
        "VIDEO_RECOGNITION" => Ok(FunctionType::VIDEO_RECOGNITION),
        _ => Err(SqliteStoreError::Message(format!(
            "unknown usage request functionType: {value}"
        ))),
    }
}

#[allow(non_snake_case)]
fn usageRequestSourceName(source: &UsageRequestSource) -> &'static str {
    match source {
        UsageRequestSource::CHAT_RESPONSE => "CHAT_RESPONSE",
        UsageRequestSource::TOOL_RESULT_RESPONSE => "TOOL_RESULT_RESPONSE",
        UsageRequestSource::SUMMARY_GENERATION => "SUMMARY_GENERATION",
        UsageRequestSource::MEMORY_ANALYSIS => "MEMORY_ANALYSIS",
    }
}

#[allow(non_snake_case)]
fn parseUsageRequestSource(value: &str) -> Result<UsageRequestSource, SqliteStoreError> {
    match value {
        "CHAT_RESPONSE" => Ok(UsageRequestSource::CHAT_RESPONSE),
        "TOOL_RESULT_RESPONSE" => Ok(UsageRequestSource::TOOL_RESULT_RESPONSE),
        "SUMMARY_GENERATION" => Ok(UsageRequestSource::SUMMARY_GENERATION),
        "MEMORY_ANALYSIS" => Ok(UsageRequestSource::MEMORY_ANALYSIS),
        _ => Err(SqliteStoreError::Message(format!(
            "unknown usage request source: {value}"
        ))),
    }
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}
