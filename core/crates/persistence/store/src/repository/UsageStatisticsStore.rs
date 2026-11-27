use crate::sqliteParams;
use crate::CoreNodeIdentityStore::CoreNodeIdentityStore;
use crate::RuntimeStorageHost::defaultRuntimeStorageHostOption;
use crate::SqliteStore::{SqliteRow, SqliteRowGet, SqliteStoreError};
use crate::SyncOperationStore::{
    NewSyncOperation, SyncOperation, SyncOperationSemantics, SyncOperationStore,
};
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::AppDatabase::AppDatabase;
use operit_model::FunctionType::FunctionType;
use operit_model::ProviderQuota::{UsageRequestStatus, UsageResetScope};

pub const USAGE_SYNC_DOMAIN: &str = "usage";
pub const USAGE_SYNC_ENTITY_TYPE: &str = "request_record";
pub const USAGE_RESET_ENTITY_TYPE: &str = "reset_watermark";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum UsageRequestSource {
    CHAT_RESPONSE,
    TOOL_RESULT_RESPONSE,
    SUMMARY_GENERATION,
    TITLE_GENERATION,
    MEMORY_ANALYSIS,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    #[serde(default)]
    pub cacheWriteTokens: i64,
    #[serde(default)]
    pub reasoningTokens: i64,
    #[serde(default)]
    pub waitDurationMs: i64,
    #[serde(default)]
    pub outputDurationMs: i64,
    #[serde(default)]
    pub totalDurationMs: i64,
    #[serde(default)]
    pub status: UsageRequestStatus,
    #[serde(default)]
    pub originNodeId: String,
    #[serde(default)]
    pub providerId: String,
    #[serde(default)]
    pub keyId: String,
    #[serde(default)]
    pub keyFingerprint: String,
    #[serde(default)]
    pub estimatedCost: f64,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct UsageRequestCapture {
    pub providerModel: String,
    pub functionType: FunctionType,
    pub source: UsageRequestSource,
    #[serde(default)]
    pub chatId: Option<String>,
    #[serde(default)]
    pub inputTokens: i64,
    #[serde(default)]
    pub outputTokens: i64,
    #[serde(default)]
    pub cachedInputTokens: i64,
    #[serde(default)]
    pub cacheWriteTokens: i64,
    #[serde(default)]
    pub reasoningTokens: i64,
    #[serde(default)]
    pub waitDurationMs: i64,
    #[serde(default)]
    pub outputDurationMs: i64,
    #[serde(default)]
    pub totalDurationMs: i64,
    #[serde(default)]
    pub status: UsageRequestStatus,
    #[serde(default)]
    pub originNodeId: String,
    #[serde(default)]
    pub providerId: String,
    #[serde(default)]
    pub keyId: String,
    #[serde(default)]
    pub keyFingerprint: String,
    #[serde(default)]
    pub estimatedCost: f64,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub stream: bool,
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
                    cachedInputTokens, cacheWriteTokens, reasoningTokens,
                    waitDurationMs, outputDurationMs, totalDurationMs, status,
                    originNodeId, providerId, keyId, keyFingerprint,
                    estimatedCost, currency, stream
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
    /// Clears usage statistics either only for this device's local ledger or across the Space.
    pub fn clearUsageStatistics(&self, scope: UsageResetScope) -> Result<(), String> {
        let database = AppDatabase::default().map_err(|error| error.to_string())?;
        match scope {
            UsageResetScope::LOCAL_ONLY => {
                if let Some(localNodeId) = CoreNodeIdentityStore::localNodeId() {
                    database
                        .store()
                        .execute(
                            "DELETE FROM usage_request_records WHERE originNodeId = '' OR originNodeId = ?1",
                            sqliteParams![localNodeId],
                        )
                        .map_err(|error| error.to_string())?;
                } else {
                    database
                        .store()
                        .execute("DELETE FROM usage_request_records", sqliteParams![])
                        .map_err(|error| error.to_string())?;
                }
                database
                    .store()
                    .execute("DELETE FROM token_usage_records", sqliteParams![])
                    .map_err(|error| error.to_string())?;
            }
            UsageResetScope::ALL_DEVICES_IN_SPACE => {
                let beforeMs = currentTimeMillis();
                database
                    .store()
                    .execute("DELETE FROM usage_request_records", sqliteParams![])
                    .map_err(|error| error.to_string())?;
                database
                    .store()
                    .execute("DELETE FROM token_usage_records", sqliteParams![])
                    .map_err(|error| error.to_string())?;
                publishUsageResetOperation(&database, beforeMs)?;
            }
        }
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
        self.recordProviderModelRequestDetailed(UsageRequestCapture {
            providerModel,
            functionType,
            source,
            chatId,
            inputTokens,
            outputTokens,
            cachedInputTokens,
            cacheWriteTokens: 0,
            reasoningTokens: 0,
            waitDurationMs: 0,
            outputDurationMs: 0,
            totalDurationMs: 0,
            status: UsageRequestStatus::COMPLETED,
            originNodeId: String::new(),
            providerId: String::new(),
            keyId: String::new(),
            keyFingerprint: String::new(),
            estimatedCost: 0.0,
            currency: String::new(),
            stream: false,
        })
    }

    #[allow(non_snake_case)]
    /// Persists one complete usage telemetry record locally and publishes it for Space sync.
    pub fn recordProviderModelRequestDetailed(
        &self,
        capture: UsageRequestCapture,
    ) -> Result<UsageRequestRecord, String> {
        let (provider, modelName) = splitProviderModel(&capture.providerModel)?;
        let originNodeId = if capture.originNodeId.trim().is_empty() {
            CoreNodeIdentityStore::localNodeId().unwrap_or_default()
        } else {
            capture.originNodeId.trim().to_string()
        };
        let record = UsageRequestRecord {
            id: Uuid::new_v4().to_string(),
            createdAtMs: currentTimeMillis(),
            providerModel: capture.providerModel,
            provider,
            modelName,
            functionType: capture.functionType,
            source: capture.source,
            chatId: capture.chatId,
            inputTokens: capture.inputTokens.max(0),
            outputTokens: capture.outputTokens.max(0),
            cachedInputTokens: capture.cachedInputTokens.max(0),
            cacheWriteTokens: capture.cacheWriteTokens.max(0),
            reasoningTokens: capture.reasoningTokens.max(0),
            waitDurationMs: capture.waitDurationMs.max(0),
            outputDurationMs: capture.outputDurationMs.max(0),
            totalDurationMs: capture.totalDurationMs.max(0),
            status: capture.status,
            originNodeId,
            providerId: capture.providerId,
            keyId: capture.keyId,
            keyFingerprint: capture.keyFingerprint,
            estimatedCost: if capture.estimatedCost.is_finite() {
                capture.estimatedCost.max(0.0)
            } else {
                0.0
            },
            currency: capture.currency,
            stream: capture.stream,
        };
        let database = AppDatabase::default().map_err(|error| error.to_string())?;
        insertOrReplaceRequestRecord(&database, &record)?;
        if record.status == UsageRequestStatus::COMPLETED
            || record.inputTokens > 0
            || record.outputTokens > 0
        {
            self.recordTokenUsage(
                None,
                Some(record.createdAtMs),
                String::new(),
                record.provider.clone(),
                record.modelName.clone(),
                1,
                Some((record.inputTokens - record.cachedInputTokens).max(0)),
                Some(record.cachedInputTokens),
                Some(0),
                Some(record.inputTokens),
                Some(record.outputTokens),
            )?;
        }
        publishUsageUpsertOperation(&database, &record)?;
        Ok(record)
    }
}

#[allow(non_snake_case)]
fn insertOrReplaceRequestRecord(
    database: &AppDatabase,
    record: &UsageRequestRecord,
) -> Result<(), String> {
    database
        .store()
        .execute(
            r#"
            INSERT OR REPLACE INTO usage_request_records (
                id, createdAtMs, providerModel, provider, modelName,
                functionType, source, chatId, inputTokens, outputTokens,
                cachedInputTokens, cacheWriteTokens, reasoningTokens,
                waitDurationMs, outputDurationMs, totalDurationMs, status,
                originNodeId, providerId, keyId, keyFingerprint,
                estimatedCost, currency, stream
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13,
                ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21,
                ?22, ?23, ?24
            )
            "#,
            sqliteParams![
                record.id.clone(),
                record.createdAtMs,
                record.providerModel.clone(),
                record.provider.clone(),
                record.modelName.clone(),
                functionTypeName(&record.functionType),
                usageRequestSourceName(&record.source),
                record.chatId.clone(),
                record.inputTokens,
                record.outputTokens,
                record.cachedInputTokens,
                record.cacheWriteTokens,
                record.reasoningTokens,
                record.waitDurationMs,
                record.outputDurationMs,
                record.totalDurationMs,
                usageRequestStatusName(&record.status),
                record.originNodeId.clone(),
                record.providerId.clone(),
                record.keyId.clone(),
                record.keyFingerprint.clone(),
                record.estimatedCost,
                record.currency.clone(),
                if record.stream { 1i64 } else { 0i64 },
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[allow(non_snake_case)]
fn usageSyncContext(database: &AppDatabase) -> Option<(SyncOperationStore, String)> {
    let storage = database
        .store()
        .storage
        .clone()
        .or_else(defaultRuntimeStorageHostOption)?;
    let nodeId = CoreNodeIdentityStore::localNodeId()?;
    Some((
        SyncOperationStore::new(storage, RUNTIME_SYNC_DIR_PATH),
        nodeId,
    ))
}

#[allow(non_snake_case)]
fn publishUsageUpsertOperation(
    database: &AppDatabase,
    record: &UsageRequestRecord,
) -> Result<(), String> {
    let Some((syncStore, nodeId)) = usageSyncContext(database) else {
        return Ok(());
    };
    let payload = serde_json::to_value(record).map_err(|error| error.to_string())?;
    syncStore
        .appendLocalOperation(
            &nodeId,
            NewSyncOperation {
                domain: USAGE_SYNC_DOMAIN.to_string(),
                entityType: USAGE_SYNC_ENTITY_TYPE.to_string(),
                entityId: record.id.clone(),
                operation: "upsert".to_string(),
                semantics: SyncOperationSemantics::EntityState,
                payload,
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[allow(non_snake_case)]
fn publishUsageResetOperation(database: &AppDatabase, beforeMs: i64) -> Result<(), String> {
    let Some((syncStore, nodeId)) = usageSyncContext(database) else {
        return Ok(());
    };
    syncStore
        .appendLocalOperation(
            &nodeId,
            NewSyncOperation {
                domain: USAGE_SYNC_DOMAIN.to_string(),
                entityType: USAGE_RESET_ENTITY_TYPE.to_string(),
                entityId: format!("reset-{beforeMs}-{}", Uuid::new_v4()),
                operation: "delete_before".to_string(),
                semantics: SyncOperationSemantics::EntityState,
                payload: serde_json::json!({ "beforeMs": beforeMs }),
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Applies one synchronized Space operation in the `usage` domain.
#[allow(non_snake_case)]
pub fn applySyncedUsageOperation(operation: &SyncOperation) -> Result<(), String> {
    if operation.domain != USAGE_SYNC_DOMAIN {
        return Err(format!("not a usage sync operation: {}", operation.domain));
    }
    let database = AppDatabase::default().map_err(|error| error.to_string())?;
    match (operation.entityType.as_str(), operation.operation.as_str()) {
        (USAGE_SYNC_ENTITY_TYPE, "upsert") => {
            let record: UsageRequestRecord = serde_json::from_value(operation.payload.clone())
                .map_err(|error| error.to_string())?;
            insertOrReplaceRequestRecord(&database, &record)?;
            Ok(())
        }
        (USAGE_SYNC_ENTITY_TYPE, "delete") => {
            database
                .store()
                .execute(
                    "DELETE FROM usage_request_records WHERE id = ?1",
                    sqliteParams![operation.entityId.clone()],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        }
        (USAGE_RESET_ENTITY_TYPE, "delete_before") => {
            let beforeMs = operation
                .payload
                .get("beforeMs")
                .and_then(serde_json::Value::as_i64)
                .ok_or_else(|| "usage reset_watermark missing beforeMs".to_string())?;
            database
                .store()
                .execute(
                    "DELETE FROM usage_request_records WHERE createdAtMs <= ?1",
                    sqliteParams![beforeMs],
                )
                .map_err(|error| error.to_string())?;
            database
                .store()
                .execute(
                    "DELETE FROM token_usage_records WHERE occurredAtMs <= ?1",
                    sqliteParams![beforeMs],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        }
        (entityType, opName) => Err(format!(
            "unsupported usage sync operation: {entityType}/{opName}"
        )),
    }
}


#[allow(non_snake_case)]
/// Maps one database row into a usage request record.
fn mapUsageRequestRecord(row: &SqliteRow) -> Result<UsageRequestRecord, SqliteStoreError> {
    let functionTypeName: String = row.get("functionType")?;
    let sourceName: String = row.get("source")?;
    let statusName: String = row.get("status")?;
    let streamInt: i64 = row.get("stream")?;
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
        cacheWriteTokens: row.get("cacheWriteTokens")?,
        reasoningTokens: row.get("reasoningTokens")?,
        waitDurationMs: row.get("waitDurationMs")?,
        outputDurationMs: row.get("outputDurationMs")?,
        totalDurationMs: row.get("totalDurationMs")?,
        status: parseUsageRequestStatus(&statusName)?,
        originNodeId: row.get("originNodeId")?,
        providerId: row.get("providerId")?,
        keyId: row.get("keyId")?,
        keyFingerprint: row.get("keyFingerprint")?,
        estimatedCost: row.get("estimatedCost")?,
        currency: row.get("currency")?,
        stream: streamInt != 0,
    })
}

#[allow(non_snake_case)]
fn usageRequestStatusName(status: &UsageRequestStatus) -> &'static str {
    match status {
        UsageRequestStatus::COMPLETED => "COMPLETED",
        UsageRequestStatus::CANCELLED => "CANCELLED",
        UsageRequestStatus::FAILED => "FAILED",
    }
}

#[allow(non_snake_case)]
fn parseUsageRequestStatus(value: &str) -> Result<UsageRequestStatus, SqliteStoreError> {
    match value {
        "COMPLETED" => Ok(UsageRequestStatus::COMPLETED),
        "CANCELLED" => Ok(UsageRequestStatus::CANCELLED),
        "FAILED" => Ok(UsageRequestStatus::FAILED),
        _ => Err(SqliteStoreError::Message(format!(
            "unknown usage request status: {value}"
        ))),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::SqlChatSyncStore::tests::{openTestStore, DATABASE_MUTEX};

    #[test]
    fn detailed_usage_record_roundtrips_and_sync_watermark_clears() {
        let _guard = DATABASE_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let (_paths, _db, _sync) = openTestStore("usage-store");
        let store = UsageStatisticsStore::new();
        let recorded = store
            .recordProviderModelRequestDetailed(UsageRequestCapture {
                providerModel: "DEEPSEEK:deepseek-reasoner".to_string(),
                functionType: FunctionType::CHAT,
                source: UsageRequestSource::CHAT_RESPONSE,
                chatId: Some("chat-1".to_string()),
                inputTokens: 1200,
                outputTokens: 320,
                cachedInputTokens: 800,
                cacheWriteTokens: 40,
                reasoningTokens: 210,
                waitDurationMs: 450,
                outputDurationMs: 2000,
                totalDurationMs: 2450,
                status: UsageRequestStatus::COMPLETED,
                originNodeId: "node-alpha".to_string(),
                providerId: "prov-ds".to_string(),
                keyId: "key-1".to_string(),
                keyFingerprint: "sk-d...1234".to_string(),
                estimatedCost: 0.0024,
                currency: "USD".to_string(),
                stream: true,
            })
            .unwrap();
        let all = store.getAllRequestRecords().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].reasoningTokens, 210);
        assert_eq!(all[0].outputDurationMs, 2000);
        assert_eq!(all[0].originNodeId, "node-alpha");
        assert!(all[0].stream);

        let peerRecord = UsageRequestRecord {
            id: "peer-record-1".to_string(),
            createdAtMs: recorded.createdAtMs + 10,
            originNodeId: "node-beta".to_string(),
            ..recorded.clone()
        };
        applySyncedUsageOperation(&SyncOperation {
            opId: "op-upsert".to_string(),
            originDeviceId: "node-beta".to_string(),
            sequence: 1,
            createdAt: peerRecord.createdAtMs,
            domain: USAGE_SYNC_DOMAIN.to_string(),
            entityType: USAGE_SYNC_ENTITY_TYPE.to_string(),
            entityId: peerRecord.id.clone(),
            operation: "upsert".to_string(),
            semantics: SyncOperationSemantics::EntityState,
            payload: serde_json::to_value(&peerRecord).unwrap(),
            schemaVersion: 1,
        })
        .unwrap();
        assert_eq!(store.getAllRequestRecords().unwrap().len(), 2);

        applySyncedUsageOperation(&SyncOperation {
            opId: "op-reset".to_string(),
            originDeviceId: "node-beta".to_string(),
            sequence: 2,
            createdAt: peerRecord.createdAtMs + 1,
            domain: USAGE_SYNC_DOMAIN.to_string(),
            entityType: USAGE_RESET_ENTITY_TYPE.to_string(),
            entityId: "reset-1".to_string(),
            operation: "delete_before".to_string(),
            semantics: SyncOperationSemantics::EntityState,
            payload: serde_json::json!({ "beforeMs": peerRecord.createdAtMs }),
            schemaVersion: 1,
        })
        .unwrap();
        assert!(store.getAllRequestRecords().unwrap().is_empty());

        AppDatabase::closeDatabase();
    }
}
