use operit_core_application::CoreApplication;
use operit_host_api::HostManager::HostManager;
use operit_proxy_local::{GeneratedCoreProxy, LocalCoreProxy};
use std::ops::{Deref, DerefMut};

use crate::create_cli_core_application;
use async_trait::async_trait;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkClient, CoreLinkError,
    CoreLinkPushSession, CoreLinkSharedClient, CoreWatchRequest,
};

/// Owns a shared local Core proxy while satisfying the generated mutable client surface.
pub(crate) struct SharedLocalCore(pub(crate) std::sync::Arc<LocalCoreProxy>);

#[async_trait(?Send)]
impl CoreLinkClient for SharedLocalCore {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        CoreLinkSharedClient::call(self.0.as_ref(), request).await
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        CoreLinkSharedClient::watchSnapshot(self.0.as_ref(), request).await
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        CoreLinkSharedClient::watch(self.0.as_ref(), request).await
    }

    #[allow(non_snake_case)]
    async fn openPush(
        &mut self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        self.0.openPushLocal(request)
    }
}

pub(crate) struct CliCore {
    proxy: GeneratedCoreProxy<SharedLocalCore>,
    localHostManager: Option<HostManager>,
    _coreApplication: CoreApplication,
}

/// Creates the local CLI Core and sends generated requests directly to its runtime proxy.
pub(crate) async fn local_cli_core() -> Result<CliCore, String> {
    let coreApplication = create_cli_core_application("client").await?;
    let localClient = coreApplication.localClient();
    let localHostManager = localClient.hostManager().clone();
    Ok(CliCore {
        proxy: GeneratedCoreProxy::new(SharedLocalCore(localClient)),
        localHostManager: Some(localHostManager),
        _coreApplication: coreApplication,
    })
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
