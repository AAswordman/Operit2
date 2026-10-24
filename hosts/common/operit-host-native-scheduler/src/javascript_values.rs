//! Bounded, direct V8 value conversion for the Host JavaScript boundary.
use operit_host_api::{HostError, HostResult, HostJavaScriptValue};
use serde_json::{Map, Number, Value as JsonValue};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

mod snapshot;
#[cfg(test)]
mod tests;
mod writer;

pub(crate) use writer::to_js;
const MAX_DEPTH: usize = 128;
const MAX_NODES: usize = 1_000_000;

type Snapshots = Rc<RefCell<BTreeMap<usize, Box<snapshot::OwnedJson>>>>;

/// Keeps conversion primitives and private request capsules local to one isolate.
pub(crate) struct JsonIntrinsics {
    is_array: v8::Global<v8::Function>,
    plain_prototype: v8::Global<v8::Value>,
    capsule_key: v8::Global<v8::Private>,
    snapshots: Snapshots,
}

impl JsonIntrinsics {
    /// Captures immutable intrinsic handles before any plugin executes.
    pub(crate) fn new(scope: &mut v8::PinScope<'_, '_>) -> HostResult<Self> {
        let is_array = function(scope, "Array.isArray")?;
        let prototype = evaluate(scope, "Object.prototype")?;
        let plain_prototype = v8::Global::new(scope, prototype);
        let key = v8::Private::new(scope, None);
        Ok(Self {
            is_array,
            plain_prototype,
            capsule_key: v8::Global::new(scope, key),
            snapshots: Default::default(),
        })
    }

    /// Reads lifecycle result fields directly with a bounded traversal.
    pub(crate) fn from_js<'s>(
        &self,
        scope: &mut v8::PinScope<'s, '_>,
        value: v8::Local<'s, v8::Value>,
    ) -> HostResult<Option<JsonValue>> {
        self.convert(scope, value, &mut Vec::new(), &mut 0, 0)
    }

    /// Applies one shared resource budget to a complete Host argument list.
    pub(crate) fn arguments_from_js<'s>(
        &self,
        scope: &mut v8::PinScope<'s, '_>,
        arguments: impl Iterator<Item = v8::Local<'s, v8::Value>>,
    ) -> HostResult<Vec<HostJavaScriptValue>> {
        let mut nodes = 0;
        arguments
            .map(|value| {
                if value.is_uint8_array() {
                    return transfer_buffer(scope, value).map(HostJavaScriptValue::Buffer);
                }
                self.convert(scope, value, &mut Vec::new(), &mut nodes, 0)?
                    .ok_or_else(|| {
                        type_error(scope, "Host arguments must be JSON-compatible values")
                    }).map(HostJavaScriptValue::Json)
            })
            .collect()
    }

    /// Recursively reads fields under the binding's explicit data contract without JSON text.
    fn convert<'s>(
        &self,
        scope: &mut v8::PinScope<'s, '_>,
        value: v8::Local<'s, v8::Value>,
        ancestors: &mut Vec<v8::Local<'s, v8::Value>>,
        nodes: &mut usize,
        depth: usize,
    ) -> HostResult<Option<JsonValue>> {
        check_budget(scope, nodes, depth)?;
        if let Some((owned, _)) = self.take_owned(scope, value, nodes, depth)? {
            return Ok(Some(owned));
        }
        if value.is_null() {
            return Ok(Some(JsonValue::Null));
        }
        if value.is_boolean() {
            return Ok(Some(JsonValue::Bool(value.boolean_value(scope))));
        }
        if value.is_number() {
            return Ok(Some(number(scope, value, false)?));
        }
        if value.is_string() {
            return Ok(Some(JsonValue::String(value.to_rust_string_lossy(scope))));
        }
        if value.is_big_int() {
            return Err(type_error(
                scope,
                "BigInt is not supported in Host data",
            ));
        }
        if value.is_undefined() || value.is_symbol() || value.is_function() {
            return Ok(None);
        }
        if ancestors
            .iter()
            .any(|ancestor| ancestor.strict_equals(value))
        {
            return Err(type_error(
                scope,
                "Converting circular structure to tool parameters",
            ));
        }
        let object = v8::Local::<v8::Object>::try_from(value).map_err(|_| engine_error())?;
        ancestors.push(value);
        let array = self.array(scope, value)?;
        let result = if array {
            let length = array_length(scope, object)?;
            if length > MAX_NODES.saturating_sub(*nodes) {
                return Err(range_error(
                    scope,
                    "Tool parameters exceed structured bridge node limit",
                ));
            }
            let mut items = Vec::with_capacity(length);
            for index in 0..length {
                let element = object
                    .get_index(scope, index as u32)
                    .ok_or_else(engine_error)?;
                items.push(
                    self.convert(
                        scope,
                        element,
                        ancestors,
                        nodes,
                        depth + 1,
                    )?
                    .unwrap_or(JsonValue::Null),
                );
            }
            JsonValue::Array(items)
        } else {
            let keys = object
                .get_own_property_names(
                    scope,
                    v8::GetPropertyNamesArgs {
                        mode: v8::KeyCollectionMode::OwnOnly,
                        key_conversion: v8::KeyConversionMode::ConvertToString,
                        ..Default::default()
                    },
                )
                .ok_or_else(engine_error)?;
            let mut items = Map::new();
            for index in 0..keys.length() {
                let key = keys.get_index(scope, index).ok_or_else(engine_error)?;
                let name = key.to_rust_string_lossy(scope);
                let element = object.get(scope, key).ok_or_else(engine_error)?;
                if let Some(element) =
                    self.convert(scope, element, ancestors, nodes, depth + 1)?
                {
                    items.insert(name, element);
                }
            }
            JsonValue::Object(items)
        };
        ancestors.pop();
        Ok(Some(result))
    }

    /// Uses the captured IsArray intrinsic for proxy arrays as well as ordinary arrays.
    fn array<'s>(
        &self,
        scope: &mut v8::PinScope<'s, '_>,
        value: v8::Local<'s, v8::Value>,
    ) -> HostResult<bool> {
        if !value.is_proxy() {
            return Ok(value.is_array());
        }
        let function = v8::Local::new(scope, &self.is_array);
        let receiver = v8::undefined(scope).into();
        Ok(function
            .call(scope, receiver, &[value])
            .ok_or_else(engine_error)?
            .boolean_value(scope))
    }
}

