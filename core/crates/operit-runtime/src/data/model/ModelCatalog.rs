#[path = "../collects/ModelCatalog.rs"]
mod ModelCatalogRows;

use crate::data::model::BillingMode::BillingMode;
use crate::data::model::ModelConfigData::{
    BuiltinToolExclusivity, BuiltinToolRequestFormat, BuiltinToolType, ModelBuiltinTool,
    ModelCapabilities, ModelCatalogEntry, ModelContextSpec, ModelPricing, ModelRequestSpec,
    PricingCurrency, ProviderCatalogEntry, ProviderOperationResultSpec, ProviderOperationSpec,
};

pub struct ModelCatalog;

const MODEL_CATALOG_PROVIDER_ROWS: &str = r#"
OPENAI|OpenAI|https://api.openai.com/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::true
OPENAI_RESPONSES|OpenAI Responses|https://api.openai.com/v1/responses|list_models:GET:/v1/models:$.data:$.id:::::::::::::true
OPENAI_RESPONSES_GENERIC|OpenAI Responses Generic||list_models:GET:/v1/models:$.data:$.id:::::::::::::true
OPENAI_GENERIC|OpenAI Generic||list_models:GET:/v1/models:$.data:$.id:::::::::::::true
ANTHROPIC|Anthropic|https://api.anthropic.com/v1/messages|list_models:GET:/v1/models:$.data:$.id:::::::::::::true
ANTHROPIC_GENERIC|Anthropic Generic||list_models:GET:/v1/models:$.data:$.id:::::::::::::true
GOOGLE|Google Gemini|https://generativelanguage.googleapis.com/v1beta/models|list_models:GET:/v1beta/models:$.models:$.name:::::::::::::true
GEMINI_GENERIC|Gemini Generic||list_models:GET:/v1beta/models:$.models:$.name:::::::::::::true
BAIDU|Baidu|https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop/chat/completions|
ALIYUN|Aliyun|https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions|list_models:GET:/compatible-mode/v1/models:$.data:$.id:::::::::::::true
XUNFEI|Xunfei|https://spark-api-open.xf-yun.com/v2/chat/completions|
ZHIPU|Zhipu AI|https://open.bigmodel.cn/api/paas/v4/chat/completions|list_models:GET:/api/paas/v4/models:$.data:$.id:::::::::::::true
BAICHUAN|Baichuan|https://api.baichuan-ai.com/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::true
MOONSHOT|Moonshot|https://api.moonshot.cn/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::true
MIMO|MiMo|https://api.xiaomimimo.com/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::true
DEEPSEEK|DeepSeek|https://api.deepseek.com/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::true,balance:GET:/user/balance:$.balance_infos[0].total_balance:$.balance_infos[0].currency:true
MISTRAL|Mistral|https://codestral.mistral.ai/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::true
SILICONFLOW|SiliconFlow|https://api.siliconflow.cn/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::true,balance:GET:/v1/user/info:$.data.balance::true
IFLOW|iFlow|https://apis.iflow.cn/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::true
OPENROUTER|OpenRouter|https://openrouter.ai/api/v1/chat/completions|list_models:GET:/api/v1/models:$.data:$.id:$.pricing.prompt:$.pricing.input_cache_read:$.pricing.completion::USD:$.context_length:$.architecture.input_modalities~image:$.architecture.input_modalities~audio:$.architecture.input_modalities~video::$.supported_parameters~tools:$.supported_parameters~tools:true,balance:GET:/api/v1/credits:$.data.total_credits::true
FOUR_ROUTER|4Router|https://4router.net/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::true
NOUS_PORTAL|Nous Portal|https://inference-api.nousresearch.com/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::true
INFINIAI|InfiniAI|https://cloud.infini-ai.com/maas/v1/chat/completions|list_models:GET:/maas/v1/models:$.data:$.id:::::::::::::true
ALIPAY_BAILING|Alipay Bailing|https://api.tbox.cn/api/llm/v1/chat/completions|list_models:GET:/api/llm/v1/models:$.data:$.id:::::::::::::true
DOUBAO|Doubao|https://ark.cn-beijing.volces.com/api/v3/chat/completions|list_models:GET:/api/v3/models:$.data:$.id:::::::::::::true
NVIDIA|NVIDIA|https://integrate.api.nvidia.com/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::true
LMSTUDIO|LM Studio|http://localhost:1234/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::false
OLLAMA|Ollama|http://localhost:11434/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::false
OPENAI_LOCAL|OpenAI Local|http://localhost:8000/v1/chat/completions|list_models:GET:/v1/models:$.data:$.id:::::::::::::false
MNN|MNN||
LLAMA_CPP|llama.cpp||
PPINFRA|PPInfra|https://api.ppinfra.com/openai/v1/chat/completions|list_models:GET:/openai/v1/models:$.data:$.id:::::::::::::true
NOVITA|Novita AI|https://api.novita.ai/openai/v1/chat/completions|list_models:GET:/openai/v1/models:$.data:$.id:::::::::::::true
OTHER|Other||
"#;

