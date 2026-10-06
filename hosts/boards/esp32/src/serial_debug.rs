//! Bounded local USB diagnostics sideband. This knows UART frames, not UI or pairing.
//! The Link reader remains the ONLY UART reader; OPD1 is observed between OPS1
//! frames. Bytes inside Link payloads are never interpreted as debug commands.
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const MAX_REQUEST: usize = 128;
pub const MAX_RESPONSE: usize = 16 * 1024;

#[derive(Clone, Copy)]
pub struct DebugRequest {
    bytes: [u8; MAX_REQUEST],
    length: usize,
}
impl DebugRequest {
    pub fn id(&self) -> u32 {
        u32::from_be_bytes(self.bytes[..4].try_into().unwrap())
    }
    pub fn operation(&self) -> u8 {
        self.bytes[4]
    }
    pub fn argument(&self) -> &[u8] {
        &self.bytes[5..self.length]
    }
}

#[derive(Default)]
pub struct SerialDebugIo {
    state: Mutex<State>,
}
#[derive(Default)]
struct State {
    parser: Parser,
    request: Option<DebugRequest>,
    response: Option<Vec<u8>>,
    busy: bool,
}
impl SerialDebugIo {
    pub(super) fn observe(&self, bytes: &[u8]) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        for byte in bytes {
            if let Some(request) = state.parser.feed(*byte) {
                // Single-flight mailbox: never grow a queue under a noisy client.
                if !state.busy {
                    state.busy = true;
                    state.request = Some(request);
                }
            }
        }
    }
    /// Only the firmware main/UI thread handles these requests.
    pub fn take_request(&self) -> Option<DebugRequest> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .request
            .take()
    }
    pub fn respond(&self, id: u32, json: &[u8]) {
        let json = if json.len() <= MAX_RESPONSE - 4 {
            json
        } else {
            b"{\"error\":\"debug response exceeds capacity\"}"
        };
        let mut frame = Vec::with_capacity(json.len() + 16);
        frame.extend_from_slice(b"OPD2");
        frame.extend_from_slice(&((json.len() + 4) as u32).to_be_bytes());
        frame.extend_from_slice(&id.to_be_bytes());
        frame.extend_from_slice(json);
        frame.extend_from_slice(&crc32(&frame[8..]).to_be_bytes());
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .response = Some(frame);
    }
    pub(super) fn take_response(&self) -> Option<Vec<u8>> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let response = state.response.take();
        if response.is_some() {
            state.busy = false;
        }
        response
    }
}

