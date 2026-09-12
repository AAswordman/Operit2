#![allow(non_snake_case)]

use std::sync::Arc;

use async_trait::async_trait;
pub use operit_edge_contract::{
    EDGE_DEVICE_IO_OBJECT_ID, EDGE_DEVICE_IO_STATE_PROPERTY, EDGE_ROBOT_FACE_OBJECT_ID,
    EDGE_ROBOT_FACE_STATE_PROPERTY,
};
use operit_host_api::HostManager::HostManager;
use operit_host_api::RobotFaceExpressionRequest;
use operit_link::{
    toCoreValue, CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream,
    CoreLinkError, CoreLinkSharedClient, CoreValue, CoreWatchRequest,
};
use serde::Deserialize;

pub mod service;

pub use service::{
    createDeviceIoService, createRobotFaceService, DeviceIoService, DeviceIoStateStream,
    EdgeServiceError, HostDeviceIoService, HostRobotFaceService, RobotFaceService,
    RobotFaceStateStream,
};

/// Owns the lightweight device-side Core services for one Edge Node.
pub struct EdgeNode {
    deviceIoService: Arc<dyn DeviceIoService>,
    robotFaceService: Option<Arc<dyn RobotFaceService>>,
}

impl EdgeNode {
    /// Creates an Edge Node from its typed device service registry.
    pub fn new(deviceIoService: Arc<dyn DeviceIoService>) -> Self {
        Self {
            deviceIoService,
            robotFaceService: None,
        }
    }

    /// Creates an Edge Node with device services supplied by one Host Manager.
    pub fn fromHostManager(hostManager: HostManager) -> Self {
        Self::new(createDeviceIoService(hostManager))
    }

    /// Registers a typed robot face service with this Edge Node.
    pub fn withRobotFaceService(mut self, robotFaceService: Arc<dyn RobotFaceService>) -> Self {
        self.robotFaceService = Some(robotFaceService);
        self
    }

