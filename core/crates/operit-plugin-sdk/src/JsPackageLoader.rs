use crate::package::{
    EnvVar, LocalizedText, PackageTool, PackageToolParameter, ToolPackage, ToolPackageState,
};
use operit_host_api::FileSystemHost;

/// Loads and parses standalone JavaScript plugin packages.
pub struct JsPackageLoader;

impl JsPackageLoader {
    /// Loads a JavaScript package through the supplied file-system host.
    pub fn load_from_file(
        fileSystemHost: &dyn FileSystemHost,
        sourcePath: &str,
    ) -> Result<ToolPackage, String> {
        let js_content = fileSystemHost
            .readFile(sourcePath)
            .map_err(|error| error.to_string())?;
        Self::parse(&js_content)
    }

    /// Parses a JavaScript package and attaches its script to every declared tool.
    pub fn parse(js_content: &str) -> Result<ToolPackage, String> {
        let metadata_string = Self::extract_metadata(js_content);
        let package_metadata = Self::parse_metadata(&metadata_string, js_content)?;
        let tools = package_metadata
            .tools
            .into_iter()
            .map(|tool| PackageTool {
                script: js_content.to_string(),
                ..tool
            })
            .collect::<Vec<_>>();
        let states = package_metadata
            .states
            .into_iter()
            .map(|state| ToolPackageState {
                tools: state
                    .tools
                    .into_iter()
                    .map(|tool| PackageTool {
                        script: js_content.to_string(),
                        ..tool
                    })
                    .collect(),
                ..state
            })
            .collect();
        Ok(ToolPackage {
            tools,
            states,
            ..package_metadata
        })
    }

    /// Parses package metadata represented as JSON, JSON5, or the supported HJSON-like form.
    pub fn parse_metadata(metadata_string: &str, script: &str) -> Result<ToolPackage, String> {
        let normalized = normalize_hjson_like_metadata(metadata_string);
        let value: serde_json::Value =
            json5::from_str(&normalized).map_err(|error| error.to_string())?;
        let object = value
            .as_object()
            .ok_or_else(|| "Package metadata must be an object".to_string())?;

        let name = string_field(object, "name");
        if name.is_empty() {
            return Err("Package metadata must have a name".to_string());
        }
        let tools_value = object
            .get("tools")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let tools = tools_value
            .iter()
            .filter_map(|value| parse_package_tool(value, script).ok())
            .collect::<Vec<_>>();
        let states = object
            .get("states")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|value| parse_package_state(value, script).ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let env = object
            .get("env")
            .and_then(serde_json::Value::as_array)
            .map(|items| items.iter().filter_map(parse_env_var).collect::<Vec<_>>())
            .unwrap_or_default();

        Ok(ToolPackage {
            name,
            description: localized_text_field(object.get("description")),
            tools,
            states,
            env,
            is_built_in: bool_field(object, "isBuiltIn") || bool_field(object, "is_built_in"),
            enabled_by_default: bool_field(object, "enabledByDefault")
                || bool_field(object, "enabled_by_default"),
            display_name: localized_text_field(
                object
                    .get("display_name")
                    .or_else(|| object.get("displayName")),
            ),
            category: non_empty_or(string_field(object, "category"), "Other"),
            author: string_list_field(object.get("author")),
        })
    }

    /// Extracts the `METADATA` block embedded in a JavaScript package.
    pub fn extract_metadata(js_content: &str) -> String {
        let metadata_pattern =
            regex::Regex::new(r"/\*\s*METADATA\s*([\s\S]*?)\*/").expect("valid metadata regex");
        metadata_pattern
            .captures(js_content)
            .and_then(|captures| captures.get(1))
            .map(|metadata| metadata.as_str().trim().to_string())
            .unwrap_or_else(|| "{}".to_string())
    }
}

/// Returns the replacement when a parsed string is blank.
fn non_empty_or(value: String, replacement: &str) -> String {
    if value.trim().is_empty() {
        replacement.to_string()
    } else {
        value
    }
}

