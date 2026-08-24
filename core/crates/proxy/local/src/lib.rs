#![allow(non_snake_case)]

extern crate self as operit_proxy_local;

use async_trait::async_trait;
use operit_proxy_bridge::{LocalApplicationBridgeTarget, LocalApplicationSharedClient};
use operit_host_api::HostManager::HostManager;
use operit_host_api::{FileSystemHost, RuntimeStorageHost};
pub use operit_rslink_runtime::{CoreReverseStreamSession, CoreStreamPool};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreHandoffRequest, CoreLinkError, CoreLinkPushSession, CoreLinkSharedClient, CoreValue,
    CoreWatchRequest,
};
use operit_tools::runtime_support::{CoreNodeToolRuntime, ToolRuntimeSupport};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use std::sync::Arc;
use tokio::sync::Mutex;

include!(concat!(env!("OUT_DIR"), "/generated_core_dispatch.rs"));

#[derive(Clone)]
pub struct LocalCoreProxy {
    application: Arc<Mutex<OperitApplication>>,
    chatRuntimeHolder: Arc<tokio::sync::Mutex<ChatRuntimeHolder>>,
    hostManager: HostManager,
    toolRuntimeSupport: Arc<dyn ToolRuntimeSupport>,
    coreStreamPool: Arc<CoreStreamPool>,
}

impl LocalCoreProxy {
    /// Creates the attachment sink used by generated Flow and State watchers.
    fn streamAttachmentAdopter(
        &self,
    ) -> Arc<dyn Fn(Vec<operit_link::CoreStreamAttachment>) + Send + Sync> {
        let pool = self.coreStreamPool.clone();
        Arc::new(move |attachments| {
            pool.adoptAll(attachments);
        })
    }

    /// Returns the host manager captured by this local proxy.
    pub fn hostManager(&self) -> &HostManager {
        &self.hostManager
    }
    /// Resolves one generated schema key to its process-local numeric object id.
    #[allow(non_snake_case)]
    pub fn generatedObjectIdForSchema(schema: &str) -> Option<u32> {
        generated_object_id_for_schema(schema)
    }

    /// Returns the generated local object ID for one concrete runtime type.
    pub fn generatedObjectIdForType(typeName: &str) -> Option<u32> {
        generated_object_id_for_type(typeName)
    }

    /// Installs the server-side CoreNode tool capability without taking the application dispatch lock.
    #[allow(non_snake_case)]
    pub fn bindCoreNodeToolRuntime(
        &self,
        runtime: Arc<dyn CoreNodeToolRuntime>,
    ) -> Result<(), CoreLinkError> {
        self.toolRuntimeSupport
            .bindCoreNodeToolRuntime(runtime)
            .map_err(CoreLinkError::internal)
    }

    /// Opens one caller-owned input stream directly on this local Core proxy.
    #[allow(non_snake_case)]
    pub fn openPushLocal(
        &self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        Ok(Box::new(self.openReverseStream(request)?))
    }

    /// Reports whether a push request is a generated reverse-stream method.
    #[allow(non_snake_case)]
    pub fn isReverseStreamRequest(&self, request: &operit_link::CorePushRequest) -> bool {
        generated_is_reverse_stream_request(request)
    }
    /// Opens one generated reverse stream selected by its proxy schema declaration.
    #[allow(non_snake_case)]
    pub fn openReverseStream(
        &self,
        request: operit_link::CorePushRequest,
    ) -> Result<CoreReverseStreamSession, CoreLinkError> {
        generated_open_reverse_stream(self, request)
    }
    /// Creates a local link client backed by an in-process application.
    pub fn new(application: OperitApplication) -> Self {
        let toolRuntimeSupport = application.toolHandler.runtimeSupport();
        let chatRuntimeHolder = application.chatRuntimeHolder.clone();
        Self {
            hostManager: application.hostManager.clone(),
            toolRuntimeSupport,
            application: Arc::new(Mutex::new(application)),
            chatRuntimeHolder,
            coreStreamPool: Arc::new(CoreStreamPool::new()),
        }
    }