/// Evaluates trusted intrinsic source inside the current isolate and context.
fn evaluate<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: &str,
) -> HostResult<v8::Local<'s, v8::Value>> {
    let source = text(scope, source)?;
    v8::Script::compile(scope, source, None)
        .ok_or_else(engine_error)?
        .run(scope)
        .ok_or_else(engine_error)
}

/// Persists one captured intrinsic function for this isolate.
fn function(
    scope: &mut v8::PinScope<'_, '_>,
    source: &str,
) -> HostResult<v8::Global<v8::Function>> {
    let value = evaluate(scope, source)?;
    let function = v8::Local::<v8::Function>::try_from(value).map_err(|_| engine_error())?;
    Ok(v8::Global::new(scope, function))
}

/// Allocates one V8 string and reports allocation failure explicitly.
pub(crate) fn text<'s>(
    scope: &v8::PinScope<'s, '_>,
    value: &str,
) -> HostResult<v8::Local<'s, v8::String>> {
    v8::String::new(scope, value).ok_or_else(engine_error)
}

/// Identifies an engine operation whose actual exception is read by the enclosing TryCatch.
pub(crate) fn engine_error() -> HostError {
    HostError::new("V8 JavaScript operation failed")
}

/// Raises a JavaScript TypeError while retaining its message for Rust callers.
pub(crate) fn type_error(scope: &mut v8::PinScope<'_, '_>, message: &str) -> HostError {
    let text = v8::String::new(scope, message).expect("static JavaScript error message");
    let exception = v8::Exception::type_error(scope, text);
    scope.throw_exception(exception);
    HostError::new(message)
}

/// Raises a JavaScript RangeError for a boundary resource limit.
pub(crate) fn range_error(scope: &mut v8::PinScope<'_, '_>, message: &str) -> HostError {
    let text = v8::String::new(scope, message).expect("static JavaScript error message");
    let exception = v8::Exception::range_error(scope, text);
    scope.throw_exception(exception);
    HostError::new(message)
}

