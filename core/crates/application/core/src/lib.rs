#![allow(non_snake_case)]

use std::sync::Arc;

use operit_access_runtime::{LinkAccessIdentity, LinkAccessStore, RemoteDeviceInfo};
#[cfg(not(target_arch = "wasm32"))]
use operit_access_runtime::{RemoteLinkServer, RemoteLinkServerConfig, RemoteWebAccessConfig};
use operit_host_api::HostManager::HostManager;
use operit_node_runtime::{
    CoreNodeRouter::{CoreNodeLocalRuntime, CoreNodeRouter},
    RuntimeRemoteLinkService::RuntimeRemoteLinkService,
};
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;

type LocalClientConfigurator = Box<dyn FnOnce(&mut LocalCoreProxy) -> Result<(), String> + Send>;

/// Contains the host-provided inputs needed to start one Core tree.
pub struct CoreApplicationConfig {
    pub hostManager: HostManager,
    pub deviceInfo: RemoteDeviceInfo,
    localClientConfigurator: Option<LocalClientConfigurator>,
}

impl CoreApplicationConfig {
    /// Creates a Core application config from host capabilities and device identity.
    pub fn new(hostManager: HostManager, deviceInfo: RemoteDeviceInfo) -> Self {
        Self {
            hostManager,
            deviceInfo,
            localClientConfigurator: None,
        }
    }

    /// Adds a setup hook that runs before the local client is shared by the Core tree.
    #[allow(non_snake_case)]
    pub fn withLocalClientConfigurator(
        mut self,
        configurator: impl FnOnce(&mut LocalCoreProxy) -> Result<(), String> + Send + 'static,
    ) -> Self {
        self.localClientConfigurator = Some(Box::new(configurator));
        self
    }
}

/// Describes the remote Link server owned by one Core application.
#[cfg(not(target_arch = "wasm32"))]
pub struct CoreRemoteLinkServerConfig {
    pub bindAddress: String,
    pub token: String,
    pub webAccess: Option<RemoteWebAccessConfig>,
    pub printStartupInfo: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl CoreRemoteLinkServerConfig {
    /// Creates the remote Link server config used by a Core application.
    pub fn new(bindAddress: String, token: String) -> Self {
        Self {
            bindAddress,
            token,
            webAccess: None,
            printStartupInfo: true,
        }
    }

    /// Attaches a Web Access surface to the remote Link server.
    #[allow(non_snake_case)]
    pub fn withWebAccess(mut self, webAccess: RemoteWebAccessConfig) -> Self {
        self.webAccess = Some(webAccess);
        self
    }

    /// Sets whether the server prints startup information.
    #[allow(non_snake_case)]
    pub fn withStartupInfo(mut self, printStartupInfo: bool) -> Self {
        self.printStartupInfo = printStartupInfo;
        self
    }
}

/// Owns the running Core tree and exposes narrow handles to host surfaces.
pub struct CoreApplication {
    localClient: Arc<LocalCoreProxy>,
    nodeRuntime: CoreNodeLocalRuntime,
    nodeRouter: CoreNodeRouter,
    accessStore: LinkAccessStore,
    accessIdentity: LinkAccessIdentity,
    accessServices: RuntimeRemoteLinkService,
}

impl CoreApplication {
    /// Starts one Core tree from explicit host and access configuration.
    pub async fn start(config: CoreApplicationConfig) -> Result<Self, String> {
        let mut runtimeApplication = OperitApplication::newWithContext(config.hostManager);
        runtimeApplication.onCreate()?;
        let mut localClient = LocalCoreProxy::new(runtimeApplication);
        if let Some(configurator) = config.localClientConfigurator {
            configurator(&mut localClient)?;
        }
        Self::startWithLocalClient(localClient, config.deviceInfo)
    }

