use std::cell::RefCell;
use std::collections::BTreeMap;

use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Debug)]
enum JavaBridgeObject {
    ApplicationContext,
}

thread_local! {
    static JAVA_BRIDGE_OBJECTS: RefCell<BTreeMap<String, JavaBridgeObject>> = RefCell::new(BTreeMap::new());
}

/// Reports the Java compatibility classes implemented by the runtime host contract.
#[allow(non_snake_case)]
pub fn javaClassExists(className: &str) -> bool {
    matches!(
        className.trim(),
        "android.content.Context" | "android.app.Application"
    )
}

/// Exposes the runtime-owned application-context handle as structured data.
#[allow(non_snake_case)]
pub fn javaGetApplicationContext() -> Value {
    exposeJavaBridgeObject(JavaBridgeObject::ApplicationContext)
}

/// Rejects unsupported construction without fabricating a Java instance.
#[allow(non_snake_case)]
pub fn javaNewInstance(className: &str, _args: &[Value]) -> Result<Value, String> {
    Err(format!("class cannot be constructed: {}", className.trim()))
}

/// Rejects unsupported static invocations through the structured error contract.
#[allow(non_snake_case)]
pub fn javaCallStatic(className: &str, methodName: &str, _args: &[Value]) -> Result<Value, String> {
    Err(format!(
        "static method '{}' not found on {}",
        methodName.trim(),
        className.trim()
    ))
}

/// Calls an implemented Java compatibility method using structured arguments.
#[allow(non_snake_case)]
pub fn javaCallInstance(
    instanceHandle: &str,
    methodName: &str,
    args: &[Value],
) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("ApplicationContext methods require zero arguments".to_string());
    }
    let object = JAVA_BRIDGE_OBJECTS
        .with(|objects| objects.borrow().get(instanceHandle.trim()).cloned())
        .ok_or_else(|| format!("java instance handle not found: {}", instanceHandle.trim()))?;
    match object {
        JavaBridgeObject::ApplicationContext => match methodName.trim() {
            "getApplicationContext" => {
                Ok(exposeJavaBridgeObject(JavaBridgeObject::ApplicationContext))
            }
            "toString" => Ok(Value::String("[ApplicationContext]".to_string())),
            method => Err(format!(
                "method '{}' not found on ApplicationContext",
                method
            )),
        },
    }
}

/// Stores an owned compatibility object and returns its exact handle descriptor.
#[allow(non_snake_case)]
fn exposeJavaBridgeObject(object: JavaBridgeObject) -> Value {
    let className = javaBridgeObjectClassName(&object).to_string();
    let handle = Uuid::new_v4().to_string();
    JAVA_BRIDGE_OBJECTS.with(|objects| {
        objects.borrow_mut().insert(handle.clone(), object);
    });
    serde_json::json!({"__javaHandle": handle, "__javaClass": className})
}

/// Identifies the public compatibility class of an owned Java object.
#[allow(non_snake_case)]
fn javaBridgeObjectClassName(object: &JavaBridgeObject) -> &'static str {
    match object {
        JavaBridgeObject::ApplicationContext => "android.app.Application",
    }
}
