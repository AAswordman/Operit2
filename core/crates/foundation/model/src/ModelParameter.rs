use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelParameter<T> {
    pub id: String,
    pub name: String,
    pub apiName: String,
    pub description: String,
    pub defaultValue: T,
    pub currentValue: T,
    pub isEnabled: bool,
    pub valueType: ParameterValueType,
    pub minValue: Option<serde_json::Value>,
    pub maxValue: Option<serde_json::Value>,
    pub category: ParameterCategory,
    pub isCustom: bool,
}

impl<T> ModelParameter<T> {
    pub fn new(
        id: String,
        name: String,
        apiName: String,
        defaultValue: T,
        currentValue: T,
        isEnabled: bool,
        valueType: ParameterValueType,
    ) -> Self {
        Self {
            id,
            name,
            apiName,
            description: String::new(),
            defaultValue,
            currentValue,
            isEnabled,
            valueType,
            minValue: None,
            maxValue: None,
            category: ParameterCategory::OTHER,
            isCustom: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterValueType {
    INT,
    FLOAT,
    STRING,
    BOOLEAN,
    OBJECT,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterCategory {
    GENERATION,
    CREATIVITY,
    REPETITION,
    OTHER,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomParameterData {
    pub id: String,
    pub name: String,
    pub apiName: String,
    pub description: String,
    pub defaultValue: String,
    pub currentValue: String,
    pub isEnabled: bool,
    pub valueType: String,
    pub minValue: Option<String>,
    pub maxValue: Option<String>,
    pub category: String,
}

impl CustomParameterData {
    pub fn new(
        id: String,
        name: String,
        apiName: String,
        defaultValue: String,
        currentValue: String,
        isEnabled: bool,
        valueType: String,
    ) -> Self {
        Self {
            id,
            name,
            apiName,
            description: String::new(),
            defaultValue,
            currentValue,
            isEnabled,
            valueType,
            minValue: None,
            maxValue: None,
            category: "OTHER".to_string(),
        }
    }
}