    /// Starts one Core tree from a configured local client owned by the caller until this point.
    #[allow(non_snake_case)]
    pub fn startWithLocalClient(
        localClient: LocalCoreProxy,
        deviceInfo: RemoteDeviceInfo,
    ) -> Result<Self, String> {
        Self::startWithSharedLocalClient(Arc::new(localClient), deviceInfo)
    }

    /// Starts one Core tree from a shared local client handle.
    #[allow(non_snake_case)]
    pub fn startWithSharedLocalClient(
        localClient: Arc<LocalCoreProxy>,
        deviceInfo: RemoteDeviceInfo,
    ) -> Result<Self, String> {
        let nodeRuntime = localClient.coreNodeLocalRuntime();
        let nodeRouter = CoreNodeRouter::new(nodeRuntime.clone());
        let accessStore = LinkAccessStore::new(nodeRuntime.runtimeStorageHost());
        let accessIdentity = accessStore.initializeIdentity(deviceInfo)?;
        let accessServices = RuntimeRemoteLinkService::newWithAccessStore(
            nodeRuntime.clone(),
            nodeRouter.clone(),
            accessStore.clone(),
        );
        let routeChangeServices = accessServices.clone();
        localClient.bindCoreRouteChangeHandler(Arc::new(move |chatId, targetNodeId| {
            let services = routeChangeServices.clone();
            Box::pin(async move { services.requestChangeRoute(chatId, targetNodeId).await })
        }))?;
        accessServices.startSpaceSync()?;
        Ok(Self {
            localClient,
            nodeRuntime,
            nodeRouter,
            accessStore,
            accessIdentity,
            accessServices,
        })
    }

    /// Returns the generated local Core client entry point.
    pub fn localClient(&self) -> Arc<LocalCoreProxy> {
        self.localClient.clone()
    }

    /// Returns the local runtime capability handle owned by the node tree.
    pub fn nodeRuntime(&self) -> CoreNodeLocalRuntime {
        self.nodeRuntime.clone()
    }

    /// Returns the router that owns Rust route dispatch for this Core tree.
    pub fn nodeRouter(&self) -> CoreNodeRouter {
        self.nodeRouter.clone()
    }

    /// Returns the Link Access store owned by this Core tree.
    pub fn accessStore(&self) -> LinkAccessStore {
        self.accessStore.clone()
    }

    /// Returns the initialized Link Access identity for this Core tree.
    pub fn accessIdentity(&self) -> &LinkAccessIdentity {
        &self.accessIdentity
    }

    /// Updates the Link Access device information owned by this Core tree.
    #[allow(non_snake_case)]
    pub fn updateAccessIdentity(
        &mut self,
        deviceInfo: RemoteDeviceInfo,
    ) -> Result<LinkAccessIdentity, String> {
        let accessIdentity = self.accessStore.updateIdentityDeviceInfo(deviceInfo)?;
        self.accessIdentity = accessIdentity.clone();
        Ok(accessIdentity)
    }

    /// Returns the Access service facade owned by this Core tree.
    pub fn accessServices(&self) -> RuntimeRemoteLinkService {
        self.accessServices.clone()
    }

    /// Serves the application-owned authenticated remote Link endpoint.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(non_snake_case)]
    pub async fn serveRemoteLink(&self, config: CoreRemoteLinkServerConfig) -> Result<(), String> {
        RemoteLinkServer::serve(
            self.nodeRouter.clone(),
            RemoteLinkServerConfig {
                bindAddress: config.bindAddress,
                token: config.token,
                deviceId: self.accessIdentity.deviceId.clone(),
                deviceInfo: self.accessIdentity.deviceInfo.clone(),
                webAccess: config.webAccess,
                printStartupInfo: config.printStartupInfo,
                accessStore: self.accessStore.clone(),
            },
        )
        .await
    }

    /// Stops application-owned global route state.
    pub async fn shutdown(self) {
        self.shutdownNow();
    }

    /// Stops application-owned global route state from a synchronous host boundary.
    #[allow(non_snake_case)]
    pub fn shutdownNow(self) {
        operit_link::clearCoreRouteRuntime();
    }
}
