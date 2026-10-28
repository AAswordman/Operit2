use jni::{
    objects::{GlobalRef, JIntArray},
    JavaVM,
};
use operit_host_api::{
    HostError, HostResult, PluginSdkIpcEndpoint, PluginSdkIpcHost, PluginSdkIpcSessionCallbacks,
    PluginSdkIpcSessionId, PLUGIN_SDK_IPC_ENDPOINT_NAME,
};
use operit_host_native_plugin_sdk_ipc::pipes::PluginSdkPipeSessions;
use std::collections::HashMap;
use std::fs::File;
use std::os::fd::FromRawFd;
use std::sync::{Arc, Mutex, OnceLock};

struct JavaBridge {
    vm: JavaVM,
    host: GlobalRef,
}

impl JavaBridge {
    /// Releases the Java binding reference associated with a native pipe session.
    fn release(&self) -> HostResult<()> {
        let mut env = crate::attachCurrentThread(&self.vm)
            .map_err(|e| HostError::new(e.to_string()))?;
        env.call_method(self.host.as_obj(), "release", "()V", &[])
            .map_err(|e| HostError::new(e.to_string()))?;
        Ok(())
    }
}
type Listener = (PluginSdkPipeSessions, PluginSdkIpcSessionCallbacks);

/// Returns the Binder client bridge registered by the embedding Android SDK.
fn bridgeSlot() -> &'static Mutex<Option<Arc<JavaBridge>>> {
    static SLOT: OnceLock<Mutex<Option<Arc<JavaBridge>>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Returns the process listener published after the Core SDK surface is ready.
fn listenerSlot() -> &'static Mutex<Option<Listener>> {
    static SLOT: OnceLock<Mutex<Option<Listener>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Registers an Android SDK client exposing activate() and open() on a Java object.
pub fn setAndroidPluginSdkBridge(vm: JavaVM, host: GlobalRef) -> HostResult<()> {
    *bridgeSlot()
        .lock()
        .map_err(|e| HostError::new(e.to_string()))? = Some(Arc::new(JavaBridge { vm, host }));
    Ok(())
}

/// Duplicates a borrowed Binder-transferred pipe descriptor with close-on-exec ownership.
fn duplicate(fd: i32) -> HostResult<File> {
    let owned = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
    if owned < 0 {
        return Err(HostError::new(std::io::Error::last_os_error().to_string()));
    }
    Ok(unsafe { File::from_raw_fd(owned) })
}

/// Accepts borrowed pipe handles from the Android Binder endpoint.
pub fn acceptAndroidPluginSdkPipes(reader: i32, writer: i32) -> HostResult<()> {
    let (sessions, callbacks) = listenerSlot()
        .lock()
        .map_err(|e| HostError::new(e.to_string()))?
        .clone()
        .ok_or_else(|| HostError::new("Android Plugin SDK listener is not ready"))?;
    sessions.attach(duplicate(reader)?, duplicate(writer)?, callbacks, true)?;
    Ok(())
}

/// Activates Operit through an empty Activity and obtains sessions through IBinder.
#[derive(Default)]
pub struct AndroidPluginSdkIpcHost {
    sessions: PluginSdkPipeSessions,
    clients: Mutex<HashMap<String, Arc<JavaBridge>>>,
}

impl AndroidPluginSdkIpcHost {
    /// Creates the Android Binder carrier.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates the endpoint and retrieves the explicitly registered Binder bridge.
    fn bridge(endpoint: &PluginSdkIpcEndpoint) -> HostResult<Arc<JavaBridge>> {
        Self::validate(endpoint)?;
        bridgeSlot()
            .lock()
            .map_err(|e| HostError::new(e.to_string()))?
            .clone()
            .ok_or_else(|| HostError::new("Android Plugin SDK Java client is not registered"))
    }

    /// Checks the application-owned Binder endpoint identity.
    fn validate(endpoint: &PluginSdkIpcEndpoint) -> HostResult<()> {
        if endpoint.name != PLUGIN_SDK_IPC_ENDPOINT_NAME {
            return Err(HostError::new(
                "Android Plugin SDK requires the standard endpoint",
            ));
        }
        Ok(())
    }
}

