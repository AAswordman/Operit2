//! Carries editor commands over one asynchronous loopback connection.
use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::net::TcpStream;

/// Owns one receive buffer without standard-input buffering or blocking workers.
pub struct EditorIpc {
    input: Lines<BufReader<TcpStream>>,
}

/// Borrows a successful command result until its single wire serialization.
#[derive(Serialize)]
struct Response<'a, T: Serialize> {
    id: &'a Value,
    value: &'a T,
}

/// Borrows a failure without constructing an intermediate JSON value tree.
#[derive(Serialize)]
struct ErrorResponse<'a> {
    id: &'a Value,
    error: &'a str,
}

impl EditorIpc {
    /// Connects to the editor endpoint supplied when this simulator was launched.
    pub async fn connect() -> Result<Self, Box<dyn std::error::Error>> {
        let address: std::net::SocketAddr = std::env::var("OPERIT_SIM_IPC_ADDRESS")?.parse()?;
        if !address.ip().is_loopback() {
            return Err("Editor IPC endpoint must be a loopback address".into());
        }
        let token = std::env::var("OPERIT_SIM_IPC_TOKEN")?;
        let mut ipc = Self::fromStream(TcpStream::connect(address).await?)?;
        ipc.send(serde_json::json!({"ipcToken": token})).await?;
        Ok(ipc)
    }

    /// Uses one small receive buffer for the owned editor connection.
    fn fromStream(stream: TcpStream) -> std::io::Result<Self> {
        stream.set_nodelay(true)?;
        Ok(Self {
            input: BufReader::with_capacity(1024, stream).lines(),
        })
    }

    /// Receives one command while retaining partial lines across route polling.
    pub async fn receive(&mut self) -> std::io::Result<Option<String>> {
        self.input.next_line().await
    }

    /// Writes either command outcome with the editor's existing response schema.
    pub async fn reply<T: Serialize>(
        &mut self,
        id: &Value,
        result: Result<T, String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match result {
            Ok(value) => self.send(Response { id, value: &value }).await,
            Err(error) => self.send(ErrorResponse { id, error: &error }).await,
        }
    }

    /// Serializes a response once and releases its output bytes after transmission.
    pub(super) async fn send(
        &mut self,
        value: impl Serialize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut bytes = serde_json::to_vec(&value)?;
        bytes.push(b'\n');
        self.input.get_mut().get_mut().write_all(&bytes).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::net::TcpListener;

    /// Creates a real loopback connection without process-global environment changes.
    async fn connection() -> (EditorIpc, TcpStream) {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let (client, accepted) = tokio::join!(
            TcpStream::connect(listener.local_addr().unwrap()),
            listener.accept()
        );
        (
            EditorIpc::fromStream(client.unwrap()).unwrap(),
            accepted.unwrap().0,
        )
    }

    /// Route ticks can cancel a partial read without losing bytes or the following command.
    #[tokio::test]
    async fn interrupted_commands_preserve_framing_and_eof() {
        let (mut ipc, mut editor) = connection().await;
        editor.write_all(b"{\"id\":1,\"text\":\"").await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(10), ipc.receive())
                .await
                .is_err()
        );
        let payload = "x".repeat(4096);
        editor
            .write_all(format!("{payload}\"}}\n{{\"id\":2}}\n").as_bytes())
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&ipc.receive().await.unwrap().unwrap()).unwrap(),
            serde_json::json!({"id": 1, "text": payload})
        );
        assert_eq!(
            serde_json::from_str::<Value>(&ipc.receive().await.unwrap().unwrap()).unwrap(),
            serde_json::json!({"id": 2})
        );
        editor.shutdown().await.unwrap();
        assert!(ipc.receive().await.unwrap().is_none());
    }

    /// Successful and failed replies retain their JSON schema and order on the same socket.
    #[tokio::test]
    async fn responses_preserve_unicode_and_order() {
        let (mut ipc, editor) = connection().await;
        let replies = [
            serde_json::json!({"id": 1, "value": "聊天内容"}),
            serde_json::json!({"id": 2, "error": "closed"}),
        ];
        let mut lines = BufReader::new(editor).lines();
        for expected in replies {
            let id = &expected["id"];
            let result = match expected.get("error") {
                Some(error) => Err(error.as_str().unwrap().to_owned()),
                None => Ok(&expected["value"]),
            };
            ipc.reply(id, result).await.unwrap();
            let actual: Value =
                serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
            assert_eq!(actual, expected);
        }
    }
}

