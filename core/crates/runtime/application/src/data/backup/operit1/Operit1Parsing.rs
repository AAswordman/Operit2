#[allow(non_snake_case)]
#[allow(non_snake_case)]
fn epochMillisToLocalDateTimeString(value: i64) -> Result<String, String> {
    let datetime = chrono::Local
        .timestamp_millis_opt(value)
        .single()
        .ok_or_else(|| format!("Operit1 聊天时间戳无效：{value}"))?;
    Ok(datetime
        .naive_local()
        .format("%Y-%m-%dT%H:%M:%S%.3f")
        .to_string())
}

#[allow(non_snake_case)]
fn epochMillisToLocalDateString(value: i64) -> Result<String, String> {
    let datetime = chrono::Local
        .timestamp_millis_opt(value)
        .single()
        .ok_or_else(|| format!("Operit1 用户偏好日期无效：{value}"))?;
    Ok(datetime.naive_local().format("%Y-%m-%d").to_string())
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}

#[allow(non_snake_case)]
fn requiredPreferenceString<'a>(
    preferences: &'a HashMap<String, Operit1PreferenceValue>,
    key: &str,
    missingMessage: &str,
) -> Result<&'a str, String> {
    let value = preferences
        .get(key)
        .ok_or_else(|| missingMessage.to_string())?;
    value
        .asString()
        .ok_or_else(|| format!("Operit1 DataStore 键不是字符串：{key}"))
}

#[allow(non_snake_case)]
fn parseCustomParameterValue(value: &str) -> Result<Value, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("Operit1 自定义参数值不是合法 JSON：{error}"))
}

#[allow(non_snake_case)]
fn parseParameterValueType(
    value: &str,
) -> Result<operit_model::ModelParameter::ParameterValueType, String> {
    match value {
        "INT" => Ok(operit_model::ModelParameter::ParameterValueType::INT),
        "FLOAT" => Ok(operit_model::ModelParameter::ParameterValueType::FLOAT),
        "STRING" => Ok(operit_model::ModelParameter::ParameterValueType::STRING),
        "BOOLEAN" => Ok(operit_model::ModelParameter::ParameterValueType::BOOLEAN),
        "OBJECT" => Ok(operit_model::ModelParameter::ParameterValueType::OBJECT),
        other => Err(format!("未知模型参数值类型：{other}")),
    }
}

#[allow(non_snake_case)]
fn parseParameterCategory(value: &str) -> Result<ParameterCategory, String> {
    match value {
        "GENERATION" => Ok(ParameterCategory::GENERATION),
        "CREATIVITY" => Ok(ParameterCategory::CREATIVITY),
        "REPETITION" => Ok(ParameterCategory::REPETITION),
        "OTHER" => Ok(ParameterCategory::OTHER),
        other => Err(format!("未知模型参数分类：{other}")),
    }
}

#[allow(non_snake_case)]
fn defaultKeyRotationMode() -> String {
    "ROUND_ROBIN".to_string()
}

#[allow(non_snake_case)]
fn defaultOperit1ConfigId() -> String {
    "default".to_string()
}

#[allow(non_snake_case)]
fn defaultOperit1TtsHttpMethod() -> String {
    "GET".to_string()
}

#[allow(non_snake_case)]
fn defaultOperit1TtsContentType() -> String {
    "application/json".to_string()
}

#[allow(non_snake_case)]
fn defaultTrue() -> bool {
    true
}

#[allow(non_snake_case)]
fn defaultApiKeyAvailabilityStatus() -> ApiKeyAvailabilityStatus {
    ApiKeyAvailabilityStatus::UNTESTED
}

#[allow(non_snake_case)]
fn defaultCustomParameters() -> String {
    "[]".to_string()
}

#[allow(non_snake_case)]
fn defaultCustomHeaders() -> String {
    "{}".to_string()
}

#[allow(non_snake_case)]
fn defaultMaxTokens() -> i32 {
    StandardModelParameters::DEFAULT_MAX_TOKENS
}

#[allow(non_snake_case)]
fn defaultTemperature() -> f32 {
    StandardModelParameters::DEFAULT_TEMPERATURE
}

#[allow(non_snake_case)]
fn defaultTopP() -> f32 {
    StandardModelParameters::DEFAULT_TOP_P
}

#[allow(non_snake_case)]
fn defaultRepetitionPenalty() -> f32 {
    StandardModelParameters::DEFAULT_REPETITION_PENALTY
}

#[allow(non_snake_case)]
fn defaultMaxContextLength() -> f32 {
    ModelConfigDefaults::DEFAULT_MAX_CONTEXT_LENGTH
}

#[allow(non_snake_case)]
fn defaultSummaryTokenThreshold() -> f32 {
    ModelConfigDefaults::DEFAULT_SUMMARY_TOKEN_THRESHOLD
}

#[allow(non_snake_case)]
fn defaultEnableSummary() -> bool {
    ModelConfigDefaults::DEFAULT_ENABLE_SUMMARY
}

#[allow(non_snake_case)]
fn defaultEnableSummaryByMessageCount() -> bool {
    ModelConfigDefaults::DEFAULT_ENABLE_SUMMARY_BY_MESSAGE_COUNT
}

#[allow(non_snake_case)]
fn defaultSummaryMessageCountThreshold() -> i32 {
    ModelConfigDefaults::DEFAULT_SUMMARY_MESSAGE_COUNT_THRESHOLD
}

#[allow(non_snake_case)]
fn defaultThreadCount() -> i32 {
    4
}

#[allow(non_snake_case)]
fn defaultLlamaContextSize() -> i32 {
    2048
}

#[allow(non_snake_case)]
fn defaultLlamaBatchSize() -> i32 {
    512
}

#[allow(non_snake_case)]
fn defaultLlamaKvUnified() -> bool {
    true
}
