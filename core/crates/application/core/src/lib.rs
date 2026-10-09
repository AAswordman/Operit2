#![allow(non_snake_case)]

use std::sync::Arc;
use operit_node_runtime::NodeServices::NodeServices;

use operit_link::protocol::LinkDeviceInfo;
use operit_host_api::HostManager::HostManager;
use operit_host_api::PluginSdkIpc::PluginSdkIpcEndpoint;
use operit_node_runtime::{
    CoreNodeRouter::{CoreNodeLocalRuntime, CoreNodeRouter},
    RuntimeRemoteLinkService::RuntimeRemoteLinkService,
};
use operit_proxy_local::LocalCoreProxy;
use operit_plugin_sdk_ipc::PluginSdkLinkTarget;
use operit_plugin_sdk_ipc_bridge::OperitPluginSdkIpcBridge;
use operit_runtime::core::application::OperitApplication::OperitApplication;

mod edge_tools;

type LocalClientConfigurator = Box<dyn FnOnce(&mut LocalCoreProxy) -> Result<(), String> + Send>;

/// Contains the host-provided inputs needed to start one Core tree.
pub struct CoreApplicationConfig {
    pub hostManager: HostManager,
    pub deviceInfo: LinkDeviceInfo,
    startSpaceSync: bool,
    localClientConfigurator: Option<LocalClientConfigurator>,
    nodeServices: Option<NodeServices>,
}

impl CoreApplicationConfig {
    /// Creates a Core application config from host capabilities and device identity.
    pub fn new(hostManager: HostManager, deviceInfo: LinkDeviceInfo) -> Self {
        Self {
            hostManager,
            deviceInfo,
            startSpaceSync: true,
            localClientConfigurator: None,
            nodeServices: None,
        }
    }

    /// 注入应用、CLI 和 Router 共用的节点服务，不在外围创建协议实现。
    pub fn withNodeServices(mut self, services: NodeServices) -> Self {
        self.nodeServices = Some(services);
        self
    }

