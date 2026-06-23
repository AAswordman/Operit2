#[path = "../collects/TtsCatalog.rs"]
mod TtsCatalogRows;

use crate::data::model::TtsConfig::AvailableTtsVoice;
use crate::data::model::TtsConfig::{
    TtsHttpHeader, TtsHttpResponsePipelineStep, TtsProviderCatalogEntry,
    TtsProviderOperationResultSpec, TtsProviderOperationSpec,
};

pub struct TtsCatalog;

const TTS_CATALOG_PROVIDER_ROWS: &str = r#"
SYSTEM_TTS|系统 TTS|||wav|POST|application/json||[]|[]|
HTTP_TTS|HTTP TTS||||POST|application/json||[]|[]|
OPENAI_COMPATIBLE|OpenAI 兼容|https://api.openai.com/v1/audio/speech|gpt-4o-mini-tts|mp3|POST|application/json|{"model":"{model}","voice":"{voice}","input":"{text}","response_format":"{responseFormat}","speed":{speed}}|[{"name":"Authorization","value":"Bearer {apiKey}"}]|[]|
MINIMAX_TTS|MiniMax|https://api.minimaxi.com/v1/t2a_v2|speech-2.8-hd|mp3|POST|application/json|{"model":"{model}","text":"{text}","stream":false,"output_format":"url","voice_setting":{"voice_id":"{voice}","speed":{speed},"vol":1,"pitch":0},"audio_setting":{"sample_rate":32000,"bitrate":128000,"format":"{responseFormat}","channel":1},"subtitle_enable":false}|[{"name":"Authorization","value":"Bearer {apiKey}"}]|[{"stepType":"parse_json","path":""},{"stepType":"pick","path":"$.data.audio"},{"stepType":"http_get","path":""}]|
MIMO_TTS|MiMo|https://api.xiaomimimo.com/v1/chat/completions|mimo-v2.5-tts|wav|POST|application/json|{"model":"{model}","messages":[{"role":"user","content":"请自然朗读。语速设置：{speed}x，音高设置：{pitch}x。"},{"role":"assistant","content":"{text}"}],"audio":{"format":"{responseFormat}","voice":"{voice}"},"stream":false}|[{"name":"api-key","value":"{apiKey}"}]|[{"stepType":"parse_json","path":""},{"stepType":"pick","path":"$.choices[0].message.audio.data"},{"stepType":"base64_decode","path":""}]|
SILICONFLOW_TTS|SiliconFlow|https://api.siliconflow.cn/v1/audio/speech|FunAudioLLM/CosyVoice2-0.5B|mp3|POST|application/json|{"model":"{model}","voice":"{voice}","input":"{text}","response_format":"{responseFormat}","speed":{speed}}|[{"name":"Authorization","value":"Bearer {apiKey}"}]|[]|list_voices^GET^/v1/audio/voice/list^$.result^$.model^$.uri^$.customName^$.text^true^Authorization^Bearer {apiKey}^
ELEVENLABS_TTS|ElevenLabs|https://api.elevenlabs.io/v1/text-to-speech/{voice}?output_format={responseFormat}|eleven_multilingual_v2|mp3_44100_128|POST|application/json|{"text":"{text}","model_id":"{model}","voice_settings":{"stability":0.5,"similarity_boost":0.75}}|[{"name":"xi-api-key","value":"{apiKey}"}]|[]|list_voices^GET^/v2/voices^$.voices^^$.voice_id^$.name^$.category^true^xi-api-key^{apiKey}^
DOUBAO_TTS|豆包火山引擎|https://openspeech.bytedance.com/api/v1/tts||mp3|POST|application/json|{"app":{"appid":"{model}","token":"{apiKey}","cluster":"volcano_tts"},"user":{"uid":"operit2"},"audio":{"voice_type":"{voice}","encoding":"mp3","speed_ratio":{speed},"pitch_ratio":1},"request":{"reqid":"{uuid}","text":"{text}","operation":"query"}}|[{"name":"Authorization","value":"Bearer;{apiKey}"}]|[{"stepType":"parse_json","path":""},{"stepType":"pick","path":"$.data"},{"stepType":"base64_decode","path":""}]|
DEEPGRAM_TTS|Deepgram|https://api.deepgram.com/v1/speak?model={model}|aura-2-thalia-en|wav|POST|application/json|{"text":"{text}"}|[{"name":"Authorization","value":"Token {apiKey}"}]|[]|
GROQ_TTS|Groq|https://api.groq.com/openai/v1/audio/speech|playai-tts|wav|POST|application/json|{"model":"{model}","voice":"{voice}","input":"{text}","response_format":"{responseFormat}","speed":{speed}}|[{"name":"Authorization","value":"Bearer {apiKey}"}]|[]|
FISH_AUDIO_TTS|Fish Audio|https://api.fish.audio/v1/tts|speech-1.6|mp3|POST|application/json|{"text":"{text}","model":"{model}","reference_id":"{voice}","format":"{responseFormat}"}|[{"name":"Authorization","value":"Bearer {apiKey}"}]|[]|
AZURE_TTS|Azure Speech|https://eastus.tts.speech.microsoft.com/cognitiveservices/v1|zh-CN|mp3|POST|application/ssml+xml|<speak version="1.0" xml:lang="{model}"><voice name="{voice}">{textXml}</voice></speak>|[{"name":"Ocp-Apim-Subscription-Key","value":"{apiKey}"},{"name":"X-Microsoft-OutputFormat","value":"audio-24khz-48kbitrate-mono-mp3"},{"name":"User-Agent","value":"operit2"}]|[]|list_voices^GET^/cognitiveservices/voices/list^$^^$.ShortName^$.LocalName^$.Locale^true^Ocp-Apim-Subscription-Key^{apiKey}^
GOOGLE_CLOUD_TTS|Google Cloud TTS|https://texttospeech.googleapis.com/v1/text:synthesize?key={apiKey}|zh-CN|MP3|POST|application/json|{"input":{"text":"{text}"},"voice":{"languageCode":"{model}","name":"{voice}"},"audioConfig":{"audioEncoding":"{responseFormat}","speakingRate":{speed}}}|[]|[{"stepType":"parse_json","path":""},{"stepType":"pick","path":"$.audioContent"},{"stepType":"base64_decode","path":""}]|list_voices^GET^/v1/voices?key={apiKey}^$.voices^$.languageCodes[0]^$.name^$.name^$.ssmlGender^false^^^
GEMINI_TTS|Gemini TTS|https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={apiKey}|gemini-2.5-flash-preview-tts|wav|POST|application/json|{"contents":[{"parts":[{"text":"{text}"}]}],"generationConfig":{"responseModalities":["AUDIO"],"speechConfig":{"voiceConfig":{"prebuiltVoiceConfig":{"voiceName":"{voice}"}}}}}|[]|[{"stepType":"parse_json","path":""},{"stepType":"pick","path":"$.candidates[0].content.parts[0].inlineData.data"},{"stepType":"base64_decode","path":""}]|
KOKORO_TTS|Kokoro FastAPI|http://localhost:8880/v1/audio/speech|kokoro|mp3|POST|application/json|{"model":"{model}","voice":"{voice}","input":"{text}","response_format":"{responseFormat}","speed":{speed}}|[]|[]|
"#;

