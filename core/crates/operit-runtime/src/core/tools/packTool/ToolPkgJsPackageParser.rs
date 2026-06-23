use crate::core::tools::ToolPackage::{
    EnvVar, LocalizedText, PackageTool, PackageToolParameter, ToolPackage, ToolPackageState,
};

#[allow(non_snake_case)]
pub fn parseToolPkgJsPackage(jsContent: &str) -> Result<ToolPackage, String> {
    let metadataString = extractMetadataFromJs(jsContent);
    let packageMetadata = parseToolPackageMetadata(&metadataString, jsContent)?;
    let tools = packageMetadata
        .tools
        .into_iter()
        .map(|tool| PackageTool {
            script: jsContent.to_string(),
            ..tool
        })
        .collect::<Vec<_>>();
    let states = packageMetadata
        .states
        .into_iter()
        .map(|state| ToolPackageState {
            tools: state
                .tools
                .into_iter()
                .map(|tool| PackageTool {
                    script: jsContent.to_string(),
                    ..tool
                })
                .collect(),
            ..state
        })
        .collect();
    Ok(ToolPackage {
        tools,
        states,
        ..packageMetadata
    })
}

#[allow(non_snake_case)]
pub fn extractMetadataFromJs(jsContent: &str) -> String {
    let metadataPattern =
        regex::Regex::new(r"/\*\s*METADATA\s*([\s\S]*?)\*/").expect("valid metadata regex");
    metadataPattern
        .captures(jsContent)
        .and_then(|captures| captures.get(1))
        .map(|metadata| metadata.as_str().trim().to_string())
        .unwrap_or_else(|| "{}".to_string())
}

#[allow(non_snake_case)]
pub fn parseToolPackageMetadata(metadataString: &str, script: &str) -> Result<ToolPackage, String> {
    let normalized = normalizeHjsonLikeMetadata(metadataString);
    let value: serde_json::Value =
        json5::from_str(&normalized).map_err(|error| error.to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "Package metadata must be an object".to_string())?;

    let name = stringField(object, "name");
    if name.is_empty() {
        return Err("Package metadata must have a name".to_string());
    }
    let toolsValue = object
        .get("tools")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tools = toolsValue
        .iter()
        .filter_map(|value| parsePackageTool(value, script).ok())
        .collect::<Vec<_>>();
    let states = object
        .get("states")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|value| parsePackageState(value, script).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let env = object
        .get("env")
        .and_then(serde_json::Value::as_array)
        .map(|items| items.iter().filter_map(parseEnvVar).collect::<Vec<_>>())
        .unwrap_or_default();

    Ok(ToolPackage {
        name,
        description: localizedTextField(object.get("description")),
        tools,
        states,
        env,
        is_built_in: boolField(object, "isBuiltIn") || boolField(object, "is_built_in"),
        enabled_by_default: boolField(object, "enabledByDefault")
            || boolField(object, "enabled_by_default"),
        display_name: localizedTextField(
            object
                .get("display_name")
                .or_else(|| object.get("displayName")),
        ),
        category: stringField(object, "category").if_empty_then("Other"),
        author: stringListField(object.get("author")),
    })
}

trait EmptyStringExt {
    fn if_empty_then(self, value: &str) -> String;
}

impl EmptyStringExt for String {
    fn if_empty_then(self, value: &str) -> String {
        if self.trim().is_empty() {
            value.to_string()
        } else {
            self
        }
    }
}

fn stringField(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    object
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn boolField(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> bool {
    match object.get(key) {
        Some(serde_json::Value::Bool(value)) => *value,
        Some(serde_json::Value::Number(value)) => value.as_i64().unwrap_or(0) != 0,
        Some(serde_json::Value::String(value)) => {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "true" | "1" | "yes" | "on"
            )
        }
        _ => false,
    }
}

fn localizedTextField(value: Option<&serde_json::Value>) -> LocalizedText {
    match value {
        Some(serde_json::Value::String(text)) => {
            let mut values = std::collections::HashMap::new();
            values.insert("default".to_string(), text.clone());
            LocalizedText { values }
        }
        Some(serde_json::Value::Object(object)) => {
            let mut values = std::collections::HashMap::new();
            for (key, value) in object {
                if let Some(text) = value.as_str() {
                    values.insert(key.clone(), text.to_string());
                }
            }
            LocalizedText { values }
        }
        _ => LocalizedText::default(),
    }
}

fn stringListField(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::String(text)) => vec![text.trim().to_string()]
            .into_iter()
            .filter(|item| !item.is_empty())
            .collect(),
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn parsePackageTool(value: &serde_json::Value, script: &str) -> Result<PackageTool, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "Package tool must be an object".to_string())?;
    let parameters = object
        .get("parameters")
        .and_then(serde_json::Value::as_array)
        .map(|items| items.iter().filter_map(parsePackageToolParameter).collect())
        .unwrap_or_default();
    Ok(PackageTool {
        name: stringField(object, "name"),
        description: localizedTextField(object.get("description")),
        parameters,
        script: script.to_string(),
        advice: boolField(object, "advice"),
    })
}

