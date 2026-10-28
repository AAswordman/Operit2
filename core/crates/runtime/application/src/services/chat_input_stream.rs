//! Framing only. Chat hooks, persistence and generation remain in sendUserMessage.
use operit_util::stream::ReverseStream::ReverseStream;
const MAX_TEXT: usize = 16384;
const CHUNK: usize = 2048;

pub(super) async fn receive(mut input: ReverseStream<String>, expected: i64) -> Result<String, String> {
    if expected <= 0 || expected > MAX_TEXT as i64 { return Err("Text length must be 1..16384 UTF-8 bytes".into()); }
    let mut result = String::with_capacity(expected as usize);
    let mut committed = false;
    loop {
        let chunk = tokio::time::timeout(std::time::Duration::from_secs(30), input.recv())
            .await.map_err(|_| "Text upload timed out")?;
        let Some(chunk) = chunk else { break; };
        if committed { return Err("Unexpected data after text commit".into()); }
        if chunk.is_empty() { committed = true; continue; }
        if chunk.len() > CHUNK || result.len() + chunk.len() > expected as usize {
            return Err("Text upload exceeds declared length or chunk limit".into());
        }
        result.push_str(&chunk);
    }
    if !committed || result.len() != expected as usize { return Err("Text upload incomplete".into()); }
    if result.trim().is_empty() { return Err("Message must not be empty".into()); }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn upload(chunks: Vec<String>, expected: i64) -> Result<String, String> {
        let (mut tx, rx) = ReverseStream::channel();
        let producer = async { for chunk in chunks { if tx.send(chunk).await.is_err() { break; } } tx.close(); };
        tokio::join!(producer, receive(rx, expected)).1
    }
    #[tokio::test]
    async fn utf8_message_spans_frames_without_losing_its_tail() {
        let chunks = vec!["长文本🙂".repeat(120); 10];
        let complete = chunks.concat();
        assert!(complete.len() > 8192);
        let mut packets = chunks; packets.push(String::new());
        assert_eq!(upload(packets, complete.len() as i64).await.unwrap(), complete);
    }
    #[tokio::test]
    async fn interruption_and_invalid_framing_cannot_submit_partial_messages() {
        for (chunks, length) in [
            (vec!["prefix".into()], 6),
            (vec!["prefix".into(), "".into()], 20),
            (vec!["prefix".into(), "".into(), "extra".into()], 11),
            (vec!["x".repeat(2049), "".into()], 2049),
            (vec!["x".into(), "".into()], 0),
            (vec!["x".into(), "".into()], 16385),
        ] { assert!(upload(chunks, length).await.is_err()); }
    }
}
