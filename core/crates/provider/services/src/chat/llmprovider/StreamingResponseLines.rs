use super::AIService::AiServiceError;

/// Frames complete SSE events before a provider interprets their JSON payload.
#[derive(Default)]
pub(super) struct SseDecoder {
    pending: Vec<u8>,
    data: Vec<String>,
    skip_lf: bool,
    started: bool,
}

impl SseDecoder {
    /// Handles UTF-8, CR/LF/CRLF, comments and multiline data across arbitrary chunks.
    pub(super) fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, AiServiceError> {
        let bytes = if self.skip_lf && !bytes.is_empty() {
            self.skip_lf = false;
            bytes.strip_prefix(b"\n").unwrap_or(bytes)
        } else {
            bytes
        };
        self.pending.extend_from_slice(bytes);
        let mut consumed = 0;
        let mut events = Vec::new();
        while let Some(offset) = self.pending[consumed..]
            .iter()
            .position(|b| matches!(b, b'\r' | b'\n'))
        {
            let end = consumed + offset;
            let mut line = std::str::from_utf8(&self.pending[consumed..end])
                .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
            if !self.started {
                self.started = true;
                line = line.strip_prefix('\u{feff}').unwrap_or(line);
            }
            consumed = end + 1;
            if self.pending[end] == b'\r' {
                if self.pending.get(consumed) == Some(&b'\n') {
                    consumed += 1;
                } else if consumed == self.pending.len() {
                    self.skip_lf = true;
                }
            }
            if line.is_empty() {
                let data = std::mem::take(&mut self.data).join("\n");
                if !data.trim().is_empty() {
                    events.push(data);
                }
            } else {
                let (field, value) = line.split_once(':').unwrap_or((line, ""));
                if field == "data" {
                    self.data
                        .push(value.strip_prefix(' ').unwrap_or(value).to_string());
                }
            }
        }
        self.pending.drain(..consumed);
        Ok(events)
    }

    /// Rejects an unfinished event rather than parsing a truncated transport tail.
    pub(super) fn finish(&self) -> Result<(), AiServiceError> {
        std::str::from_utf8(&self.pending)
            .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
        if !self.pending.is_empty() || !self.data.is_empty() {
            return Err(AiServiceError::ConnectionFailed(
                "response ended during an SSE event".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SseDecoder;

    /// Preserves multiline JSON and multilingual text at every transport split and line ending.
    #[test]
    fn every_split_preserves_complete_events() {
        for newline in ["\n", "\r\n", "\r"] {
            let wire = format!("\u{feff}: heartbeat{newline}{newline}event: message{newline}data: {{\"text\":{newline}data: \"你好😀\"}}{newline}{newline}");
            for split in 0..=wire.len() {
                let mut decoder = SseDecoder::default();
                let mut events = decoder.push(&wire.as_bytes()[..split]).unwrap();
                events.extend(decoder.push(&wire.as_bytes()[split..]).unwrap());
                assert_eq!(events, ["{\"text\":\n\"你好😀\"}"]);
                decoder.finish().unwrap();
            }
        }
    }

    /// Empty data, comments and metadata never become JSON parse failures.
    #[test]
    fn ignores_empty_events_and_comments() {
        let mut decoder = SseDecoder::default();
        assert!(decoder
            .push(b": ping\n\ndata:\n\nevent: ping\nid: 1\n\n")
            .unwrap()
            .is_empty());
        assert_eq!(decoder.push(b"data: [DONE]\n\n").unwrap(), ["[DONE]"]);
    }

    /// A partial event at EOF is never passed to a provider as complete JSON.
    #[test]
    fn rejects_unfinished_event_at_eof() {
        for wire in ["data: {}", "data: {}\n", "data: {\n"] {
            let mut decoder = SseDecoder::default();
            assert!(decoder.push(wire.as_bytes()).unwrap().is_empty());
            assert!(decoder.finish().is_err());
        }
    }

    /// Rejects invalid UTF-8 without inserting replacement characters into tool parameters.
    #[test]
    fn rejects_invalid_and_truncated_utf8() {
        let mut decoder = SseDecoder::default();
        assert!(decoder.push(&[b'd', 0xff, b'\n']).is_err());
        let mut decoder = SseDecoder::default();
        assert!(decoder.push(&[b'd', 0xe4, 0xb8]).unwrap().is_empty());
        assert!(decoder.finish().is_err());
    }
}
