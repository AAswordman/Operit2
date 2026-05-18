#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Language {
    pub code: String,
    pub display_name: String,
    pub native_name: String,
}

pub struct LanguageCodes;

impl LanguageCodes {
    pub const AUTO: &'static str = "system";
    pub const CHINESE: &'static str = "zh";
    pub const ENGLISH: &'static str = "en";
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
            language(LanguageCodes::SPANISH, "Spanish", "Español"),
            language(LanguageCodes::MALAY, "Malay", "Bahasa Melayu"),
            language(LanguageCodes::INDONESIAN, "Indonesian", "Bahasa Indonesia"),
            language(LanguageCodes::PORTUGUESE_BRAZIL, "Portuguese (Brazil)", "Português (Brasil)"),
        ]
    }

    pub fn get_locale_for_language_code(language_code: &str) -> String {
        let resolved = if language_code.trim().is_empty() || language_code == LanguageCodes::AUTO {
            Self::resolve_supported_language_code(&system_language_tag())
        } else {
            Self::resolve_supported_language_code(language_code)
        };
        if resolved.is_empty() {
            LanguageCodes::ENGLISH.to_string()
        } else {
            resolved
        }
    }

    pub fn normalize_stored_language_code(language_code: &str) -> String {
        if language_code.trim().is_empty() || language_code == LanguageCodes::AUTO {
            return language_code.to_string();
        }
        let normalized = language_code.replace('_', "-").replace("-r", "-");
        match normalized.as_str() {
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
        if supported.iter().any(|code| code.eq_ignore_ascii_case(&normalized)) {
            return supported
                .into_iter()
                .find(|code| code.eq_ignore_ascii_case(&normalized))
                .unwrap()
                .to_string();
        }
        let language = normalized.split('-').next().unwrap_or("").to_ascii_lowercase();
        let matches: Vec<&str> = supported
            .iter()
            .copied()
            .filter(|code| code.split('-').next().unwrap_or("").eq_ignore_ascii_case(&language))
            .collect();
        if matches.len() == 1 {
            matches[0].to_string()
        } else {
            normalized
        }
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

fn system_language_tag() -> String {
    std::env::var("LANG")
        .ok()
        .and_then(|lang| lang.split('.').next().map(|value| value.replace('_', "-")))
        .unwrap_or_else(|| LanguageCodes::ENGLISH.to_string())
}
