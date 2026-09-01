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
    TITLE_GENERATION,
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
    pub inputTokens: i64,
    pub outputTokens: i64,
    pub cachedInputTokens: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
/// Stores one completed token-usage ledger entry using the OP1 accounting schema.
pub struct TokenUsageRecord {
    pub id: i64,
    pub importKey: Option<String>,
    pub occurredAtMs: Option<i64>,
    pub configId: String,
    pub provider: String,
    pub model: String,
    pub requestCount: i64,
    pub uncachedInputTokens: Option<i64>,
    pub cachedInputTokens: Option<i64>,
    pub cacheWriteTokens: Option<i64>,
    pub totalInputTokens: Option<i64>,
    pub outputTokens: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
/// Stores one configurable provider/model pricing identity from the OP1 schema.
pub struct TokenStatsModel {
    pub configId: String,
    pub provider: String,
    pub model: String,
    pub billingMode: Option<String>,
    pub currency: Option<String>,
    pub inputPricePerMillion: Option<f64>,
    pub cachedInputPricePerMillion: Option<f64>,
    pub cacheWritePricePerMillion: Option<f64>,
    pub outputPricePerMillion: Option<f64>,
    pub pricePerRequest: Option<f64>,
}

pub struct UsageStatisticsStore;

impl UsageStatisticsStore {
    /// Creates a usage statistics repository.
    pub fn new() -> Self {
        Self
    }

    #[allow(non_snake_case)]
    /// Reads all recorded provider model requests ordered by creation time.
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
    /// Deletes every recorded provider model request.
    pub fn clearAllRequestRecords(&self) -> Result<(), String> {
        let database = AppDatabase::default().map_err(|error| error.to_string())?;
        database
            .store()
            .execute("DELETE FROM usage_request_records", sqliteParams![])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Reads every OP1-compatible token ledger entry in chronological order.
    pub fn getAllTokenUsageRecords(&self) -> Result<Vec<TokenUsageRecord>, String> {
        let database = AppDatabase::default().map_err(|error| error.to_string())?;
        database
            .store()
            .queryRows(
                r#"
                SELECT id, importKey, occurredAtMs, configId, provider, model,
                    requestCount, uncachedInputTokens, cachedInputTokens,
                    cacheWriteTokens, totalInputTokens, outputTokens
                FROM token_usage_records
                ORDER BY occurredAtMs IS NULL ASC, occurredAtMs ASC, id ASC
                "#,
                sqliteParams![],
            )
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|row| mapTokenUsageRecord(&row).map_err(|error| error.to_string()))
            .collect()
    }

    #[allow(non_snake_case)]
    /// Reads every configured provider/model pricing identity.
    pub fn getAllTokenStatsModels(&self) -> Result<Vec<TokenStatsModel>, String> {
        let database = AppDatabase::default().map_err(|error| error.to_string())?;
        database
            .store()
            .queryRows(
                r#"
                SELECT configId, provider, model, billingMode, currency,
                    inputPricePerMillion, cachedInputPricePerMillion,
                    cacheWritePricePerMillion, outputPricePerMillion, pricePerRequest
                FROM token_stats_models
                ORDER BY provider ASC, model ASC, configId ASC
                "#,
                sqliteParams![],
            )
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|row| mapTokenStatsModel(&row).map_err(|error| error.to_string()))
            .collect()
    }

