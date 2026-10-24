//! Direct JSON-to-V8 construction using own data properties.
use super::{check_budget, engine_error, range_error, text, MAX_NODES};
use operit_host_api::HostResult;
use serde_json::Value as JsonValue;

/// Materializes one owned result without invoking prototype setters or JSON globals.
pub(crate) fn to_js<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: &JsonValue,
) -> HostResult<v8::Local<'s, v8::Value>> {
    write(scope, value, &mut 0, 0)
}

/// Writes one tree under the common structured bridge depth and node limits.
fn write<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: &JsonValue,
    nodes: &mut usize,
    depth: usize,
) -> HostResult<v8::Local<'s, v8::Value>> {
    check_budget(scope, nodes, depth)?;
    Ok(match value {
        JsonValue::Null => v8::null(scope).into(),
        JsonValue::Bool(value) => v8::Boolean::new(scope, *value).into(),
        JsonValue::Number(value) => {
            v8::Number::new(scope, value.as_f64().expect("JSON number")).into()
        }
        JsonValue::String(value) => text(scope, value)?.into(),
        JsonValue::Array(values) => {
            if values.len() > MAX_NODES.saturating_sub(*nodes) {
                return Err(range_error(
                    scope,
                    "Host result exceeds structured node limit",
                ));
            }
            let elements = values
                .iter()
                .map(|value| write(scope, value, nodes, depth + 1))
                .collect::<HostResult<Vec<_>>>()?;
            v8::Array::new_with_elements(scope, &elements).into()
        }
        JsonValue::Object(values) => {
            let object = v8::Object::new(scope);
            for (key, value) in values {
                let key = text(scope, key)?;
                let value = write(scope, value, nodes, depth + 1)?;
                if object.create_data_property(scope, key.into(), value) != Some(true) {
                    return Err(engine_error());
                }
            }
            object.into()
        }
    })
}
