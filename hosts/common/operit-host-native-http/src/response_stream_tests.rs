use super::*;
use std::io::Read;
use std::net::{TcpListener, TcpStream};

fn readRequest(stream: &mut TcpStream) {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
    }
}

fn request(address: std::net::SocketAddr) -> HttpRequestData {
    HttpRequestData {
        url: format!("http://{address}/stream"),
        method: "GET".into(),
        headers: Vec::new(),
        body: Vec::new(),
        formFields: Vec::new(),
        fileParts: Vec::new(),
        connectTimeoutSeconds: 2,
        readTimeoutSeconds: 0,
        followRedirects: true,
        ignoreSsl: false,
        proxyHost: String::new(),
        proxyPort: 0,
    }
}

enum Event {
    Head(HttpResponseHead),
    Bytes(Vec<u8>),
    Closed(Result<(), String>),
}

/// Reuses one real keep-alive socket across streamed and buffered Host requests.
#[test]
fn streamedAndBufferedRequestsShareConnections() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        for _ in 0..3 {
            readRequest(&mut socket);
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: keep-alive\r\n\r\ntest",
                )
                .unwrap();
        }
    });
    let host = NativeHttpHost::new();
    for index in 0..2 {
        let (sender, events) = mpsc::channel();
        let chunk_sender = sender.clone();
        host.openHttpResponseStream(
            format!("reuse-stream-{index}"),
            request(address),
            Arc::new(|head| assert_eq!(head.statusCode, 200)),
            Arc::new(move |bytes| {
                chunk_sender.send(Event::Bytes(bytes)).unwrap();
            }),
            Arc::new(move |result| {
                sender.send(Event::Closed(result)).unwrap();
            }),
        )
        .unwrap();
        let mut body = Vec::new();
        loop {
            match events.recv_timeout(Duration::from_secs(4)).unwrap() {
                Event::Bytes(bytes) => body.extend(bytes),
                Event::Closed(result) => {
                    result.unwrap();
                    break;
                }
                Event::Head(_) => unreachable!(),
            }
        }
        assert_eq!(body, b"test");
    }
    assert_eq!(
        host.executeHttpRequest(request(address)).unwrap().body,
        b"test"
    );
    server.join().unwrap();
}

#[test]
fn httpResponseStreamDeliversMetadataBeforeChunksAndCancelsOpenBody() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        readRequest(&mut socket);
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nMcp-Session-Id: fixture-session\r\nTransfer-Encoding: chunked\r\n\r\n4\r\ntest\r\n").unwrap();
        // No HTTP terminator: host cancellation must close the response/socket.
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).unwrap(), 0);
    });
    let host = NativeHttpHost::new();
    let (sender, events) = mpsc::channel();
    let headSender = sender.clone();
    let chunkSender = sender.clone();
    host.openHttpResponseStream(
        "metadata-cancel".into(),
        request(address),
        Arc::new(move |head| {
            headSender.send(Event::Head(head)).unwrap();
        }),
        Arc::new(move |bytes| {
            chunkSender.send(Event::Bytes(bytes)).unwrap();
        }),
        Arc::new(move |result| {
            sender.send(Event::Closed(result)).unwrap();
        }),
    )
    .unwrap();
    match events.recv_timeout(Duration::from_secs(2)).unwrap() {
        Event::Head(head) => {
            assert_eq!(head.statusCode, 200);
            assert!(head
                .headers
                .iter()
                .any(|(name, value)| name == "mcp-session-id" && value == "fixture-session"));
        }
        _ => panic!("response metadata must precede body chunks"),
    }
    match events.recv_timeout(Duration::from_secs(2)).unwrap() {
        Event::Bytes(bytes) => assert_eq!(bytes, b"test"),
        _ => panic!("expected body chunk"),
    }
    host.closeHttpByteStream("metadata-cancel").unwrap();
    match events.recv_timeout(Duration::from_secs(2)).unwrap() {
        Event::Closed(result) => result.unwrap(),
        _ => panic!("expected cancellation completion"),
    }
    server.join().unwrap();
}

#[test]
fn httpResponseStreamCanCancelBeforeHeadersArrive() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (acceptedSender, accepted) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        readRequest(&mut socket);
        acceptedSender.send(()).unwrap();
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).unwrap(), 0);
    });
    let host = NativeHttpHost::new();
    let (sender, closed) = mpsc::channel();
    host.openHttpResponseStream(
        "cancel-headers".into(),
        request(address),
        Arc::new(|_| panic!("fixture never sends response headers")),
        Arc::new(|_| panic!("unexpected body")),
        Arc::new(move |result| {
            sender.send(result).unwrap();
        }),
    )
    .unwrap();
    accepted.recv_timeout(Duration::from_secs(2)).unwrap();
    host.closeHttpByteStream("cancel-headers").unwrap();
    closed
        .recv_timeout(Duration::from_secs(2))
        .unwrap()
        .unwrap();
    server.join().unwrap();
}

#[test]
fn httpResponseStreamPreservesHttpErrorStatusAndBody() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        readRequest(&mut socket);
        socket
            .write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 6\r\n\r\ndenied")
            .unwrap();
    });
    let host = NativeHttpHost::new();
    let (sender, events) = mpsc::channel();
    let headSender = sender.clone();
    let chunkSender = sender.clone();
    host.openHttpResponseStream(
        "http-error".into(),
        request(address),
        Arc::new(move |head| {
            headSender.send(Event::Head(head)).unwrap();
        }),
        Arc::new(move |bytes| {
            chunkSender.send(Event::Bytes(bytes)).unwrap();
        }),
        Arc::new(move |result| {
            sender.send(Event::Closed(result)).unwrap();
        }),
    )
    .unwrap();
    match events.recv_timeout(Duration::from_secs(2)).unwrap() {
        Event::Head(head) => assert_eq!(head.statusCode, 401),
        _ => panic!("expected error response metadata"),
    }
    let mut body = Vec::new();
    loop {
        match events.recv_timeout(Duration::from_secs(2)).unwrap() {
            Event::Bytes(bytes) => body.extend(bytes),
            Event::Closed(result) => {
                result.unwrap();
                break;
            }
            _ => panic!("duplicate response headers"),
        }
    }
    assert_eq!(body, b"denied");
    server.join().unwrap();
}
