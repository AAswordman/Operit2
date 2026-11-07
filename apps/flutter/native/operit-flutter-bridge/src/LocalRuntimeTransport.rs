//! Authenticated local-runtime carriage of the shared Link operations and binary buffers.

use std::ffi::{c_char, c_void};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex, OnceLock,
};
use std::time::Duration;

use crate::BridgeCodec::NativeFrame;
use crate::OperitFlutterBridge;
use crate::PlatformRuntimeAbi::FfiTransport::{connectionDescriptor, FfiSession};
use hmac::{Hmac, Mac};
use operit_host_native_common::DartPort::{DartPort, PostCObject};
use operit_host_native_common::SharedBuffer::SharedBuffer;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

const MAX_FRAME: usize = 64 * 1024 * 1024;
const QUEUE_CAPACITY: usize = 32;
const MAX_CONNECTIONS: usize = 16;
const PROTOCOL: u32 = 1;
type Control = Arc<dyn Fn(&[u8]) -> Result<String, String> + Send + Sync>;
type WireRequest = (u32, i64, serde_bytes::ByteBuf, Vec<serde_bytes::ByteBuf>);
type WireFrame = (serde_bytes::ByteBuf, Vec<serde_bytes::ByteBuf>);

/// Carries a private deployment credential; callers must never log this object.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModuleEndpoint {
    pub protocol: u32,
    pub port: u16,
    pub token: String,
    pub bootstrap: serde_json::Value,
}

impl ModuleEndpoint {
    /// Rejects unsupported deployments before any socket or Core is opened.
    fn validate(&self) -> Result<(), String> {
        if self.protocol != PROTOCOL
            || self.port == 0
            || self.token.len() != 64
            || !self.token.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("Invalid or incompatible module endpoint".into());
        }
        Ok(())
    }
}

/// Computes a role-specific proof without putting the deployment credential on the socket.
fn proof(token: &str, role: &[u8], nonce: &[u8]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(token.as_bytes()).expect("HMAC accepts byte keys");
    mac.update(b"operit-module-link-v1");
    mac.update(role);
    mac.update(nonce);
    mac.finalize().into_bytes().into()
}

/// Verifies one proof in constant time using the HMAC implementation.
fn verify(token: &str, role: &[u8], nonce: &[u8], received: &[u8]) -> Result<(), String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(token.as_bytes()).map_err(|e| e.to_string())?;
    mac.update(b"operit-module-link-v1");
    mac.update(role);
    mac.update(nonce);
    mac.verify_slice(received)
        .map_err(|_| "Module authentication failed".into())
}

/// Bounds authentication independently from the lifetime of an established watch stream.
fn configure(stream: &TcpStream, timeout: Option<Duration>) -> Result<(), String> {
    stream.set_nodelay(true).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(timeout)
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(15)))
        .map_err(|e| e.to_string())
}

/// Authenticates both endpoints before accepting any Link operation.
fn authenticate_server(stream: &mut TcpStream, token: &str) -> Result<(), String> {
    configure(stream, Some(Duration::from_secs(5)))?;
    let mut nonce = [0; 32];
    OsRng.fill_bytes(&mut nonce);
    stream.write_all(&nonce).map_err(|e| e.to_string())?;
    stream
        .write_all(&proof(token, b"server", &nonce))
        .map_err(|e| e.to_string())?;
    let mut received = [0; 32];
    stream
        .read_exact(&mut received)
        .map_err(|e| e.to_string())?;
    verify(token, b"client", &nonce, &received)?;
    stream.write_all(&[1]).map_err(|e| e.to_string())?;
    configure(stream, None)
}

/// Connects only to the fixed loopback carrier and checks the module's possession proof.
fn connect(endpoint: &ModuleEndpoint) -> Result<TcpStream, String> {
    endpoint.validate()?;
    let mut stream = TcpStream::connect_timeout(
        &([127, 0, 0, 1], endpoint.port).into(),
        Duration::from_secs(5),
    )
    .map_err(|e| e.to_string())?;
    configure(&stream, Some(Duration::from_secs(5)))?;
    let mut nonce = [0; 32];
    let mut received = [0; 32];
    stream.read_exact(&mut nonce).map_err(|e| e.to_string())?;
    stream
        .read_exact(&mut received)
        .map_err(|e| e.to_string())?;
    verify(&endpoint.token, b"server", &nonce, &received)?;
    stream
        .write_all(&proof(&endpoint.token, b"client", &nonce))
        .map_err(|e| e.to_string())?;
    let mut accepted = [0];
    stream
        .read_exact(&mut accepted)
        .map_err(|e| e.to_string())?;
    if accepted != [1] {
        return Err("Module rejected the connection".into());
    }
    configure(&stream, None)?;
    Ok(stream)
}

