use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum BillingMode {
    TOKEN,
    COUNT,
}

impl BillingMode {
    #[allow(non_snake_case)]
    pub fn fromString(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_uppercase().as_str() {
            "TOKEN" => Ok(Self::TOKEN),
            "COUNT" => Ok(Self::COUNT),
            other => Err(format!("invalid BillingMode: {other}")),
        }
    }
}
