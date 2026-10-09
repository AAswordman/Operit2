use std::collections::HashMap;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use operit_host_api::{
    HostError, HostResult, PluginSdkIpcEndpoint, PluginSdkIpcHost, PluginSdkIpcSessionCallbacks,
    PluginSdkIpcSessionId,
};

use operit_host_native_plugin_sdk_ipc::stream::{
    closePluginSdkIpcStream, sendPluginSdkIpcStream, spawnPluginSdkIpcReadLoop,
    PluginSdkIpcStreamSession,
};

/// Unix-domain-socket Plugin SDK IPC carrier used as a byte stream helper.
pub struct UnixPluginSdkIpcHost {
    inner: Arc<Mutex<UnixInner>>,
    nextSession: Arc<AtomicU64>,
}

struct UnixInner {
    listenerStop: Option<Arc<AtomicBool>>,
    listenerThread: Option<JoinHandle<()>>,
    socketPath: Option<PathBuf>,
    sessions: HashMap<String, Arc<PluginSdkIpcStreamSession>>,
}

impl UnixPluginSdkIpcHost {
    /// Creates a Unix-domain-socket Plugin SDK IPC carrier.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(UnixInner {
                listenerStop: None,
                listenerThread: None,
                socketPath: None,
                sessions: HashMap::new(),
            })),
            nextSession: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Maps the logical endpoint name to the Unix socket path.
    #[allow(non_snake_case)]
    fn socketPathForEndpoint(endpoint: &PluginSdkIpcEndpoint) -> PathBuf {
        Self::socketPathInDirectory(std::env::temp_dir(), endpoint)
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
            Box::new(writer),
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
        let socketPath = Self::socketPathForEndpoint(&endpoint);
        if socketPath.exists() {
            std::fs::remove_file(&socketPath).map_err(|error| HostError::new(error.to_string()))?;
        }
        let listener =
            UnixListener::bind(&socketPath).map_err(|error| HostError::new(error.to_string()))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| HostError::new(error.to_string()))?;
        let stop = Arc::new(AtomicBool::new(false));
        {
            let mut inner = self.lock()?;
            if inner.listenerThread.is_some() {
                return Err(HostError::new("Plugin SDK IPC listener is already started"));
            }
            inner.listenerStop = Some(stop.clone());
            inner.socketPath = Some(socketPath.clone());
        }
        let host = self.inner.clone();
        let nextSession = self.nextSession.clone();
        let thread = std::thread::Builder::new()
            .name("operit-plugin-sdk-ipc-listen".to_string())
            .spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
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
                                Box::new(writer),
                                true,
                            ));
                            if let Ok(mut inner) = host.lock() {
                                inner.sessions.insert(sessionId.0.clone(), session);
                            }
                            let sessions = host.clone();
                            let closedId = sessionId.clone();
                            let _ = spawnPluginSdkIpcReadLoop(
                                sessionId.clone(),
                                reader,
                                callbacks.clone(),
                                move || {
                                    if let Ok(mut inner) = sessions.lock() {
                                        inner.sessions.remove(&closedId.0);
                                    }
                                },
                            );
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
            })
            .map_err(|error| HostError::new(error.to_string()))?;
        self.lock()?.listenerThread = Some(thread);
        Ok(())
    }

    /// Connects to the Operit Unix-domain Plugin SDK listener.
    fn connect(
        &self,
        endpoint: PluginSdkIpcEndpoint,
        callbacks: PluginSdkIpcSessionCallbacks,
    ) -> HostResult<PluginSdkIpcSessionId> {
        let stream = UnixStream::connect(Self::socketPathForEndpoint(&endpoint))
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
        let (stop, thread, socketPath, owned);
        {
            let mut inner = self.lock()?;
            stop = inner
                .listenerStop
                .take()
                .ok_or_else(|| HostError::new("Plugin SDK IPC listener is not started"))?;
            thread = inner.listenerThread.take();
            socketPath = inner.socketPath.take();
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
