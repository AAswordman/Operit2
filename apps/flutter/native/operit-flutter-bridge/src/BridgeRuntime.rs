//! Platform-neutral bridge ownership and local Core dispatch.
//! Host assembly and transport execution are selected by PlatformRuntimeFactory.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use operit_core_application::CoreApplication;
use operit_host_api::{HostRuntimeTaskSchedulerHost, RuntimeStorageHost};
use operit_link::{CoreCallRequest, CoreCallResponse, CoreLinkSharedClient};
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::services::RuntimeHostInteractionService::{
    requestChatToolPermissionAsync, RuntimeHostInteractionToolPermissionTool,
    RuntimeHostInteractionToolPermissionToolParameter,
};
use operit_tools::tools::ToolPermissionSystem::PermissionRequestResult;
use operit_tools::ToolExecutionManager::AITool;

use crate::PlatformRuntimeFactory::PlatformBridgeState;

/// Owns one Core tree independently of the frontend's transport or operating system.
pub struct OperitFlutterBridge {
    pub(crate) localCore: Arc<LocalCoreProxy>,
    pub(crate) runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
    pub(crate) coreApplication: Mutex<Option<CoreApplication>>,
    pub(crate) platform: PlatformBridgeState,
    pub(crate) taskScheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
}

/// Carries an owned local call across the host scheduler boundary.
pub(crate) struct CoreCallTask {
    core: Arc<LocalCoreProxy>,
    request: CoreCallRequest,
}

impl CoreCallTask {
    /// Executes the sole shared local-call implementation.
    pub(crate) async fn execute(self) -> CoreCallResponse {
        CoreLinkSharedClient::call(self.core.as_ref(), self.request).await
    }
}

const PERMISSION_REQUEST_TIMEOUT_MS: u64 = 60_000;

impl OperitFlutterBridge {
    /// Starts exactly one local Core tree from a fully assembled host bundle.
    pub(crate) fn start(
        mut core: LocalCoreProxy,
        deviceInfo: operit_link::LinkDeviceInfo,
    ) -> Result<Self, String> {
        let startedAt = operit_host_api::TimeUtils::currentTimeMillis();
        let taskScheduler = core
            .hostManager()
            .hostRuntimeTaskSchedulerHost
            .clone()
            .ok_or_else(|| {
                "Flutter bridge requires its runtime's task scheduler host".to_string()
            })?;
        core.localApplicationMut().onCreate()?;
        install_permission_requester(&mut core);
        let runtimeStorageHost = core.runtimeStorageHost();
        let localCore = Arc::new(core);
        let coreApplication =
            CoreApplication::startWithSharedLocalClient(localCore.clone(), deviceInfo)?;
        operit_util::AppLogger::AppLogger::i(
            "OperitFlutterBridge",
            &format!(
                "Core tree started elapsedMs={}",
                operit_host_api::TimeUtils::currentTimeMillis() - startedAt,
            ),
        );
        Ok(Self {
            localCore,
            runtimeStorageHost,
            coreApplication: Mutex::new(Some(coreApplication)),
            platform: PlatformBridgeState::new(),
            taskScheduler,
        })
    }

    /// Executes the same local Core request on every frontend transport.
    pub(crate) async fn callShared(&self, request: CoreCallRequest) -> CoreCallResponse {
        self.prepareCall(request).execute().await
    }

    /// Prepares an owned call without retaining a borrowed ABI handle.
    pub(crate) fn prepareCall(&self, request: CoreCallRequest) -> CoreCallTask {
        CoreCallTask {
            core: self.localCore.clone(),
            request,
        }
    }

    /// Returns the scheduler installed on this runtime, never a process-global substitute.
    pub(crate) fn runtimeTaskScheduler(&self) -> Arc<dyn HostRuntimeTaskSchedulerHost> {
        self.taskScheduler.clone()
    }
}

impl Drop for OperitFlutterBridge {
    /// Releases the platform transport before destroying the shared Core tree.
    fn drop(&mut self) {
        self.platform.beforeShutdown(self);
        if let Ok(application) = self.coreApplication.get_mut() {
            if let Some(application) = application.take() {
                application.shutdownNow();
            }
        }
        self.platform.releaseHost();
    }
}

/// Installs the asynchronous controller permission requester for every runtime.
fn install_permission_requester(core: &mut LocalCoreProxy) {
    let handler = core.localApplicationMut().toolHandler.clone();
    handler
        .getToolPermissionSystem()
        .setAsyncPermissionRequester(move |tool, description, chatId| async move {
            let Some(chatId) = chatId else {
                return PermissionRequestResult::DENY;
            };
            let response = requestChatToolPermissionAsync(
                chatId,
                tool_to_permission_payload(&tool),
                description,
                Duration::from_millis(PERMISSION_REQUEST_TIMEOUT_MS),
            )
            .await;
            let response = match response {
                Ok(response) => response,
                Err(error) => {
                    operit_util::AppLogger::AppLogger::e(
                        "OperitFlutterBridge",
                        &format!("tool permission request failed: {error}"),
                    );
                    return PermissionRequestResult::DENY;
                }
            };
            match response.as_str() {
                "allow" => PermissionRequestResult::ALLOW,
                "allow_session" => PermissionRequestResult::ALLOW_SESSION,
                "deny" => PermissionRequestResult::DENY,
                other => {
                    operit_util::AppLogger::AppLogger::e(
                        "OperitFlutterBridge",
                        &format!("unknown tool permission response result: {other}"),
                    );
                    PermissionRequestResult::DENY
                }
            }
        });
}

/// Converts a typed tool into the owner permission request schema.
fn tool_to_permission_payload(tool: &AITool) -> RuntimeHostInteractionToolPermissionTool {
    RuntimeHostInteractionToolPermissionTool {
        name: tool.name.clone(),
        parameters: tool
            .parameters
            .iter()
            .map(
                |parameter| RuntimeHostInteractionToolPermissionToolParameter {
                    name: parameter.name.clone(),
                    value: parameter.value.clone(),
                },
            )
            .collect(),
    }
}

/// Reads the host clock for bridge-owned request identifiers.
pub(crate) fn current_time_millis_u64() -> u64 {
    operit_host_api::TimeUtils::currentTimeMillisU128().min(u64::MAX as u128) as u64
}