impl ModelCatalog {
    pub fn provider(providerTypeId: &str) -> Result<ProviderCatalogEntry, String> {
        Self::providers()?
            .into_iter()
            .find(|provider| provider.providerTypeId.eq_ignore_ascii_case(providerTypeId))
            .ok_or_else(|| format!("catalog provider not found: {providerTypeId}"))
    }

    pub fn model(providerTypeId: &str, modelId: &str) -> Result<ModelCatalogEntry, String> {
        Self::provider(providerTypeId)?
            .models
            .into_iter()
            .find(|model| model.modelId.eq_ignore_ascii_case(modelId))
            .ok_or_else(|| format!("catalog model not found: {providerTypeId}:{modelId}"))
    }

    pub fn providers() -> Result<Vec<ProviderCatalogEntry>, String> {
        let models = parseModelRows(ModelCatalogRows::MODEL_CATALOG_MODEL_ROWS)?;
        let mut providers = Vec::new();
        for line in dataLines(MODEL_CATALOG_PROVIDER_ROWS) {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() != 4 {
                return Err(format!("invalid provider catalog row: {line}"));
            }
            let providerTypeId = parts[0].trim().to_string();
            let providerModels = models
                .iter()
                .filter(|model| model.providerTypeId.eq_ignore_ascii_case(&providerTypeId))
                .cloned()
                .collect();
            providers.push(ProviderCatalogEntry {
                providerTypeId,
                displayName: parts[1].trim().to_string(),
                defaultEndpoint: parts[2].trim().to_string(),
                operations: parseOperations(parts[3])?,
                models: providerModels,
            });
        }
        Ok(providers)
    }
}

fn dataLines(rows: &str) -> impl Iterator<Item = &str> {
    rows.lines().map(str::trim).filter(|line| !line.is_empty())
}

#[allow(non_snake_case)]
fn parseModelRows(rows: &str) -> Result<Vec<ModelCatalogEntry>, String> {
    dataLines(rows).map(parseModelRow).collect()
}

#[allow(non_snake_case)]
fn parseModelRow(line: &str) -> Result<ModelCatalogEntry, String> {
    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() != 16 {
        return Err(format!("invalid model catalog row: {line}"));
    }
    let billingMode = BillingMode::fromString(parts[2])?;
    let inputPricePerMillion = parseF64(parts[3], "input price", line)?;
    let cachedInputPricePerMillion = parseOptionalF64(parts[4], "cached input price", line)?;
    let outputPricePerMillion = parseF64(parts[5], "output price", line)?;
    let pricePerRequest = parseF64(parts[6], "request price", line)?;
    let currency = parseCurrency(parts[7])?;
    Ok(ModelCatalogEntry {
        providerTypeId: parts[0].trim().to_string(),
        modelId: parts[1].trim().to_string(),
        aliases: Vec::new(),
        pricing: Some(ModelPricing {
            billingMode,
            inputPricePerMillion,
            cachedInputPricePerMillion,
            outputPricePerMillion,
            pricePerRequest,
            currency,
        }),
        context: Some(ModelContextSpec {
            maxContextLength: parseF32(parts[8], "max context length", line)?,
            enableMaxContextMode: parseBool(parts[9], "enable max context mode", line)?,
        }),
        capabilities: Some(ModelCapabilities {
            directImage: parseBool(parts[10], "direct image", line)?,
            directAudio: parseBool(parts[11], "direct audio", line)?,
            directVideo: parseBool(parts[12], "direct video", line)?,
            toolCall: parseBool(parts[14], "tool call", line)?,
        }),
        builtinTools: parseCatalogBuiltinTools(parts[13], line)?,
        request: Some(ModelRequestSpec {
            supportsStructuredTools: parseBool(parts[15], "structured tools", line)?,
        }),
    })
}

#[allow(non_snake_case)]
fn parseCatalogBuiltinTools(value: &str, line: &str) -> Result<Vec<ModelBuiltinTool>, String> {
    if !parseBool(value, "builtin web search", line)? {
        return Ok(Vec::new());
    }
    Ok(vec![ModelBuiltinTool::disabled(
        BuiltinToolType::WebSearch,
        "内置联网搜索".to_string(),
        BuiltinToolRequestFormat::GeminiGoogleSearch,
        BuiltinToolExclusivity::ExclusiveWithExternalTools,
    )])
}

#[allow(non_snake_case)]
fn parseOperations(value: &str) -> Result<Vec<ProviderOperationSpec>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    trimmed.split(',').map(parseOperation).collect()
}

