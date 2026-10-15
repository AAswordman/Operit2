//! Explicit-device tool execution. Routing metadata never becomes a tool parameter.
use crate::tools::AIToolHandler::{AIToolHandler, ToolRegistrationVisibility};
use crate::tools::ToolResultDataClasses::ToolResultData;
use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::{AITool, ToolCall, ToolExecutionPolicy, ToolParameter};
use operit_link::{
    toCoreValue, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError, CoreWatchRequest,
};
use operit_plugin_sdk::javascript::{
    JsExecutionCompletion, JsExecutionHost, JsToolCallRequest, JsToolCallResult,
    JsToolCallResultData,
};

pub const DEVICE_TOOL_TARGET: &str = "core.tools";

pub type DeviceToolPreflight = JsExecutionCompletion<Result<(), String>>;

/// Outbound capability installed by the owning Core assembly.
pub trait DeviceToolRuntime: Send + Sync {
    fn close_transports(&self, _owner: &str) {}
    fn dispatch(&self, request: JsToolCallRequest) -> JsExecutionCompletion<JsToolCallResult> {
        self.dispatch_with_preflight(request, None)
    }
    fn dispatch_with_preflight(
        &self,
        request: JsToolCallRequest,
        preflight: Option<DeviceToolPreflight>,
    ) -> JsExecutionCompletion<JsToolCallResult>;
}

/// Incoming capability owned by one device's existing tool registry.
pub trait DeviceToolService: Send + Sync {
    fn open(
        &self,
        request: CoreWatchRequest,
        origin_node_id: String,
    ) -> Result<CoreEventStream, CoreLinkError>;
}

pub fn failure(message: impl Into<String>) -> JsToolCallResult {
    JsToolCallResult {
        success: false,
        error: Some(message.into()),
        ..Default::default()
    }
}

pub fn tool_from_request(request: &JsToolCallRequest) -> AITool {
    AITool {
        name: request.qualified_tool_name(),
        parameters: request
            .parameters
            .iter()
            .map(|(name, value)| ToolParameter {
                name: name.clone(),
                value: match value {
                    serde_json::Value::Null => String::new(),
                    serde_json::Value::String(value) => value.clone(),
                    value => value.to_string(),
                },
            })
            .collect(),
    }
}

pub fn result_to_js(result: ToolResult) -> JsToolCallResult {
    let data = match result.result {
        ToolResultData::JsonResultData(data) => JsToolCallResultData::Value(data.value),
        ToolResultData::BinaryResultData(data) => JsToolCallResultData::Binary(data.value),
        ToolResultData::StringResultData(data) => JsToolCallResultData::Value(data.value.into()),
        ToolResultData::BooleanResultData(data) => JsToolCallResultData::Value(data.value.into()),
        ToolResultData::IntResultData(data) => JsToolCallResultData::Value(data.value.into()),
        data => match serde_json::from_str(&data.toJson()) {
            Ok(value) => JsToolCallResultData::Value(value),
            Err(error) => return failure(format!("Invalid tool result: {error}")),
        },
    };
    JsToolCallResult {
        success: result.success,
        data,
        error: result.error,
    }
}

impl AIToolHandler {
    /// Runs only public tools, with receiver-owned validation and permission preflight.
    async fn execute_device_tool(
        &mut self,
        request: JsToolCallRequest,
        origin: &str,
    ) -> JsToolCallResult {
        let name = request.qualified_tool_name();
        if let Some((package, _)) = name.split_once(':') {
            let manager = self.getOrCreatePackageManager();
            let manager = manager.lock().map_err(|error| error.to_string());
            match manager {
                Ok(manager)
                    if manager.getAvailablePackages().contains_key(package)
                        && !manager.getAvailableServerPackages().contains_key(package)
                        && !manager.isPackageEnabled(package) =>
                {
                    return failure(format!("Package is disabled on target device: {package}"));
                }
                Err(error) => return failure(error),
                _ => {}
            }
        }
        match self.getToolExecutorOrActivate(&name).await {
            Ok(true) => {},
            Ok(false) => return failure(format!("Tool not found on target device: {name}")),
            Err(error) => return failure(error),
        }
        if self.getToolVisibility(&name) != Some(ToolRegistrationVisibility::PUBLIC) {
            return failure(format!("Tool is not public on target device: {name}"));
        }
        let tool = tool_from_request(&request);
        operit_util::AppLogger::AppLogger::i(
            "DeviceTools",
            &format!("Execute {} from authenticated node {origin}", tool.name),
        );
        let call = ToolCall { name, parameters: request.parameters };
        match self.executeToolInvocation(&call, ToolExecutionPolicy::Device).await {
            Ok(results) => result_to_js(results.into_iter().last()
                .expect("The invocation pipeline rejects empty results")),
            Err(error) => failure(error),
        }
    }