/// Enforces the common structured bridge traversal budget.
fn check_budget(
    scope: &mut v8::PinScope<'_, '_>,
    nodes: &mut usize,
    depth: usize,
) -> HostResult<()> {
    *nodes += 1;
    if *nodes > MAX_NODES || depth > MAX_DEPTH {
        return Err(range_error(
            scope,
            "Tool request exceeds structured depth/node limit",
        ));
    }
    Ok(())
}

/// Preserves integral JSON numbers and JavaScript's decimal spelling for large integers.
fn number(
    scope: &mut v8::PinScope<'_, '_>,
    value: v8::Local<'_, v8::Value>,
    strict: bool,
) -> HostResult<JsonValue> {
    let number = value.number_value(scope).ok_or_else(engine_error)?;
    if !number.is_finite() {
        return if strict {
            Err(type_error(scope, "Tool request contains a non-JSON value"))
        } else {
            Ok(JsonValue::Null)
        };
    }
    if number.fract() == 0.0 {
        if number.abs() <= 9_007_199_254_740_991.0 {
            return Ok(JsonValue::Number(if number < 0.0 {
                Number::from(number as i64)
            } else {
                Number::from(number as u64)
            }));
        }
        let decimal = value
            .to_string(scope)
            .ok_or_else(engine_error)?
            .to_rust_string_lossy(scope);
        return decimal
            .parse::<Number>()
            .map(JsonValue::Number)
            .map_err(|error| HostError::new(error.to_string()));
    }
    Ok(JsonValue::Number(
        Number::from_f64(number).expect("finite JSON number"),
    ))
}

/// Applies JavaScript ToLength and rejects arrays exceeding the traversal budget.
fn array_length(
    scope: &mut v8::PinScope<'_, '_>,
    value: v8::Local<'_, v8::Object>,
) -> HostResult<usize> {
    let key = text(scope, "length")?;
    let length = value
        .get(scope, key.into())
        .ok_or_else(engine_error)?
        .number_value(scope)
        .ok_or_else(engine_error)?;
    if length.is_nan() || length <= 0.0 {
        return Ok(0);
    }
    if length > MAX_NODES as f64 {
        return Err(range_error(
            scope,
            "Tool request exceeds structured node limit",
        ));
    }
    Ok(length.floor() as usize)
}

/// Retains detached V8 storage until its final Rust or Dart owner releases it.
struct TransferredBuffer(v8::SharedRef<v8::BackingStore>);

impl AsRef<[u8]> for TransferredBuffer {
    /// Exposes storage only after every JavaScript view has been detached.
    fn as_ref(&self) -> &[u8] {
        let length = self.0.byte_length();
        if length == 0 { return &[]; }
        // SAFETY: the retained backing store owns this initialized allocation and
        // detachment removes all JavaScript writers before this owner is created.
        unsafe { std::slice::from_raw_parts(self.0.data().expect("nonempty backing store").as_ptr().cast(), length) }
    }
}

/// Detaches one bounded byte view and moves its backing allocation without reading its payload.
fn transfer_buffer<'s>(scope: &mut v8::PinScope<'s, '_>, value: v8::Local<'s, v8::Value>) -> HostResult<bytes::Bytes> {
    let view = v8::Local::<v8::Uint8Array>::try_from(value).map_err(|_| engine_error())?;
    let offset = view.byte_offset();
    let length = view.byte_length();
    let buffer = view.buffer(scope).ok_or_else(engine_error)?;
    let store = buffer.get_backing_store();
    if store.is_shared() || store.is_resizable_by_user_javascript() || buffer.was_detached() {
        return Err(type_error(scope, "Host transfer requires an attached fixed ArrayBuffer"));
    }
    let end = offset.checked_add(length).filter(|end| *end <= store.byte_length())
        .ok_or_else(|| type_error(scope, "Host byte view exceeds its backing storage"))?;
    if buffer.detach(None) != Some(true) { return Err(type_error(scope, "Host buffer cannot be detached")); }
    Ok(bytes::Bytes::from_owner(TransferredBuffer(store)).slice(offset..end))
}