struct Parser {
    header: [u8; 8],
    header_at: usize,
    body: [u8; MAX_REQUEST + 4],
    body_at: usize,
    length: usize,
    skip: usize,
    last_byte: Instant,
}
impl Default for Parser {
    fn default() -> Self {
        Self {
            header: [0; 8],
            header_at: 0,
            body: [0; MAX_REQUEST + 4],
            body_at: 0,
            length: 0,
            skip: 0,
            last_byte: Instant::now(),
        }
    }
}
impl Parser {
    fn feed(&mut self, byte: u8) -> Option<DebugRequest> {
        // A disconnected client must not leave a half-frame blocking debugging.
        if self.last_byte.elapsed() > Duration::from_secs(2) {
            self.header_at = 0;
            self.length = 0;
            self.skip = 0;
        }
        self.last_byte = Instant::now();
        if self.skip > 0 {
            self.skip -= 1;
            return None;
        }
        if self.length > 0 {
            self.body[self.body_at] = byte;
            self.body_at += 1;
            if self.body_at < self.length + 4 {
                return None;
            }
            let length = self.length;
            self.length = 0;
            if crc32(&self.body[..length])
                != u32::from_be_bytes(self.body[length..length + 4].try_into().unwrap())
            {
                return None;
            }
            let mut bytes = [0; MAX_REQUEST];
            bytes[..length].copy_from_slice(&self.body[..length]);
            return Some(DebugRequest { bytes, length });
        }
        self.header[self.header_at] = byte;
        self.header_at += 1;
        if self.header_at <= 4 {
            while self.header_at > 0
                && !b"OPD1".starts_with(&self.header[..self.header_at])
                && !b"OPS1".starts_with(&self.header[..self.header_at])
            {
                self.header.copy_within(1..self.header_at, 0);
                self.header_at -= 1;
            }
            return None;
        }
        if self.header_at < 8 {
            return None;
        }
        self.header_at = 0;
        let length = u32::from_be_bytes(self.header[4..8].try_into().unwrap()) as usize;
        if &self.header[..4] == b"OPS1" {
            if (1..=8192).contains(&length) {
                self.skip = length + 4;
            }
        } else if (5..=MAX_REQUEST).contains(&length) {
            self.length = length;
            self.body_at = 0;
        }
        None
    }
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(magic: &[u8], payload: &[u8]) -> Vec<u8> {
        [
            magic,
            &(payload.len() as u32).to_be_bytes(),
            payload,
            &crc32(payload).to_be_bytes(),
        ]
        .concat()
    }
    fn request(id: u32, op: u8, arg: &[u8]) -> Vec<u8> {
        frame(b"OPD1", &[id.to_be_bytes().as_slice(), &[op], arg].concat())
    }
    #[test]
    fn split_noise_crc_and_single_flight() {
        let io = SerialDebugIo::default();
        io.observe(b"console OP noise\r\n");
        let mut bad = request(1, 1, b"");
        bad[8] ^= 1;
        io.observe(&bad);
        assert!(io.take_request().is_none());
        for byte in request(7, 3, b"action:edge_pair") {
            io.observe(&[byte]);
        }
        io.observe(&request(8, 1, b""));
        let req = io.take_request().unwrap();
        assert_eq!(req.id(), 7);
        assert_eq!(req.operation(), 3);
        assert_eq!(req.argument(), b"action:edge_pair");
        assert!(io.take_request().is_none());
        io.respond(req.id(), b"{\"ok\":true}");
        let result = io.take_response().unwrap();
        assert_eq!(&result[..4], b"OPD2");
        assert_eq!(&result[8..12], &7u32.to_be_bytes());
        assert_eq!(
            crc32(&result[8..result.len() - 4]),
            u32::from_be_bytes(result[result.len() - 4..].try_into().unwrap())
        );
        io.observe(&request(8, 1, b""));
        assert_eq!(io.take_request().unwrap().id(), 8);
    }
    #[test]
    fn never_executes_commands_embedded_in_link_payload() {
        let io = SerialDebugIo::default();
        let link = frame(b"OPS1", &request(123, 3, b"dangerous"));
        for part in link.chunks(3) {
            io.observe(part);
        }
        assert!(io.take_request().is_none());
        io.observe(&request(456, 1, b""));
        assert_eq!(io.take_request().unwrap().id(), 456);
    }
    #[test]
    fn rejects_oversized_requests_and_bounds_response() {
        let io = SerialDebugIo::default();
        io.observe(&frame(b"OPD1", &[0; MAX_REQUEST + 1]));
        assert!(io.take_request().is_none());
        io.observe(&request(1, 1, b""));
        assert!(io.take_request().is_some());
        io.respond(1, &vec![0; MAX_RESPONSE]);
        assert!(io.take_response().unwrap().len() < 100);
    }
    #[test]
    fn abandoned_partial_frame_expires() {
        let mut parser = Parser::default();
        for byte in &request(1, 1, b"")[..10] {
            parser.feed(*byte);
        }
        parser.last_byte = Instant::now() - Duration::from_secs(3);
        let mut result = None;
        for byte in request(2, 1, b"") {
            result = parser.feed(byte).or(result);
        }
        assert_eq!(result.unwrap().id(), 2);
    }
}
