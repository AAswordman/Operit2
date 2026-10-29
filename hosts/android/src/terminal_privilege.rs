use operit_host_api::{HostError, HostResult};
use serde_json::{json, Value};

/// Describes approved privileges without initiating an authorization request.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShellPrivileges {
    pub root_su_arguments: Option<Vec<String>>,
    pub shizuku_authorized: bool,
}

/// Selects one transport for the lifetime of a system-shell session.
#[derive(Debug, PartialEq)]
pub(crate) enum ShellTransport {
    Root(Vec<String>),
    Shizuku,
    App,
}

impl ShellPrivileges {
    /// Applies the explicitly requested Root, Shizuku, app-shell priority.
    pub fn select(self) -> HostResult<ShellTransport> {
        match self.root_su_arguments {
            Some(arguments) if arguments.is_empty() || arguments[0].is_empty() => {
                Err(HostError::new("Android Root terminal has no su executable"))
            }
            Some(arguments) => Ok(ShellTransport::Root(arguments)),
            None if self.shizuku_authorized => Ok(ShellTransport::Shizuku),
            None => Ok(ShellTransport::App),
        }
    }
}

/// Reads approved Android privileges through the runtime-owned JNI bridge.
pub(crate) fn privileges() -> HostResult<ShellPrivileges> {
    serde_json::from_value(call(json!({"operation": "privileges"}))?)
        .map_err(|error| HostError::new(error.to_string()))
}

/// Executes Binder-only terminal operations without moving shell policy out of Rust.
#[cfg(target_os = "android")]
pub(crate) fn call(request: Value) -> HostResult<Value> {
    use jni::objects::{JObject, JString, JValue};
    let bridge = crate::secret_store::androidHostSecretStoreBridge()?;
    let mut env = crate::attachCurrentThread(&bridge.vm)
        .map_err(|error| HostError::new(error.to_string()))?;
    let text = env.with_local_frame(8, |env| -> jni::errors::Result<String> {
        let request = JObject::from(env.new_string(request.to_string())?);
        let object = env
            .call_method(
                bridge.host.as_obj(),
                "terminalPrivilegeOperation",
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[JValue::Object(&request)],
            )?
            .l()?;
        let value = JString::from(object);
        let text = env.get_string(&value)?;
        Ok(String::from(text))
    });
    let text = match text {
        Ok(text) => text,
        Err(error) => {
            let _ = env.exception_clear();
            return Err(HostError::new(format!(
                "Android terminal privilege bridge failed: {error}"
            )));
        }
    };
    let response: Value =
        serde_json::from_str(&text).map_err(|error| HostError::new(error.to_string()))?;
    if let Some(error) = response.get("error").and_then(Value::as_str) {
        return Err(HostError::new(error));
    }
    Ok(response)
}

/// Rejects Android privilege operations on non-Android hosts.
#[cfg(not(target_os = "android"))]
pub(crate) fn call(_request: Value) -> HostResult<Value> {
    Err(HostError::new(
        "Android terminal privileges require Android",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Root wins even when Shizuku is authorized.
    #[test]
    fn root_precedes_shizuku() {
        let privileges = ShellPrivileges {
            root_su_arguments: Some(vec!["/custom/su".into(), "--flag".into()]),
            shizuku_authorized: true,
        };
        assert_eq!(
            privileges.select().unwrap(),
            ShellTransport::Root(vec!["/custom/su".into(), "--flag".into()])
        );
    }

    /// Shizuku wins only when Root is not granted.
    #[test]
    fn shizuku_precedes_app_shell() {
        assert_eq!(
            ShellPrivileges {
                root_su_arguments: None,
                shizuku_authorized: true
            }
            .select()
            .unwrap(),
            ShellTransport::Shizuku
        );
        assert_eq!(
            ShellPrivileges {
                root_su_arguments: None,
                shizuku_authorized: false
            }
            .select()
            .unwrap(),
            ShellTransport::App
        );
    }

    /// Broken privileged configuration must not silently execute as the app.
    #[test]
    fn invalid_root_configuration_is_an_error() {
        assert!(ShellPrivileges {
            root_su_arguments: Some(vec![]),
            shizuku_authorized: true
        }
        .select()
        .is_err());
    }
}