    /// Dispatches one Link call to the registered Edge Service.
    pub(crate) fn dispatchCall(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let result = match request.targetObjectId {
            EDGE_DEVICE_IO_OBJECT_ID => match request.methodName.as_str() {
                "setDigitalOutput" => self.setDigitalOutput(request.args),
                "getDigitalOutput" => self.getDigitalOutput(request.args),
                _ => Err(CoreLinkError::methodNotFound(&request.registryKey())),
            },
            EDGE_ROBOT_FACE_OBJECT_ID => match request.methodName.as_str() {
                "setExpression" => self.setExpression(request.args),
                "getExpression" => self.getExpression(),
                _ => Err(CoreLinkError::methodNotFound(&request.registryKey())),
            },
            _ => Err(CoreLinkError::methodNotFound(&request.registryKey())),
        };
        match result {
            Ok(value) => CoreCallResponse::ok(requestId, value),
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    /// Reads one Link watch snapshot from the registered Edge Service.
    pub(crate) fn dispatchWatchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        match request.targetObjectId {
            EDGE_DEVICE_IO_OBJECT_ID => {
                self.validateDeviceIoWatch(&request)?;
                let pin = decodePin(request.args)?;
                let state = self
                    .deviceIoService
                    .getDigitalOutput(pin)
                    .map_err(serviceError)?;
                Ok(CoreEvent {
                    requestId: Some(request.requestId),
                    targetObjectId: request.targetObjectId,
                    propertyName: request.propertyName,
                    kind: CoreEventKind::Snapshot,
                    value: toCoreValue(state)
                        .map_err(|error| CoreLinkError::internal(error.to_string()))?,
                })
            }
            EDGE_ROBOT_FACE_OBJECT_ID => {
                self.validateRobotFaceWatch(&request)?;
                let state = self
                    .robotFaceService(&request.registryKey())?
                    .getExpression()
                    .map_err(serviceError)?;
                Ok(CoreEvent {
                    requestId: Some(request.requestId),
                    targetObjectId: request.targetObjectId,
                    propertyName: request.propertyName,
                    kind: CoreEventKind::Snapshot,
                    value: toCoreValue(state)
                        .map_err(|error| CoreLinkError::internal(error.to_string()))?,
                })
            }
            _ => Err(CoreLinkError::watchNotFound(&request.registryKey())),
        }
    }

    /// Opens one Link watch backed by a typed Edge Service state stream.
    pub(crate) fn dispatchWatch(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        match request.targetObjectId {
            EDGE_DEVICE_IO_OBJECT_ID => self.dispatchDeviceIoWatch(request),
            EDGE_ROBOT_FACE_OBJECT_ID => self.dispatchRobotFaceWatch(request),
            _ => Err(CoreLinkError::watchNotFound(&request.registryKey())),
        }
    }

    /// Opens one Link watch backed by a typed device I/O state stream.
    fn dispatchDeviceIoWatch(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        self.validateDeviceIoWatch(&request)?;
        let pin = decodePin(request.args)?;
        let source = self
            .deviceIoService
            .watchDigitalOutput(pin)
            .map_err(serviceError)?;
        let sourceClose = source.closeHandle();
        let requestId = request.requestId;
        let targetObjectId = request.targetObjectId;
        let propertyName = request.propertyName;
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        std::thread::spawn(move || {
            let mut kind = CoreEventKind::Snapshot;
            while let Ok(state) = source.recv() {
                let value = match toCoreValue(state) {
                    Ok(value) => value,
                    Err(_) => break,
                };
                if sender
                    .send(CoreEvent {
                        requestId: Some(requestId.clone()),
                        targetObjectId,
                        propertyName: propertyName.clone(),
                        kind: kind.clone(),
                        value,
                    })
                    .is_err()
                {
                    break;
                }
                kind = CoreEventKind::Changed;
            }
        });
        Ok(CoreEventStream::new(receiver).withOnClose(move || {
            sourceClose.close();
        }))
    }

    /// Validates the Edge device watch address and property name.
    fn validateDeviceIoWatch(&self, request: &CoreWatchRequest) -> Result<(), CoreLinkError> {
        if request.targetObjectId != EDGE_DEVICE_IO_OBJECT_ID
            || request.propertyName != EDGE_DEVICE_IO_STATE_PROPERTY
        {
            return Err(CoreLinkError::watchNotFound(&request.registryKey()));
        }
        Ok(())
    }

    /// Opens one Link watch backed by a typed robot face state stream.
    fn dispatchRobotFaceWatch(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        self.validateRobotFaceWatch(&request)?;
        let source = self
            .robotFaceService(&request.registryKey())?
            .watchExpression()
            .map_err(serviceError)?;
        let sourceClose = source.closeHandle();
        let requestId = request.requestId;
        let targetObjectId = request.targetObjectId;
        let propertyName = request.propertyName;
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        std::thread::spawn(move || {
            let mut kind = CoreEventKind::Snapshot;
            while let Ok(state) = source.recv() {
                let value = match toCoreValue(state) {
                    Ok(value) => value,
                    Err(_) => break,
                };
                if sender
                    .send(CoreEvent {
                        requestId: Some(requestId.clone()),
                        targetObjectId,
                        propertyName: propertyName.clone(),
                        kind: kind.clone(),
                        value,
                    })
                    .is_err()
                {
                    break;
                }
                kind = CoreEventKind::Changed;
            }
        });
        Ok(CoreEventStream::new(receiver).withOnClose(move || {
            sourceClose.close();
        }))
    }

    /// Validates the Edge robot face watch address and property name.
    fn validateRobotFaceWatch(&self, request: &CoreWatchRequest) -> Result<(), CoreLinkError> {
        if request.targetObjectId != EDGE_ROBOT_FACE_OBJECT_ID
            || request.propertyName != EDGE_ROBOT_FACE_STATE_PROPERTY
        {
            return Err(CoreLinkError::watchNotFound(&request.registryKey()));
        }
        Ok(())
    }

    /// Returns the registered robot face service for one routed request.
    fn robotFaceService(
        &self,
        registryKey: &str,
    ) -> Result<&Arc<dyn RobotFaceService>, CoreLinkError> {
        self.robotFaceService
            .as_ref()
            .ok_or_else(|| CoreLinkError::methodNotFound(registryKey))
    }

    /// Decodes and executes one typed digital-output write.
    fn setDigitalOutput(&self, args: CoreValue) -> Result<CoreValue, CoreLinkError> {
        let request: operit_host_api::DeviceDigitalOutputRequest = decodeValue(args)?;
        let state = self
            .deviceIoService
            .setDigitalOutput(request)
            .map_err(serviceError)?;
        encodeValue(state)
    }

    /// Decodes and executes one typed digital-output read.
    fn getDigitalOutput(&self, args: CoreValue) -> Result<CoreValue, CoreLinkError> {
        let request: PinRequest = decodeValue(args)?;
        let state = self
            .deviceIoService
            .getDigitalOutput(request.pin)
            .map_err(serviceError)?;
        encodeValue(state)
    }

    /// Decodes and executes one typed robot face expression write.
    fn setExpression(&self, args: CoreValue) -> Result<CoreValue, CoreLinkError> {
        let request: RobotFaceExpressionRequest = decodeValue(args)?;
        let state = self
            .robotFaceService("robot-face.setExpression")?
            .setExpression(request)
            .map_err(serviceError)?;
        encodeValue(state)
    }

    /// Executes one typed robot face expression read.
    fn getExpression(&self) -> Result<CoreValue, CoreLinkError> {
        let state = self
            .robotFaceService("robot-face.getExpression")?
            .getExpression()
            .map_err(serviceError)?;
        encodeValue(state)
    }
}

#[async_trait(?Send)]
impl CoreLinkSharedClient for EdgeNode {
    /// Dispatches one local Edge Core call through the shared Link interface.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        self.dispatchCall(request)
    }

