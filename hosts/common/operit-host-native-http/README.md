# Native HTTP transport

`NativeHttpHost` owns native API requests, response streams and downloads. Every
`reqwest` declaration explicitly enables `http2` because default features are
disabled. HTTPS requests use TLS ALPN to negotiate the HTTP version. Provider
code continues to use the shared HTTP Host; it does not select transport versions
or own network clients. Browser requests use the browser's Fetch transport.

`NativeHttpServerHost` uses one connection driver for both directly bound sockets
and accepted Host TCP streams. The driver uses Hyper's automatic HTTP/1 and
HTTP/2 protocol detection, preserves the remote address, and supplies WebSocket
upgrade handles. Direct listeners only accept sockets and manage their tasks;
they do not own a separate Axum serving path. Cleartext HTTP/2 clients must send
the HTTP/2 connection preface. This capability does not add TLS to bound sockets.

Explicit shutdown stops acceptance and cancels active HTTP connections and
upgraded WebSocket handlers. Connection tasks stay alive until their upgrade ends
or shutdown fires, including when a Host executor owns a runtime per task.

Run the focused transport suite with:

```sh
cargo test --locked --manifest-path hosts/common/operit-host-native-http/Cargo.toml --lib
```

The suite checks TLS ALPN negotiation through the production client builder,
simultaneous HTTP/2 streams through both server entries, HTTP/1 requests,
WebSocket upgrades, connection reuse, streamed responses, downloads and cancellation.
