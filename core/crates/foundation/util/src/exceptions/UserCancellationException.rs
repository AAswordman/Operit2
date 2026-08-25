use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UserCancellationException {
    pub message: String,
    pub cause: Option<String>,
}

impl UserCancellationException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cause: None,
        }
    }

    pub fn with_cause(message: impl Into<String>, cause: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cause: Some(cause.into()),
        }
    }
}

impl Display for UserCancellationException {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(cause) = &self.cause {
            write!(formatter, "{}: {}", self.message, cause)
        } else {
            write!(formatter, "{}", self.message)
        }
    }
}

impl Error for UserCancellationException {}
