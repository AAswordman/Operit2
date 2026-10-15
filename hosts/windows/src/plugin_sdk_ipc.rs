use std::collections::HashMap;
use std::fs::File;
use std::os::windows::io::{FromRawHandle, OwnedHandle, RawHandle};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use operit_host_native_plugin_sdk_ipc::stream::{
    closePluginSdkIpcStream, sendPluginSdkIpcStream, spawnPluginSdkIpcReadLoop,
    PluginSdkIpcStreamSession,
};

use operit_host_api::{
    HostError, HostResult, PluginSdkIpcEndpoint, PluginSdkIpcHost, PluginSdkIpcSessionCallbacks,
    PluginSdkIpcSessionId,
};
use windows_sys::Win32::Foundation::{
    GetLastError, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    CreateNamedPipeW, WaitNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
    PIPE_WAIT,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

#[path = "plugin_sdk_pipe.rs"]
mod pipe_io;
const ACTIVATION_URI: &str = "operit2://plugin-sdk";
const PIPE_READY_TIMEOUT_MS: u32 = 30_000;

/// Windows named-pipe Plugin SDK IPC carrier.
pub struct WindowsPluginSdkIpcHost {
    instance: String,
    inner: Arc<Mutex<WindowsInner>>,
    nextSession: Arc<AtomicU64>,
}

struct WindowsInner {
    listenerStop: Option<Arc<AtomicBool>>,
    listenerThread: Option<JoinHandle<()>>,
    sessions: HashMap<String, Arc<PluginSdkIpcStreamSession>>,
    readers: HashMap<String, JoinHandle<()>>,
}

/// Joins cancelled overlapped I/O before releasing the SDK library.
fn joinPipeThread(thread: JoinHandle<()>) -> HostResult<()> {
    thread
        .join()
        .map_err(|_| HostError::new("Plugin SDK pipe thread panicked"))
}

impl WindowsPluginSdkIpcHost {
    /// Creates a Windows named-pipe Plugin SDK IPC carrier.
    pub fn new() -> Self {
        Self::withInstance(&std::env::var("OPERIT_PLUGIN_SDK_INSTANCE").unwrap_or_default())
    }

    /// Gives each application profile its own named-pipe address.
    pub fn withInstance(instance: &str) -> Self {
        Self {
            instance: instance.to_owned(),
            inner: Arc::new(Mutex::new(WindowsInner {
                listenerStop: None,
                listenerThread: None,
                sessions: HashMap::new(),
                readers: HashMap::new(),
            })),
            nextSession: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Maps the logical endpoint name to the Windows named-pipe path.
    #[allow(non_snake_case)]
    fn pipeName(&self, endpoint: &PluginSdkIpcEndpoint) -> String {
        let name = operit_host_native_plugin_sdk_ipc::pluginSdkIpcInstanceName(
            &endpoint.name,
            &self.instance,
        );
        format!(r"\\.\pipe\{}", name.replace('.', "_"))
    }

    /// Encodes a Windows path as a null-terminated UTF-16 buffer.
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Allocates the next session identifier.
    fn nextSessionId(&self) -> PluginSdkIpcSessionId {
        PluginSdkIpcSessionId::new(format!(
            "plugin-sdk-windows-{}",
            self.nextSession.fetch_add(1, Ordering::Relaxed)
        ))
    }

    /// Locks carrier state.
    fn lock(&self) -> HostResult<std::sync::MutexGuard<'_, WindowsInner>> {
        self.inner.lock().map_err(|error| {
            HostError::new(format!("Plugin SDK Windows IPC lock poisoned: {error}"))
        })
    }

    /// Wraps a Windows pipe handle as a std File.
    fn fileFromHandle(handle: HANDLE) -> HostResult<File> {
        if handle == INVALID_HANDLE_VALUE {
            return Err(HostError::new("Plugin SDK named pipe handle is invalid"));
        }
        let owned = unsafe { OwnedHandle::from_raw_handle(handle as RawHandle) };
        Ok(File::from(owned))
    }

    /// Attaches one connected named-pipe File as a session.
    fn attachFile(
        &self,
        file: File,
        callbacks: PluginSdkIpcSessionCallbacks,
        listenerOwned: bool,
    ) -> HostResult<PluginSdkIpcSessionId> {
        let sessionId = self.nextSessionId();
        let reader = file
            .try_clone()
            .map_err(|error| HostError::new(error.to_string()))?;
        let writer = file
            .try_clone()
            .map_err(|error| HostError::new(error.to_string()))?;
        let closed = Arc::new(AtomicBool::new(false));
        let mut session = PluginSdkIpcStreamSession::new(
            sessionId.clone(),
            Box::new(pipe_io::PipeIo::new(writer, closed.clone())),
            listenerOwned,
        );
        session.closed = closed.clone();
        let session = Arc::new(session);
        self.lock()?
            .sessions
            .insert(sessionId.0.clone(), session.clone());
        let sessions = self.inner.clone();
        let closedId = sessionId.clone();
        let reader = pipe_io::PipeIo::new(reader, closed);
        (callbacks.onConnected)(sessionId.clone());
        let thread =
            spawnPluginSdkIpcReadLoop(sessionId.clone(), reader, callbacks.clone(), move || {
                closePluginSdkIpcStream(&session);
                if let Ok(mut inner) = sessions.lock() {
                    inner.sessions.remove(&closedId.0);
                }
            })?;
        self.lock()?.readers.insert(sessionId.0.clone(), thread);
        Ok(sessionId)
    }
}

impl Default for WindowsPluginSdkIpcHost {
    /// Creates a Windows named-pipe Plugin SDK IPC carrier.
    fn default() -> Self {
        Self::new()
    }
}

impl PluginSdkIpcHost for WindowsPluginSdkIpcHost {
    /// Starts Operit through the registered operit2 URI protocol and waits for the named pipe.
    fn activate(&self, endpoint: PluginSdkIpcEndpoint) -> HostResult<()> {
        let pipeName = Self::wide(&self.pipeName(&endpoint));
        if unsafe { WaitNamedPipeW(pipeName.as_ptr(), 0) } != 0 {
            return Ok(());
        }
        if !self.instance.is_empty() {
            return Err(HostError::new(
                "Start the selected Plugin SDK instance before connecting",
            ));
        }
        let verb = Self::wide("open");
        let uri = Self::wide(ACTIVATION_URI);
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                uri.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL as i32,
            )
        };
        if result as isize <= 32 {
            return Err(HostError::new(format!(
                "ShellExecuteW failed to start Operit for Plugin SDK: {}",
                result as isize
            )));
        }
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_millis(PIPE_READY_TIMEOUT_MS as u64);
        loop {
            if unsafe { WaitNamedPipeW(pipeName.as_ptr(), 100) } != 0 {
                break;
            }
            let error = unsafe { GetLastError() };
            if !matches!(error, 2 | 121 | 231) {
                return Err(HostError::new(format!(
                    "Operit Plugin SDK pipe readiness failed: {error}"
                )));
            }
            if std::time::Instant::now() >= deadline {
                return Err(HostError::new(
                    "Operit Plugin SDK named pipe did not become ready within 30 seconds",
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        Ok(())
    }

    /// Creates the named-pipe listener and accepts Plugin SDK clients.
    fn startListener(
        &self,
        endpoint: PluginSdkIpcEndpoint,
        callbacks: PluginSdkIpcSessionCallbacks,
    ) -> HostResult<()> {
        let pipeName = self.pipeName(&endpoint);
        let stop = Arc::new(AtomicBool::new(false));
        {
            let mut inner = self.lock()?;
            if inner.listenerThread.is_some() {
                return Err(HostError::new("Plugin SDK IPC listener is already started"));
            }
            inner.listenerStop = Some(stop.clone());
        }
        let host = self.inner.clone();
        let nextSession = self.nextSession.clone();
        let instance = self.instance.clone();
        let thread = std::thread::Builder::new()
            .name("operit-plugin-sdk-ipc-pipe-listen".to_string())
            .spawn(move || {
                let wide = WindowsPluginSdkIpcHost::wide(&pipeName);
                while !stop.load(Ordering::SeqCst) {
                    let handle = unsafe {
                        CreateNamedPipeW(
                            wide.as_ptr(),
                            PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                            PIPE_UNLIMITED_INSTANCES,
                            65536,
                            65536,
                            0,
                            std::ptr::null(),
                        )
                    };
                    if handle == INVALID_HANDLE_VALUE {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        continue;
                    }
                    let Ok(file) = WindowsPluginSdkIpcHost::fileFromHandle(handle) else {
                        continue;
                    };
                    if let Err(error) = pipe_io::accept(&file, &stop) {
                        if !stop.load(Ordering::SeqCst) {
                            operit_host_api::logHostError(
                                "WindowsPluginSdkIpc",
                                &format!("Plugin SDK pipe accept failed: {error}"),
                            );
                        }
                        break;
                    }
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let carrier = WindowsPluginSdkIpcHost {
                        instance: instance.clone(),
                        inner: host.clone(),
                        nextSession: nextSession.clone(),
                    };
                    if let Err(error) = carrier.attachFile(file, callbacks.clone(), true) {
                        operit_host_api::logHostError(
                            "WindowsPluginSdkIpc",
                            &format!("Plugin SDK pipe accept failed: {error}"),
                        );
                    }
                }
            })
            .map_err(|error| HostError::new(error.to_string()))?;
        self.lock()?.listenerThread = Some(thread);
        Ok(())
    }

    /// Starts Operit and connects to the named-pipe Plugin SDK listener.
    fn connect(
        &self,
        endpoint: PluginSdkIpcEndpoint,
        callbacks: PluginSdkIpcSessionCallbacks,
    ) -> HostResult<PluginSdkIpcSessionId> {
        self.activate(endpoint.clone())?;
        let pipeName = self.pipeName(&endpoint);
        let wide = Self::wide(&pipeName);
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                std::ptr::null_mut(),
            )
        };
        let file = Self::fileFromHandle(handle)?;
        self.attachFile(file, callbacks, false)
    }

    /// Sends one framed payload on a named-pipe session.
    fn send(&self, sessionId: &PluginSdkIpcSessionId, bytes: Vec<u8>) -> HostResult<()> {
        let session = {
            let inner = self.lock()?;
            inner.sessions.get(&sessionId.0).cloned().ok_or_else(|| {
                HostError::new(format!("Plugin SDK IPC session is closed: {}", sessionId.0))
            })?
        };
        sendPluginSdkIpcStream(&session, bytes)
    }

    /// Closes one named-pipe Plugin SDK session.
    fn closeSession(&self, sessionId: &PluginSdkIpcSessionId) -> HostResult<()> {
        let (session, reader) = {
            let mut inner = self.lock()?;
            (
                inner.sessions.remove(&sessionId.0),
                inner.readers.remove(&sessionId.0),
            )
        };
        if session.is_none() && reader.is_none() {
            return Err(HostError::new("Plugin SDK IPC session is closed"));
        }
        if let Some(session) = session {
            closePluginSdkIpcStream(&session);
        }
        if let Some(reader) = reader {
            joinPipeThread(reader)?;
        }
        Ok(())
    }

    /// Stops the named-pipe listener and closes listener-owned sessions.
    fn stopListener(&self) -> HostResult<()> {
        let (stop, thread);
        {
            let mut inner = self.lock()?;
            stop = inner
                .listenerStop
                .take()
                .ok_or_else(|| HostError::new("Plugin SDK IPC listener is not started"))?;
            thread = inner.listenerThread.take();
        }
        stop.store(true, Ordering::SeqCst);
        if let Some(thread) = thread {
            joinPipeThread(thread)?;
        }
        let owned = self
            .lock()?
            .sessions
            .iter()
            .filter(|(_, session)| session.listenerOwned)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in owned {
            self.closeSession(&PluginSdkIpcSessionId::new(id))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;
    use std::sync::mpsc;
    use std::time::Duration;

    /// Verifies named-pipe delivery and cancellation without launching the application UI.
    #[test]
    fn named_pipe_session_closes_reader() {
        let endpoint = PluginSdkIpcEndpoint {
            name: format!("operit-sdk-test-{}", std::process::id()),
        };
        let instance = format!("test-{}", std::process::id());
        let server = WindowsPluginSdkIpcHost::withInstance(&instance);
        let (tx, rx) = mpsc::channel();
        server
            .startListener(
                endpoint.clone(),
                PluginSdkIpcSessionCallbacks::new(
                    Arc::new(|_| {}),
                    Arc::new(move |_, bytes| {
                        tx.send(bytes).unwrap();
                    }),
                    Arc::new(|_| {}),
                ),
            )
            .unwrap();
        let path = WindowsPluginSdkIpcHost::wide(&server.pipeName(&endpoint));
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while unsafe { WaitNamedPipeW(path.as_ptr(), 100) } == 0 {
            assert!(
                std::time::Instant::now() < deadline,
                "test listener did not become ready"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(FILE_FLAG_OVERLAPPED)
            .open(server.pipeName(&endpoint))
            .unwrap();
        let client = WindowsPluginSdkIpcHost::withInstance(&instance);
        let other = WindowsPluginSdkIpcHost::withInstance(&format!("{instance}-other"));
        assert_ne!(server.pipeName(&endpoint), other.pipeName(&endpoint));
        assert!(other.activate(endpoint.clone()).is_err());
        let (closed_tx, closed_rx) = mpsc::channel();
        let id = client
            .attachFile(
                file,
                PluginSdkIpcSessionCallbacks::new(
                    Arc::new(|_| {}),
                    Arc::new(|_, _| {}),
                    Arc::new(move |_| {
                        closed_tx.send(()).unwrap();
                    }),
                ),
                false,
            )
            .unwrap();
        client.send(&id, vec![7; 128 * 1024]).unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(3)).unwrap().len(),
            128 * 1024
        );
        client.closeSession(&id).unwrap();
        closed_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        server.stopListener().unwrap();
    }
}
