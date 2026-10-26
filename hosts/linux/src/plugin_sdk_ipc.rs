use operit_host_api::{
    HostError, HostResult, PluginSdkIpcEndpoint, PluginSdkIpcHost, PluginSdkIpcSessionCallbacks,
    PluginSdkIpcSessionId, PLUGIN_SDK_IPC_ENDPOINT_NAME,
};
use operit_host_native_plugin_sdk_ipc::pipes::PluginSdkPipeSessions;
use std::fs::File;
use std::os::fd::{FromRawFd, OwnedFd};
use std::sync::Mutex;
use zbus::blocking::{connection::Builder, Connection};

const BUS_NAME: &str = "org.operit.PluginSdk";
const OBJECT_PATH: &str = "/org/operit/PluginSdk";

struct Endpoint {
    sessions: PluginSdkPipeSessions,
    callbacks: PluginSdkIpcSessionCallbacks,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Arc};
    use std::time::Duration;

    /// Exercises D-Bus descriptor transfer, large frame delivery, and EOF on listener shutdown.
    #[test]
    fn dbus_pipe_session_lifecycle() {
        let instance = format!("test-{}", std::process::id());
        let server = Arc::new(LinuxPluginSdkIpcHost::withInstance(&instance));
        let (received_tx, received_rx) = mpsc::channel();
        let (closed_tx, closed_rx) = mpsc::channel();
        server
            .startListener(
                PluginSdkIpcEndpoint::standard(),
                PluginSdkIpcSessionCallbacks::new(
                    Arc::new(|_| {}),
                    Arc::new(move |_, bytes| {
                        received_tx.send(bytes).unwrap();
                    }),
                    Arc::new(|_| {}),
                ),
            )
            .unwrap();
        let client = LinuxPluginSdkIpcHost::withInstance(&instance);
        client.activate(PluginSdkIpcEndpoint::standard()).unwrap();
        let other = LinuxPluginSdkIpcHost::withInstance(&format!("{instance}-other"));
        assert!(other.activate(PluginSdkIpcEndpoint::standard()).is_err());
        assert!(other
            .connect(
                PluginSdkIpcEndpoint::standard(),
                PluginSdkIpcSessionCallbacks::new(
                    Arc::new(|_| {}),
                    Arc::new(|_, _| {}),
                    Arc::new(|_| {}),
                )
            )
            .is_err());
        let id = client
            .connect(
                PluginSdkIpcEndpoint::standard(),
                PluginSdkIpcSessionCallbacks::new(
                    Arc::new(|_| {}),
                    Arc::new(|_, _| {}),
                    Arc::new(move |_| {
                        closed_tx.send(()).unwrap();
                    }),
                ),
            )
            .unwrap();
        let payload = vec![42; 256 * 1024];
        client.send(&id, payload.clone()).unwrap();
        assert_eq!(
            received_rx.recv_timeout(Duration::from_secs(3)).unwrap(),
            payload
        );
        server.stopListener().unwrap();
        closed_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(client.send(&id, vec![1]).is_err());
    }
}

/// Creates a close-on-exec anonymous pipe owned by the returned descriptors.
fn pipe() -> std::io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [-1; 2];
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

#[zbus::interface(name = "org.operit.PluginSdk")]
impl Endpoint {
    /// Transfers the client ends of a pipe pair over the authenticated session bus.
    fn open(&self) -> zbus::fdo::Result<(zbus::zvariant::OwnedFd, zbus::zvariant::OwnedFd)> {
        let (server_read, client_write) =
            pipe().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        let (client_read, server_write) =
            pipe().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        self.sessions
            .attach(
                File::from(server_read),
                File::from(server_write),
                self.callbacks.clone(),
                true,
            )
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        Ok((client_read.into(), client_write.into()))
    }
}

/// Uses D-Bus activation and descriptor passing without an application socket listener.
pub struct LinuxPluginSdkIpcHost {
    busName: String,
    listener: Mutex<Option<Connection>>,
    sessions: PluginSdkPipeSessions,
}

impl LinuxPluginSdkIpcHost {
    /// Creates a Linux session-bus carrier.
    pub fn new() -> Self {
        Self::withInstance(&std::env::var("OPERIT_PLUGIN_SDK_INSTANCE").unwrap_or_default())
    }

    /// Gives each application profile its own D-Bus service name.
    pub fn withInstance(instance: &str) -> Self {
        Self {
            busName: operit_host_native_plugin_sdk_ipc::pluginSdkIpcInstanceName(
                BUS_NAME, instance,
            ),
            listener: Mutex::new(None),
            sessions: PluginSdkPipeSessions::default(),
        }
    }

