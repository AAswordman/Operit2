#![allow(non_snake_case)]

use std::fmt::{Display, Formatter};

use async_trait::async_trait;
use operit_edge_contract::{
    EDGE_DEVICE_IO_OBJECT_ID, EDGE_DEVICE_IO_STATE_PROPERTY, EDGE_ROBOT_FACE_OBJECT_ID,
    EDGE_ROBOT_FACE_STATE_PROPERTY,
};
use operit_host_api::{DeviceDigitalOutputState, RobotFaceState};
use operit_link::{
    fromCoreValue, CoreCallRequest, CoreEventStream, CoreLinkError, CoreLinkSharedClient,
    CoreValue, CoreWatchRequest,
};

/// Owns the typed proxy entry point for Edge Core services.
pub struct EdgeProxy<C> {
    client: C,
}

impl<C> EdgeProxy<C> {
    /// Creates a typed Edge proxy over one Core Link client.
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Returns the typed device I/O proxy.
    pub fn deviceIo(&mut self) -> EdgeDeviceIoProxy<'_, C> {
        EdgeDeviceIoProxy {
            client: &mut self.client,
        }
    }

    /// Returns the typed robot face proxy.
    pub fn robotFace(&mut self) -> EdgeRobotFaceProxy<'_, C> {
        EdgeRobotFaceProxy {
            client: &mut self.client,
        }
    }

    /// Returns the underlying Link client after proxy use is complete.
    pub fn intoInner(self) -> C {
        self.client
    }
}

/// Defines the typed device operations consumed by an Edge app layer.
#[async_trait(?Send)]
pub trait EdgeDeviceIoClient {
    /// Writes one digital output through the Edge Service.
    async fn setDigitalOutput(
        &mut self,
        pin: u8,
        level: bool,
    ) -> Result<DeviceDigitalOutputState, EdgeProxyError>;

    /// Reads one digital output through the Edge Service.
    async fn getDigitalOutput(
        &mut self,
        pin: u8,
    ) -> Result<DeviceDigitalOutputState, EdgeProxyError>;

    /// Opens one digital-output state watch through the Edge Service.
    async fn watchDigitalOutput(&mut self, pin: u8) -> Result<EdgeStateStream, EdgeProxyError>;
}

/// Defines the typed robot face operations consumed by an Edge app layer.
#[async_trait(?Send)]
pub trait EdgeRobotFaceClient {
    /// Writes one robot expression through the Edge Service.
    async fn setExpression(&mut self, expression: String)
        -> Result<RobotFaceState, EdgeProxyError>;

    /// Reads the current robot expression through the Edge Service.
    async fn getExpression(&mut self) -> Result<RobotFaceState, EdgeProxyError>;

    /// Opens one robot face expression watch through the Edge Service.
    async fn watchExpression(&mut self) -> Result<EdgeRobotFaceStateStream, EdgeProxyError>;
}

#[async_trait(?Send)]
impl<C> EdgeDeviceIoClient for EdgeProxy<C>
where
    C: CoreLinkSharedClient,
{
    /// Writes one digital output through the typed Edge Proxy.
    async fn setDigitalOutput(
        &mut self,
        pin: u8,
        level: bool,
    ) -> Result<DeviceDigitalOutputState, EdgeProxyError> {
        self.deviceIo().setDigitalOutput(pin, level).await
    }

    /// Reads one digital output through the typed Edge Proxy.
    async fn getDigitalOutput(
        &mut self,
        pin: u8,
    ) -> Result<DeviceDigitalOutputState, EdgeProxyError> {
        self.deviceIo().getDigitalOutput(pin).await
    }

    /// Opens one digital-output state watch through the typed Edge Proxy.
    async fn watchDigitalOutput(&mut self, pin: u8) -> Result<EdgeStateStream, EdgeProxyError> {
        self.deviceIo().watchDigitalOutput(pin).await
    }
}

#[async_trait(?Send)]
impl<C> EdgeRobotFaceClient for EdgeProxy<C>
where
    C: CoreLinkSharedClient,
{
    /// Writes one robot expression through the typed Edge Proxy.
    async fn setExpression(
        &mut self,
        expression: String,
    ) -> Result<RobotFaceState, EdgeProxyError> {
        self.robotFace().setExpression(expression).await
    }

    /// Reads the current robot expression through the typed Edge Proxy.
    async fn getExpression(&mut self) -> Result<RobotFaceState, EdgeProxyError> {
        self.robotFace().getExpression().await
    }

    /// Opens one robot face state watch through the typed Edge Proxy.
    async fn watchExpression(&mut self) -> Result<EdgeRobotFaceStateStream, EdgeProxyError> {
        self.robotFace().watchExpression().await
    }
}

/// Provides typed operations for the Edge device I/O service.
pub struct EdgeDeviceIoProxy<'a, C> {
    client: &'a mut C,
}

