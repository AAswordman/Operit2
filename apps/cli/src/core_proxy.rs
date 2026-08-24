use std::ops::{Deref, DerefMut};
use operit_proxy_local::{
    GeneratedCoreProxy, LocalCoreProxy,
};
use operit_host_api::HostManager::HostManager;
use operit_node_runtime::CoreNodeRouter::CoreNodeRouter;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;

use crate::create_local_core;
use async_trait::async_trait;
use operit_link::{CoreEvent, CoreEventStream, CoreLinkClient, CoreLinkError, CoreLinkPushSession, CoreLinkSharedClient, CoreCallRequest, CoreCallResponse, CoreWatchRequest};

/// Owns a shared local Core proxy while satisfying the generated mutable client surface.
pub(crate) struct SharedLocalCore(pub(crate) std::sync::Arc<LocalCoreProxy>);

#[async_trait(?Send)]
impl CoreLinkClient for SharedLocalCore {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        CoreLinkSharedClient::call(self.0.as_ref(), request).await
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(&mut self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        CoreLinkSharedClient::watchSnapshot(self.0.as_ref(), request).await
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        CoreLinkSharedClient::watch(self.0.as_ref(), request).await
    }

    #[allow(non_snake_case)]
    async fn openPush(&mut self, request: operit_link::CorePushRequest) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        self.0.openPushLocal(request)
    }
}

pub(crate) struct CliCore {
    proxy: GeneratedCoreProxy<SharedLocalCore>,
    localHostManager: Option<HostManager>,
    _coreNodeRouter: CoreNodeRouter,
}

/// Creates the local CLI Core and sends generated requests directly to its runtime proxy.
pub(crate) fn local_cli_core() -> Result<CliCore, String> {
    let mut core = initialized_cli_runtime()?;
    let localHostManager = core.0.hostManager().clone();
    Ok(CliCore {
        proxy: GeneratedCoreProxy::new(SharedLocalCore(core.0)),
        localHostManager: Some(localHostManager),
        _coreNodeRouter: core.1,
    })
}

/// Creates and initializes one CLI-owned local runtime before proxy construction.
fn initialized_cli_runtime() -> Result<(std::sync::Arc<LocalCoreProxy>, CoreNodeRouter), String> {
    let mut core = create_local_core();
    core.localApplicationMut().onCreate()?;
    {
        let application = core.localApplicationMut();
        let enhanced_ai_service = EnhancedAIService::new(
            application.toolHandler.clone(),
            application.providerRuntimeContext.clone(),
        );
        let mut holder = application
            .chatRuntimeHolder
            .try_lock()
            .map_err(|_| "Chat runtime holder is busy".to_string())?;
        holder.getCore(ChatRuntimeSlot::MAIN).enhancedAiService = Some(enhanced_ai_service);
    }
    let core = std::sync::Arc::new(core);
    let localRuntime = crate::cli::link::local_core_runtime(core.clone());
    operit_node_runtime::RuntimeRemoteLinkService::RuntimeRemoteLinkService::new(
        localRuntime.clone(),
    )
    .startSpaceSync()?;
    let _router = CoreNodeRouter::new(localRuntime);
    Ok((core, _router))
}

impl CliCore {
    /// Returns the host context owned by an in-process CLI runtime.
    pub(crate) fn localHostManager(&self) -> Result<&HostManager, String> {
        self.localHostManager
            .as_ref()
            .ok_or_else(|| "this CLI command requires an in-process runtime".to_string())
    }
}

impl Deref for CliCore {
    type Target = GeneratedCoreProxy<SharedLocalCore>;

    fn deref(&self) -> &Self::Target {
        &self.proxy
    }
}

impl DerefMut for CliCore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.proxy
    }
}