    /// Returns mutable access to the hosted local application.
    #[allow(non_snake_case)]
    pub fn localApplicationMut(&mut self) -> &mut OperitApplication {
        Arc::get_mut(&mut self.application)
            .expect("LocalCoreProxy application must not be shared while mutating setup")
            .get_mut()
    }

    /// Returns the runtime storage capability owned by this local core.
    #[allow(non_snake_case)]
    pub fn runtimeStorageHost(&self) -> Arc<dyn RuntimeStorageHost> {
        self.hostManager
            .runtimeStorageHost
            .clone()
            .expect("LocalCoreProxy requires a RuntimeStorageHost")
    }

    /// Returns the runtime holder used by generated local server dispatch.
    pub fn chatRuntimeHolder(
        &self,
    ) -> Arc<tokio::sync::Mutex<ChatRuntimeHolder>> {
        self.chatRuntimeHolder.clone()
    }

    /// Creates the server-internal client that dispatches only to the local application object.
    #[allow(non_snake_case)]
    pub fn localApplicationSharedClient(
        &self,
    ) -> Arc<dyn CoreLinkSharedClient + Send + Sync> {
        Arc::new(LocalApplicationSharedClient::new(Arc::new(self.clone()), 0))
    }

    /// Builds the native server capability container for this local Core.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn coreNodeLocalRuntime(
        &self,
    ) -> operit_node_runtime::CoreNodeRouter::CoreNodeLocalRuntime {
        let proxy = Arc::new(self.clone());
        let spaceRuntime = Arc::new(
            operit_node_runtime::SpaceRuntime::SpaceRuntime::new(self.chatRuntimeHolder()),
        );
        let sharedClient: Arc<dyn CoreLinkSharedClient + Send + Sync> = proxy.clone();
        let applicationClient = self.localApplicationSharedClient();
        let bindCoreNodeToolRuntime = {
            let proxy = proxy.clone();
            Arc::new(move |runtime| proxy.bindCoreNodeToolRuntime(runtime))
        };
        let handoffAtBoundary = {
            let spaceRuntime = spaceRuntime.clone();
            Arc::new(move |request| {
                let spaceRuntime = spaceRuntime.clone();
                Box::pin(async move { spaceRuntime.handoffAtBoundaryLocal(request).await })
                    as std::pin::Pin<
                        Box<dyn std::future::Future<
                            Output = Result<
                                operit_link::CoreHandoffResponse,
                                operit_link::CoreLinkError,
                            >,
                        > + Send>,
                    >
            })
        };
        let openPush = {
            let proxy = proxy.clone();
            Arc::new(move |request| proxy.openPushLocal(request))
        };
        operit_node_runtime::CoreNodeRouter::CoreNodeLocalRuntime::new(
            sharedClient,
            applicationClient,
            self.runtimeStorageHost(),
            Arc::new(LocalCoreProxy::generatedObjectIdForSchema),
            bindCoreNodeToolRuntime,
            handoffAtBoundary,
            openPush,
            spaceRuntime,
        )
    }

    /// Returns the file-system capability owned by this local core.
    #[allow(non_snake_case)]
    pub fn fileSystemHost(&self) -> Arc<dyn FileSystemHost> {
        self.hostManager
            .fileSystemHost
            .clone()
            .expect("LocalCoreProxy requires a FileSystemHost")
    }
}

#[async_trait(?Send)]
impl CoreLinkClient for LocalCoreProxy {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        CoreLinkSharedClient::call(self, request).await
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        CoreLinkSharedClient::watchSnapshot(self, request).await
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        CoreLinkSharedClient::watch(self, request).await
    }

    #[allow(non_snake_case)]
    async fn openPush(
        &mut self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        self.openPushLocal(request)
    }
}

