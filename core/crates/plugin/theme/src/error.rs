use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidationStage {
    Input,
    Archive,
    Contract,
    Integrity,
    Media,
    Trust,
}

/// Stable, transportable errors. Paths are package-local JSON pointers or files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeValidationError {
    pub code: &'static str,
    pub stage: ValidationStage,
    pub path: String,
    pub message: String,
}

impl ThemeValidationError {
    pub(crate) fn new(
        code: &'static str,
        stage: ValidationStage,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self { code, stage, path: path.into(), message: message.into() }
    }

    pub(crate) fn contract(path: &str, message: impl Into<String>) -> Self {
        Self::new("theme-schema-invalid", ValidationStage::Contract, path, message)
    }
}

impl fmt::Display for ThemeValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}: {}", self.code, self.path, self.message)
    }
}

impl std::error::Error for ThemeValidationError {}
