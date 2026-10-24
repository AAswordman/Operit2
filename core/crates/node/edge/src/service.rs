#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{UnboundedReceiver as Receiver, UnboundedSender as Sender};

use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    DeviceDigitalOutputRequest, DeviceDigitalOutputState, RobotFaceExpressionRequest,
    RobotFaceState,
};

/// Describes a failure raised by one typed Edge Service operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeServiceError {
    pub message: String,
}

impl EdgeServiceError {
    /// Creates a typed Edge Service error from a displayable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Carries typed digital-output states from one Edge Service watch.
pub struct DeviceIoStateStream {
    receiver: Option<Receiver<DeviceDigitalOutputState>>,
    close: DeviceIoStateStreamClose,
}

/// Owns the idempotent cancellation callback for one typed state stream.
#[derive(Clone)]
pub(crate) struct DeviceIoStateStreamClose {
    callback: Arc<Mutex<Option<Box<dyn FnOnce() + Send + 'static>>>>,
}

impl DeviceIoStateStreamClose {
    /// Creates a close handle without a cancellation callback.
    fn empty() -> Self {
        Self {
            callback: Arc::new(Mutex::new(None)),
        }
    }

    /// Creates a close handle that invokes one cancellation callback once.
    fn new(onClose: impl FnOnce() + Send + 'static) -> Self {
        Self {
            callback: Arc::new(Mutex::new(Some(Box::new(onClose)))),
        }
    }

    /// Cancels the associated typed state subscription exactly once.
    pub(crate) fn close(&self) {
        let onClose = self
            .callback
            .lock()
            .expect("device I/O stream close lock must not be poisoned")
            .take();
        if let Some(onClose) = onClose {
            onClose();
        }
    }
}

impl DeviceIoStateStream {
    /// Creates a typed state stream from its service-owned receiver.
    pub fn new(receiver: Receiver<DeviceDigitalOutputState>) -> Self {
        Self {
            receiver: Some(receiver),
            close: DeviceIoStateStreamClose::empty(),
        }
    }

    /// Creates a typed state stream with a callback that unregisters its subscription.
    fn withOnClose(
        receiver: Receiver<DeviceDigitalOutputState>,
        onClose: impl FnOnce() + Send + 'static,
    ) -> Self {
        Self {
            receiver: Some(receiver),
            close: DeviceIoStateStreamClose::new(onClose),
        }
    }

    /// Returns a handle that can cancel this stream from its Link adapter.
    pub(crate) fn closeHandle(&self) -> DeviceIoStateStreamClose {
        self.close.clone()
    }

    /// Transfers the typed receiver to Link while retaining subscription cancellation.
    pub(crate) fn intoEventStream(
        mut self,
        map: impl FnMut(DeviceDigitalOutputState) -> Option<operit_link::CoreEvent> + Send + 'static,
    ) -> operit_link::CoreEventStream {
        let receiver = self
            .receiver
            .take()
            .expect("typed stream receiver must be owned");
        operit_link::CoreEventStream::fromMappedReceiver(receiver, map)
            .withOnClose(move || drop(self))
    }
}

impl Drop for DeviceIoStateStream {
    /// Cancels the service subscription when the typed stream is released.
    fn drop(&mut self) {
        self.close.close();
    }
}

/// Carries typed robot face states from one Edge Service watch.
pub struct RobotFaceStateStream {
    receiver: Option<Receiver<RobotFaceState>>,
    close: RobotFaceStateStreamClose,
}

/// Owns the idempotent cancellation callback for one robot face stream.
#[derive(Clone)]
pub(crate) struct RobotFaceStateStreamClose {
    callback: Arc<Mutex<Option<Box<dyn FnOnce() + Send + 'static>>>>,
}

impl RobotFaceStateStreamClose {
    /// Creates a robot face close handle without a cancellation callback.
    fn empty() -> Self {
        Self {
            callback: Arc::new(Mutex::new(None)),
        }
    }

    /// Creates a robot face close handle that invokes one callback once.
    fn new(onClose: impl FnOnce() + Send + 'static) -> Self {
        Self {
            callback: Arc::new(Mutex::new(Some(Box::new(onClose)))),
        }
    }