/// Long drafts arrive from the desktop editor. Decode their JSON string in the
/// owned IPC buffer instead of retaining a line, parser scratch, JSON Value and
/// a second draft at the same time on the fixed device heap.
pub enum Command {
    Send { id: Value, text: String, send_id: Option<String> },
    Other(Value),
}
pub fn decode_command(line: String) -> Result<Command, String> {
    #[derive(serde::Deserialize)]
    struct Envelope<'a> {
        id: Value,
        #[serde(borrow)]
        command: &'a str,
        #[serde(borrow)]
        text: Option<&'a serde_json::value::RawValue>,
        #[serde(rename = "sendId")]
        send_id: Option<String>,
    }
    let envelope: Envelope<'_> = serde_json::from_str(&line).map_err(|e| e.to_string())?;
    if envelope.command != "send" {
        return serde_json::from_str(&line).map(Command::Other).map_err(|e| e.to_string());
    }
    let raw = envelope.text.ok_or("Missing text")?.get();
    if !raw.starts_with('"') { return Err("Expected a text string".into()); }
    let start = raw.as_ptr() as usize - line.as_ptr() as usize + 1;
    let end = start + raw.len() - 2;
    let id = envelope.id;
    let send_id = envelope.send_id;
    let mut bytes = line.into_bytes();
    let mut read = start;
    let mut write = 0;
    while read < end {
        let byte = bytes[read]; read += 1;
        if byte != b'\\' { bytes[write] = byte; write += 1; continue; }
        let escaped = *bytes.get(read).ok_or("Truncated escape")?; read += 1;
        let decoded = match escaped {
            b'"' | b'\\' | b'/' => escaped,
            b'b' => 8, b'f' => 12, b'n' => b'\n', b'r' => b'\r', b't' => b'\t',
            b'u' => {
                fn hex(bytes: &[u8], at: usize) -> Result<u32, String> {
                    let text = std::str::from_utf8(bytes.get(at..at+4).ok_or("Truncated Unicode escape")?).map_err(|e| e.to_string())?;
                    u32::from_str_radix(text, 16).map_err(|e| e.to_string())
                }
                let mut code = hex(&bytes, read)?; read += 4;
                if (0xd800..=0xdbff).contains(&code) {
                    if bytes.get(read..read+2) != Some(b"\\u") { return Err("Missing low surrogate".into()); }
                    read += 2;
                    let low = hex(&bytes, read)?; read += 4;
                    if !(0xdc00..=0xdfff).contains(&low) { return Err("Invalid low surrogate".into()); }
                    code = 0x10000 + ((code - 0xd800) << 10) + low - 0xdc00;
                }
                let character = char::from_u32(code).ok_or("Invalid Unicode escape")?;
                let mut encoded = [0; 4];
                let encoded = character.encode_utf8(&mut encoded).as_bytes();
                bytes[write..write+encoded.len()].copy_from_slice(encoded); write += encoded.len();
                continue;
            }
            _ => return Err("Invalid JSON escape".into()),
        };
        bytes[write] = decoded; write += 1;
    }
    bytes.truncate(write);
    Ok(Command::Send { id, text: String::from_utf8(bytes).map_err(|e| e.to_string())?, send_id })
}

#[cfg(test)]
mod draft_tests {
    use super::*;
    #[test]
    fn draft_decode_preserves_all_utf8_and_json_escapes() {
        for text in ["引号\"与斜线\\\n\t🙂".into(), "长文本🙂".repeat(800)] {
            let line = serde_json::json!({"id":7,"command":"send","text":text,"sendId":"turn"}).to_string();
            let Command::Send { id, text: decoded, send_id } = decode_command(line).unwrap() else { panic!("send") };
            assert_eq!(id, 7); assert_eq!(decoded, text); assert_eq!(send_id.as_deref(), Some("turn"));
        }
        let Command::Send { text, .. } = decode_command(r#"{"id":1,"command":"send","text":"\ud83d\ude42\u4e2d"}"#.into()).unwrap() else { panic!("send") };
        assert_eq!(text, "🙂中");
    }
}
