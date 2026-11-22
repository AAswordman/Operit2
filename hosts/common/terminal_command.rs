//! Encodes POSIX scripts as one interactive shell submission.

use std::fmt::Write;

/// Keeps script control characters and history-expansion characters out of interactive input
/// while preserving shell state.
pub(crate) fn posixCommandLine(command: &str) -> String {
    if !command
        .chars()
        .any(|character| character.is_control() || character == '!')
    {
        return command.to_string();
    }
    let mut encoded = String::with_capacity(command.len());
    for ch in command.chars() {
        match ch {
            '\\' => encoded.push_str("\\\\"),
            '\'' => encoded.push_str("\\0047"),
            '!' => encoded.push_str("\\0041"),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors `printf '%b'`: after `\0` it greedily consumes up to three octal digits.
    fn decode_printf_b(encoded: &str) -> String {
        let bytes = encoded.as_bytes();
        let mut decoded = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'\\' && index + 1 < bytes.len() && bytes[index + 1] == b'0' {
                let mut value = 0u32;
                let mut cursor = index + 2;
                while cursor < bytes.len()
                    && cursor < index + 5
                    && (b'0'..=b'7').contains(&bytes[cursor])
                {
                    value = value * 8 + u32::from(bytes[cursor] - b'0');
                    cursor += 1;
                }
                decoded.push(value as u8);
                index = cursor;
            } else if bytes[index] == b'\\' && index + 1 < bytes.len() && bytes[index + 1] == b'\\'
            {
                decoded.push(b'\\');
                index += 2;
            } else {
                decoded.push(bytes[index]);
                index += 1;
            }
        }
        String::from_utf8(decoded).expect("decoded command must stay valid UTF-8")
    }

    fn round_trip(command: &str) -> String {
        let line = posixCommandLine(command);
        let encoded = line
            .strip_prefix("eval \"$(printf '%b' '")
            .and_then(|rest| rest.strip_suffix("')\""))
            .unwrap_or(&line);
        decode_printf_b(encoded)
    }

    #[test]
    fn plain_command_passes_through_unchanged() {
        assert_eq!(posixCommandLine("echo '3abc'"), "echo '3abc'");
    }

    #[test]
    fn quote_followed_by_any_hex_digit_round_trips() {
        for digit in "0123456789abcdef".chars() {
            let command = format!("echo '{digit}abc'\ntrue");
            assert_eq!(round_trip(&command), command, "digit {digit}");
        }
    }

    #[test]
    fn multiline_command_with_quotes_bang_and_backslash_round_trips() {
        let command = "cat <<'EOF'\nhash '327429f1' ! !! \\012 中文\nEOF";
        assert_eq!(round_trip(command), command);
    }
}
