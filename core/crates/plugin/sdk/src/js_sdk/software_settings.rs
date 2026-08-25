//! Environment-variable access and core command execution exposed to plugins.
use super::results::*;
use super::{JsDate, JsFuture};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
/// Reads and updates environment variables and executes core commands.
pub trait SoftwareSettingsHost: Send + Sync {
    ///
    ///Read current value of an environment variable.
    ///@param key - Environment variable key
    ///
    fn readEnvironmentVariable(&self, key: String) -> JsFuture<EnvironmentVariableReadResultData>;
    ///
    ///Write an environment variable; empty value clears the variable.
    ///@param key - Environment variable key
    ///@param value - Variable value (empty string clears)
    ///
    fn writeEnvironmentVariable(
        &self,
        key: String,
        value: Option<String>,
    ) -> JsFuture<EnvironmentVariableWriteResultData>;
    ///
    ///Execute a core command with CLI-style arguments.
    ///@param args - Command arguments, for example ['plugin', 'list']
    ///
    fn exec(&self, args: Vec<String>) -> JsFuture<String>;
}
