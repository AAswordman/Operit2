#![allow(non_snake_case)]

#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current Unix time in milliseconds as an `i64`.
pub fn currentTimeMillis() -> i64 {
    tryCurrentTimeMillis().expect("system time must be after UNIX_EPOCH")
}

/// Returns the current Unix time in milliseconds as a `u128`.
pub fn currentTimeMillisU128() -> u128 {
    tryCurrentTimeMillisU128().expect("system time must be after UNIX_EPOCH")
}

/// Returns the current Unix time in milliseconds without panicking on clock errors.
pub fn tryCurrentTimeMillis() -> Result<i64, String> {
    tryCurrentTimeMillisU128().map(|value| value as i64)
}

/// Reads the native or JavaScript clock and converts it to Unix milliseconds.
pub fn tryCurrentTimeMillisU128() -> Result<u128, String> {
    #[cfg(target_arch = "wasm32")]
    {
        Ok(js_sys::Date::now() as u128)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .map_err(|error| error.to_string())
    }
}
