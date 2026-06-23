#[path = "../collects/TtsCatalog.rs"]
mod TtsCatalogRows;

use crate::data::model::TtsConfig::AvailableTtsVoice;

pub struct TtsCatalog;

struct TtsCatalogVoiceRow {
    providerType: String,
    model: String,
    voice: String,
    displayName: String,
    description: String,
}

impl TtsCatalog {
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
