//! Encodes POSIX scripts as one interactive shell submission.

use std::fmt::Write;

/// Keeps script control characters out of interactive input while preserving shell state.
pub(crate) fn posixCommandLine(command: &str) -> String {
    if !command.chars().any(char::is_control) {
        return command.to_string();
    }
    let mut encoded = String::with_capacity(command.len());
    for ch in command.chars() {
        match ch {
            '\\' => encoded.push_str("\\\\"),
            '\'' => encoded.push_str("\\047"),
            _ if ch.is_control() => {
                let mut utf8 = [0u8; 4];
                for byte in ch.encode_utf8(&mut utf8).bytes() {
                    write!(encoded, "\\0{byte:03o}").expect("writing to a String cannot fail");
                }
            }
            _ => encoded.push(ch),
        }
    }
    format!("eval \"$(printf '%b' '{encoded}')\"")
}
