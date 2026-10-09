#![allow(non_snake_case)]

pub mod PeerRouter;

use std::sync::Arc;
use operit_node_runtime::NodeServices::NodeServices;

use async_trait::async_trait;
pub use operit_edge_contract::{
    EDGE_DEVICE_IO_OBJECT_ID, EDGE_DEVICE_IO_STATE_PROPERTY, EDGE_ROBOT_FACE_OBJECT_ID,
    EDGE_ROBOT_FACE_STATE_PROPERTY, EDGE_SCREEN_OBJECT_ID, EDGE_SCREEN_STATE_PROPERTY,
    EDGE_PLUGIN_TARGET,
};
use operit_host_api::HostManager::HostManager;
use operit_host_api::RobotFaceExpressionRequest;
use operit_link::{
    toCoreValue, CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream,
    CoreLinkError, CoreLinkSharedClient, CoreValue, CoreWatchRequest,
};
use serde::Deserialize;

pub mod service;
pub mod plugin;
pub mod scene;
pub mod events;

pub use plugin::EdgePlugin;
pub use operit_edge_contract::{EdgePluginManifest, EdgeScreenSnapshot, EdgeScreenInputRequest, EdgeScreenInputState};

pub use service::{
    createDeviceIoService, createRobotFaceService, DeviceIoService, DeviceIoStateStream,
    EdgeServiceError, HostDeviceIoService, HostRobotFaceService, RobotFaceService,
    RobotFaceStateStream,
};

/// Defines the minimal display operations shared by small embedded targets.
pub trait ScreenService: Send + Sync {
    fn getScreenSnapshot(&self) -> Result<EdgeScreenSnapshot, EdgeServiceError>;
    fn sendScreenInput(
        &self,
        request: EdgeScreenInputRequest,
    ) -> Result<EdgeScreenInputState, EdgeServiceError>;
}

/// Owns the lightweight device-side Core services for one Edge Node.
#[derive(Clone)]
pub struct EdgeNode {
    deviceIoService: Arc<dyn DeviceIoService>,
    robotFaceService: Option<Arc<dyn RobotFaceService>>,
    screenService: Option<Arc<dyn ScreenService>>,
    plugins: Vec<Arc<dyn EdgePlugin>>,
    nodeServices: Option<NodeServices>,
}

impl EdgeNode {
    /// Creates an Edge Node from its typed device service registry.
    pub fn new(deviceIoService: Arc<dyn DeviceIoService>) -> Self {
        Self {
            deviceIoService,
            robotFaceService: None,
            screenService: None,
            plugins: Vec::new(),
            nodeServices: None,
        }
    }

    /// Edge 和普通节点注入同一个外层接口，不自行实现配对或鉴权。
    pub fn withNodeServices(mut self, services: NodeServices) -> Self {
        self.nodeServices = Some(services);
        self
    }

