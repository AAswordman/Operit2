pub struct JsTimeoutConfig;

impl JsTimeoutConfig {
    pub const MAIN_TIMEOUT_SECONDS: u64 = 1800;
    pub const PRE_TIMEOUT_SECONDS: u64 = Self::MAIN_TIMEOUT_SECONDS - 5;
    pub const SCRIPT_TIMEOUT_MS: u64 = Self::MAIN_TIMEOUT_SECONDS * 1000;
    pub const TOOL_CALL_TIMEOUT_MS: u64 = Self::SCRIPT_TIMEOUT_MS;
}
