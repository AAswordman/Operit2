//! Owned JSON-compatible values at the native engine boundary. No JSON text is
//! generated on the ordinary path. Only JS handles on the owner thread are used.
use rquickjs::{
    convert::Coerced, function::This, object::Property, qjs, Array, Ctx, Exception, FromJs,
    Function, IntoJs, Object, Persistent, Value,
};
use serde_json::{Map, Number, Value as JsonValue};

const MAX_DEPTH: usize = 128;
const MAX_NODES: usize = 1_000_000;

/// Captured intrinsics implement JSON's toJSON/unboxing rules even when globals
/// are subsequently replaced by a plugin. Class ids are discovered, not hardcoded.
pub(crate) struct JsonIntrinsics {
    prepare: Persistent<Function<'static>>,
    boolean_value: Persistent<Function<'static>>,
    bigint_value: Persistent<Function<'static>>,
    classes: [qjs::JSClassID; 5],
    is_array: Persistent<Function<'static>>,
}

impl JsonIntrinsics {
    pub(crate) fn new(ctx: &Ctx<'_>) -> rquickjs::Result<Self> {
        let wrappers: Array = ctx.eval(
            "[new Number(0), new String(''), new Boolean(false), Object(0n), new Proxy({}, {})]",
        )?;
        let mut classes = [0; 5];
        for (index, class) in classes.iter_mut().enumerate() {
            let value: Value = wrappers.get(index)?;
            // SAFETY: live object values belong to this context; GetClassID only
            // reads the object's class and neither consumes nor retains the value.
            *class = unsafe { qjs::JS_GetClassID(value.as_raw()) };
        }
        let prepare: Function = ctx.eval("(function(apply) { return function(value, key) { var f = value.toJSON; return typeof f === 'function' ? apply(f, value, [key]) : value; }; })(Reflect.apply)")?;
        let boolean_value: Function = ctx.eval("Boolean.prototype.valueOf")?;
        let bigint_value: Function = ctx.eval("BigInt.prototype.valueOf")?;
        let is_array: Function = ctx.eval("Array.isArray")?;
        Ok(Self {
            prepare: Persistent::save(ctx, prepare),
            boolean_value: Persistent::save(ctx, boolean_value),
            bigint_value: Persistent::save(ctx, bigint_value),
            is_array: Persistent::save(ctx, is_array),
            classes,
        })
    }