    /// Reads one local Edge Core watch snapshot through the shared Link interface.
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        self.dispatchWatchSnapshot(request)
    }

    /// Opens one local Edge Core watch through the shared Link interface.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.dispatchWatch(request)
    }
}

/// Carries the pin selected by a typed read or watch request.
#[derive(Clone, Debug, Deserialize)]
struct PinRequest {
    pin: u8,
}

/// Decodes one Link value into a typed Edge Service request.
fn decodeValue<T>(value: CoreValue) -> Result<T, CoreLinkError>
where
    T: for<'de> Deserialize<'de>,
{
    operit_link::fromCoreValue(value)
        .map_err(|error| CoreLinkError::new("INVALID_ARGS", error.to_string()))
}

/// Encodes one typed Edge Service result into a Link value.
fn encodeValue<T>(value: T) -> Result<CoreValue, CoreLinkError>
where
    T: serde::Serialize,
{
    toCoreValue(value).map_err(|error| CoreLinkError::internal(error.to_string()))
}

/// Decodes the pin field shared by digital-output watch requests.
fn decodePin(args: CoreValue) -> Result<u8, CoreLinkError> {
    Ok(decodeValue::<PinRequest>(args)?.pin)
}

/// Converts one typed service failure into a Link error at the node boundary.
fn serviceError(error: EdgeServiceError) -> CoreLinkError {
    CoreLinkError::internal(error.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::{DeviceIoService, RobotFaceService, RobotFaceStateStream};
    use operit_host_api::RobotFaceState;

    /// Provides a deterministic typed service for Edge Node tests.
    struct TestDeviceIoService;

    impl DeviceIoService for TestDeviceIoService {
        /// Returns the requested output state unchanged.
        fn setDigitalOutput(
            &self,
            request: operit_host_api::DeviceDigitalOutputRequest,
        ) -> Result<operit_host_api::DeviceDigitalOutputState, EdgeServiceError> {
            Ok(operit_host_api::DeviceDigitalOutputState {
                pin: request.pin,
                level: request.level,
            })
        }

        /// Returns a deterministic low output state.
        fn getDigitalOutput(
            &self,
            pin: u8,
        ) -> Result<operit_host_api::DeviceDigitalOutputState, EdgeServiceError> {
            Ok(operit_host_api::DeviceDigitalOutputState { pin, level: false })
        }

        /// Opens a typed stream with one deterministic snapshot.
        fn watchDigitalOutput(&self, pin: u8) -> Result<DeviceIoStateStream, EdgeServiceError> {
            let (sender, receiver) = std::sync::mpsc::channel();
            sender
                .send(operit_host_api::DeviceDigitalOutputState { pin, level: false })
                .expect("test state receiver must be alive");
            Ok(DeviceIoStateStream::new(receiver))
        }
    }

    /// Provides a deterministic typed robot face service for Edge Node tests.
    struct TestRobotFaceService;

    impl RobotFaceService for TestRobotFaceService {
        /// Returns the requested expression as the committed state.
        fn setExpression(
            &self,
            request: RobotFaceExpressionRequest,
        ) -> Result<RobotFaceState, EdgeServiceError> {
            Ok(RobotFaceState {
                expression: request.expression,
            })
        }

        /// Returns a deterministic neutral expression state.
        fn getExpression(&self) -> Result<RobotFaceState, EdgeServiceError> {
            Ok(RobotFaceState {
                expression: "neutral".to_string(),
            })
        }

        /// Opens a typed robot face stream with one deterministic snapshot.
        fn watchExpression(&self) -> Result<RobotFaceStateStream, EdgeServiceError> {
            let (sender, receiver) = std::sync::mpsc::channel();
            sender
                .send(RobotFaceState {
                    expression: "neutral".to_string(),
                })
                .expect("test robot face receiver must be alive");
            Ok(RobotFaceStateStream::new(receiver))
        }
    }

    /// Builds one Edge Node around the deterministic test service.
    fn testNode() -> EdgeNode {
        EdgeNode::new(Arc::new(TestDeviceIoService))
    }

    /// Builds one Edge Node around the deterministic robot face test service.
    fn testNodeWithRobotFace() -> EdgeNode {
        testNode().withRobotFaceService(Arc::new(TestRobotFaceService))
    }

    /// Verifies typed writes cross the node boundary and preserve their value.
    #[test]
    fn dispatchesTypedDeviceWrite() {
        let node = testNode();
        let response = node.dispatchCall(CoreCallRequest::new(
            "write-1",
            EDGE_DEVICE_IO_OBJECT_ID,
            "setDigitalOutput",
            CoreValue::Map(std::collections::BTreeMap::from([
                ("pin".to_string(), CoreValue::Unsigned(2)),
                ("level".to_string(), CoreValue::Bool(true)),
            ])),
        ));
        let value = response.result.expect("typed device write must succeed");
        let state: operit_host_api::DeviceDigitalOutputState =
            operit_link::fromCoreValue(value).expect("device state must decode");
        assert_eq!(state.pin, 2);
        assert!(state.level);
    }

    /// Verifies an Edge watch emits its initial typed service snapshot.
    #[test]
    fn dispatchesTypedDeviceWatch() {
        let node = testNode();
        let mut stream = node
            .dispatchWatch(CoreWatchRequest::new(
                "watch-1",
                EDGE_DEVICE_IO_OBJECT_ID,
                "digitalOutputState",
                CoreValue::Map(std::collections::BTreeMap::from([(
                    "pin".to_string(),
                    CoreValue::Unsigned(2),
                )])),
            ))
            .expect("typed device watch must open");
        let event = loop {
            match stream.try_recv() {
                Ok(event) => break event,
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    std::thread::yield_now();
                }
                Err(error) => panic!("typed watch ended unexpectedly: {error}"),
            }
        };
        assert_eq!(event.kind, CoreEventKind::Snapshot);
        let state: operit_host_api::DeviceDigitalOutputState =
            operit_link::fromCoreValue(event.value).expect("watch state must decode");
        assert_eq!(state.pin, 2);
        assert!(!state.level);
    }

    /// Verifies typed robot face writes cross the node boundary and preserve their value.
    #[test]
    fn dispatchesTypedRobotFaceWrite() {
        let node = testNodeWithRobotFace();
        let response = node.dispatchCall(CoreCallRequest::new(
            "face-write-1",
            EDGE_ROBOT_FACE_OBJECT_ID,
            "setExpression",
            CoreValue::Map(std::collections::BTreeMap::from([(
                "expression".to_string(),
                CoreValue::String("happy".to_string()),
            )])),
        ));
        let value = response
            .result
            .expect("typed robot face write must succeed");
        let state: RobotFaceState =
            operit_link::fromCoreValue(value).expect("robot face state must decode");
        assert_eq!(state.expression, "happy");
    }

    /// Verifies an Edge robot face watch emits its initial typed service snapshot.
    #[test]
    fn dispatchesTypedRobotFaceWatch() {
        let node = testNodeWithRobotFace();
        let mut stream = node
            .dispatchWatch(CoreWatchRequest::new(
                "face-watch-1",
                EDGE_ROBOT_FACE_OBJECT_ID,
                EDGE_ROBOT_FACE_STATE_PROPERTY,
                CoreValue::emptyMap(),
            ))
            .expect("typed robot face watch must open");
        let event = loop {
            match stream.try_recv() {
                Ok(event) => break event,
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    std::thread::yield_now();
                }
                Err(error) => panic!("typed robot face watch ended unexpectedly: {error}"),
            }
        };
        assert_eq!(event.kind, CoreEventKind::Snapshot);
        let state: RobotFaceState =
            operit_link::fromCoreValue(event.value).expect("robot face state must decode");
        assert_eq!(state.expression, "neutral");
    }
}