    fn device_tool_catalog(&self) -> JsToolCallResult {
        let mut catalog = match JsExecutionHost::get_tool_catalog(self) {
            Ok(catalog) => catalog,
            Err(error) => return failure(error),
        };
        // The local catalog includes internal tools and disabled packages.
        if let Some(tools) = catalog
            .get_mut("tools")
            .and_then(|value| value.as_array_mut())
        {
            tools.retain(|tool| {
                if tool["source"] == "package" {
                    tool["packageName"].as_str().is_some_and(|package| {
                        self.getOrCreatePackageManager()
                            .lock()
                            .is_ok_and(|manager| {
                                manager.isPackageEnabled(package)
                                    || manager.getAvailableServerPackages().contains_key(package)
                            })
                    })
                } else {
                    tool["name"].as_str().is_some_and(|name| {
                        self.getToolVisibility(name) == Some(ToolRegistrationVisibility::PUBLIC)
                    })
                }
            });
        }
        JsToolCallResult {
            success: true,
            data: JsToolCallResultData::Value(catalog),
            error: None,
        }
    }
}

impl DeviceToolService for AIToolHandler {
    fn open(
        &self,
        request: CoreWatchRequest,
        origin_node_id: String,
    ) -> Result<CoreEventStream, CoreLinkError> {
        if request.target != DEVICE_TOOL_TARGET
            || !matches!(request.propertyName.as_str(), "execute" | "catalog")
        {
            return Err(CoreLinkError::new(
                "DEVICE_TOOL_METHOD_INVALID",
                "Unknown device tool operation",
            ));
        }
        let invocation = if request.propertyName == "execute" {
            let invocation: JsToolCallRequest = operit_link::fromCoreValue(request.args.clone())
                .map_err(|error| {
                    CoreLinkError::new("DEVICE_TOOL_REQUEST_INVALID", error.to_string())
                })?;
            if invocation.device.is_some() || invocation.qualified_tool_name().trim().is_empty() {
                return Err(CoreLinkError::new(
                    "DEVICE_TOOL_REQUEST_INVALID",
                    "Invalid incoming tool identity or nested device selection",
                ));
            }
            Some(invocation)
        } else {
            None
        };
        let scheduler = self
            .getContext()
            .hostRuntimeTaskSchedulerHost
            .clone()
            .ok_or_else(|| {
                CoreLinkError::new(
                    "DEVICE_TOOL_SCHEDULER_UNAVAILABLE",
                    "Device tool execution requires a host scheduler",
                )
            })?;
        let mut handler = self.clone();
        let (sender, stream) = CoreEventStream::channel();
        scheduler
            .scheduleHostRuntimeAsyncTask(
                "device-tool-receiver",
                Box::new(move || {
                    Box::pin(async move {
                        let execution = async {
                            match invocation {
                                Some(invocation) => {
                                    handler
                                        .execute_device_tool(invocation, &origin_node_id)
                                        .await
                                }
                                None => handler.device_tool_catalog(),
                            }
                        };
                        tokio::select! {
                            biased;
                            _ = sender.closed() => {},
                            result = execution => {
                                if let Ok(value) = toCoreValue(result) {
                                    let _ = sender.send(CoreEvent {
                                        requestId: Some(request.requestId),
                                        target: DEVICE_TOOL_TARGET.into(),
                                        propertyName: request.propertyName,
                                        kind: CoreEventKind::Completed,
                                        value,
                                    });
                                }
                            }
                        }
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        Ok(stream)
    }
}

/// Owns the channels of one JS engine, including its cloned invocation hosts.
pub(crate) struct DeviceTransportScope {
    pub id: String,
    support: std::sync::Mutex<Option<std::sync::Weak<dyn crate::runtime_support::ToolRuntimeSupport>>>,
}
impl Default for DeviceTransportScope {
    fn default() -> Self { Self {id: uuid::Uuid::new_v4().to_string(), support:Default::default()} }
}
impl DeviceTransportScope {
    pub fn bind(&self, support: &std::sync::Arc<dyn crate::runtime_support::ToolRuntimeSupport>) {
        *self.support.lock().unwrap()=Some(std::sync::Arc::downgrade(support));
    }
}
impl Drop for DeviceTransportScope {
    fn drop(&mut self) {
        if let Some(support)=self.support.get_mut().unwrap().as_ref().and_then(std::sync::Weak::upgrade) {
            support.closeDeviceTransports(&self.id);
        }
    }
}
