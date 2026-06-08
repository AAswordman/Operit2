use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;

pub type ToolPkgTextResourceResolver = Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>;

#[derive(Clone, Debug)]
pub struct LogSnapshot {
    pub pluginTag: String,
    pub functionName: String,
    pub codeSnippet: String,
}

#[derive(Clone, Default)]
pub struct JsToolPkgExecutionContext {
    temporaryTextResolver: Arc<Mutex<Option<ToolPkgTextResourceResolver>>>,
}

impl JsToolPkgExecutionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn capture(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
    ) -> LogSnapshot {
        LogSnapshot {
            pluginTag: resolvePluginTag(functionName, params),
            functionName: functionName.trim().to_string(),
            codeSnippet: buildSnippet(script, functionName),
        }
    }

    pub fn hasActivePluginIdForLogs(&self, snapshot: Option<&LogSnapshot>) -> bool {
        snapshot
            .map(|snapshot| !snapshot.pluginTag.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn withPluginTag(&self, snapshot: Option<&LogSnapshot>, message: &str) -> String {
        let tag = compactTag(snapshot.map(|snapshot| snapshot.pluginTag.as_str()));
        let prefix = format!("[{tag}] ");
        if message.starts_with(&prefix) {
            message.to_string()
        } else {
            format!("{prefix}{message}")
        }
    }

    pub fn withCodeContext(&self, snapshot: Option<&LogSnapshot>, message: &str) -> String {
        let functionName = snapshot
            .map(|snapshot| snapshot.functionName.trim())
            .unwrap_or("");
        let codeSnippet = snapshot
            .map(|snapshot| snapshot.codeSnippet.trim())
            .unwrap_or("");
        if functionName.is_empty() && codeSnippet.is_empty() {
            return message.to_string();
        }
        let mut output = message.to_string();
        if !functionName.is_empty() {
            output.push_str("\nExecution Function: ");
            output.push_str(functionName);
        }
        if !codeSnippet.is_empty() {
            output.push_str("\nCode Context:\n");
            output.push_str(codeSnippet);
        }
        output
    }

    pub fn setTemporaryTextResourceResolver(&self, resolver: Option<ToolPkgTextResourceResolver>) {
        let mut guard = self
            .temporaryTextResolver
            .lock()
            .expect("toolpkg text resolver mutex poisoned");
        *guard = resolver;
    }

    pub fn resolveTemporaryTextResource(
        &self,
        packageNameOrSubpackageId: &str,
        resourcePath: &str,
    ) -> Option<String> {
        let resolver = self
            .temporaryTextResolver
            .lock()
            .expect("toolpkg text resolver mutex poisoned")
            .clone();
        resolver.and_then(|resolver| resolver(packageNameOrSubpackageId, resourcePath))
    }

    pub fn hasTemporaryTextResourceResolver(&self) -> bool {
        self.temporaryTextResolver
            .lock()
            .expect("toolpkg text resolver mutex poisoned")
            .is_some()
    }
}

fn resolvePluginTag(functionName: &str, params: &BTreeMap<String, Value>) -> String {
    for key in ["__operit_plugin_id", "pluginId", "hookId"] {
        if let Some(value) = firstNonBlank(params.get(key)) {
            return value;
        }
    }

    let packageName = [
        "toolPkgId",
        "__operit_ui_package_name",
        "__operit_ui_toolpkg_id",
        "packageName",
    ]
    .iter()
    .find_map(|key| firstNonBlank(params.get(*key)));
    let normalizedFunction = functionName.trim();
    let normalizedFunction = if normalizedFunction.is_empty() {
        "runtime"
    } else {
        normalizedFunction
    };
    match packageName {
        Some(packageName) => format!("{normalizedFunction}:{packageName}"),
        None => normalizedFunction.to_string(),
    }
}

fn buildSnippet(script: &str, functionName: &str) -> String {
    let normalized = script.replace("\r\n", "\n");
    if normalized.trim().is_empty() {
        return String::new();
    }
    let lines = normalized.lines().collect::<Vec<_>>();
    let functionName = functionName.trim();
    let anchor = if functionName.is_empty() {
        None
    } else {
        lines.iter().position(|line| {
            line.contains(&format!("function {functionName}"))
                || line.contains(&format!("{functionName}: function"))
                || line.contains(&format!("{functionName} ="))
                || line.contains(&format!("exports.{functionName}"))
                || line.contains(&format!("module.exports.{functionName}"))
        })
    };
    let anchorIndex = anchor.unwrap_or(0);
    let start = anchorIndex.saturating_sub(6);
    let end = (anchorIndex + 6).min(lines.len().saturating_sub(1));
    let mut output = String::new();
    if let Some(anchor) = anchor {
        output.push_str(&format!("anchorLine={}\n", anchor + 1));
    }
    for index in start..=end {
        output.push_str(&format!("{:>4} | {}\n", index + 1, lines[index]));
    }
    output.trim_end().chars().take(2200).collect()
}

fn compactTag(raw: Option<&str>) -> String {
    let value = raw.unwrap_or("").trim();
    if value.is_empty() {
        return "runtime".to_string();
    }
    let compact = value
        .rsplit('/')
        .next()
        .unwrap_or(value)
        .rsplit('.')
        .next()
        .unwrap_or(value)
        .trim_end_matches("_bundle")
        .trim_end_matches(".toolpkg");
    if compact.is_empty() {
        value.to_string()
    } else {
        compact.to_string()
    }
}

fn firstNonBlank(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|value| {
            value
                .as_str()
                .map(ToString::to_string)
                .or_else(|| Some(value.to_string()))
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