    /// Cancels the associated robot face subscription exactly once.
    pub(crate) fn close(&self) {
        let onClose = self
            .callback
            .lock()
            .expect("robot face stream close lock must not be poisoned")
            .take();
        if let Some(onClose) = onClose {
            onClose();
        }
    }
}

impl RobotFaceStateStream {
    /// Creates a typed robot face stream from its service-owned receiver.
    pub fn new(receiver: Receiver<RobotFaceState>) -> Self {
        Self {
            receiver: Some(receiver),
            close: RobotFaceStateStreamClose::empty(),
        }
    }

    /// Creates a typed robot face stream with a subscription close callback.
    fn withOnClose(
        receiver: Receiver<RobotFaceState>,
        onClose: impl FnOnce() + Send + 'static,
    ) -> Self {
        Self {
            receiver: Some(receiver),
            close: RobotFaceStateStreamClose::new(onClose),
        }
    }

    /// Returns a handle that can cancel this stream from its Link adapter.
    pub(crate) fn closeHandle(&self) -> RobotFaceStateStreamClose {
        self.close.clone()
    }

    /// Transfers robot face updates to Link without a forwarding task.
    pub(crate) fn intoEventStream(
        mut self,
        map: impl FnMut(RobotFaceState) -> Option<operit_link::CoreEvent> + Send + 'static,
    ) -> operit_link::CoreEventStream {
        let receiver = self
            .receiver
            .take()
            .expect("typed stream receiver must be owned");
        operit_link::CoreEventStream::fromMappedReceiver(receiver, map)
            .withOnClose(move || drop(self))
    }
}

impl Drop for RobotFaceStateStream {
    /// Cancels the robot face subscription when the typed stream is released.
    fn drop(&mut self) {
        self.close.close();
    }
}

/// Defines typed device I/O operations owned by the Edge Service layer.
pub trait DeviceIoService: Send + Sync {
    /// Writes one digital output through the configured Host API.
    fn setDigitalOutput(
        &self,
        request: DeviceDigitalOutputRequest,
    ) -> Result<DeviceDigitalOutputState, EdgeServiceError>;

    /// Reads one digital output through the configured Host API.
    fn getDigitalOutput(&self, pin: u8) -> Result<DeviceDigitalOutputState, EdgeServiceError>;

    /// Opens a typed state stream for one digital output.
    fn watchDigitalOutput(&self, pin: u8) -> Result<DeviceIoStateStream, EdgeServiceError>;
}

/// Defines typed robot face operations owned by the Edge Service layer.
pub trait RobotFaceService: Send + Sync {
    /// Writes one robot expression through the configured Host API.
    fn setExpression(
        &self,
        request: RobotFaceExpressionRequest,
    ) -> Result<RobotFaceState, EdgeServiceError>;

    /// Reads the current robot expression through the configured Host API.
    fn getExpression(&self) -> Result<RobotFaceState, EdgeServiceError>;

    /// Opens a typed state stream for robot face expression changes.
    fn watchExpression(&self) -> Result<RobotFaceStateStream, EdgeServiceError>;
}

/// Implements typed device I/O operations over Host API capabilities.
pub struct HostDeviceIoService {
    hostManager: HostManager,
    watchers: Arc<Mutex<BTreeMap<u8, BTreeMap<u64, Sender<DeviceDigitalOutputState>>>>>,
    nextWatcherId: AtomicU32,
}

impl HostDeviceIoService {
    /// Creates a device I/O service over one host capability set.
    pub fn new(hostManager: HostManager) -> Self {
        Self {
            hostManager,
            watchers: Arc::new(Mutex::new(BTreeMap::new())),
            nextWatcherId: AtomicU32::new(1),
        }
    }

    /// Publishes one committed output state to active typed subscribers.
    fn publishState(&self, state: DeviceDigitalOutputState) -> Result<(), EdgeServiceError> {
        let mut watchers = self.watchers.lock().map_err(|error| {
            EdgeServiceError::new(format!("device I/O watcher lock poisoned: {error}"))
        })?;
        if let Some(senders) = watchers.get_mut(&state.pin) {
            senders.retain(|_, sender| sender.send(state.clone()).is_ok());
            if senders.is_empty() {
                watchers.remove(&state.pin);
            }
        }
        Ok(())
    }

