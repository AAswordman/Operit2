use operit_context::OperitApplicationContext::OperitApplicationContext;
use crate::data::preferences::UserPreferencesManager::UserPreferencesManager;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Language {
    pub code: String,
    pub display_name: String,
    pub native_name: String,
}

pub struct LanguageCodes;

impl LanguageCodes {
    pub const AUTO: &'static str = "system";
    pub const CHINESE: &'static str = "zh-CN";
    pub const ENGLISH: &'static str = "en";
    pub const KOREAN: &'static str = "ko";
    pub const SPANISH: &'static str = "es";
    pub const MALAY: &'static str = "ms";
    pub const INDONESIAN: &'static str = "id";
    pub const PORTUGUESE_BRAZIL: &'static str = "pt-BR";
}

pub struct LocaleUtils;

impl LocaleUtils {
    pub fn get_supported_languages() -> Vec<Language> {
        vec![
            language(LanguageCodes::AUTO, "Follow system", "跟随系统"),
            language(LanguageCodes::CHINESE, "Chinese", "中文"),
            language(LanguageCodes::ENGLISH, "English", "English"),
            language(LanguageCodes::KOREAN, "Korean", "한국어"),
            language(LanguageCodes::SPANISH, "Spanish", "Español"),
            language(LanguageCodes::MALAY, "Malay", "Bahasa Melayu"),
            language(LanguageCodes::INDONESIAN, "Indonesian", "Bahasa Indonesia"),
            language(
                LanguageCodes::PORTUGUESE_BRAZIL,
                "Portuguese (Brazil)",
                "Português (Brasil)",
            ),
        ]
    }

    pub fn get_locale_for_language_code(
        language_code: &str,
        context: &OperitApplicationContext,
    ) -> Result<String, String> {
        let resolved = if language_code.trim().is_empty() || language_code == LanguageCodes::AUTO {
            current_system_language_code(context)?
        } else {
            Self::resolve_supported_language_code(language_code)
        };
        Ok(resolved)
    }

    pub fn normalize_stored_language_code(language_code: &str) -> String {
        if language_code.trim().is_empty() || language_code == LanguageCodes::AUTO {
            return language_code.to_string();
        }
        let normalized = language_code.replace('_', "-").replace("-r", "-");
        match normalized.as_str() {
            "zh" | "zh-Hans" | "zh-Hans-CN" => LanguageCodes::CHINESE.to_string(),
            "pt" => LanguageCodes::PORTUGUESE_BRAZIL.to_string(),
            "in" => LanguageCodes::INDONESIAN.to_string(),
            other => canonical_language_tag(other),
        }
    }

    pub fn resolve_supported_language_code(language_code: &str) -> String {
        let normalized = Self::normalize_stored_language_code(language_code);
        if normalized.trim().is_empty() || normalized == LanguageCodes::AUTO {
            return normalized;
        }
        let supported = supported_language_codes();
        if supported
            .iter()
            .any(|code| code.eq_ignore_ascii_case(&normalized))
        {
            return supported
                .into_iter()
                .find(|code| code.eq_ignore_ascii_case(&normalized))
                .unwrap()
                .to_string();
        }
        let language = normalized
            .split('-')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let matches: Vec<&str> = supported
            .iter()
            .copied()
            .filter(|code| {
                code.split('-')
                    .next()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(&language)
            })
            .collect();
        if matches.len() == 1 {
            matches[0].to_string()
        } else {
            normalized
        }
    }

    #[allow(non_snake_case)]
    pub fn getCurrentLanguage(context: &OperitApplicationContext) -> Result<String, String> {
        let savedLanguage = UserPreferencesManager::getInstance()
            .getCurrentLanguage()
            .map_err(|error| error.to_string())?;
        if !savedLanguage.trim().is_empty() && savedLanguage.trim() != LanguageCodes::AUTO {
            return Ok(Self::resolve_supported_language_code(&savedLanguage));
        }
        current_system_language_code(context)
    }
}

fn language(code: &str, display_name: &str, native_name: &str) -> Language {
    Language {
        code: code.to_string(),
        display_name: display_name.to_string(),
        native_name: native_name.to_string(),
    }
}

fn supported_language_codes() -> Vec<&'static str> {
    vec![
        LanguageCodes::CHINESE,
        LanguageCodes::ENGLISH,
        LanguageCodes::KOREAN,
        LanguageCodes::SPANISH,
        LanguageCodes::MALAY,
        LanguageCodes::INDONESIAN,
        LanguageCodes::PORTUGUESE_BRAZIL,
    ]
}

fn canonical_language_tag(code: &str) -> String {
    let mut pieces = code.split('-');
    let language = pieces.next().unwrap_or("").to_ascii_lowercase();
    let rest: Vec<String> = pieces
        .map(|piece| {
            if piece.len() == 2 {
                piece.to_ascii_uppercase()
            } else {
                piece.to_string()
            }
        })
        .collect();
    if rest.is_empty() {
        language
    } else {
        format!("{}-{}", language, rest.join("-"))
    }
}

fn current_system_language_code(context: &OperitApplicationContext) -> Result<String, String> {
    let host = context
        .systemOperationHost
        .as_ref()
        .ok_or_else(|| "SystemOperationHost is required to resolve system language".to_string())?;
    let languageCode = host
        .getSystemLanguageCode()
        .map_err(|error| error.to_string())?;
    Ok(LocaleUtils::resolve_supported_language_code(&languageCode))
}