impl PluginSdkIpcHost for AndroidPluginSdkIpcHost {
    /// Starts the activation Activity and waits for its ready Binder result.
    fn activate(&self, endpoint: PluginSdkIpcEndpoint) -> HostResult<()> {
        let bridge = Self::bridge(&endpoint)?;
        let mut env = crate::attachCurrentThread(&bridge.vm)
            .map_err(|e| HostError::new(e.to_string()))?;
        env.call_method(bridge.host.as_obj(), "activate", "()V", &[])
            .map_err(|e| HostError::new(e.to_string()))?;
        Ok(())
    }

    /// Publishes the ready Core SDK listener to the Binder activation Activity.
    fn startListener(
        &self,
        endpoint: PluginSdkIpcEndpoint,
        callbacks: PluginSdkIpcSessionCallbacks,
    ) -> HostResult<()> {
        Self::validate(&endpoint)?;
        let mut slot = listenerSlot()
            .lock()
            .map_err(|e| HostError::new(e.to_string()))?;
        if slot.is_some() {
            return Err(HostError::new(
                "Android Plugin SDK listener already started",
            ));
        }
        *slot = Some((self.sessions.clone(), callbacks));
        Ok(())
    }

    /// Obtains owned pipe handles from the Java Binder client.
    fn connect(
        &self,
        endpoint: PluginSdkIpcEndpoint,
        callbacks: PluginSdkIpcSessionCallbacks,
    ) -> HostResult<PluginSdkIpcSessionId> {
        let bridge = Self::bridge(&endpoint)?;
        let mut env = crate::attachCurrentThread(&bridge.vm)
            .map_err(|e| HostError::new(e.to_string()))?;
        let result = env
            .call_method(bridge.host.as_obj(), "open", "()[I", &[])
            .map_err(|e| HostError::new(e.to_string()))?
            .l()
            .map_err(|e| HostError::new(e.to_string()))?;
        let array = JIntArray::from(result);
        if env
            .get_array_length(&array)
            .map_err(|e| HostError::new(e.to_string()))?
            != 2
        {
            return Err(HostError::new("Binder open requires two pipe descriptors"));
        }
        let mut fds = [-1; 2];
        env.get_int_array_region(&array, 0, &mut fds)
            .map_err(|e| HostError::new(e.to_string()))?;
        if fds[0] < 0 || fds[1] < 0 {
            return Err(HostError::new("Binder returned invalid pipe descriptors"));
        }
        let result = self.sessions.attach(
            unsafe { File::from_raw_fd(fds[0]) },
            unsafe { File::from_raw_fd(fds[1]) },
            callbacks,
            false,
        );
        match result {
            Ok(id) => {
                self.clients
                    .lock()
                    .map_err(|e| HostError::new(e.to_string()))?
                    .insert(id.0.clone(), bridge.clone());
                Ok(id)
            }
            Err(error) => {
                bridge.release()?;
                Err(error)
            }
        }
    }

    /// Sends one Link frame over the Binder-transferred pipe.
    fn send(&self, id: &PluginSdkIpcSessionId, bytes: Vec<u8>) -> HostResult<()> {
        self.sessions.send(id, bytes)
    }

    /// Closes a Binder-established data session.
    fn closeSession(&self, id: &PluginSdkIpcSessionId) -> HostResult<()> {
        let result = self.sessions.close(id);
        let client = self
            .clients
            .lock()
            .map_err(|e| HostError::new(e.to_string()))?
            .remove(&id.0);
        if let Some(client) = client {
            client.release()?;
        }
        result
    }

    /// Unpublishes the Binder endpoint and closes its pipe sessions.
    fn stopListener(&self) -> HostResult<()> {
        listenerSlot()
            .lock()
            .map_err(|e| HostError::new(e.to_string()))?
            .take()
            .ok_or_else(|| HostError::new("Android Plugin SDK listener is not started"))?;
        self.sessions.closeAll()
    }
}