    /// Removes one typed state subscription after its stream is dropped.
    fn removeWatcher(
        watchers: &mut BTreeMap<u8, BTreeMap<u64, Sender<DeviceDigitalOutputState>>>,
        pin: u8,
        watcherId: u64,
    ) {
        if let Some(senders) = watchers.get_mut(&pin) {
            senders.remove(&watcherId);
            if senders.is_empty() {
                watchers.remove(&pin);
            }
        }
    }
}

impl DeviceIoService for HostDeviceIoService {
    /// Writes one digital output and publishes its committed state.
    fn setDigitalOutput(
        &self,
        request: DeviceDigitalOutputRequest,
    ) -> Result<DeviceDigitalOutputState, EdgeServiceError> {
        let host = self
            .hostManager
            .deviceIoHost
            .as_ref()
            .ok_or_else(|| EdgeServiceError::new("device I/O Host API is not installed"))?;
        let state = host
            .setDigitalOutput(request)
            .map_err(|error| EdgeServiceError::new(error.message))?;
        self.publishState(state.clone())?;
        Ok(state)
    }

    /// Reads one digital output from the configured Host API.
    fn getDigitalOutput(&self, pin: u8) -> Result<DeviceDigitalOutputState, EdgeServiceError> {
        self.hostManager
            .deviceIoHost
            .as_ref()
            .ok_or_else(|| EdgeServiceError::new("device I/O Host API is not installed"))?
            .getDigitalOutput(pin)
            .map_err(|error| EdgeServiceError::new(error.message))
    }

    /// Opens a typed digital-output state stream with an initial snapshot.
    fn watchDigitalOutput(&self, pin: u8) -> Result<DeviceIoStateStream, EdgeServiceError> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut watchers = self
            .watchers
            .lock()
            .map_err(|error| EdgeServiceError::new(error.to_string()))?;
        let state = self.getDigitalOutput(pin)?;
        sender
            .send(state)
            .map_err(|error| EdgeServiceError::new(error.to_string()))?;
        let watcherId = u64::from(self.nextWatcherId.fetch_add(1, Ordering::Relaxed));
        watchers.entry(pin).or_default().insert(watcherId, sender);
        let watchersForClose = Arc::clone(&self.watchers);
        Ok(DeviceIoStateStream::withOnClose(receiver, move || {
            let mut watchers = watchersForClose
                .lock()
                .expect("device I/O watcher lock must not be poisoned during close");
            HostDeviceIoService::removeWatcher(&mut watchers, pin, watcherId);
        }))
    }
}

/// Builds the device I/O Service used by an Edge Node.
pub fn createDeviceIoService(hostManager: HostManager) -> Arc<dyn DeviceIoService> {
    Arc::new(HostDeviceIoService::new(hostManager))
}

/// Implements typed robot face operations over Host API capabilities.
pub struct HostRobotFaceService {
    hostManager: HostManager,
    watchers: Arc<Mutex<BTreeMap<u64, Sender<RobotFaceState>>>>,
    nextWatcherId: AtomicU32,
}

impl HostRobotFaceService {
    /// Creates a robot face service over one host capability set.
    pub fn new(hostManager: HostManager) -> Self {
        Self {
            hostManager,
            watchers: Arc::new(Mutex::new(BTreeMap::new())),
            nextWatcherId: AtomicU32::new(1),
        }
    }

    /// Publishes one committed robot face state to active typed subscribers.
    fn publishState(&self, state: RobotFaceState) -> Result<(), EdgeServiceError> {
        let mut watchers = self.watchers.lock().map_err(|error| {
            EdgeServiceError::new(format!("robot face watcher lock poisoned: {error}"))
        })?;
        watchers.retain(|_, sender| sender.send(state.clone()).is_ok());
        Ok(())
    }

    /// Removes one robot face subscription after its stream is dropped.
    fn removeWatcher(watchers: &mut BTreeMap<u64, Sender<RobotFaceState>>, watcherId: u64) {
        watchers.remove(&watcherId);
    }
}

impl RobotFaceService for HostRobotFaceService {
    /// Writes one robot expression and publishes its committed state.
    fn setExpression(
        &self,
        request: RobotFaceExpressionRequest,
    ) -> Result<RobotFaceState, EdgeServiceError> {
        let host = self
            .hostManager
            .robotFaceHost
            .as_ref()
            .ok_or_else(|| EdgeServiceError::new("robot face Host API is not installed"))?;
        let state = host
            .setExpression(request)
            .map_err(|error| EdgeServiceError::new(error.message))?;
        self.publishState(state.clone())?;
        Ok(state)
    }