/// Reads a trimmed string field from a metadata object.
fn string_field(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    object
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Reads a permissive boolean field from a metadata object.
fn bool_field(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> bool {
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

/// Parses localized text from a string or language-keyed object.
fn localized_text_field(value: Option<&serde_json::Value>) -> LocalizedText {
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

/// Parses a string or string array into a normalized list.
fn string_list_field(value: Option<&serde_json::Value>) -> Vec<String> {
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

/// Parses one package tool declaration.
fn parse_package_tool(value: &serde_json::Value, script: &str) -> Result<PackageTool, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "Package tool must be an object".to_string())?;
    let parameters = object
        .get("parameters")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(parse_package_tool_parameter)
                .collect()
        })
        .unwrap_or_default();
    Ok(PackageTool {
        name: string_field(object, "name"),
        description: localized_text_field(object.get("description")),
        parameters,
        script: script.to_string(),
        advice: bool_field(object, "advice"),
    })
}

/// Parses one package tool parameter declaration.
fn parse_package_tool_parameter(value: &serde_json::Value) -> Option<PackageToolParameter> {
    let object = value.as_object()?;
    Some(PackageToolParameter {
        name: string_field(object, "name"),
        description: localized_text_field(object.get("description")),
        parameter_type: non_empty_or(string_field(object, "type"), "string"),
        required: object
            .get("required")
            .map(|_| bool_field(object, "required"))
            .unwrap_or(true),
    })
}

/// Parses one conditional package state.
fn parse_package_state(
    value: &serde_json::Value,
    script: &str,
) -> Result<ToolPackageState, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "Package state must be an object".to_string())?;
    let tools = object
        .get("tools")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| parse_package_tool(item, script).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(ToolPackageState {
        id: string_field(object, "id"),
        condition: non_empty_or(string_field(object, "condition"), "true"),
        inherit_tools: object
            .get("inheritTools")
            .or_else(|| object.get("inherit_tools"))
            .map(|_| bool_field(object, "inheritTools") || bool_field(object, "inherit_tools"))
            .unwrap_or(false),
        exclude_tools: object
            .get("excludeTools")
            .or_else(|| object.get("exclude_tools"))
            .map(|value| string_list_field(Some(value)))
            .unwrap_or_default(),
        tools,
    })
}

/// Parses one package environment variable declaration.
fn parse_env_var(value: &serde_json::Value) -> Option<EnvVar> {
    match value {
        serde_json::Value::String(name) => Some(EnvVar {
            name: name.trim().to_string(),
            description: LocalizedText::default(),
            required: true,
            default_value: None,
        }),
        serde_json::Value::Object(object) => Some(EnvVar {
            name: string_field(object, "name"),
            description: localized_text_field(object.get("description")),
            required: object
                .get("required")
                .map(|_| bool_field(object, "required"))
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

/// Normalizes the supported HJSON-like package metadata into JSON5 text.
fn normalize_hjson_like_metadata(input: &str) -> String {
    let mut lines = Vec::new();
    for raw_line in input.lines() {
        let line = strip_inline_comment(raw_line).trim().to_string();
        if line.is_empty() {
            continue;
        }
        lines.push(normalize_bare_words(&line));
    }

    let mut output = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            let previous = lines[index - 1].trim_end();
            let current = line.trim_start();
            if needs_comma_between(previous, current) {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str(line);
    }
    output
}

/// Removes a line comment while preserving quoted content.
fn strip_inline_comment(line: &str) -> String {
    let mut in_string = false;
    let mut quote = '\0';
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if in_string {
            if ch == quote && (index == 0 || chars[index - 1] != '\\') {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_string = true;
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

/// Quotes bare metadata values while preserving JSON primitives and containers.
fn normalize_bare_words(line: &str) -> String {
    let mut out = String::new();
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut in_string = false;
    let mut quote = '\0';
    while index < chars.len() {
        let ch = chars[index];
        out.push(ch);
        if in_string {
            if ch == quote && (index == 0 || chars[index - 1] != '\\') {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_string = true;
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
                let current = chars[end];
                if current == ',' || current == '}' || current == ']' {
                    break;
                }
                end += 1;
            }
            let raw_value = chars[lookahead..end].iter().collect::<String>();
            let value = raw_value.trim();
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

/// Returns whether two normalized metadata lines require a separating comma.
fn needs_comma_between(previous: &str, current: &str) -> bool {
    !(previous.is_empty()
        || previous.ends_with(',')
        || previous.ends_with('{')
        || previous.ends_with('[')
        || current.starts_with('}')
        || current.starts_with(']'))
}

#[cfg(test)]
mod tests {
    use super::JsPackageLoader;

    /// Verifies standalone JavaScript package metadata parsing.
    #[test]
    fn parses_embedded_package_metadata() {
        let source = r#"
            /* METADATA
            {
              name: demo_package
              displayName: Demo Package
              enabledByDefault: true
              tools: [
                {
                  name: echo
                  description: Echo text
                  parameters: [
                    { name: text, description: Text to echo, required: true }
                  ]
                }
              ]
            }
            */
            async function echo() {}
        "#;

        let package = JsPackageLoader::parse(source).expect("package metadata should parse");

        assert_eq!(package.name, "demo_package");
        assert_eq!(package.display_name.resolve(true), "Demo Package");
        assert!(package.enabled_by_default);
        assert_eq!(package.tools.len(), 1);
        assert_eq!(package.tools[0].name, "echo");
        assert_eq!(package.tools[0].script, source);
    }
}
