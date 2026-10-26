use operit_host_native_common::{scheduleNativeServiceTask, NativeTaskHandle as JoinHandle};
use std::collections::HashMap;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use operit_host_api::{
    HostError, HostResult, PluginSdkIpcEndpoint, PluginSdkIpcHost, PluginSdkIpcSessionCallbacks,
    PluginSdkIpcSessionId,
};

use operit_host_native_plugin_sdk_ipc::stream::{
    closePluginSdkIpcStream, sendPluginSdkIpcStream, spawnPluginSdkIpcReadLoop,
    PluginSdkIpcStreamSession,
};

/// Closing the retained writer must also wake readers holding cloned Unix stream handles.
struct SocketWriter(UnixStream);
impl std::io::Write for SocketWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        std::io::Write::write(&mut self.0, bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::Write::flush(&mut self.0)
    }
}
impl Drop for SocketWriter {
    fn drop(&mut self) {
        let _ = self.0.shutdown(std::net::Shutdown::Both);
    }
}

/// Unix-domain-socket Plugin SDK IPC carrier used as a byte stream helper.
pub struct UnixPluginSdkIpcHost {
    inner: Arc<Mutex<UnixInner>>,
    nextSession: Arc<AtomicU64>,
    directory: PathBuf,
}

struct UnixInner {
    listenerStop: Option<Arc<AtomicBool>>,
    listenerThread: Option<JoinHandle<()>>,
    listenerLock: Option<std::fs::File>,
    socketPath: Option<PathBuf>,
    sessions: HashMap<String, Arc<PluginSdkIpcStreamSession>>,
}

impl UnixPluginSdkIpcHost {
    /// Creates a Unix-domain-socket Plugin SDK IPC carrier.
    pub fn new() -> Self {
        Self::withInstance(&std::env::var("OPERIT_PLUGIN_SDK_INSTANCE").unwrap_or_default())
    }