impl<'a, C> EdgeDeviceIoProxy<'a, C>
where
    C: CoreLinkSharedClient,
{
    /// Writes one digital output through the Edge Core service.
    pub async fn setDigitalOutput(
        &mut self,
        pin: u8,
        level: bool,
    ) -> Result<DeviceDigitalOutputState, EdgeProxyError> {
        let response = self
            .client
            .call(CoreCallRequest::new(
                "edge-device-io-write",
                EDGE_DEVICE_IO_OBJECT_ID,
                "setDigitalOutput",
                CoreValue::Map(std::collections::BTreeMap::from([
                    ("pin".to_string(), CoreValue::Unsigned(pin as u64)),
                    ("level".to_string(), CoreValue::Bool(level)),
                ])),
            ))
            .await;
        decodeResponse(response.result)
    }

    /// Reads one digital output through the Edge Core service.
    pub async fn getDigitalOutput(
        &mut self,
        pin: u8,
    ) -> Result<DeviceDigitalOutputState, EdgeProxyError> {
        let response = self
            .client
            .call(CoreCallRequest::new(
                "edge-device-io-read",
                EDGE_DEVICE_IO_OBJECT_ID,
                "getDigitalOutput",
                CoreValue::Map(std::collections::BTreeMap::from([(
                    "pin".to_string(),
                    CoreValue::Unsigned(pin as u64),
                )])),
            ))
            .await;
        decodeResponse(response.result)
    }

    /// Opens a typed digital-output state watch through Edge Core.
    pub async fn watchDigitalOutput(&mut self, pin: u8) -> Result<EdgeStateStream, EdgeProxyError> {
        let stream = self
            .client
            .watch(CoreWatchRequest::new(
                "edge-device-io-watch",
                EDGE_DEVICE_IO_OBJECT_ID,
                EDGE_DEVICE_IO_STATE_PROPERTY,
                CoreValue::Map(std::collections::BTreeMap::from([(
                    "pin".to_string(),
                    CoreValue::Unsigned(pin as u64),
                )])),
            ))
            .await
            .map_err(EdgeProxyError::from)?;
        Ok(EdgeStateStream { stream })
    }
}

/// Provides typed operations for the Edge robot face service.
pub struct EdgeRobotFaceProxy<'a, C> {
    client: &'a mut C,
}

impl<'a, C> EdgeRobotFaceProxy<'a, C>
where
    C: CoreLinkSharedClient,
{
    /// Writes one robot expression through the Edge Core service.
    pub async fn setExpression(
        &mut self,
        expression: String,
    ) -> Result<RobotFaceState, EdgeProxyError> {
        let response = self
            .client
            .call(CoreCallRequest::new(
                "edge-robot-face-write",
                EDGE_ROBOT_FACE_OBJECT_ID,
                "setExpression",
                CoreValue::Map(std::collections::BTreeMap::from([(
                    "expression".to_string(),
                    CoreValue::String(expression),
                )])),
            ))
            .await;
        decodeResponse(response.result)
    }

    /// Reads the current robot expression through the Edge Core service.
    pub async fn getExpression(&mut self) -> Result<RobotFaceState, EdgeProxyError> {
        let response = self
            .client
            .call(CoreCallRequest::new(
                "edge-robot-face-read",
                EDGE_ROBOT_FACE_OBJECT_ID,
                "getExpression",
                CoreValue::emptyMap(),
            ))
            .await;
        decodeResponse(response.result)
    }

    /// Opens a typed robot face state watch through Edge Core.
    pub async fn watchExpression(&mut self) -> Result<EdgeRobotFaceStateStream, EdgeProxyError> {
        let stream = self
            .client
            .watch(CoreWatchRequest::new(
                "edge-robot-face-watch",
                EDGE_ROBOT_FACE_OBJECT_ID,
                EDGE_ROBOT_FACE_STATE_PROPERTY,
                CoreValue::emptyMap(),
            ))
            .await
            .map_err(EdgeProxyError::from)?;
        Ok(EdgeRobotFaceStateStream { stream })
    }
}

/// Wraps an internal Link stream as a typed Edge state stream.
pub struct EdgeStateStream {
    stream: CoreEventStream,
}

impl EdgeStateStream {
    /// Waits for the next digital-output state event.
    pub async fn recv(&mut self) -> Result<Option<DeviceDigitalOutputState>, EdgeProxyError> {
        let Some(event) = self.stream.recv().await else {
            return Ok(None);
        };
        decodeValue(event.value).map(Some)
    }
}

/// Wraps an internal Link stream as a typed robot face state stream.
pub struct EdgeRobotFaceStateStream {
    stream: CoreEventStream,
}

impl EdgeRobotFaceStateStream {
    /// Waits for the next robot face state event.
    pub async fn recv(&mut self) -> Result<Option<RobotFaceState>, EdgeProxyError> {
        let Some(event) = self.stream.recv().await else {
            return Ok(None);
        };
        decodeValue(event.value).map(Some)
    }
}

/// Describes an error crossing the typed Edge Proxy boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeProxyError {
    pub code: String,
    pub message: String,
}