#[async_trait(?Send)]
impl CoreLinkSharedClient for LocalCoreProxy {
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let result = operit_link::withCoreForceLocal(self.dispatchCall(request)).await;
        match result {
            Ok(value) => CoreCallResponse::ok(requestId, value),
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        let (result, attachments) = operit_link::withCoreForceLocal(
            operit_link::withCoreStreamCapture(
                generated_dispatch_core_proxy_watch_snapshot_async(self, request),
            ),
        )
        .await;
        self.adoptCoreStreamAttachments(attachments);
        result
    }

    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.targetObjectId == operit_link::CORE_STREAM_POOL_OBJECT_ID {
            return self.openCoreStreamWatch(request);
        }
        operit_link::withCoreForceLocal(generated_dispatch_core_proxy_watch_async(self, request))
            .await
    }
}

#[async_trait(?Send)]
impl LocalApplicationBridgeTarget for LocalCoreProxy {
    /// Dispatches one application-owned call without re-entering server service objects.
    async fn callLocalApplication(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let result = {
            let mut application = self.application.lock().await;
            generated_dispatch_application_call(&mut application, request).await
        };
        match result {
            Ok(value) => CoreCallResponse::ok(requestId, value),
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    /// Reads one application-owned watch snapshot without entering the proxy dispatcher.
    #[allow(non_snake_case)]
    async fn watchLocalApplicationSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        let propertyName = request.propertyName.clone();
        let targetObjectId = request.targetObjectId;
        let mut application = self.application.lock().await;
        let value = generated_dispatch_application_watch_snapshot(&mut application, &request)?;
        Ok(CoreEvent {
            requestId: Some(request.requestId),
            targetObjectId,
            propertyName,
            kind: CoreEventKind::Snapshot,
            value,
        })
    }

    /// Opens one application-owned watch without entering the proxy dispatcher.
    async fn watchLocalApplication(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let mut application = self.application.lock().await;
        generated_dispatch_application_watch(
            &mut application,
            request,
            self.streamAttachmentAdopter(),
        )
    }
}

impl LocalCoreProxy {
    #[allow(non_snake_case)]
    async fn dispatchCall(&self, request: CoreCallRequest) -> Result<CoreValue, CoreLinkError> {
        let (result, attachments) = operit_link::withCoreStreamCapture(
            generated_dispatch_core_proxy_call(self, request),
        )
        .await;
        self.adoptCoreStreamAttachments(attachments);
        result
    }

    /// Executes a watch snapshot through the generated synchronous dispatcher.
    #[allow(non_snake_case)]
    pub fn watchSnapshotSync(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        self.dispatchWatchSnapshot(request)
    }

    /// Opens a watch stream through the generated synchronous dispatcher.
    #[allow(non_snake_case)]
    pub fn watchSync(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.dispatchWatch(request)
    }

    #[allow(non_snake_case)]
    fn dispatchWatchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        let (result, attachments) = operit_link::withCoreStreamCaptureSync(|| {
            generated_dispatch_core_proxy_watch_snapshot(self, request)
        });
        self.adoptCoreStreamAttachments(attachments);
        result
    }

    #[allow(non_snake_case)]
    fn dispatchWatch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.targetObjectId == operit_link::CORE_STREAM_POOL_OBJECT_ID {
            return self.openCoreStreamWatch(request);
        }
        generated_dispatch_core_proxy_watch(self, request)
    }

    /// Transfers serialized stream sources into this proxy's owned pool.
    fn adoptCoreStreamAttachments(&self, attachments: Vec<operit_link::CoreStreamAttachment>) {
        self.coreStreamPool.adoptAll(attachments);
    }

    /// Opens one anonymous stream source from the proxy-owned stream pool.
    fn openCoreStreamWatch(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        self.coreStreamPool.openCoreStreamWatch(request)
    }
}