struct TtsCatalogVoiceRow {
    providerType: String,
    model: String,
    voice: String,
    displayName: String,
    description: String,
}

impl TtsCatalog {
    pub fn provider(providerTypeId: &str) -> Result<TtsProviderCatalogEntry, String> {
        Self::providers()?
            .into_iter()
            .find(|provider| provider.providerTypeId.eq_ignore_ascii_case(providerTypeId))
            .ok_or_else(|| format!("tts catalog provider not found: {providerTypeId}"))
    }

    pub fn providers() -> Result<Vec<TtsProviderCatalogEntry>, String> {
        dataLines(TTS_CATALOG_PROVIDER_ROWS)
            .map(parseProviderRow)
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn voicesForProvider(
        providerType: &str,
        model: &str,
        responseFormat: &str,
        speed: f64,
    ) -> Result<Vec<AvailableTtsVoice>, String> {
        Ok(parseVoiceRows(TtsCatalogRows::TTS_CATALOG_VOICE_ROWS)?
            .into_iter()
            .filter(|row| row.providerType.eq_ignore_ascii_case(providerType))
            .map(|row| AvailableTtsVoice {
                model: catalogModel(&row.model, model),
                voice: row.voice,
                displayName: row.displayName,
                description: row.description,
                responseFormat: responseFormat.trim().to_string(),
                speed,
            })
            .collect())
    }
}

#[allow(non_snake_case)]
fn parseProviderRow(line: &str) -> Result<TtsProviderCatalogEntry, String> {
    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() != 11 {
        return Err(format!("invalid tts provider catalog row: {line}"));
    }
    Ok(TtsProviderCatalogEntry {
        providerTypeId: parts[0].trim().to_string(),
        displayName: parts[1].trim().to_string(),
        defaultEndpoint: parts[2].trim().to_string(),
        defaultModel: parts[3].trim().to_string(),
        defaultResponseFormat: parts[4].trim().to_string(),
        defaultHttpMethod: parts[5].trim().to_string(),
        defaultContentType: parts[6].trim().to_string(),
        defaultRequestBody: parts[7].trim().to_string(),
        defaultHeaders: parseJsonArray::<TtsHttpHeader>(parts[8], "tts provider default headers")?,
        defaultResponsePipeline: parseJsonArray::<TtsHttpResponsePipelineStep>(
            parts[9],
            "tts provider default response pipeline",
        )?,
        operations: parseOperations(parts[10])?,
    })
}

#[allow(non_snake_case)]
fn parseJsonArray<T>(value: &str, label: &str) -> Result<Vec<T>, String>
where
    T: serde::de::DeserializeOwned,
{
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str::<Vec<T>>(trimmed).map_err(|error| format!("{label}: {error}"))
}

#[allow(non_snake_case)]
fn parseOperations(value: &str) -> Result<Vec<TtsProviderOperationSpec>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    trimmed.split(";;").map(parseOperation).collect()
}