fn parsePackageToolParameter(value: &serde_json::Value) -> Option<PackageToolParameter> {
    let object = value.as_object()?;
    Some(PackageToolParameter {
        name: stringField(object, "name"),
        description: localizedTextField(object.get("description")),
        parameter_type: stringField(object, "type").if_empty_then("string"),
        required: object
            .get("required")
            .map(|_| boolField(object, "required"))
            .unwrap_or(true),
    })
}

fn parsePackageState(value: &serde_json::Value, script: &str) -> Result<ToolPackageState, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "Package state must be an object".to_string())?;
    let tools = object
        .get("tools")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| parsePackageTool(item, script).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(ToolPackageState {
        id: stringField(object, "id"),
        condition: stringField(object, "condition").if_empty_then("true"),
        inherit_tools: object
            .get("inheritTools")
            .or_else(|| object.get("inherit_tools"))
            .and_then(|_| {
                Some(boolField(object, "inheritTools") || boolField(object, "inherit_tools"))
            })
            .unwrap_or(false),
        exclude_tools: object
            .get("excludeTools")
            .or_else(|| object.get("exclude_tools"))
            .and_then(|value| Some(stringListField(Some(value))))
            .unwrap_or_default(),
        tools,
    })
}

fn parseEnvVar(value: &serde_json::Value) -> Option<EnvVar> {
    match value {
        serde_json::Value::String(name) => Some(EnvVar {
            name: name.trim().to_string(),
            description: LocalizedText::default(),
            required: true,
            default_value: None,
        }),
        serde_json::Value::Object(object) => Some(EnvVar {
            name: stringField(object, "name"),
            description: localizedTextField(object.get("description")),
            required: object
                .get("required")
                .map(|_| boolField(object, "required"))
                .unwrap_or(true),
            default_value: object
                .get("defaultValue")
                .or_else(|| object.get("default_value"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
        }),
        _ => None,
    }
}

#[allow(non_snake_case)]
fn normalizeHjsonLikeMetadata(input: &str) -> String {
    let mut lines = Vec::new();
    for rawLine in input.lines() {
        let line = stripInlineComment(rawLine).trim().to_string();
        if line.is_empty() {
            continue;
        }
        lines.push(normalizeBareWords(&line));
    }

    let mut output = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            let previous = lines[index - 1].trim_end();
            let current = line.trim_start();
            if needsCommaBetween(previous, current) {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str(line);
    }
    output
}

#[allow(non_snake_case)]
fn stripInlineComment(line: &str) -> String {
    let mut inString = false;
    let mut quote = '\0';
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if inString {
            if ch == quote && (index == 0 || chars[index - 1] != '\\') {
                inString = false;
            }
            index += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            inString = true;
            quote = ch;
            index += 1;
            continue;
        }
        if ch == '/' && index + 1 < chars.len() && chars[index + 1] == '/' {
            return chars[..index].iter().collect();
        }
        index += 1;
    }
    line.to_string()
}

#[allow(non_snake_case)]
fn normalizeBareWords(line: &str) -> String {
    let mut out = String::new();
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut inString = false;
    let mut quote = '\0';
    while index < chars.len() {
        let ch = chars[index];
        out.push(ch);
        if inString {
            if ch == quote && (index == 0 || chars[index - 1] != '\\') {
                inString = false;
            }
            index += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            inString = true;
            quote = ch;
            index += 1;
            continue;
        }
        if ch == ':' {
            let mut lookahead = index + 1;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                out.push(chars[lookahead]);
                lookahead += 1;
            }
            if lookahead >= chars.len() {
                index = lookahead;
                continue;
            }
            let next = chars[lookahead];
            if next == '"'
                || next == '\''
                || next == '{'
                || next == '['
                || next == '-'
                || next.is_ascii_digit()
            {
                index = lookahead;
                continue;
            }
            let mut end = lookahead;
            while end < chars.len() {
                let c = chars[end];
                if c == ',' || c == '}' || c == ']' {
                    break;
                }
                end += 1;
            }
            let rawValue = chars[lookahead..end].iter().collect::<String>();
            let value = rawValue.trim();
            let lower = value.to_ascii_lowercase();
            if matches!(lower.as_str(), "true" | "false" | "null") || value.is_empty() {
                out.push_str(value);
            } else {
                out.push('"');
                out.push_str(&value.replace('"', "\\\""));
                out.push('"');
            }
            index = end;
            continue;
        }
        index += 1;
    }
    out
}

#[allow(non_snake_case)]
fn needsCommaBetween(previous: &str, current: &str) -> bool {
    if previous.is_empty()
        || previous.ends_with(',')
        || previous.ends_with('{')
        || previous.ends_with('[')
        || current.starts_with('}')
        || current.starts_with(']')
    {
        return false;
    }
    true
}