    pub fn nodeServices(&self) -> Result<NodeServices, EdgeServiceError> {
        self.nodeServices.clone().ok_or_else(|| EdgeServiceError::new("RuntimePeerService has not been installed"))
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

    /// Registers the optional generic display capability.
    pub fn withScreenService(mut self, screenService: Arc<dyn ScreenService>) -> Self {
        self.screenService = Some(screenService);
        self
    }

    /// Registers a firmware-owned plugin with a unique stable id.
    pub fn withPlugin(mut self, plugin: Arc<dyn EdgePlugin>) -> Result<Self, EdgeServiceError> {
        let id = plugin.manifest().id;
        if id.is_empty() || self.plugins.iter().any(|entry| entry.manifest().id == id) {
            return Err(EdgeServiceError::new("invalid or duplicate Edge plugin id"));
        }
        self.plugins.push(plugin);
        Ok(self)
    }

    /// Dispatches one Link call to the registered Edge Service.
    pub fn dispatchCall(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let result = match request.target.as_str() {
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
            EDGE_SCREEN_OBJECT_ID => match request.methodName.as_str() {
                "getScreenSnapshot" => self.getScreenSnapshot(),
                "sendScreenInput" => self.sendScreenInput(request.args),
                _ => Err(CoreLinkError::methodNotFound(&request.registryKey())),
            },
            EDGE_PLUGIN_TARGET => match request.methodName.as_str() {
                "list" => self.listPlugins(),
                "invoke" => self.invokePlugin(request.args),
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
    pub fn dispatchWatchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        match request.target.as_str() {
            EDGE_DEVICE_IO_OBJECT_ID => {
                self.validateDeviceIoWatch(&request)?;
                let pin = decodePin(request.args)?;
                let state = self
                    .deviceIoService
                    .getDigitalOutput(pin)
                    .map_err(serviceError)?;
                Ok(CoreEvent {
                    requestId: Some(request.requestId),
                    target: request.target.clone(),
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
                    target: request.target.clone(),
                    propertyName: request.propertyName,
                    kind: CoreEventKind::Snapshot,
                    value: toCoreValue(state)
                        .map_err(|error| CoreLinkError::internal(error.to_string()))?,
                })
            }
            EDGE_SCREEN_OBJECT_ID => Err(CoreLinkError::watchNotFound(&request.registryKey())),
            _ => Err(CoreLinkError::watchNotFound(&request.registryKey())),
        }
    }

    /// Opens one Link watch backed by a typed Edge Service state stream.
    pub fn dispatchWatch(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        match request.target.as_str() {
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
        let target = request.target.clone();
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
                        target: target.clone(),
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
        if request.target != EDGE_DEVICE_IO_OBJECT_ID
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
        let target = request.target.clone();
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
                        target: target.clone(),
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
        if request.target != EDGE_ROBOT_FACE_OBJECT_ID
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

    fn getScreenSnapshot(&self) -> Result<CoreValue, CoreLinkError> {
        let service = self
            .screenService
            .as_ref()
            .ok_or_else(|| CoreLinkError::methodNotFound("screen.getScreenSnapshot"))?;
        encodeValue(service.getScreenSnapshot().map_err(serviceError)?)
    }

    fn listPlugins(&self) -> Result<CoreValue, CoreLinkError> {
        encodeValue(self.plugins.iter().map(|plugin| plugin.manifest()).collect::<Vec<_>>())
    }

    fn invokePlugin(&self, args: CoreValue) -> Result<CoreValue, CoreLinkError> {
        let request: plugin::EdgePluginCall = decodeValue(args)?;
        let plugin = self.plugins.iter().find(|plugin| plugin.manifest().id == request.pluginId)
            .ok_or_else(|| CoreLinkError::new("EDGE_PLUGIN_NOT_FOUND", "Edge plugin is not installed"))?;
        if !plugin.manifest().actions.iter().any(|action| *action == request.action) {
            return Err(CoreLinkError::new("EDGE_PLUGIN_ACTION_NOT_FOUND", "Edge plugin action is not declared"));
        }
        plugin.invoke(&request.action, request.args).map_err(serviceError)
    }

    async fn invokePluginAsync(&self, args: CoreValue) -> Result<CoreValue, CoreLinkError> {
        let request: plugin::EdgePluginCall = decodeValue(args)?;
        let plugin = self.plugins.iter().find(|plugin| plugin.manifest().id == request.pluginId)
            .ok_or_else(|| CoreLinkError::new("EDGE_PLUGIN_NOT_FOUND", "Edge plugin is not installed"))?;
        if !plugin.manifest().actions.iter().any(|action| *action == request.action) {
            return Err(CoreLinkError::new("EDGE_PLUGIN_ACTION_NOT_FOUND", "Edge plugin action is not declared"));
        }
        plugin.invokeAsync(&request.action, request.args).await.map_err(serviceError)
    }

    async fn dispatchAsyncCall(&self, request: CoreCallRequest) -> CoreCallResponse {
        if request.target == EDGE_PLUGIN_TARGET && request.methodName == "invoke" {
            let id = request.requestId.clone();
            CoreCallResponse { requestId: id, result: self.invokePluginAsync(request.args).await }
        } else { self.dispatchCall(request) }
    }

    fn sendScreenInput(&self, args: CoreValue) -> Result<CoreValue, CoreLinkError> {
        let service = self
            .screenService
            .as_ref()
            .ok_or_else(|| CoreLinkError::methodNotFound("screen.sendScreenInput"))?;
        let request: EdgeScreenInputRequest = decodeValue(args)?;
        encodeValue(service.sendScreenInput(request).map_err(serviceError)?)
    }
}

#[async_trait(?Send)]
impl CoreLinkSharedClient for EdgeNode {
    /// Dispatches one local Edge Core call through the shared Link interface.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        self.dispatchAsyncCall(request).await
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

/// The lightweight node accepts the same Link interface as a full node.
/// Board capabilities are selected by services through HostManager, not by transport.
#[async_trait(?Send)]
impl operit_link::CoreLinkClient for EdgeNode {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        self.dispatchAsyncCall(request).await
    }
    async fn watchSnapshot(&mut self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        self.dispatchWatchSnapshot(request)
    }
    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.dispatchWatch(request)
    }
    async fn openPush(&mut self, request: operit_link::CorePushRequest)
        -> Result<Box<dyn operit_link::CoreLinkPushSession>, CoreLinkError> {
        // No current board service declares an input-stream method. Do not
        // invent a transport-specific push operation or silently turn it into call.
        Err(CoreLinkError::methodNotFound(&format!("{}.{}", request.target, request.methodName)))
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

    #[tokio::test]
    async fn asynchronous_plugin_wait_does_not_use_the_blocking_entrypoint() {
        struct AsyncPlugin;
        #[async_trait(?Send)]
        impl EdgePlugin for AsyncPlugin {
            fn manifest(&self) -> EdgePluginManifest { EdgePluginManifest { id: "ui".into(), name: "UI".into(), actions:vec!["screen".into()] } }
            fn invoke(&self, _: &str, _: CoreValue) -> Result<CoreValue,EdgeServiceError> { panic!("Must not block the runtime") }
            async fn invokeAsync(&self, _: &str, _: CoreValue) -> Result<CoreValue,EdgeServiceError> {
                tokio::task::yield_now().await;
                Ok(CoreValue::String("main-thread-snapshot".into()))
            }
        }
        let node = testNode().withPlugin(Arc::new(AsyncPlugin)).unwrap();
        let request = |action:&str| CoreCallRequest::new("ui-test",EDGE_PLUGIN_TARGET,"invoke",CoreValue::Map(std::collections::BTreeMap::from([
            ("pluginId".into(),CoreValue::String("ui".into())),
            ("action".into(),CoreValue::String(action.into())),("args".into(),CoreValue::emptyMap())
        ])));
        let response = CoreLinkSharedClient::call(&node,request("screen")).await;
        assert_eq!(response.result.unwrap(),CoreValue::String("main-thread-snapshot".into()));
        let response = CoreLinkSharedClient::call(&node,request("undeclared")).await;
        assert_eq!(response.result.unwrap_err().code,"EDGE_PLUGIN_ACTION_NOT_FOUND");
    }

    #[tokio::test]
    async fn standard_link_dispatches_to_board_host_and_publishes_watch() {
        use operit_host_api::{DeviceDigitalOutputRequest, DeviceDigitalOutputState, DeviceIoHost, HostResult};
        struct Board(std::sync::Mutex<bool>);
        impl DeviceIoHost for Board {
            fn setDigitalOutput(&self, request: DeviceDigitalOutputRequest) -> HostResult<DeviceDigitalOutputState> {
                *self.0.lock().unwrap() = request.level;
                Ok(DeviceDigitalOutputState { pin: request.pin, level: request.level })
            }
            fn getDigitalOutput(&self, pin: u8) -> HostResult<DeviceDigitalOutputState> {
                Ok(DeviceDigitalOutputState { pin, level: *self.0.lock().unwrap() })
            }
        }
        let board = Arc::new(Board(std::sync::Mutex::new(false)));
        let mut node = EdgeNode::fromHostManager(HostManager::new().withDeviceIoHost(board.clone()));
        let args = || toCoreValue(DeviceDigitalOutputRequest { pin: 2, level: true }).unwrap();
        let mut watch = operit_link::CoreLinkClient::watch(&mut node,
            CoreWatchRequest::new("watch", EDGE_DEVICE_IO_OBJECT_ID, EDGE_DEVICE_IO_STATE_PROPERTY, args()))
            .await.unwrap();
        assert_eq!(watch.recv().await.unwrap().kind, CoreEventKind::Snapshot);
        let response = operit_link::CoreLinkClient::call(&mut node,
            CoreCallRequest::new("write", EDGE_DEVICE_IO_OBJECT_ID, "setDigitalOutput", args())).await;
        assert!(response.result.is_ok());
        assert!(*board.0.lock().unwrap());
        let event = watch.recv().await.unwrap();
        assert_eq!(event.kind, CoreEventKind::Changed);
        assert!(operit_link::fromCoreValue::<DeviceDigitalOutputState>(event.value).unwrap().level);
        drop(watch);
        let push = operit_link::CoreLinkClient::openPush(&mut node,
            operit_link::CorePushRequest::new("push", EDGE_DEVICE_IO_OBJECT_ID, "notDeclared")).await;
        assert!(push.is_err());
    }

    /// The transport-neutral ingress must execute the existing service/Host path,
    /// not a PeerFrame adapter or a second Edge method registry.
    #[tokio::test]
    async fn standard_wire_session_executes_edge_service_and_host() {
        use operit_host_api::{DeviceDigitalOutputRequest, DeviceDigitalOutputState, DeviceIoHost, HostResult};
        use operit_link::{CoreLinkSession, CoreLinkRequest, CoreLinkResponse, CoreLinkWatchRequest,
            CoreLinkWatchResponse, CoreLinkPushRequestMessage, CorePushRequest, encodeLink, decodeLink};
        struct Board(std::sync::Mutex<bool>);
        impl DeviceIoHost for Board {
            fn setDigitalOutput(&self, request: DeviceDigitalOutputRequest) -> HostResult<DeviceDigitalOutputState> {
                *self.0.lock().unwrap() = request.level;
                Ok(DeviceDigitalOutputState { pin: request.pin, level: request.level })
            }
            fn getDigitalOutput(&self, pin: u8) -> HostResult<DeviceDigitalOutputState> {
                Ok(DeviceDigitalOutputState { pin, level: *self.0.lock().unwrap() })
            }
        }
        async fn exchange(session: &mut CoreLinkSession<EdgeNode>, request: CoreLinkRequest) -> CoreLinkResponse {
            let request = decodeLink(&encodeLink(&request).unwrap()).unwrap();
            let response = session.dispatch(request).await;
            decodeLink(&encodeLink(&response).unwrap()).unwrap()
        }
        let board = Arc::new(Board(std::sync::Mutex::new(false)));
        let node = EdgeNode::fromHostManager(HostManager::new().withDeviceIoHost(board.clone()));
        let mut session = CoreLinkSession::new(node, 4);
        let args = || toCoreValue(DeviceDigitalOutputRequest { pin: 2, level: true }).unwrap();
        let watch = || CoreWatchRequest::new("watch", EDGE_DEVICE_IO_OBJECT_ID, EDGE_DEVICE_IO_STATE_PROPERTY, args());
        assert!(matches!(exchange(&mut session, CoreLinkRequest::Watch(CoreLinkWatchRequest::Open(watch()))).await,
            CoreLinkResponse::Watch { result: Ok(CoreLinkWatchResponse::Opened), .. }));
        assert!(matches!(session.nextWatchEvent().await,
            Some(CoreLinkResponse::Watch { result: Ok(CoreLinkWatchResponse::Event(CoreEvent { kind: CoreEventKind::Snapshot, .. })), .. })));
        assert!(matches!(exchange(&mut session, CoreLinkRequest::Call(CoreCallRequest::new(
            "write", EDGE_DEVICE_IO_OBJECT_ID, "setDigitalOutput", args()))).await,
            CoreLinkResponse::Call(CoreCallResponse { result: Ok(_), .. })));
        assert!(*board.0.lock().unwrap());
        let Some(CoreLinkResponse::Watch { result: Ok(CoreLinkWatchResponse::Event(event)), .. }) = session.nextWatchEvent().await else {
            panic!("Expected service event after Host write");
        };
        assert_eq!(event.kind, CoreEventKind::Changed);
        assert!(operit_link::fromCoreValue::<DeviceDigitalOutputState>(event.value).unwrap().level);
        assert!(matches!(exchange(&mut session, CoreLinkRequest::Watch(CoreLinkWatchRequest::Snapshot(watch()))).await,
            CoreLinkResponse::Watch { result: Ok(CoreLinkWatchResponse::Snapshot(_)), .. }));
        let expected = operit_link::CoreLinkError::methodNotFound("unused").code;
        assert!(matches!(exchange(&mut session, CoreLinkRequest::Push(CoreLinkPushRequestMessage::Open(
            CorePushRequest::new("push", EDGE_DEVICE_IO_OBJECT_ID, "notDeclared")))).await,
            CoreLinkResponse::Push { pushId, result: Err(error) } if pushId == "push" && error.code == expected));
        exchange(&mut session, CoreLinkRequest::Watch(CoreLinkWatchRequest::Close {
            requestId: operit_link::CoreRequestId::new("watch"),
        })).await;
        assert!(!session.hasWatches());
    }

    /// Exercises the shared runtime adapter, not a second Edge wire registry.
    #[tokio::test]
    async fn shared_peer_router_reuses_local_call_and_watch() {
        use crate::PeerRouter::EdgePeerRouter;
        use operit_link::{RoutedCoreRequest, RoutedCoreRequestKind};
        use operit_node_runtime::PeerRouter::PeerRouter;
        let router = EdgePeerRouter::new("edge-test".into());
        router
            .install(Arc::new(EdgeNode::new(Arc::new(TestDeviceIoService))))
            .unwrap();
        assert!(router
            .install(Arc::new(EdgeNode::new(Arc::new(TestDeviceIoService))))
            .is_err());
        assert_eq!(router.localNodeId(), "edge-test");
        assert_eq!(router.spaceChannelScope("paired-core").unwrap(), None);
        let route = |payload| RoutedCoreRequest {
            spaceId: String::new(),
            originNodeId: "paired-core".into(),
            targetNodeId: "edge-test".into(),
            ttl: 1,
            routeKind: RoutedCoreRequestKind::Target,
            payload,
        };
        let args = toCoreValue(operit_host_api::DeviceDigitalOutputRequest {
            pin: 2,
            level: true,
        })
        .unwrap();
        let call = CoreCallRequest::new(
            "write",
            EDGE_DEVICE_IO_OBJECT_ID,
            "setDigitalOutput",
            args.clone(),
        );
        let response = router.routedCall("paired-core".into(), route(call)).await;
        assert_eq!(response.requestId, operit_link::CoreRequestId::new("write"));
        let state: operit_host_api::DeviceDigitalOutputState =
            operit_link::fromCoreValue(response.result.unwrap()).unwrap();
        assert!(state.level);
        assert_eq!(state.pin, 2);
        let request = RoutedCoreRequest {
            spaceId: String::new(),
            originNodeId: "paired-core".into(),
            targetNodeId: "edge-test".into(),
            ttl: 1,
            routeKind: RoutedCoreRequestKind::Target,
            payload: CoreWatchRequest::new(
                "watch",
                EDGE_DEVICE_IO_OBJECT_ID,
                EDGE_DEVICE_IO_STATE_PROPERTY,
                args,
            ),
        };
        assert_eq!(
            router
                .routedWatchSnapshot("paired-core".into(), request.clone())
                .await
                .unwrap()
                .kind,
            CoreEventKind::Snapshot
        );
        let mut stream = router
            .routedWatch("paired-core".into(), request)
            .await
            .unwrap();
        assert_eq!(stream.recv().await.unwrap().kind, CoreEventKind::Snapshot);
    }

    #[tokio::test]
    async fn shared_peer_router_rejects_other_nodes_and_space_routes() {
        use crate::PeerRouter::EdgePeerRouter;
        use operit_link::{RoutedCoreRequest, RoutedCoreRequestKind};
        use operit_node_runtime::PeerRouter::PeerRouter;
        let router = EdgePeerRouter::new("edge-test".into());
        let request = RoutedCoreRequest {
            spaceId: String::new(),
            originNodeId: "paired-core".into(),
            targetNodeId: "other-node".into(),
            ttl: 1,
            routeKind: RoutedCoreRequestKind::Target,
            payload: CoreCallRequest::new(
                "call",
                EDGE_DEVICE_IO_OBJECT_ID,
                "getDigitalOutput",
                CoreValue::Null,
            ),
        };
        assert_eq!(
            router
                .routedCall("paired-core".into(), request.clone())
                .await
                .result
                .unwrap_err()
                .code,
            "EDGE_ROUTE_TARGET_MISMATCH"
        );
        let mut request = request;
        request.targetNodeId = "edge-test".into();
        for kind in [
            RoutedCoreRequestKind::SpaceRoute,
            RoutedCoreRequestKind::SpaceBinding,
        ] {
            request.routeKind = kind;
            assert_eq!(
                router
                    .routedCall("paired-core".into(), request.clone())
                    .await
                    .result
                    .unwrap_err()
                    .code,
                "EDGE_ROUTE_UNSUPPORTED"
            );
        }
        request.routeKind = RoutedCoreRequestKind::Target;
        assert!(router
            .routedCall("paired-core".into(), request)
            .await
            .result
            .is_err());
    }

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

#[cfg(test)]
#[path = "space_admission_tests.rs"]
mod space_admission_tests;