#[allow(non_snake_case)]
fn parseOperation(value: &str) -> Result<TtsProviderOperationSpec, String> {
    let parts: Vec<&str> = value.split('^').collect();
    if parts.len() != 12 {
        return Err(format!("invalid tts provider operation row: {value}"));
    }
    let operationType = parts[0].trim().to_string();
    if operationType != "list_voices" {
        return Err(format!(
            "invalid tts provider operation type: {operationType}"
        ));
    }
    Ok(TtsProviderOperationSpec {
        operationType,
        handlerId: "http_json".to_string(),
        method: parts[1].trim().to_string(),
        path: parts[2].trim().to_string(),
        requiresApiKey: parseBool(parts[8], "tts operation requires api key", value)?,
        authHeaderName: parts[9].trim().to_string(),
        authHeaderValue: parts[10].trim().to_string(),
        body: parts[11].trim().to_string(),
        result: TtsProviderOperationResultSpec {
            itemsJsonPath: optionalString(parts[3]),
            modelJsonPath: optionalString(parts[4]),
            voiceJsonPath: optionalString(parts[5]),
            displayNameJsonPath: optionalString(parts[6]),
            descriptionJsonPath: optionalString(parts[7]),
        },
    })
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
fn parseBool(value: &str, label: &str, line: &str) -> Result<bool, String> {
    value
        .trim()
        .parse::<bool>()
        .map_err(|error| format!("invalid {label}: {line}: {error}"))
}

fn dataLines(rows: &str) -> impl Iterator<Item = &str> {
    rows.lines().map(str::trim).filter(|line| !line.is_empty())
}

#[allow(non_snake_case)]
fn parseVoiceRows(rows: &str) -> Result<Vec<TtsCatalogVoiceRow>, String> {
    dataLines(rows).map(parseVoiceRow).collect()
}

#[allow(non_snake_case)]
fn parseVoiceRow(line: &str) -> Result<TtsCatalogVoiceRow, String> {
    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() != 5 {
        return Err(format!("invalid tts voice catalog row: {line}"));
    }
    Ok(TtsCatalogVoiceRow {
        providerType: parts[0].trim().to_string(),
        model: parts[1].trim().to_string(),
        voice: parts[2].trim().to_string(),
        displayName: parts[3].trim().to_string(),
        description: parts[4].trim().to_string(),
    })
}

#[allow(non_snake_case)]
fn catalogModel(rowModel: &str, providerModel: &str) -> String {
    let rowModel = rowModel.trim();
    if rowModel.is_empty() {
        providerModel.trim().to_string()
    } else {
        rowModel.to_string()
    }
}