    /// Validates the installed D-Bus service identity.
    fn validate(endpoint: &PluginSdkIpcEndpoint) -> HostResult<()> {
        if endpoint.name != PLUGIN_SDK_IPC_ENDPOINT_NAME {
            return Err(HostError::new(
                "Linux Plugin SDK requires the standard endpoint",
            ));
        }
        Ok(())
    }
}

impl Default for LinuxPluginSdkIpcHost {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginSdkIpcHost for LinuxPluginSdkIpcHost {
    /// Asks the session bus to start Operit and waits for its registered service name.
    fn activate(&self, endpoint: PluginSdkIpcEndpoint) -> HostResult<()> {
        Self::validate(&endpoint)?;
        let connection = Connection::session().map_err(|e| HostError::new(e.to_string()))?;
        if self.busName != BUS_NAME {
            let bus = zbus::blocking::fdo::DBusProxy::new(&connection)
                .map_err(|e| HostError::new(e.to_string()))?;
            let name = self
                .busName
                .as_str()
                .try_into()
                .map_err(|e: zbus::names::Error| HostError::new(e.to_string()))?;
            return if bus
                .name_has_owner(name)
                .map_err(|e| HostError::new(e.to_string()))?
            {
                Ok(())
            } else {
                Err(HostError::new(
                    "Start the selected Plugin SDK instance before connecting",
                ))
            };
        }
        let reply = connection
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "StartServiceByName",
                &(self.busName.as_str(), 0u32),
            )
            .map_err(|e| HostError::new(e.to_string()))?;
        let _: u32 = reply
            .body()
            .deserialize()
            .map_err(|e| HostError::new(e.to_string()))?;
        Ok(())
    }

    /// Exports the pipe factory before claiming the activatable service name.
    fn startListener(
        &self,
        endpoint: PluginSdkIpcEndpoint,
        callbacks: PluginSdkIpcSessionCallbacks,
    ) -> HostResult<()> {
        Self::validate(&endpoint)?;
        let mut listener = self
            .listener
            .lock()
            .map_err(|e| HostError::new(e.to_string()))?;
        if listener.is_some() {
            return Err(HostError::new("Plugin SDK listener already started"));
        }
        let connection = Builder::session()
            .map_err(|e| HostError::new(e.to_string()))?
            .serve_at(
                OBJECT_PATH,
                Endpoint {
                    sessions: self.sessions.clone(),
                    callbacks,
                },
            )
            .map_err(|e| HostError::new(e.to_string()))?
            .name(self.busName.as_str())
            .map_err(|e| HostError::new(e.to_string()))?
            .build()
            .map_err(|e| HostError::new(e.to_string()))?;
        *listener = Some(connection);
        Ok(())
    }

    /// Opens a pipe session; D-Bus activates the service and queues this call until ready.
    fn connect(
        &self,
        endpoint: PluginSdkIpcEndpoint,
        callbacks: PluginSdkIpcSessionCallbacks,
    ) -> HostResult<PluginSdkIpcSessionId> {
        Self::validate(&endpoint)?;
        let connection = Connection::session().map_err(|e| HostError::new(e.to_string()))?;
        let reply = connection
            .call_method(
                Some(self.busName.as_str()),
                OBJECT_PATH,
                Some(BUS_NAME),
                "Open",
                &(),
            )
            .map_err(|e| HostError::new(e.to_string()))?;
        let (reader, writer): (zbus::zvariant::OwnedFd, zbus::zvariant::OwnedFd) = reply
            .body()
            .deserialize()
            .map_err(|e| HostError::new(e.to_string()))?;
        self.sessions.attach(
            File::from(OwnedFd::from(reader)),
            File::from(OwnedFd::from(writer)),
            callbacks,
            false,
        )
    }

    /// Sends a frame through the session's pipe writer.
    fn send(&self, id: &PluginSdkIpcSessionId, bytes: Vec<u8>) -> HostResult<()> {
        self.sessions.send(id, bytes)
    }

    /// Ends a pipe session and propagates EOF to its peer.
    fn closeSession(&self, id: &PluginSdkIpcSessionId) -> HostResult<()> {
        self.sessions.close(id)
    }

    /// Releases the service name and all accepted sessions.
    fn stopListener(&self) -> HostResult<()> {
        let connection = self
            .listener
            .lock()
            .map_err(|e| HostError::new(e.to_string()))?
            .take()
            .ok_or_else(|| HostError::new("Plugin SDK listener is not started"))?;
        connection
            .release_name(self.busName.as_str())
            .map_err(|e| HostError::new(e.to_string()))?;
        self.sessions.closeAll()
    }
}