    /// Reads the robot face state from the configured Host API.
    fn getExpression(&self) -> Result<RobotFaceState, EdgeServiceError> {
        self.hostManager
            .robotFaceHost
            .as_ref()
            .ok_or_else(|| EdgeServiceError::new("robot face Host API is not installed"))?
            .getExpression()
            .map_err(|error| EdgeServiceError::new(error.message))
    }

    /// Opens a typed robot face state stream with an initial snapshot.
    fn watchExpression(&self) -> Result<RobotFaceStateStream, EdgeServiceError> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut watchers = self
            .watchers
            .lock()
            .map_err(|error| EdgeServiceError::new(error.to_string()))?;
        let state = self.getExpression()?;
        sender
            .send(state)
            .map_err(|error| EdgeServiceError::new(error.to_string()))?;
        let watcherId = u64::from(self.nextWatcherId.fetch_add(1, Ordering::Relaxed));
        watchers.insert(watcherId, sender);
        let watchersForClose = Arc::clone(&self.watchers);
        Ok(RobotFaceStateStream::withOnClose(receiver, move || {
            let mut watchers = watchersForClose
                .lock()
                .expect("robot face watcher lock must not be poisoned during close");
            HostRobotFaceService::removeWatcher(&mut watchers, watcherId);
        }))
    }
}

/// Builds the robot face Service used by board-backed Edge Nodes.
pub fn createRobotFaceService(hostManager: HostManager) -> Arc<dyn RobotFaceService> {
    Arc::new(HostRobotFaceService::new(hostManager))
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::{DeviceIoHost, HostResult, RobotFaceHost};

    /// Provides a deterministic GPIO host for Service subscription tests.
    struct TestDeviceIoHost;

    impl DeviceIoHost for TestDeviceIoHost {
        /// Returns the requested output as the committed test state.
        fn setDigitalOutput(
            &self,
            request: DeviceDigitalOutputRequest,
        ) -> HostResult<DeviceDigitalOutputState> {
            Ok(DeviceDigitalOutputState {
                pin: request.pin,
                level: request.level,
            })
        }

        /// Returns a low state for the requested test output.
        fn getDigitalOutput(&self, pin: u8) -> HostResult<DeviceDigitalOutputState> {
            Ok(DeviceDigitalOutputState { pin, level: false })
        }
    }

    /// Provides a deterministic robot face host for Service subscription tests.
    struct TestRobotFaceHost;

    impl RobotFaceHost for TestRobotFaceHost {
        /// Returns the requested expression as the committed test state.
        fn setExpression(&self, request: RobotFaceExpressionRequest) -> HostResult<RobotFaceState> {
            Ok(RobotFaceState {
                expression: request.expression,
            })
        }

        /// Returns a deterministic neutral expression state.
        fn getExpression(&self) -> HostResult<RobotFaceState> {
            Ok(RobotFaceState {
                expression: "neutral".to_string(),
            })
        }
    }

    /// Verifies dropping a typed stream unregisters its Service subscription.
    #[test]
    fn droppingStateStreamRemovesWatcher() {
        let hostManager = HostManager::new().withDeviceIoHost(Arc::new(TestDeviceIoHost));
        let service = HostDeviceIoService::new(hostManager);
        let stream = service
            .watchDigitalOutput(2)
            .expect("test state stream must open");

        assert_eq!(
            service
                .watchers
                .lock()
                .expect("test watcher lock must be usable")
                .get(&2)
                .map(BTreeMap::len),
            Some(1)
        );

        drop(stream);

        assert!(service
            .watchers
            .lock()
            .expect("test watcher lock must be usable")
            .get(&2)
            .is_none());
    }

    /// Verifies dropping a robot face stream unregisters its Service subscription.
    #[test]
    fn droppingRobotFaceStateStreamRemovesWatcher() {
        let hostManager = HostManager::new().withRobotFaceHost(Arc::new(TestRobotFaceHost));
        let service = HostRobotFaceService::new(hostManager);
        let stream = service
            .watchExpression()
            .expect("test robot face stream must open");

        assert_eq!(
            service
                .watchers
                .lock()
                .expect("test robot face watcher lock must be usable")
                .len(),
            1
        );

        drop(stream);

        assert!(service
            .watchers
            .lock()
            .expect("test robot face watcher lock must be usable")
            .is_empty());
    }
}