    /// Selects one instance while respecting an explicit socket directory override.
    pub fn withInstance(instance: &str) -> Self {
        let directory = std::env::var_os("OPERIT_PLUGIN_SDK_SOCKET_DIR")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let root = std::env::temp_dir();
                if instance.is_empty() {
                    root
                } else {
                    root.join(operit_host_native_plugin_sdk_ipc::pluginSdkIpcInstanceName(
                        "operit-ipc",
                        instance,
                    ))
                }
            });
        Self::withDirectory(directory)
    }

    /// Uses an explicit directory shared by one application profile and its SDK clients.
    pub fn withDirectory(directory: PathBuf) -> Self {
        Self {
            inner: Arc::new(Mutex::new(UnixInner {
                listenerStop: None,
                listenerThread: None,
                listenerLock: None,
                socketPath: None,
                sessions: HashMap::new(),
            })),
            nextSession: Arc::new(AtomicU64::new(1)),
            directory,
        }
    }

    /// Maps the logical endpoint name to the Unix socket path.
    #[allow(non_snake_case)]
    fn socketPathForEndpoint(&self, endpoint: &PluginSdkIpcEndpoint) -> PathBuf {
        Self::socketPathInDirectory(self.directory.clone(), endpoint)
    }

    /// Keeps the carrier inside its permitted temp directory and macOS SUN_LEN.
    #[allow(non_snake_case)]
    fn socketPathInDirectory(directory: PathBuf, endpoint: &PluginSdkIpcEndpoint) -> PathBuf {
        use std::os::unix::ffi::OsStrExt;
        let legacy = directory.join(format!("{}.sock", endpoint.name.replace('.', "_")));
        // sockaddr_un.sun_path has 104 bytes on Apple platforms, including NUL.
        if legacy.as_os_str().as_bytes().len() < 104 {
            return legacy;
        }
        // Sandbox temp roots include the bundle identifier. Shorten only the
        // endpoint filename; do not move IPC outside the sandbox or change cwd.
        let filename = if endpoint.name == operit_host_api::PLUGIN_SDK_IPC_ENDPOINT_NAME {
            "sdk".to_string()
        } else {
            // Stable FNV-1a, unlike DefaultHasher's unspecified implementation.
            let hash = endpoint
                .name
                .bytes()
                .fold(0xcbf29ce484222325u64, |hash, byte| {
                    (hash ^ byte as u64).wrapping_mul(0x100000001b3)
                });
            format!("{hash:016x}")
        };
        directory.join(filename)
    }

    /// Allocates the next session identifier.
    fn nextSessionId(&self) -> PluginSdkIpcSessionId {
        PluginSdkIpcSessionId::new(format!(
            "plugin-sdk-unix-{}",
            self.nextSession.fetch_add(1, Ordering::Relaxed)
        ))
    }

    /// Locks carrier state.
    fn lock(&self) -> HostResult<std::sync::MutexGuard<'_, UnixInner>> {
        self.inner
            .lock()
            .map_err(|error| HostError::new(format!("Plugin SDK Unix IPC lock poisoned: {error}")))
    }

    /// Attaches one connected Unix stream as a session.
    fn attachStream(
        &self,
        stream: UnixStream,
        callbacks: PluginSdkIpcSessionCallbacks,
        listenerOwned: bool,
    ) -> HostResult<PluginSdkIpcSessionId> {
        let sessionId = self.nextSessionId();
        let reader = stream
            .try_clone()
            .map_err(|error| HostError::new(error.to_string()))?;
        let writer = stream
            .try_clone()
            .map_err(|error| HostError::new(error.to_string()))?;
        let session = Arc::new(PluginSdkIpcStreamSession::new(
            sessionId.clone(),
            Box::new(SocketWriter(writer)),
            listenerOwned,
        ));
        {
            let mut inner = self.lock()?;
            inner.sessions.insert(sessionId.0.clone(), session.clone());
        }
        let sessions = self.inner.clone();
        let closedId = sessionId.clone();
        spawnPluginSdkIpcReadLoop(sessionId.clone(), reader, callbacks.clone(), move || {
            if let Ok(mut inner) = sessions.lock() {
                inner.sessions.remove(&closedId.0);
            }
        })?;
        (callbacks.onConnected)(sessionId.clone());
        Ok(sessionId)
    }
}

impl Default for UnixPluginSdkIpcHost {
    /// Creates a Unix-domain-socket Plugin SDK IPC carrier.
    fn default() -> Self {
        Self::new()
    }
}

impl PluginSdkIpcHost for UnixPluginSdkIpcHost {
    /// Rejects activation because process launch belongs to a platform host crate.
    fn activate(&self, _endpoint: PluginSdkIpcEndpoint) -> HostResult<()> {
        Err(HostError::new(
            "Plugin SDK process activation is implemented by the platform host crate",
        ))
    }

