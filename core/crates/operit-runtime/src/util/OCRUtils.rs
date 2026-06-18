use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::util::AppLogger::AppLogger;
use operit_host_api::{OCRLanguage, OCRQuality};

const TAG: &str = "OCRUtils";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    LATIN,
    CHINESE,
    JAPANESE,
    KOREAN,
}

impl Language {
    fn toHostLanguage(self) -> OCRLanguage {
        match self {
            Language::LATIN => OCRLanguage::Latin,
            Language::CHINESE => OCRLanguage::Chinese,
            Language::JAPANESE => OCRLanguage::Japanese,
            Language::KOREAN => OCRLanguage::Korean,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    LOW,
    HIGH,
}

impl Quality {
    fn toHostQuality(self) -> OCRQuality {
        match self {
            Quality::LOW => OCRQuality::Low,
            Quality::HIGH => OCRQuality::High,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OCRResult {
    Success(String),
    Error(String),
}

impl OCRResult {
    #[allow(non_snake_case)]
    pub fn getFullText(&self) -> String {
        match self {
            OCRResult::Success(text) => text.clone(),
            OCRResult::Error(_) => String::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OCRUtils;

impl OCRUtils {
    #[allow(non_snake_case)]
    fn recognizeTextFromPathInternal(
        context: &OperitApplicationContext,
        imagePath: &str,
        language: Language,
        quality: Quality,
        logError: bool,
    ) -> OCRResult {
        let host = match context.systemOperationHost.as_ref() {
            Some(host) => host,
            None => {
                let message = "SystemOperationHost is required for OCR".to_string();
                if logError {
                    AppLogger::e(TAG, &message);
                }
                return OCRResult::Error(message);
            }
        };
        match host.recognizeText(
            imagePath,
            language.toHostLanguage(),
            quality.toHostQuality(),
        ) {
            Ok(text) => OCRResult::Success(text),
            Err(error) => {
                if logError {
                    AppLogger::e(TAG, &format!("Text recognition failed: {}", error.message));
                }
                OCRResult::Error(error.message)
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn recognizeTextFromPath(
        context: &OperitApplicationContext,
        imagePath: &str,
        language: Language,
        quality: Quality,
    ) -> OCRResult {
        Self::recognizeTextFromPathInternal(context, imagePath, language, quality, true)
    }

    #[allow(non_snake_case)]
    pub fn recognizeText(
        context: &OperitApplicationContext,
        imagePath: &str,
        quality: Quality,
    ) -> String {
        let latinResult = Self::recognizeTextFromPathInternal(
            context,
            imagePath,
            Language::LATIN,
            quality,
            false,
        );
        let chineseResult = Self::recognizeTextFromPathInternal(
            context,
            imagePath,
            Language::CHINESE,
            quality,
            false,
        );

        let latinText = match latinResult {
            OCRResult::Success(text) => text,
            OCRResult::Error(_) => String::new(),
        };
        let chineseText = match chineseResult {
            OCRResult::Success(text) => text,
            OCRResult::Error(_) => String::new(),
        };

        if latinText.is_empty() {
            return chineseText;
        }
        if chineseText.is_empty() {
            return latinText;
        }
        if latinText == chineseText {
            return latinText;
        }
        format!("{latinText}\n{chineseText}")
    }

    #[allow(non_snake_case)]
    pub fn recognizeTextWithLanguage(
        context: &OperitApplicationContext,
        imagePath: &str,
        language: Language,
        quality: Quality,
    ) -> String {
        match Self::recognizeTextFromPath(context, imagePath, language, quality) {
            OCRResult::Success(text) => text,
            OCRResult::Error(message) => {
                AppLogger::e(TAG, &format!("Text recognition failed: {message}"));
                String::new()
            }
        }
    }
}