/// Writes a bounded length-prefixed transport envelope, never a pointer or native handle.
fn write_packet<T: Serialize>(stream: &mut TcpStream, packet: &T) -> Result<(), String> {
    let bytes = operit_link::encodeLink(packet).map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > MAX_FRAME {
        return Err("Module frame exceeds 64 MiB".into());
    }
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(&bytes).map_err(|e| e.to_string())
}

/// Checks the advertised size before allocating storage for an incoming packet.
fn read_packet<T: serde::de::DeserializeOwned>(stream: &mut TcpStream) -> Result<T, String> {
    let mut prefix = [0; 4];
    stream.read_exact(&mut prefix).map_err(|e| e.to_string())?;
    let length = u32::from_be_bytes(prefix) as usize;
    if length == 0 || length > MAX_FRAME {
        return Err("Invalid module frame size".into());
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    operit_link::decodeLink(&bytes).map_err(|e| e.to_string())
}

/// Converts shared immutable buffers to wire-owned bytes at the process boundary.
fn wire_frame(frame: NativeFrame) -> WireFrame {
    let (bytes, buffers) = frame.into_parts();
    (
        bytes.into(),
        buffers
            .iter()
            .map(|b| serde_bytes::ByteBuf::from(b.to_vec()))
            .collect(),
    )
}

/// Starts bounded independent sessions around one already-running module Core.
pub(crate) fn serve(
    listener: TcpListener,
    token: String,
    bridge: Arc<OperitFlutterBridge>,
    control: Control,
    stopping: &AtomicBool,
) -> Result<(), String> {
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    while !stopping.load(Ordering::Acquire) {
        let socket = match listener.accept() {
            Ok((socket, _)) => socket,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::park_timeout(Duration::from_millis(100));
                continue;
            }
            Err(error) => return Err(error.to_string()),
        };
        if active.fetch_add(1, Ordering::AcqRel) >= MAX_CONNECTIONS {
            active.fetch_sub(1, Ordering::AcqRel);
            drop(socket);
            continue;
        }
        let active = active.clone();
        let token = token.clone();
        let bridge = bridge.clone();
        let control = control.clone();
        std::thread::Builder::new()
            .name("module-link".into())
            .spawn(move || {
                if let Err(error) = serve_connection(socket, &token, bridge, control) {
                    eprintln!("Module Link connection ended: {error}");
                }
                active.fetch_sub(1, Ordering::AcqRel);
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Shares the existing connection executor while owning socket I/O on bounded queues.
fn serve_connection(
    mut socket: TcpStream,
    token: &str,
    bridge: Arc<OperitFlutterBridge>,
    control: Control,
) -> Result<(), String> {
    authenticate_server(&mut socket, token)?;
    let (sender, receiver) = mpsc::sync_channel::<Vec<NativeFrame>>(QUEUE_CAPACITY);
    let mut writer = socket.try_clone().map_err(|e| e.to_string())?;
    let stopped = Arc::new(AtomicBool::new(false));
    let writing_stopped = stopped.clone();
    let writer_thread = std::thread::spawn(move || {
        while !writing_stopped.load(Ordering::Acquire) {
            let batch = match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(batch) => batch,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            for frame in batch {
                if write_packet(&mut writer, &wire_frame(frame)).is_err() {
                    let _ = writer.shutdown(Shutdown::Both);
                    return;
                }
            }
        }
        let _ = writer.shutdown(Shutdown::Both);
    });
    let delivery = sender.clone();
    let delivery_socket = socket.try_clone().map_err(|e| e.to_string())?;
    let session = FfiSession::forWire(
        bridge,
        Arc::new(move |frames| {
            if delivery.try_send(frames).is_ok() {
                true
            } else {
                let _ = delivery_socket.shutdown(Shutdown::Both);
                false
            }
        }),
    )?;
    let result = (|| loop {
        let (operation, request, payload, buffers): WireRequest = read_packet(&mut socket)?;
        if buffers.len() > 256 {
            return Err("Too many module binary attachments".into());
        }
        if operation == 7 {
            let result =
                control(&payload).map_err(|e| operit_link::CoreLinkError::new("MODULE_CONTROL", e));
            sender
                .try_send(vec![NativeFrame::result(0, request, result)])
                .map_err(|e| e.to_string())?;
            continue;
        }
        let payload = payload.into_vec().into_boxed_slice();
        let length = payload.len();
        session.submitRequest(
            operation,
            request,
            payload,
            length,
            buffers
                .into_iter()
                .map(|b| operit_link::CoreBytes::from(b.into_vec()))
                .collect(),
        );
    })();
    session.close();
    stopped.store(true, Ordering::Release);
    drop(session);
    drop(sender);
    let _ = socket.shutdown(Shutdown::Both);
    let _ = writer_thread.join();
    result
}

/// Owns only the local client connection; it has no runtime, storage lease or Core tree.
struct RemoteSession {
    socket: TcpStream,
    sender: mpsc::SyncSender<WireRequest>,
    receiver: Mutex<Option<mpsc::Receiver<WireRequest>>>,
    port: OnceLock<DartPort>,
    closed: AtomicBool,
}

impl RemoteSession {
    /// Stops both socket directions so pending reads and writes wake immediately.
    fn close(&self) {
        self.closed.store(true, Ordering::Release);
        let _ = self.socket.shutdown(Shutdown::Both);
    }

    /// Delivers a terminal connection error that releases Dart calls, watches and pushes.
    fn fail(&self, error: String) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        let _ = self.socket.shutdown(Shutdown::Both);
        if let Some(port) = self.port.get() {
            let frame = NativeFrame::error(4, 0, "MODULE_DISCONNECTED", error);
            let _ = port.sendBatch(vec![frame.into_parts()]);
        }
    }
}

/// Opens a module client and returns an ABI descriptor whose pointers are local to the App.
pub(crate) fn connect_ffi(encoded: &str) -> Result<*mut c_char, String> {
    let endpoint: ModuleEndpoint = serde_json::from_str(encoded).map_err(|e| e.to_string())?;
    let socket = connect(&endpoint)?;
    let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
    let session = Arc::new(RemoteSession {
        socket,
        sender,
        receiver: Mutex::new(Some(receiver)),
        port: OnceLock::new(),
        closed: AtomicBool::new(false),
    });
    Ok(connectionDescriptor(
        Arc::into_raw(session) as usize,
        remote_attach as *const () as usize,
        remote_submit as *const () as usize,
        remote_release as *const () as usize,
    ))
}

/// Starts exactly one socket reader and one ordered writer after Dart installs its reply port.
unsafe extern "C" fn remote_attach(
    pointer: *const RemoteSession,
    post: PostCObject,
    port: i64,
) -> bool {
    Arc::increment_strong_count(pointer);
    let session = Arc::from_raw(pointer);
    if session.port.set(DartPort::new(post, port)).is_err() {
        return false;
    }
    let Some(receiver) = session.receiver.lock().expect("module queue lock").take() else {
        return false;
    };
    let reader = session.socket.try_clone();
    let writer = session.socket.try_clone();
    let (Ok(mut reader), Ok(mut writer)) = (reader, writer) else {
        session.close();
        return false;
    };
    let writing = Arc::downgrade(&session);
    std::thread::spawn(move || {
        for request in receiver {
            let Some(session) = writing.upgrade() else {
                return;
            };
            if session.closed.load(Ordering::Acquire) {
                return;
            }
            if let Err(error) = write_packet(&mut writer, &request) {
                session.fail(error);
                return;
            }
        }
    });
    let reading = Arc::downgrade(&session);
    std::thread::spawn(move || loop {
        let result = read_packet::<WireFrame>(&mut reader);
        let Some(session) = reading.upgrade() else {
            return;
        };
        match result {
            Ok((bytes, buffers)) => {
                let frames = vec![(
                    bytes.into_vec(),
                    buffers.into_iter().map(|b| b.into_vec().into()).collect(),
                )];
                if !session
                    .port
                    .get()
                    .expect("module port installed")
                    .sendBatch(frames)
                {
                    session.close();
                    return;
                }
            }
            Err(error) => {
                session.fail(error);
                return;
            }
        }
    });
    true
}

/// Captures local shared allocations before serializing an ordered socket request.
unsafe extern "C" fn remote_submit(
    pointer: *const RemoteSession,
    operation: u32,
    request: i64,
    bytes: *mut u8,
    length: usize,
    handles: *const *const SharedBuffer,
    count: usize,
) {
    Arc::increment_strong_count(pointer);
    let session = Arc::from_raw(pointer);
    let width = std::mem::size_of::<usize>();
    let allocation = (length + width - 1) / width * width + count * width;
    let bytes = Box::from_raw(std::ptr::slice_from_raw_parts_mut(bytes, allocation));
    if session.closed.load(Ordering::Acquire) {
        session.fail("Module client is closed".into());
        return;
    }
    let buffers = (0..count)
        .map(|i| {
            let buffer = SharedBuffer::retain(*handles.add(i));
            serde_bytes::ByteBuf::from(buffer.to_vec())
        })
        .collect();
    let packet = (
        operation,
        request,
        serde_bytes::ByteBuf::from(bytes[..length].to_vec()),
        buffers,
    );
    if session.sender.try_send(packet).is_err() {
        session.fail("Module client queue is full or closed".into());
    }
}

/// Detaches the App client without stopping the module Core.
unsafe extern "C" fn remote_release(pointer: *mut c_void) {
    Arc::from_raw(pointer.cast::<RemoteSession>()).close();
}

/// Persists bootstrap metadata over the same authenticated carrier without starting an App Core.
pub(crate) fn write_bootstrap(endpoint: &str, content: &str) -> Result<(), String> {
    let endpoint = serde_json::from_str(endpoint).map_err(|e| e.to_string())?;
    let mut socket = connect(&endpoint)?;
    configure(&socket, Some(Duration::from_secs(15)))?;
    write_packet(
        &mut socket,
        &(
            7u32,
            1i64,
            serde_bytes::ByteBuf::from(content.as_bytes().to_vec()),
            Vec::<serde_bytes::ByteBuf>::new(),
        ),
    )?;
    let (bytes, _): WireFrame = read_packet(&mut socket)?;
    if bytes.len() < 9 || bytes[0] != 0 {
        return Err("Invalid module control reply".into());
    }
    let result: Vec<operit_link::CoreValue> =
        operit_link::decodeLink(&bytes[9..]).map_err(|e| e.to_string())?;
    match result.first() {
        Some(operit_link::CoreValue::Unsigned(0)) => Ok(()),
        _ => Err(
            "Module rejected bootstrap changes; identity and storage roots are fixed while running"
                .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Proves mutual authentication accepts matching credentials and rejects a forged client.
    #[test]
    fn loopback_authentication_and_binary_frames() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = ModuleEndpoint {
            protocol: PROTOCOL,
            port: listener.local_addr().unwrap().port(),
            token: "a".repeat(64),
            bootstrap: serde_json::Value::Null,
        };
        let token = endpoint.token.clone();
        let worker = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            authenticate_server(&mut socket, &token).unwrap();
            let packet: WireRequest = read_packet(&mut socket).unwrap();
            assert_eq!(packet.2.as_slice(), &[0, 255, 2]);
            assert_eq!(packet.3[0].as_slice(), &[255, 0]);
        });
        let mut socket = connect(&endpoint).unwrap();
        write_packet(
            &mut socket,
            &(
                0u32,
                42i64,
                serde_bytes::ByteBuf::from(vec![0, 255, 2]),
                vec![serde_bytes::ByteBuf::from(vec![255, 0])],
            ),
        )
        .unwrap();
        worker.join().unwrap();
        let nonce = [17; 32];
        assert!(verify(
            &endpoint.token,
            b"client",
            &nonce,
            &proof(&"b".repeat(64), b"client", &nonce)
        )
        .is_err());
        assert!(verify(
            &endpoint.token,
            b"server",
            &nonce,
            &proof(&endpoint.token, b"client", &nonce)
        )
        .is_err());
    }

    /// Rejects oversized packets before attempting to read or allocate their payload.
    #[test]
    fn oversized_frames_are_rejected() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .write_all(&((MAX_FRAME + 1) as u32).to_be_bytes())
                .unwrap();
        });
        let mut socket = TcpStream::connect(address).unwrap();
        assert!(read_packet::<WireRequest>(&mut socket)
            .unwrap_err()
            .contains("size"));
        worker.join().unwrap();
    }
}