    /// Binds a Unix-domain socket and accepts Plugin SDK clients.
    fn startListener(
        &self,
        endpoint: PluginSdkIpcEndpoint,
        callbacks: PluginSdkIpcSessionCallbacks,
    ) -> HostResult<()> {
        let mut inner = self.lock()?;
        if inner.listenerThread.is_some() {
            return Err(HostError::new("Plugin SDK IPC listener is already started"));
        }
        let socketPath = self.socketPathForEndpoint(&endpoint);
        std::fs::create_dir_all(&self.directory)
            .map_err(|error| HostError::new(error.to_string()))?;
        // Hold the endpoint lock through shutdown. Only its owner may remove a stale socket.
        let ownership = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(socketPath.with_extension("lock"))
            .map_err(|error| HostError::new(error.to_string()))?;
        ownership.try_lock().map_err(|error| {
            HostError::new(format!(
                "Plugin SDK IPC endpoint is already in use: {error}"
            ))
        })?;
        match UnixStream::connect(&socketPath) {
            Ok(_) => return Err(HostError::new("Plugin SDK IPC endpoint is already in use")),
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
                std::fs::remove_file(&socketPath)
                    .map_err(|error| HostError::new(error.to_string()))?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(HostError::new(error.to_string())),
        }
        let listener =
            UnixListener::bind(&socketPath).map_err(|error| HostError::new(error.to_string()))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| HostError::new(error.to_string()))?;
        let stop = Arc::new(AtomicBool::new(false));
        let host = self.inner.clone();
        let nextSession = self.nextSession.clone();
        let workerStop = stop.clone();
        let thread = scheduleNativeServiceTask("operit-plugin-sdk-ipc-listener", move || {
            while !workerStop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        // The frame reader is blocking; accepted sockets may inherit O_NONBLOCK.
                        if stream.set_nonblocking(false).is_err() {
                            continue;
                        }
                        let sessionId = PluginSdkIpcSessionId::new(format!(
                            "plugin-sdk-unix-{}",
                            nextSession.fetch_add(1, Ordering::Relaxed)
                        ));
                        let Ok(reader) = stream.try_clone() else {
                            continue;
                        };
                        let Ok(writer) = stream.try_clone() else {
                            continue;
                        };
                        let session = Arc::new(PluginSdkIpcStreamSession::new(
                            sessionId.clone(),
                            Box::new(SocketWriter(writer)),
                            true,
                        ));
                        if let Ok(mut inner) = host.lock() {
                            inner.sessions.insert(sessionId.0.clone(), session);
                        }
                        let sessions = host.clone();
                        let closedId = sessionId.clone();
                        let scheduled = spawnPluginSdkIpcReadLoop(
                            sessionId.clone(),
                            reader,
                            callbacks.clone(),
                            move || {
                                if let Ok(mut inner) = sessions.lock() {
                                    inner.sessions.remove(&closedId.0);
                                }
                            },
                        );
                        if let Err(error) = scheduled {
                            operit_host_api::logHostError(
                                "PluginSdkIpc",
                                &format!("Plugin SDK reader rejected: {error}"),
                            );
                            continue;
                        }
                        (callbacks.onConnected)(sessionId);
                    }
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            || error.kind() == std::io::ErrorKind::Interrupted =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    Err(_) => break,
                }
            }
        });
        let thread = match thread {
            Ok(thread) => thread,
            Err(error) => {
                std::fs::remove_file(&socketPath).map_err(|cleanup| {
                    HostError::new(format!("{error}; remove rejected listener: {cleanup}"))
                })?;
                return Err(error);
            }
        };
        inner.listenerStop = Some(stop);
        inner.listenerThread = Some(thread);
        inner.socketPath = Some(socketPath);
        inner.listenerLock = Some(ownership);
        Ok(())
    }

    /// Connects to the Operit Unix-domain Plugin SDK listener.
    fn connect(
        &self,
        endpoint: PluginSdkIpcEndpoint,
        callbacks: PluginSdkIpcSessionCallbacks,
    ) -> HostResult<PluginSdkIpcSessionId> {
        let stream = UnixStream::connect(self.socketPathForEndpoint(&endpoint))
            .map_err(|error| HostError::new(error.to_string()))?;
        self.attachStream(stream, callbacks, false)
    }

    /// Sends one framed payload on a Unix-domain session.
    fn send(&self, sessionId: &PluginSdkIpcSessionId, bytes: Vec<u8>) -> HostResult<()> {
        let session = {
            let inner = self.lock()?;
            inner.sessions.get(&sessionId.0).cloned().ok_or_else(|| {
                HostError::new(format!("Plugin SDK IPC session is closed: {}", sessionId.0))
            })?
        };
        sendPluginSdkIpcStream(&session, bytes)
    }

    /// Closes one Unix-domain Plugin SDK session.
    fn closeSession(&self, sessionId: &PluginSdkIpcSessionId) -> HostResult<()> {
        let session = {
            let mut inner = self.lock()?;
            inner.sessions.remove(&sessionId.0).ok_or_else(|| {
                HostError::new(format!("Plugin SDK IPC session is closed: {}", sessionId.0))
            })?
        };
        closePluginSdkIpcStream(&session);
        Ok(())
    }

    /// Stops the Unix-domain listener and closes listener-owned sessions.
    fn stopListener(&self) -> HostResult<()> {
        let (stop, thread, socketPath, owned, _ownership);
        {
            let mut inner = self.lock()?;
            stop = inner
                .listenerStop
                .take()
                .ok_or_else(|| HostError::new("Plugin SDK IPC listener is not started"))?;
            thread = inner.listenerThread.take();
            socketPath = inner.socketPath.take();
            _ownership = inner.listenerLock.take();
            owned = inner
                .sessions
                .iter()
                .filter(|(_, session)| session.listenerOwned)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for id in &owned {
                if let Some(session) = inner.sessions.remove(id) {
                    closePluginSdkIpcStream(&session);
                }
            }
        }
        stop.store(true, Ordering::SeqCst);
        if let Some(thread) = thread {
            let _ = thread.join();
        }
        if let Some(socketPath) = socketPath {
            let _ = std::fs::remove_file(socketPath);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStrExt;

    fn testDirectory() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "ipc-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn callbacks(received: std::sync::mpsc::Sender<Vec<u8>>) -> PluginSdkIpcSessionCallbacks {
        PluginSdkIpcSessionCallbacks::new(
            Arc::new(|_| {}),
            Arc::new(move |_, bytes| {
                let _ = received.send(bytes);
            }),
            Arc::new(|_| {}),
        )
    }

    /// A second listener cannot unlink a live endpoint, and shutdown releases its ownership.
    #[test]
    fn endpoint_ownership_survives_duplicate_start_and_releases_on_stop() {
        let directory = testDirectory();
        let first = UnixPluginSdkIpcHost::withDirectory(directory.clone());
        let second = UnixPluginSdkIpcHost::withDirectory(directory.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        let endpoint = PluginSdkIpcEndpoint::standard();
        first
            .startListener(endpoint.clone(), callbacks(tx.clone()))
            .unwrap();
        assert!(first
            .startListener(endpoint.clone(), callbacks(tx.clone()))
            .is_err());
        assert!(second
            .startListener(endpoint.clone(), callbacks(tx.clone()))
            .is_err());
        assert!(second.stopListener().is_err());
        let client = UnixPluginSdkIpcHost::withDirectory(directory.clone());
        let id = client
            .connect(endpoint.clone(), callbacks(tx.clone()))
            .unwrap();
        client.send(&id, b"still first".to_vec()).unwrap();
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_secs(3)).unwrap(),
            b"still first"
        );
        client.closeSession(&id).unwrap();
        first.stopListener().unwrap();
        second.startListener(endpoint, callbacks(tx)).unwrap();
        second.stopListener().unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }

    /// Detects older listeners without an ownership lock and recovers their stale socket after exit.
    #[test]
    fn active_legacy_socket_is_preserved_and_stale_socket_is_recovered() {
        let directory = testDirectory();
        let host = UnixPluginSdkIpcHost::withDirectory(directory.clone());
        let endpoint = PluginSdkIpcEndpoint::standard();
        let path = host.socketPathForEndpoint(&endpoint);
        let listener = UnixListener::bind(&path).unwrap();
        let (tx, _) = std::sync::mpsc::channel();
        assert!(host
            .startListener(endpoint.clone(), callbacks(tx.clone()))
            .is_err());
        assert!(UnixStream::connect(&path).is_ok());
        drop(listener);
        host.startListener(endpoint, callbacks(tx)).unwrap();
        host.stopListener().unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }

    /// Stopping a listener must wake an idle client's blocking reader and close its session.
    #[test]
    fn listener_shutdown_closes_idle_client() {
        let directory = testDirectory();
        let server = UnixPluginSdkIpcHost::withDirectory(directory.clone());
        let client = UnixPluginSdkIpcHost::withDirectory(directory.clone());
        let (tx, _) = std::sync::mpsc::channel();
        let (connected, accepted) = std::sync::mpsc::channel();
        let (closed, disconnected) = std::sync::mpsc::channel();
        let mut incoming = callbacks(tx.clone());
        incoming.onConnected = Arc::new(move |_| {
            let _ = connected.send(());
        });
        let mut outgoing = callbacks(tx);
        outgoing.onClosed = Arc::new(move |_| {
            let _ = closed.send(());
        });
        let endpoint = PluginSdkIpcEndpoint::standard();
        server.startListener(endpoint.clone(), incoming).unwrap();
        let id = client.connect(endpoint, outgoing).unwrap();
        accepted
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        server.stopListener().unwrap();
        disconnected
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        assert!(client.send(&id, vec![1]).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    /// Two profiles can use the standard endpoint concurrently without mixing client traffic.
    #[test]
    fn instances_keep_sdk_clients_separate() {
        let mut directories = Vec::new();
        let mut servers = Vec::new();
        for index in 0..2 {
            let instance = format!("test-{}-{index}", std::process::id());
            let server = UnixPluginSdkIpcHost::withInstance(&instance);
            let client = UnixPluginSdkIpcHost::withInstance(&instance);
            directories.push(server.directory.clone());
            let (tx, rx) = std::sync::mpsc::channel();
            let endpoint = PluginSdkIpcEndpoint::standard();
            let (connected, accepted) = std::sync::mpsc::channel();
            let mut incoming = callbacks(tx.clone());
            incoming.onConnected = Arc::new(move |_| {
                let _ = connected.send(());
            });
            server.startListener(endpoint.clone(), incoming).unwrap();
            let id = client.connect(endpoint, callbacks(tx)).unwrap();
            accepted
                .recv_timeout(std::time::Duration::from_secs(3))
                .unwrap();
            servers.push((server, client, id, rx, index as u8));
        }
        assert_ne!(directories[0], directories[1]);
        for (_, client, id, _, index) in &servers {
            client.send(id, vec![*index]).unwrap();
        }
        for (server, client, id, rx, index) in servers {
            assert_eq!(
                rx.recv_timeout(std::time::Duration::from_secs(3)).unwrap(),
                vec![index]
            );
            client.closeSession(&id).unwrap();
            server.stopListener().unwrap();
        }
        for directory in directories {
            std::fs::remove_dir_all(directory).unwrap();
        }
    }

    #[test]
    fn short_temp_directories_preserve_existing_sdk_endpoint() {
        let path = UnixPluginSdkIpcHost::socketPathInDirectory(
            PathBuf::from("/tmp"),
            &PluginSdkIpcEndpoint::standard(),
        );
        assert_eq!(path, PathBuf::from("/tmp/operit_plugin_sdk.sock"));
    }

    #[test]
    fn long_sandbox_directory_can_bind_sdk_socket() {
        let root = std::env::temp_dir().join(format!("ipc-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let count = 90usize
            .checked_sub(root.as_os_str().as_bytes().len() + 1)
            .unwrap();
        let directory = root.join("a".repeat(count));
        std::fs::create_dir_all(&directory).unwrap();
        let endpoint = PluginSdkIpcEndpoint::standard();
        let path = UnixPluginSdkIpcHost::socketPathInDirectory(directory.clone(), &endpoint);
        assert_eq!(path.parent(), Some(directory.as_path()));
        assert!(path.as_os_str().as_bytes().len() < 104);
        let listener = UnixListener::bind(&path).unwrap();
        let client = UnixStream::connect(&path).unwrap();
        let _accepted = listener.accept().unwrap();
        drop(client);
        drop(listener);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compact_custom_endpoints_are_stable_and_distinct() {
        let directory = PathBuf::from(format!("/{}", "a".repeat(85)));
        let first = PluginSdkIpcEndpoint {
            name: "operit.custom.first".into(),
        };
        let second = PluginSdkIpcEndpoint {
            name: "operit.custom.second".into(),
        };
        let path = UnixPluginSdkIpcHost::socketPathInDirectory(directory.clone(), &first);
        assert_eq!(
            path,
            UnixPluginSdkIpcHost::socketPathInDirectory(directory.clone(), &first)
        );
        assert_ne!(
            path,
            UnixPluginSdkIpcHost::socketPathInDirectory(directory, &second)
        );
        assert!(path.as_os_str().as_bytes().len() < 104);
    }
}