    pub(crate) fn from_js<'js>(
        &self,
        ctx: &Ctx<'js>,
        value: Value<'js>,
    ) -> rquickjs::Result<Option<JsonValue>> {
        let mut nodes = 0;
        self.convert(ctx, value, "", &mut Vec::new(), &mut nodes, 0)
    }

    fn convert<'js>(
        &self,
        ctx: &Ctx<'js>,
        mut value: Value<'js>,
        key: &str,
        stack: &mut Vec<Value<'js>>,
        nodes: &mut usize,
        depth: usize,
    ) -> rquickjs::Result<Option<JsonValue>> {
        *nodes += 1;
        if depth > MAX_DEPTH || *nodes > MAX_NODES {
            return Err(Exception::throw_range(
                ctx,
                "Tool parameters exceed structured bridge depth/node limit",
            ));
        }
        // SerializeJSONProperty: toJSON is called once, before unboxing and
        // before detecting an ancestor cycle. Primitive BigInts can have toJSON.
        if value.is_object() || value.is_big_int() {
            value = self.prepare.clone().restore(ctx)?.call((value, key))?;
        }
        let mut proxy_array = false;
        if value.is_object() {
            // SAFETY: GetClassID receives a live object and does not retain it.
            let class = unsafe { qjs::JS_GetClassID(value.as_raw()) };
            if class == self.classes[4] {
                proxy_array = self.is_array.clone().restore(ctx)?.call((value.clone(),))?;
            }
            if class == self.classes[0] {
                value = Coerced::<f64>::from_js(ctx, value)?.0.into_js(ctx)?;
            } else if class == self.classes[1] {
                value = Coerced::<String>::from_js(ctx, value)?.0.into_js(ctx)?;
            } else if class == self.classes[2] {
                value = self
                    .boolean_value
                    .clone()
                    .restore(ctx)?
                    .call((This(value),))?;
            } else if class == self.classes[3] {
                value = self
                    .bigint_value
                    .clone()
                    .restore(ctx)?
                    .call((This(value),))?;
            }
        }
        if value.is_null() {
            return Ok(Some(JsonValue::Null));
        }
        if let Some(flag) = value.as_bool() {
            return Ok(Some(JsonValue::Bool(flag)));
        }
        if let Some(number) = value.as_number() {
            // JSON text represented integral JS numbers as integer tokens. Keep
            // that distinction for SDK tools using Value::as_u64/as_i64.
            let number = if number.is_finite() && number.fract() == 0.0 {
                if number.abs() <= 9_007_199_254_740_991.0 {
                    Some(if number < 0.0 {
                        Number::from(number as i64)
                    } else {
                        Number::from(number as u64)
                    })
                } else {
                    // Beyond JS's exact integer range, the old text protocol
                    // parsed the shortest decimal spelling, not an f64->u64
                    // cast. Preserve that rare scalar case without serializing
                    // the request tree or invoking a mutable JSON global.
                    let decimal = Coerced::<String>::from_js(ctx, value.clone())?.0;
                    Some(
                        decimal
                            .parse::<Number>()
                            .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?,
                    )
                }
            } else {
                Number::from_f64(number)
            };
            return Ok(Some(
                number.map(JsonValue::Number).unwrap_or(JsonValue::Null),
            ));
        }
        if let Some(text) = value.as_string() {
            return Ok(Some(JsonValue::String(text.to_string()?)));
        }
        if value.is_big_int() {
            return Err(Exception::throw_type(
                ctx,
                "BigInt cannot be serialized in tool parameters",
            ));
        }
        if value.is_undefined() || value.is_symbol() || value.is_function() {
            return Ok(None);
        }
        let object = Object::from_js(ctx, value.clone())?;
        if stack.iter().any(|ancestor| ancestor == &value) {
            return Err(Exception::throw_type(
                ctx,
                "Converting circular structure to tool parameters",
            ));
        }
        stack.push(value.clone());
        let result = if value.is_array() || proxy_array {
            let array = value.as_array();
            let len = if let Some(array) = array {
                array.len()
            } else {
                // JSON's IsArray follows proxies; the low-level JS_IsArray does
                // not. Proxy length uses ToLength and may execute a get trap.
                let length: Coerced<f64> = object.get("length")?;
                if length.0.is_nan() || length.0 <= 0.0 {
                    0
                } else if length.0 > MAX_NODES as f64 {
                    MAX_NODES + 1
                } else {
                    length.0.floor() as usize
                }
            };
            if len > MAX_NODES.saturating_sub(*nodes) {
                return Err(Exception::throw_range(
                    ctx,
                    "Tool parameters exceed structured bridge node limit",
                ));
            }
            let mut items = Vec::with_capacity(len);
            for index in 0..len {
                let element: Value = if let Some(array) = array {
                    array.get(index)?
                } else {
                    object.get(index.to_string())?
                };
                items.push(
                    self.convert(ctx, element, &index.to_string(), stack, nodes, depth + 1)?
                        .unwrap_or(JsonValue::Null),
                );
            }
            JsonValue::Array(items)
        } else {
            // Snapshot enumerable own keys before invoking getters. This matches
            // JSON.stringify when a getter mutates sibling properties.
            let keys = object
                .keys::<String>()
                .collect::<rquickjs::Result<Vec<_>>>()?;
            let mut items = Map::new();
            for key in keys {
                let element: Value = object.get(key.as_str())?;
                if let Some(element) = self.convert(ctx, element, &key, stack, nodes, depth + 1)? {
                    items.insert(key, element);
                }
            }
            JsonValue::Object(items)
        };
        stack.pop();
        Ok(Some(result))
    }
}

/// Creates JS data properties, not assignments: a JSON key such as __proto__
/// must not invoke an inherited setter or change the result object's prototype.
pub(crate) fn to_js<'js>(ctx: &Ctx<'js>, value: &JsonValue) -> rquickjs::Result<Value<'js>> {
    to_js_inner(ctx, value, 0, &mut 0)
}

