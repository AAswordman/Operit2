use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{cleanup_endpoint, connect, control_endpoint, ControlListener};

static NEXT_ENDPOINT: AtomicU64 = AtomicU64::new(0);

pub(super) struct TestEndpoint(pub(super) PathBuf);

impl TestEndpoint {
    /// Allocates a transport endpoint distinct from application identities and parallel tests.
    pub(super) fn new() -> Self {
        let mut hash = DefaultHasher::new();
        std::process::id().hash(&mut hash);
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock")
            .as_nanos()
            .hash(&mut hash);
        NEXT_ENDPOINT
            .fetch_add(1, Ordering::Relaxed)
            .hash(&mut hash);
        let key = format!("{:016x}", hash.finish()).repeat(4);
        let directory = std::env::temp_dir().join(format!("operit2-{}", &key[..8]));
        Self(control_endpoint(&directory, &key))
    }
}

impl Drop for TestEndpoint {
    /// Removes only the test's transport endpoint after all handles have closed.
    fn drop(&mut self) {
        cleanup_endpoint(&self.0).expect("test endpoint cleanup");
    }
}

/// Checks that a missing endpoint is reported as a connection error.
#[tokio::test]
async fn missing_endpoint_does_not_connect() {
    let endpoint = TestEndpoint::new();
    assert!(connect(&endpoint.0).await.is_err());
}

/// Checks that dropping a listener permits a new owner to bind the same endpoint.
#[tokio::test]
async fn endpoint_is_available_after_listener_drop() {
    let endpoint = TestEndpoint::new();
    let listener = ControlListener::bind(&endpoint.0).expect("first bind");
    drop(listener);
    let listener = ControlListener::bind(&endpoint.0).expect("bind after owner exit");
    drop(listener);
}

/// Exercises overlapping connections and large responses using the actual OS transport.
#[tokio::test]
async fn concurrent_clients_receive_complete_responses() {
    let endpoint = TestEndpoint::new();
    let mut listener = ControlListener::bind(&endpoint.0).expect("bind");
    let server = tokio::spawn(async move {
        let mut requests = tokio::task::JoinSet::new();
        for _ in 0..6 {
            let mut stream = listener.accept().await.expect("accept");
            requests.spawn(async move {
                let id = stream.read_u8().await.expect("request");
                let response = vec![id; 256 * 1024 + usize::from(id)];
                stream.write_all(&response).await.expect("write response");
                stream.flush().await.expect("flush response");
            });
        }
        while let Some(result) = requests.join_next().await {
            result.expect("server request");
        }
    });
    let mut clients = tokio::task::JoinSet::new();
    for id in 0..6u8 {
        let path = endpoint.0.clone();
        clients.spawn(async move {
            let mut stream = connect(&path).await.expect("connect");
            stream.write_u8(id).await.expect("send request");
            let mut response = vec![0; 256 * 1024 + usize::from(id)];
            stream
                .read_exact(&mut response)
                .await
                .expect("complete response");
            assert!(response.iter().all(|byte| *byte == id));
        });
    }
    tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(result) = clients.join_next().await {
            result.expect("client");
        }
        server.await.expect("server");
    })
    .await
    .expect("transport exchange deadline");
}