#[allow(non_snake_case)]
fn parseOperation(value: &str) -> Result<ProviderOperationSpec, String> {
    let parts: Vec<&str> = value.split(':').collect();
    let operationType = parts[0].trim().to_string();
    let mut result = emptyOperationResult();
    match operationType.as_str() {
        "list_models" => {
            result.itemsJsonPath = optionalString(requiredOperationPart(&parts, 3, value)?);
            result.itemIdJsonPath = optionalString(requiredOperationPart(&parts, 4, value)?);
            result.inputPricePerTokenJsonPath = optionalOperationPart(&parts, 5);
            result.cachedInputPricePerTokenJsonPath = optionalOperationPart(&parts, 6);
            result.outputPricePerTokenJsonPath = optionalOperationPart(&parts, 7);
            result.pricePerRequestJsonPath = optionalOperationPart(&parts, 8);
            result.currencyJsonPath = optionalOperationPart(&parts, 9);
            result.maxContextLengthJsonPath = optionalOperationPart(&parts, 10);
            result.directImageJsonPath = optionalOperationPart(&parts, 11);
            result.directAudioJsonPath = optionalOperationPart(&parts, 12);
            result.directVideoJsonPath = optionalOperationPart(&parts, 13);
            result.toolCallJsonPath = optionalOperationPart(&parts, 15);
            result.supportsStructuredToolsJsonPath = optionalOperationPart(&parts, 16);
            requiredOperationPart(&parts, 17, value)?;
        }
        "balance" => {
            result.amountJsonPath = optionalString(requiredOperationPart(&parts, 3, value)?);
            result.amountCurrencyJsonPath =
                optionalString(requiredOperationPart(&parts, 4, value)?);
            requiredOperationPart(&parts, 5, value)?;
        }
        _ => return Err(format!("invalid provider operation type: {operationType}")),
    }
    let requiresApiKeyPartIndex = requiresApiKeyPartIndex(operationType.as_str());
    Ok(ProviderOperationSpec {
        operationType,
        handlerId: "http_json".to_string(),
        method: requiredOperationPart(&parts, 1, value)?.trim().to_string(),
        path: requiredOperationPart(&parts, 2, value)?.trim().to_string(),
        requiresApiKey: parseBool(
            requiredOperationPart(&parts, requiresApiKeyPartIndex, value)?,
            "requires api key",
            value,
        )?,
        result,
    })
}

#[allow(non_snake_case)]
fn requiresApiKeyPartIndex(operationType: &str) -> usize {
    match operationType {
        "list_models" => 17,
        "balance" => 5,
        _ => 0,
    }
}

#[allow(non_snake_case)]
fn emptyOperationResult() -> ProviderOperationResultSpec {
    ProviderOperationResultSpec {
        itemsJsonPath: None,
        itemIdJsonPath: None,
        inputPricePerTokenJsonPath: None,
        cachedInputPricePerTokenJsonPath: None,
        outputPricePerTokenJsonPath: None,
        pricePerRequestJsonPath: None,
        currencyJsonPath: None,
        maxContextLengthJsonPath: None,
        directImageJsonPath: None,
        directAudioJsonPath: None,
        directVideoJsonPath: None,
        toolCallJsonPath: None,
        supportsStructuredToolsJsonPath: None,
        amountJsonPath: None,
        amountCurrencyJsonPath: None,
    }
}

#[allow(non_snake_case)]
fn optionalOperationPart(parts: &[&str], index: usize) -> Option<String> {
    parts.get(index).and_then(|value| optionalString(value))
}

#[allow(non_snake_case)]
fn requiredOperationPart<'a>(
    parts: &'a [&str],
    index: usize,
    operation: &str,
) -> Result<&'a str, String> {
    parts
        .get(index)
        .copied()
        .ok_or_else(|| format!("invalid provider operation row: {operation}"))
}

#[allow(non_snake_case)]
fn optionalString(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[allow(non_snake_case)]
fn parseCurrency(value: &str) -> Result<PricingCurrency, String> {
    match value.trim().to_ascii_uppercase().as_str() {
        "CNY" => Ok(PricingCurrency::CNY),
        "USD" => Ok(PricingCurrency::USD),
        other => Err(format!("invalid pricing currency: {other}")),
    }
}

#[allow(non_snake_case)]
fn parseF64(value: &str, field: &str, line: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .map_err(|error| format!("invalid {field} in row `{line}`: {error}"))
}

#[allow(non_snake_case)]
fn parseOptionalF64(value: &str, field: &str, line: &str) -> Result<Option<f64>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        parseF64(trimmed, field, line).map(Some)
    }
}

#[allow(non_snake_case)]
fn parseF32(value: &str, field: &str, line: &str) -> Result<f32, String> {
    value
        .trim()
        .parse::<f32>()
        .map_err(|error| format!("invalid {field} in row `{line}`: {error}"))
}

#[allow(non_snake_case)]
fn parseBool(value: &str, field: &str, line: &str) -> Result<bool, String> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("invalid {field} bool `{other}` in row `{line}`")),
    }
}
