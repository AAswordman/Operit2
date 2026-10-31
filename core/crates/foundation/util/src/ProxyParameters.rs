//! The public proxy payload is YAML text; older JSON objects and text remain readable.

use serde_json::{Map, Value};

pub const PROXY_PARAMS_DESCRIPTION_EN: &str = "YAML mapping text containing the target tool parameters. Use indented literal blocks (|, |-, |+) for multiline code or text; |- omits the final newline, | keeps one, and |+ keeps all. Quote strings that look like numbers, booleans, or null. Example:\npath: /tmp/demo.py\ncontent: |-\n  print(\"hello\")";
pub const PROXY_PARAMS_DESCRIPTION_CN: &str = "目标工具参数的 YAML 映射文本。代码和多行文本使用缩进的原文块（|、|-、|+）：|- 不保留末尾换行，| 保留一个，|+ 保留全部。看起来像数字、布尔值或 null 的字符串需加引号。示例：\npath: /tmp/demo.py\ncontent: |-\n  print(\"hello\")";

/// Decodes the current YAML payload and both historical JSON representations.
/// The outer object must always have parameter names as keys.
pub fn parse_proxy_parameters(value: &Value) -> Result<Map<String, Value>, String> {
    match value {
        Value::Object(object) => Ok(object.clone()),
        Value::String(text) => {
            // Keep historical JSON number and string semantics exactly as stored.
            let decoded = match serde_json::from_str::<Value>(text) {
                Ok(value) => value,
                Err(_) => serde_saphyr::from_str::<Value>(text)
                    .map_err(|error| format!("Invalid proxy params YAML mapping: {error}"))?,
            };
            match decoded {
                Value::Object(object) => Ok(object),
                _ => Err(
                    "Proxy params must contain a YAML mapping of parameter names to values".into(),
                ),
            }
        }
        _ => Err("Proxy params must be YAML mapping text".into()),
    }
}

/// Emits all newly generated proxy payloads as YAML, using literal blocks for multiline strings.
pub fn format_proxy_parameters(parameters: &Map<String, Value>) -> String {
    serde_saphyr::to_string(parameters)
        .expect("JSON-compatible parameter mappings must serialize as YAML")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn yaml_preserves_code_and_parameter_types() {
        let payload = r#"path: C:\temp\demo.py
content: |+
  import re
  pattern = r"\d+\s+\w+"
  message = '他说："你好"'
  print(f"{message}\n匹配：{pattern}")

options:
  overwrite: true
  limit: 3
items: [one, two]
optional: null
numeric_text: "0012"
boolean_text: "true"
"#;
        let parsed = parse_proxy_parameters(&Value::String(payload.into())).unwrap();
        assert_eq!(parsed["path"], r"C:\temp\demo.py");
        assert_eq!(parsed["content"], "import re\npattern = r\"\\d+\\s+\\w+\"\nmessage = '他说：\"你好\"'\nprint(f\"{message}\\n匹配：{pattern}\")\n\n");
        assert_eq!(parsed["options"], json!({"overwrite": true, "limit": 3}));
        assert_eq!(parsed["items"], json!(["one", "two"]));
        assert_eq!(parsed["optional"], Value::Null);
        assert_eq!(parsed["numeric_text"], "0012");
        assert_eq!(parsed["boolean_text"], "true");
    }

    #[test]
    fn historical_json_objects_and_text_keep_their_values() {
        let object = json!({"content": "  \"quoted\" \\d+\n\n", "large": 18446744073709551615u64, "items": [1, true, null]});
        assert_eq!(
            parse_proxy_parameters(&object).unwrap(),
            object.as_object().unwrap().clone()
        );
        assert_eq!(
            parse_proxy_parameters(&Value::String(object.to_string())).unwrap(),
            object.as_object().unwrap().clone()
        );
    }

    #[test]
    fn generated_yaml_round_trips_literal_blocks_and_ambiguous_strings() {
        let parameters = json!({
            "content": "  print(\"\\d+\")\n\n",
            "no_newline": "first\nsecond",
            "number": "0012",
            "large_number": 18446744073709551615u64,
            "crlf": "first\r\nsecond\r\n",
            "tabs": "\tfirst\n\tsecond\n",
            "boolean": "true",
            "null": "null",
            "nested": {"enabled": true, "values": [1, 2]},
        })
        .as_object()
        .unwrap()
        .clone();
        let encoded = format_proxy_parameters(&parameters);
        assert!(
            encoded.contains('|'),
            "multiline strings must use literal blocks: {encoded}"
        );
        assert_eq!(
            parse_proxy_parameters(&Value::String(encoded)).unwrap(),
            parameters
        );
    }

    #[test]
    fn literal_block_chomping_preserves_requested_final_newlines() {
        for (indicator, expected) in [
            ("|-", "first\nsecond"),
            ("|", "first\nsecond\n"),
            ("|+", "first\nsecond\n\n"),
        ] {
            let text = format!("content: {indicator}\n  first\n  second\n\n");
            assert_eq!(
                parse_proxy_parameters(&Value::String(text)).unwrap()["content"],
                expected
            );
        }
    }

    #[test]
    fn rejects_non_mapping_and_malformed_payloads() {
        for payload in [
            json!(null),
            json!(true),
            json!([]),
            json!(""),
            json!("just text"),
            json!("[1, 2]"),
            json!("content: [unfinished"),
            json!("a: 1\n---\nb: 2"),
        ] {
            assert!(
                parse_proxy_parameters(&payload).is_err(),
                "must reject {payload}"
            );
        }
    }
}