fn to_js_inner<'js>(
    ctx: &Ctx<'js>,
    value: &JsonValue,
    depth: usize,
    nodes: &mut usize,
) -> rquickjs::Result<Value<'js>> {
    *nodes += 1;
    if depth > MAX_DEPTH || *nodes > MAX_NODES {
        return Err(Exception::throw_range(
            ctx,
            "Tool result exceeds structured bridge depth/node limit",
        ));
    }
    match value {
        JsonValue::Null => Ok(Value::new_null(ctx.clone())),
        JsonValue::Bool(value) => value.into_js(ctx),
        JsonValue::Number(value) => value.as_f64().unwrap_or(f64::NAN).into_js(ctx),
        JsonValue::String(value) => value.into_js(ctx),
        JsonValue::Array(values) => {
            let array = Array::new(ctx.clone())?;
            for (index, value) in values.iter().enumerate() {
                array.prop(
                    index as u32,
                    Property::from(to_js_inner(ctx, value, depth + 1, nodes)?)
                        .writable()
                        .enumerable()
                        .configurable(),
                )?;
            }
            Ok(array.into_value())
        }
        JsonValue::Object(values) => {
            let object = Object::new(ctx.clone())?;
            for (key, value) in values {
                object.prop(
                    key.as_str(),
                    Property::from(to_js_inner(ctx, value, depth + 1, nodes)?)
                        .writable()
                        .enumerable()
                        .configurable(),
                )?;
            }
            Ok(object.into_value())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rquickjs::{Context, Runtime};

    #[test]
    fn structured_parameters_match_json_stringify_semantics() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        context.with(|ctx| {
            let intrinsics = JsonIntrinsics::new(&ctx).unwrap();
            for expression in [
                "({text:'引号\\\"\\n\\u2028', flag:true, nil:null, integer:42, large:1e16})",
                "({rounded:18446744073709550000, negative:-9223372036854775808, exp:1e21, huge:1.79e308})",
                "({drop:undefined, fn:function(){}, symbol:Symbol('x'), nan:NaN, inf:Infinity})",
                "({array:[undefined, function(){}, Symbol('x'), NaN, , 5], nested:{a:1}})",
                "({date:new Date('2020-01-01T00:00:00Z'), n:new Number(3), s:new String('x'), b:new Boolean(false)})",
                "({value:{toJSON(key){return {key:key, text:'done'};}}})",
                "({value: (()=>{ var f=function(k){return k;}; f.call=null; return {toJSON:f}; })()})",
                "({a:1, get b(){delete this.a; this.c=3; return 2;}})",
                "({shared:(()=>{var a={v:2};return [a,a];})()})",
                "({__proto__:null, ['__proto__']:{polluted:true}, ['constructor']:'plain'})",
                "({boxed:(()=>{var n=new Number(3);n.valueOf=()=>5;return n;})()})",
                "({array:new Proxy([1,,3],{})})",
                "({array:new Proxy([1,2,3],{get(t,k){return k==='length'?2.9:Reflect.get(t,k);}})})",
                "({typed:new Uint8Array([1,2,3]), map:new Map([['x',1]])})",
                "({get a(){return undefined;}, get b(){return null;}})",
                "({value:{toJSON(){return function(){};}}})",
            ] {
                // Evaluate separately because getters can mutate the object.
                let expected: String = ctx.eval(format!("JSON.stringify({expression})")).unwrap();
                let value: Value = ctx.eval(expression).unwrap();
                let actual = intrinsics.from_js(&ctx, value).unwrap().unwrap();
                let expected: JsonValue = serde_json::from_str(&expected).unwrap();
                assert_eq!(actual, expected, "{expression}");
            }
        });
    }

    #[test]
    fn cycles_bigints_getters_and_depth_are_rejected_without_poisoning_context() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        context.with(|ctx| {
            let intrinsics = JsonIntrinsics::new(&ctx).unwrap();
            for source in [
                "(()=>{let x={};x.self=x;return x;})()",
                "({value:1n})",
                "({value:Object(1n)})",
                "({get value(){throw new Error('getter failure');}})",
                "(()=>{let x={};for(let i=0;i<140;i++)x={x};return x;})()",
                "({value:new Array(1000001)})",
            ] {
                let value: Value = ctx.eval(source).unwrap();
                assert!(intrinsics.from_js(&ctx, value).is_err(), "{source}");
                ctx.catch();
                assert_eq!(ctx.eval::<i32, _>("1+1").unwrap(), 2);
            }
            let value: Value = ctx.eval("({value:{toJSON(){return 4;}}})").unwrap();
            assert_eq!(
                intrinsics.from_js(&ctx, value).unwrap(),
                Some(serde_json::json!({"value":4}))
            );
        });
    }

    #[test]
    fn result_arrays_do_not_invoke_inherited_index_setters() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        context.with(|ctx| {
            ctx.eval::<(), _>("globalThis.setterCalls=0; Object.defineProperty(Array.prototype,'0',{set(){setterCalls++;},configurable:true});").unwrap();
            let result = to_js(&ctx, &serde_json::json!([7, {"nested":[8]}])).unwrap();
            ctx.globals().set("result", result).unwrap();
            assert_eq!(ctx.eval::<i32, _>("setterCalls").unwrap(), 0);
            assert!(ctx.eval::<bool, _>("Object.hasOwn(result,'0') && Object.hasOwn(result[1].nested,'0')").unwrap());
            assert_eq!(ctx.eval::<String, _>("JSON.stringify(result)").unwrap(), "[7,{\"nested\":[8]}]");
        });
    }

    #[test]
    fn results_preserve_proto_keys_as_own_data_properties() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        context.with(|ctx| {
            let value = serde_json::json!({"__proto__":{"polluted":true}, "text":"\"\n\u{2028}中"});
            ctx.globals().set("result", to_js(&ctx, &value).unwrap()).unwrap();
            assert!(ctx.eval::<bool,_>("Object.getPrototypeOf(result)===Object.prototype && Object.hasOwn(result,'__proto__') && result.polluted===undefined").unwrap());
            let serialized: String = ctx.eval("JSON.stringify(result)").unwrap();
            assert_eq!(serde_json::from_str::<JsonValue>(&serialized).unwrap(), value);
        });
    }
}
