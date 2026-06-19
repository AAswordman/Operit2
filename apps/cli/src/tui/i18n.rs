use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::data::preferences::UserPreferencesManager::UserPreferencesManager;
use operit_runtime::util::LocaleUtils::{LanguageCodes, LocaleUtils};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TuiLanguage {
    English,
    Chinese,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TuiText {
    language: TuiLanguage,
}

include!(concat!(env!("OUT_DIR"), "/tui_i18n_generated.rs"));

impl TuiLanguage {
    pub(super) fn from_context(context: &OperitApplicationContext) -> Result<Self, String> {
        let language_code = LocaleUtils::getCurrentLanguage(context)?;
        Self::from_language_code(&language_code)
    }

    pub(super) fn from_language_code(language_code: &str) -> Result<Self, String> {
        let resolved = LocaleUtils::resolve_supported_language_code(language_code);
        match resolved.as_str() {
            LanguageCodes::ENGLISH => Ok(Self::English),
            LanguageCodes::CHINESE => Ok(Self::Chinese),
            other => Err(format!(
                "unsupported TUI language: {other}; supported languages: en, zh-CN"
            )),
        }
    }

    pub(super) fn save(self) -> Result<(), String> {
        UserPreferencesManager::getInstance()
            .saveAppLanguage(self.code().to_string())
            .map_err(|error| error.to_string())
    }

    pub(super) fn code(self) -> &'static str {
        match self {
            Self::English => LanguageCodes::ENGLISH,
            Self::Chinese => LanguageCodes::CHINESE,
        }
    }

    pub(super) fn display_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Chinese => "中文",
        }
    }

    pub(super) fn text(self) -> TuiText {
        TuiText { language: self }
    }
}

impl TuiText {
    pub(super) fn language(self) -> TuiLanguage {
        self.language
    }

    pub(super) fn raw(self, key: TuiTextKey) -> &'static str {
        lookup_text(self.language, key)
    }

    pub(super) fn render(self, key: TuiTextKey, args: &[(&str, String)]) -> String {
        let mut value = self.raw(key).to_string();
        for (name, arg) in args {
            value = value.replace(&format!("{{{name}}}"), arg);
        }
        value
    }

    pub(super) fn help_lines(self) -> &'static [&'static str] {
        match self.language {
            TuiLanguage::English => HELP_LINES_EN,
            TuiLanguage::Chinese => HELP_LINES_ZH_CN,
        }
    }

    pub(super) fn language_status(self, language: TuiLanguage) -> String {
        self.render(
            TuiTextKey::LanguageStatus,
            &[
                ("display_name", language.display_name().to_string()),
                ("code", language.code().to_string()),
            ],
        )
    }

    pub(super) fn language_updated(self, language: TuiLanguage) -> String {
        self.render(
            TuiTextKey::LanguageUpdated,
            &[
                ("display_name", language.display_name().to_string()),
                ("code", language.code().to_string()),
            ],
        )
    }

    pub(super) fn context_usage_raw(self, current: i32, max_tokens: i32) -> String {
        self.render(
            TuiTextKey::ContextUsageRaw,
            &[
                ("current", current.max(0).to_string()),
                ("max_tokens", max_tokens.to_string()),
            ],
        )
    }

    pub(super) fn context_usage(self, percent: i32, current: i32, max_tokens: i32) -> String {
        self.render(
            TuiTextKey::ContextUsage,
            &[
                ("percent", percent.to_string()),
                ("current", current.to_string()),
                ("max_tokens", max_tokens.to_string()),
            ],
        )
    }

    pub(super) fn processing_message(self, message: &str) -> String {
        match message {
            "enhanced_processing_input" => {
                self.raw(TuiTextKey::EnhancedProcessingInput).to_string()
            }
            "enhanced_processing_message" | "message_processing" => {
                self.raw(TuiTextKey::EnhancedProcessingMessage).to_string()
            }
            "enhanced_connecting_service" => self.connecting_ai_service().to_string(),
            "enhanced_receiving_response" => {
                self.raw(TuiTextKey::EnhancedReceivingResponse).to_string()
            }
            "enhanced_receiving_tool_result" => self
                .raw(TuiTextKey::EnhancedReceivingToolResult)
                .to_string(),
            "chat_processing_attachment" => {
                self.raw(TuiTextKey::ChatProcessingAttachment).to_string()
            }
            "chat_processing_shared_files" => {
                self.raw(TuiTextKey::ChatProcessingSharedFiles).to_string()
            }
            "chat_summarizing_memory" => self.raw(TuiTextKey::ChatSummarizingMemory).to_string(),
            "chat_summarizing_generating" => {
                self.raw(TuiTextKey::ChatSummarizingGenerating).to_string()
            }
            "compressing history" => self.raw(TuiTextKey::CompressingHistory).to_string(),
            _ => message.trim().to_string(),
        }
    }
}
