use std::collections::BTreeMap;

use operit_tools::packTool::ToolPkgParser::ToolPkgLoadResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgRuntimeOptions {
    pub languageCode: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgLoadOutcome {
    pub package: ToolPkgLoadResult,
    pub packageLoadErrors: Vec<ToolPkgPackageLoadError>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgPackageLoadError {
    pub packageName: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgFunctionCall {
    pub script: String,
    pub functionName: String,
    #[serde(default)]
    pub params: BTreeMap<String, Value>,
    #[serde(default)]
    pub envOverrides: BTreeMap<String, String>,
    pub executionContextKey: Option<String>,
    pub timeoutSeconds: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgMainHookCall {
    pub containerPackageName: String,
    pub functionName: String,
    pub event: String,
    pub eventName: Option<String>,
    pub pluginId: Option<String>,
    pub functionSource: Option<String>,
    #[serde(default)]
    pub eventPayload: Value,
    pub executionContextKey: Option<String>,
    pub runtimeKind: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgIpcCall {
    pub packageTarget: String,
    pub callerContextKey: Option<String>,
    pub targetContextKey: Option<String>,
    pub targetRuntime: Option<String>,
    pub channel: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgExecutionOutcome {
    pub value: Option<String>,
}
