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

/// Returns the host's current local date and time with its explicit UTC offset.
pub fn currentLocalDateTime() -> String {
    let timestampMillis = currentTimeMillis();
    let utc = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(timestampMillis)
        .expect("host time must be a valid chrono timestamp");
    #[cfg(not(target_arch = "wasm32"))]
    let local = utc.with_timezone(&chrono::Local).fixed_offset();
    #[cfg(target_arch = "wasm32")]
    let local = {
        let date = js_sys::Date::new(&js_sys::wasm_bindgen::JsValue::from_f64(timestampMillis as f64));
        let offset = chrono::FixedOffset::east_opt((-date.get_timezone_offset() * 60.0) as i32)
            .expect("host timezone offset must be valid");
        utc.with_timezone(&offset)
    };
    local.to_rfc3339_opts(chrono::SecondsFormat::Millis, false)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// Keeps the instant and the host's timezone in the same unambiguous timestamp.
    #[test]
    fn local_datetime_preserves_clock_and_host_timezone() {
        let before = currentTimeMillis();
        let formatted = currentLocalDateTime();
        let after = currentTimeMillis();
        let parsed = chrono::DateTime::parse_from_rfc3339(&formatted).unwrap();
        assert!((before..=after).contains(&parsed.timestamp_millis()));
        let expected = parsed.with_timezone(&chrono::Local);
        assert_eq!(parsed.offset().local_minus_utc(), expected.offset().local_minus_utc());
        assert_eq!(parsed.naive_local(), expected.naive_local());
        assert_eq!(formatted, expected.to_rfc3339_opts(chrono::SecondsFormat::Millis, false));
    }
}