    /// Selects whether this Core application owns the persistent Space synchronizer.
    #[allow(non_snake_case)]
    pub fn withSpaceSync(mut self, enabled: bool) -> Self {
        self.startSpaceSync = enabled;
        self
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

/// Owns the running Core tree and exposes narrow handles to host surfaces.
pub struct CoreApplication {
    localClient: Arc<LocalCoreProxy>,
    nodeRuntime: CoreNodeLocalRuntime,
    nodeRouter: Arc<CoreNodeRouter>,
    deviceInfo: LinkDeviceInfo,
    startSpaceSync: bool,
    accessServices: RuntimeRemoteLinkService,
    pluginSdkIpcBridge: Option<OperitPluginSdkIpcBridge>,
}

impl CoreApplication {
    /// Starts one Core tree from explicit host and access configuration.
    pub async fn start(config: CoreApplicationConfig) -> Result<Self, String> {
        let startSpaceSync = config.startSpaceSync;
        let nodeServices = config.nodeServices;
        let mut runtimeApplication = OperitApplication::newWithContext(config.hostManager);
        runtimeApplication.onCreate()?;
        let mut localClient = LocalCoreProxy::new(runtimeApplication);
        if let Some(configurator) = config.localClientConfigurator {
            configurator(&mut localClient)?;
        }
        let mut application = Self::startWithSharedLocalClientConfigured(
            Arc::new(localClient), config.deviceInfo, startSpaceSync,
        )?;
        let services = match nodeServices {
            Some(services) => services,
            None => NodeServices::new(operit_node_runtime::HostRuntimePeerService::HostRuntimePeerService::new(
                Arc::new(application.localClient.hostManager().clone()), &application.nodeRouter, application.deviceInfo.clone())?),
        };
        application.installNodeServices(services)?;
        Ok(application)
    }

    /// Starts one Core tree from a configured local client owned by the caller until this point.
    #[allow(non_snake_case)]
    pub fn startWithLocalClient(
        localClient: LocalCoreProxy,
        deviceInfo: LinkDeviceInfo,
    ) -> Result<Self, String> {
        Self::startWithSharedLocalClient(Arc::new(localClient), deviceInfo)
    }

    /// Starts one Core tree from a shared local client handle.
    #[allow(non_snake_case)]
    pub fn startWithSharedLocalClient(
        localClient: Arc<LocalCoreProxy>,
        deviceInfo: LinkDeviceInfo,
    ) -> Result<Self, String> {
        let mut application = Self::startWithSharedLocalClientConfigured(localClient, deviceInfo, true)?;
        let peers = operit_node_runtime::HostRuntimePeerService::HostRuntimePeerService::new(
            Arc::new(application.localClient.hostManager().clone()), &application.nodeRouter, application.deviceInfo.clone())?;
        application.installNodeServices(NodeServices::new(peers))?;
        Ok(application)
    }

    /// Starts one Core tree while explicitly selecting whether its persistence worker is owned here.
    #[allow(non_snake_case)]
    fn startWithSharedLocalClientConfigured(
        localClient: Arc<LocalCoreProxy>,
        deviceInfo: LinkDeviceInfo,
        startSpaceSync: bool,
    ) -> Result<Self, String> {
        let nodeRuntime = localClient.coreNodeLocalRuntime();
        let nodeRouter = Arc::new(CoreNodeRouter::new(nodeRuntime.clone()));
        // Reduced/test hosts may omit asynchronous hardware support. Do not
        // break Core startup or invent a global scheduler; port calls then fail explicitly.
        if let Some(scheduler) = localClient.hostManager().hostRuntimeTaskSchedulerHost.clone() {
            localClient.bindEdgeToolRuntime(Arc::new(edge_tools::CoreEdgeToolRuntime::new(Arc::downgrade(&nodeRouter), scheduler)))?;
        }
        let accessServices = RuntimeRemoteLinkService::newWithRouter(
            nodeRuntime.clone(), (*nodeRouter).clone(),
        );
        let deviceInfo = accessServices.initializeDeviceInfo(deviceInfo)?;
        let routeChangeServices = accessServices.clone();
        localClient.bindCoreRouteChangeHandler(Arc::new(
            move |chatId, targetNodeId, resumeContext| {
                let services = routeChangeServices.clone();
                Box::pin(async move {
                    services
                        .requestChangeRoute(chatId, targetNodeId, resumeContext)
                        .await
                })
            },
        ))?;
        let pluginSdkIpcBridge = localClient
            .hostManager()
            .pluginSdkIpcHost
            .clone()
            .map(|host| {
                let target: Arc<dyn PluginSdkLinkTarget> = nodeRouter.clone();
                OperitPluginSdkIpcBridge::new(
                    host,
                    PluginSdkIpcEndpoint::standard(),
                    target,
                    operit_proxy_local::pluginSdkSurface(),
                )
            });
        if let Some(bridge) = &pluginSdkIpcBridge {
            bridge.start().map_err(|error| error.to_string())?;
        }
        Ok(Self {
            localClient,
            nodeRuntime,
            nodeRouter,
            deviceInfo,
            startSpaceSync,
            accessServices,
            pluginSdkIpcBridge,
        })
    }

    /// 在启动装配或宿主注入阶段调用；Router 和外围共享同一个服务实例。
    pub fn installNodeServices(&mut self, services: NodeServices) -> Result<(), String> {
        self.nodeRouter.installNodeServices(services)?;
        // 同步必须在通信服务注入后启动；不能先启动旧连接管理器填补空缺。
        if self.startSpaceSync { self.accessServices.startSpaceSync()?; }
        Ok(())
    }

    /// 未装配核心时明确报错，不把缺失服务伪装成配对成功。
    pub fn nodeServices(&self) -> Result<NodeServices, String> {
        self.nodeRouter.nodeServices().cloned()
    }

    /// 本机管理入口，不能注册为远端 Link Call，也不能出现在普通状态快照中。
    pub fn localPairingToken(&self) -> Result<String, String> {
        operit_node_runtime::PeerStateStore::PeerStateStore::new(self.nodeRuntime.runtimeStorageHost())
            .localPairingToken()
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
        (*self.nodeRouter).clone()
    }

    /// 设备资料和稳定节点身份不再由旧配对仓库持有。
    pub fn deviceInfo(&self) -> &LinkDeviceInfo { &self.deviceInfo }

    pub fn localNodeId(&self) -> String { self.nodeRouter.localNodeId() }

    pub fn updateDeviceInfo(&mut self, info: LinkDeviceInfo) -> Result<LinkDeviceInfo, String> {
        self.deviceInfo = self.accessServices.updateDeviceInfo(info)?;
        Ok(self.deviceInfo.clone())
    }

    /// Returns the Access service facade owned by this Core tree.
    pub fn accessServices(&self) -> RuntimeRemoteLinkService {
        self.accessServices.clone()
    }

    /// Stops application-owned global route state.
    pub async fn shutdown(self) {
        let _ = self.accessServices.stopSpaceSync();
        if let Ok(services) = self.nodeServices() {
            let _ = services.peers().stop().await;
        }
        self.shutdownNow();
    }

    /// Stops application-owned global route state from a synchronous host boundary.
    #[allow(non_snake_case)]
    pub fn shutdownNow(self) {
        if let Some(bridge) = &self.pluginSdkIpcBridge {
            let _ = bridge.stop();
        }
        let _ = self.accessServices.stopSpaceSync();
        operit_link::clearCoreRouteRuntime();
    }
}

impl Drop for CoreApplication {
    /// Releases process-local synchronization ownership when a Core tree is dropped.
    fn drop(&mut self) {
        if let Some(bridge) = &self.pluginSdkIpcBridge {
            let _ = bridge.stop();
        }
        let _ = self.accessServices.stopSpaceSync();
    }
}
