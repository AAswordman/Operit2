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

#[allow(non_snake_case)]
pub fn nativeJavaClassExistsString(className: String) -> String {
    matches!(
        className.trim(),
        "android.content.Context" | "android.app.Application"
    )
    .to_string()
}

#[allow(non_snake_case)]
pub fn nativeJavaGetApplicationContextString() -> String {
    exposeJavaBridgeObject(JavaBridgeObject::ApplicationContext)
}

#[allow(non_snake_case)]
pub fn nativeJavaNewInstanceString(className: String) -> String {
    javaBridgeFailure(&format!(
        "class cannot be constructed: {}",
        className.trim()
    ))
}

#[allow(non_snake_case)]
pub fn nativeJavaCallStaticString(className: String, methodName: String) -> String {
    javaBridgeFailure(&format!(
        "static method '{}' not found on {}",
        methodName.trim(),
        className.trim()
    ))
}

#[allow(non_snake_case)]
pub fn nativeJavaCallInstanceStrings(
    instanceHandle: String,
    methodName: String,
    argsJson: String,
) -> String {
    let handle = instanceHandle.trim().to_string();
    let methodName = methodName.trim().to_string();
    if let Err(error) = parseJavaBridgeArgs(&argsJson) {
        return javaBridgeFailure(&error);
    }
    let object = JAVA_BRIDGE_OBJECTS.with(|objects| objects.borrow().get(&handle).cloned());
    let Some(object) = object else {
        return javaBridgeFailure(&format!("java instance handle not found: {}", handle));
    };
    match object {
        JavaBridgeObject::ApplicationContext => nativeJavaCallApplicationContext(&methodName),
    }
}

#[allow(non_snake_case)]
fn nativeJavaCallApplicationContext(methodName: &str) -> String {
    match methodName {
        "getApplicationContext" => exposeJavaBridgeObject(JavaBridgeObject::ApplicationContext),
        "toString" => javaBridgeSuccess(Value::String("[ApplicationContext]".to_string())),
        _ => javaBridgeFailure(&format!(
            "method '{}' not found on ApplicationContext",
            methodName
        )),
    }
}

#[allow(non_snake_case)]
fn parseJavaBridgeArgs(argsJson: &str) -> Result<Vec<Value>, String> {
    serde_json::from_str::<Vec<Value>>(argsJson).map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn exposeJavaBridgeObject(object: JavaBridgeObject) -> String {
    let className = javaBridgeObjectClassName(&object).to_string();
    let handle = Uuid::new_v4().to_string();
    JAVA_BRIDGE_OBJECTS.with(|objects| {
        objects.borrow_mut().insert(handle.clone(), object);
    });
    javaBridgeExistingObject(&handle, &className)
}

#[allow(non_snake_case)]
fn javaBridgeExistingObject(handle: &str, className: &str) -> String {
    javaBridgeSuccess(serde_json::json!({
        "__javaHandle": handle,
        "__javaClass": className
    }))
}

#[allow(non_snake_case)]
fn javaBridgeObjectClassName(object: &JavaBridgeObject) -> &'static str {
    match object {
        JavaBridgeObject::ApplicationContext => "android.app.Application",
    }
}

#[allow(non_snake_case)]
fn javaBridgeSuccess(data: Value) -> String {
    serde_json::json!({
        "success": true,
        "data": data
    })
    .to_string()
}

#[allow(non_snake_case)]
fn javaBridgeFailure(message: &str) -> String {
    serde_json::json!({
        "success": false,
        "message": message
    })
    .to_string()
}