    #[allow(non_snake_case)]
    /// Removes token ledger entries while retaining configured pricing identities.
    pub fn clearAllTokenUsageRecords(&self) -> Result<(), String> {
        let database = AppDatabase::default().map_err(|error| error.to_string())?;
        database
            .store()
            .execute("DELETE FROM token_usage_records", sqliteParams![])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Inserts one OP1-compatible token ledger entry and returns its stored row.
    pub fn recordTokenUsage(
        &self,
        importKey: Option<String>,
        occurredAtMs: Option<i64>,
        configId: String,
        provider: String,
        model: String,
        requestCount: i64,
        uncachedInputTokens: Option<i64>,
        cachedInputTokens: Option<i64>,
        cacheWriteTokens: Option<i64>,
        totalInputTokens: Option<i64>,
        outputTokens: Option<i64>,
    ) -> Result<TokenUsageRecord, String> {
        let database = AppDatabase::default().map_err(|error| error.to_string())?;
        database
            .store()
            .execute(
                r#"
                INSERT INTO token_usage_records (
                    importKey, occurredAtMs, configId, provider, model, requestCount,
                    uncachedInputTokens, cachedInputTokens, cacheWriteTokens,
                    totalInputTokens, outputTokens
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(importKey) DO UPDATE SET
                    occurredAtMs = excluded.occurredAtMs,
                    configId = excluded.configId,
                    provider = excluded.provider,
                    model = excluded.model,
                    requestCount = excluded.requestCount,
                    uncachedInputTokens = excluded.uncachedInputTokens,
                    cachedInputTokens = excluded.cachedInputTokens,
                    cacheWriteTokens = excluded.cacheWriteTokens,
                    totalInputTokens = excluded.totalInputTokens,
                    outputTokens = excluded.outputTokens
                "#,
                sqliteParams![
                    importKey,
                    occurredAtMs,
                    configId,
                    provider,
                    model,
                    requestCount,
                    uncachedInputTokens,
                    cachedInputTokens,
                    cacheWriteTokens,
                    totalInputTokens,
                    outputTokens,
                ],
            )
            .map_err(|error| error.to_string())?;
        let id = if let Some(importKey) = importKey.as_ref() {
            database
                .store()
                .queryRows(
                    "SELECT id FROM token_usage_records WHERE importKey = ?1",
                    sqliteParams![importKey],
                )
                .map_err(|error| error.to_string())?
                .first()
                .ok_or_else(|| "token usage upsert did not return its row".to_string())?
                .get::<usize, i64>(0)
                .map_err(|error| error.to_string())?
        } else {
            database
                .store()
                .queryRows("SELECT last_insert_rowid()", sqliteParams![])
                .map_err(|error| error.to_string())?
                .first()
                .ok_or_else(|| "token usage insert did not return a row id".to_string())?
                .get::<usize, i64>(0)
                .map_err(|error| error.to_string())?
        };
        Ok(TokenUsageRecord {
            id,
            importKey,
            occurredAtMs,
            configId,
            provider,
            model,
            requestCount,
            uncachedInputTokens,
            cachedInputTokens,
            cacheWriteTokens,
            totalInputTokens,
            outputTokens,
        })
    }

    #[allow(non_snake_case)]
    /// Upserts one provider/model pricing identity.
    pub fn upsertTokenStatsModel(&self, model: TokenStatsModel) -> Result<(), String> {
        let database = AppDatabase::default().map_err(|error| error.to_string())?;
        database
            .store()
            .execute(
                r#"
                INSERT INTO token_stats_models (
                    configId, provider, model, billingMode, currency,
                    inputPricePerMillion, cachedInputPricePerMillion,
                    cacheWritePricePerMillion, outputPricePerMillion, pricePerRequest
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ON CONFLICT(configId, provider, model) DO UPDATE SET
                    billingMode = excluded.billingMode,
                    currency = excluded.currency,
                    inputPricePerMillion = excluded.inputPricePerMillion,
                    cachedInputPricePerMillion = excluded.cachedInputPricePerMillion,
                    cacheWritePricePerMillion = excluded.cacheWritePricePerMillion,
                    outputPricePerMillion = excluded.outputPricePerMillion,
                    pricePerRequest = excluded.pricePerRequest
                "#,
                sqliteParams![
                    model.configId,
                    model.provider,
                    model.model,
                    model.billingMode,
                    model.currency,
                    model.inputPricePerMillion,
                    model.cachedInputPricePerMillion,
                    model.cacheWritePricePerMillion,
                    model.outputPricePerMillion,
                    model.pricePerRequest,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Records token usage for one provider model request.
    pub fn recordProviderModelRequest(
        &self,
        providerModel: String,
        functionType: FunctionType,
        source: UsageRequestSource,
        chatId: Option<String>,
        inputTokens: i64,
        outputTokens: i64,
        cachedInputTokens: i64,
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
        self.recordTokenUsage(
            None,
            Some(record.createdAtMs),
            String::new(),
            record.provider.clone(),
            record.modelName.clone(),
            1,
            Some((inputTokens - cachedInputTokens).max(0)),
            Some(cachedInputTokens),
            Some(0),
            Some(inputTokens),
            Some(outputTokens),
        )?;
        Ok(record)
    }
}

#[allow(non_snake_case)]
/// Maps one database row into a usage request record.
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
/// Maps one token ledger row while preserving nullable usage values.
fn mapTokenUsageRecord(row: &SqliteRow) -> Result<TokenUsageRecord, SqliteStoreError> {
    Ok(TokenUsageRecord {
        id: row.get("id")?,
        importKey: row.get("importKey")?,
        occurredAtMs: row.get("occurredAtMs")?,
        configId: row.get("configId")?,
        provider: row.get("provider")?,
        model: row.get("model")?,
        requestCount: row.get("requestCount")?,
        uncachedInputTokens: row.get("uncachedInputTokens")?,
        cachedInputTokens: row.get("cachedInputTokens")?,
        cacheWriteTokens: row.get("cacheWriteTokens")?,
        totalInputTokens: row.get("totalInputTokens")?,
        outputTokens: row.get("outputTokens")?,
    })
}

#[allow(non_snake_case)]
/// Maps one token pricing row into its serializable model.
fn mapTokenStatsModel(row: &SqliteRow) -> Result<TokenStatsModel, SqliteStoreError> {
    Ok(TokenStatsModel {
        configId: row.get("configId")?,
        provider: row.get("provider")?,
        model: row.get("model")?,
        billingMode: row.get("billingMode")?,
        currency: row.get("currency")?,
        inputPricePerMillion: row.get("inputPricePerMillion")?,
        cachedInputPricePerMillion: row.get("cachedInputPricePerMillion")?,
        cacheWritePricePerMillion: row.get("cacheWritePricePerMillion")?,
        outputPricePerMillion: row.get("outputPricePerMillion")?,
        pricePerRequest: row.get("pricePerRequest")?,
    })
}

#[allow(non_snake_case)]
/// Splits a provider-model identifier into its provider and model components.
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
/// Serializes a functional model role for usage storage.
fn functionTypeName(functionType: &FunctionType) -> &'static str {
    match functionType {
        FunctionType::CHAT => "CHAT",
        FunctionType::SUMMARY => "SUMMARY",
        FunctionType::TITLE_GENERATION => "TITLE_GENERATION",
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
/// Parses a functional model role stored with usage statistics.
fn parseFunctionType(value: &str) -> Result<FunctionType, SqliteStoreError> {
    match value {
        "CHAT" => Ok(FunctionType::CHAT),
        "SUMMARY" => Ok(FunctionType::SUMMARY),
        "TITLE_GENERATION" => Ok(FunctionType::TITLE_GENERATION),
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
/// Serializes a usage request source for storage.
fn usageRequestSourceName(source: &UsageRequestSource) -> &'static str {
    match source {
        UsageRequestSource::CHAT_RESPONSE => "CHAT_RESPONSE",
        UsageRequestSource::TOOL_RESULT_RESPONSE => "TOOL_RESULT_RESPONSE",
        UsageRequestSource::SUMMARY_GENERATION => "SUMMARY_GENERATION",
        UsageRequestSource::TITLE_GENERATION => "TITLE_GENERATION",
        UsageRequestSource::MEMORY_ANALYSIS => "MEMORY_ANALYSIS",
    }
}

#[allow(non_snake_case)]
/// Parses a stored usage request source.
fn parseUsageRequestSource(value: &str) -> Result<UsageRequestSource, SqliteStoreError> {
    match value {
        "CHAT_RESPONSE" => Ok(UsageRequestSource::CHAT_RESPONSE),
        "TOOL_RESULT_RESPONSE" => Ok(UsageRequestSource::TOOL_RESULT_RESPONSE),
        "SUMMARY_GENERATION" => Ok(UsageRequestSource::SUMMARY_GENERATION),
        "TITLE_GENERATION" => Ok(UsageRequestSource::TITLE_GENERATION),
        "MEMORY_ANALYSIS" => Ok(UsageRequestSource::MEMORY_ANALYSIS),
        _ => Err(SqliteStoreError::Message(format!(
            "unknown usage request source: {value}"
        ))),
    }
}

#[allow(non_snake_case)]
/// Reads the current host-provided time in milliseconds.
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}