impl Display for EdgeProxyError {
    /// Formats one typed Edge Proxy error for logging.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for EdgeProxyError {}

impl From<CoreLinkError> for EdgeProxyError {
    /// Converts one internal Link error into a typed Edge Proxy error.
    fn from(error: CoreLinkError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }
}

/// Decodes one successful Link response into a typed Edge value.
fn decodeResponse<T>(result: Result<CoreValue, CoreLinkError>) -> Result<T, EdgeProxyError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    decodeValue(result.map_err(EdgeProxyError::from)?)
}

/// Decodes one internal Link value into a typed Edge value.
fn decodeValue<T>(value: CoreValue) -> Result<T, EdgeProxyError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    fromCoreValue(value).map_err(|error| EdgeProxyError {
        code: "INVALID_RESPONSE".to_string(),
        message: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::{DeviceDigitalOutputRequest, RobotFaceExpressionRequest};
    use operit_node_edge::{
        DeviceIoService, DeviceIoStateStream, EdgeNode, EdgeServiceError, RobotFaceService,
        RobotFaceStateStream,
    };
    use std::sync::mpsc;
    use std::sync::Arc;

    /// Provides typed device operations for a deterministic Edge Proxy test.
    struct TestDeviceIoService;

    impl DeviceIoService for TestDeviceIoService {
        /// Returns the requested digital-output state unchanged.
        fn setDigitalOutput(
            &self,
            request: DeviceDigitalOutputRequest,
        ) -> Result<DeviceDigitalOutputState, EdgeServiceError> {
            Ok(DeviceDigitalOutputState {
                pin: request.pin,
                level: request.level,
            })
        }

        /// Returns a deterministic low digital-output state.
        fn getDigitalOutput(&self, pin: u8) -> Result<DeviceDigitalOutputState, EdgeServiceError> {
            Ok(DeviceDigitalOutputState { pin, level: false })
        }

        /// Opens a typed stream containing one deterministic snapshot.
        fn watchDigitalOutput(&self, pin: u8) -> Result<DeviceIoStateStream, EdgeServiceError> {
            let (sender, receiver) = mpsc::channel();
            sender
                .send(DeviceDigitalOutputState { pin, level: false })
                .map_err(|error| EdgeServiceError::new(error.to_string()))?;
            Ok(DeviceIoStateStream::new(receiver))
        }
    }

    /// Provides typed robot face operations for a deterministic Edge Proxy test.
    struct TestRobotFaceService;

    impl RobotFaceService for TestRobotFaceService {
        /// Returns the requested robot expression state unchanged.
        fn setExpression(
            &self,
            request: RobotFaceExpressionRequest,
        ) -> Result<RobotFaceState, EdgeServiceError> {
            Ok(RobotFaceState {
                expression: request.expression,
            })
        }

        /// Returns a deterministic neutral robot expression state.
        fn getExpression(&self) -> Result<RobotFaceState, EdgeServiceError> {
            Ok(RobotFaceState {
                expression: "neutral".to_string(),
            })
        }

        /// Opens a typed robot face stream containing one deterministic snapshot.
        fn watchExpression(&self) -> Result<RobotFaceStateStream, EdgeServiceError> {
            let (sender, receiver) = mpsc::channel();
            sender
                .send(RobotFaceState {
                    expression: "neutral".to_string(),
                })
                .map_err(|error| EdgeServiceError::new(error.to_string()))?;
            Ok(RobotFaceStateStream::new(receiver))
        }
    }

    /// Verifies the typed Proxy can call and watch an Edge Service without Link values in its API.
    #[tokio::test]
    async fn typedProxyCallsAndWatchesEdgeService() {
        let node = EdgeNode::new(Arc::new(TestDeviceIoService))
            .withRobotFaceService(Arc::new(TestRobotFaceService));
        let mut proxy = EdgeProxy::new(node);

        let state = proxy
            .setDigitalOutput(2, true)
            .await
            .expect("typed output write must succeed");
        assert_eq!(
            state,
            DeviceDigitalOutputState {
                pin: 2,
                level: true
            }
        );

        let mut stream = proxy
            .watchDigitalOutput(2)
            .await
            .expect("typed output watch must open");
        let snapshot = stream
            .recv()
            .await
            .expect("typed output watch must decode")
            .expect("typed output watch must emit a snapshot");
        assert_eq!(
            snapshot,
            DeviceDigitalOutputState {
                pin: 2,
                level: false
            }
        );

        let face = proxy
            .setExpression("happy".to_string())
            .await
            .expect("typed robot face write must succeed");
        assert_eq!(face.expression, "happy");

        let mut faceStream = proxy
            .watchExpression()
            .await
            .expect("typed robot face watch must open");
        let faceSnapshot = faceStream
            .recv()
            .await
            .expect("typed robot face watch must decode")
            .expect("typed robot face watch must emit a snapshot");
        assert_eq!(faceSnapshot.expression, "neutral");
    }
}
