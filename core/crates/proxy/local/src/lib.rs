#![allow(non_snake_case)]

extern crate self as operit_proxy_local;

use async_trait::async_trait;
use operit_host_api::HostManager::HostManager;
use operit_host_api::{FileSystemHost, RuntimeStorageHost};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkPushSession, CoreLinkSharedClient, CoreValue, CoreWatchRequest,
};
use operit_proxy_bridge::{LocalApplicationBridgeTarget, LocalApplicationSharedClient};
pub use operit_rslink_runtime::{CoreReverseStreamSession, CoreStreamPool};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use operit_tools::runtime_support::{
    CoreNodeToolRuntime, CoreRouteChangeHandler, ToolRuntimeSupport,
};
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

    /// Installs the route change controller used after a completed AI turn.
    #[allow(non_snake_case)]
    pub fn bindCoreRouteChangeHandler(
        &self,
        handler: CoreRouteChangeHandler,
    ) -> Result<(), String> {
        self.toolRuntimeSupport.bindCoreRouteChangeHandler(handler)
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
    pub fn chatRuntimeHolder(&self) -> Arc<tokio::sync::Mutex<ChatRuntimeHolder>> {
        self.chatRuntimeHolder.clone()
    }

    /// Creates the server-internal client that dispatches only to the local application object.
    #[allow(non_snake_case)]
    pub fn localApplicationSharedClient(&self) -> Arc<dyn CoreLinkSharedClient + Send + Sync> {
        Arc::new(LocalApplicationSharedClient::new(Arc::new(self.clone()), 0))
    }

    /// Builds the native server capability container for this local Core.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn coreNodeLocalRuntime(
        &self,
    ) -> operit_node_runtime::CoreNodeRouter::CoreNodeLocalRuntime {
        let proxy = Arc::new(self.clone());
        let spaceRuntime = Arc::new(operit_node_runtime::SpaceRuntime::SpaceRuntime::new(
            self.chatRuntimeHolder(),
        ));
        let sharedClient: Arc<dyn CoreLinkSharedClient + Send + Sync> = proxy.clone();
        let applicationClient = self.localApplicationSharedClient();
        let bindCoreNodeToolRuntime = {
            let proxy = proxy.clone();
            Arc::new(move |runtime| proxy.bindCoreNodeToolRuntime(runtime))
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
        let result = self.dispatchCall(request).await;
        match result {
            Ok(value) => CoreCallResponse::ok(requestId, value),
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        let (result, attachments) = operit_link::withCoreStreamCapture(
            generated_dispatch_core_proxy_watch_snapshot_async(self, request),
        )
        .await;
        self.adoptCoreStreamAttachments(attachments);
        result
    }

    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.targetObjectId == operit_link::CORE_STREAM_POOL_OBJECT_ID {
            return self.openCoreStreamWatch(request);
        }
        generated_dispatch_core_proxy_watch_async(self, request).await
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
        let (result, attachments) =
            operit_link::withCoreStreamCapture(generated_dispatch_core_proxy_call(self, request))
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

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::HostManager::HostManager;
    use operit_host_native_common::{
        NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost,
        NativeRuntimeStorageHost, PosixFileSystemHost,
    };
    use operit_link::{
        CoreEventKind, CoreStreamAttachment, CoreStreamSource, CORE_STREAM_POOL_OBJECT_ID,
    };
    use operit_util::RuntimeStorageLayout::{RUNTIME_ROOT_DIR_PATH, WORKSPACE_DIR_PATH};
    use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
    use std::collections::BTreeMap;

    /// Verifies generated watch dispatch treats the protocol stream pool id as a stream source.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generated_async_watch_dispatch_opens_core_stream_pool() {
        let mut hostManager = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
        let root = std::env::temp_dir().join(format!(
            "operit-proxy-local-generated-dispatch-{}",
            std::process::id()
        ));
        let runtimeRoot = root.join(RUNTIME_ROOT_DIR_PATH);
        let workspaceRoot = root.join(WORKSPACE_DIR_PATH);
        std::fs::create_dir_all(&runtimeRoot).expect("test runtime root must be created");
        std::fs::create_dir_all(&workspaceRoot).expect("test workspace root must be created");
        setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
            runtimeRoot.clone(),
            workspaceRoot.clone(),
        ));
        let storageHost = Arc::new(NativeRuntimeStorageHost::new(runtimeRoot, workspaceRoot));
        hostManager.runtimeStorageHost = Some(storageHost.clone());
        hostManager.runtimeSqliteHost = Some(storageHost);
        hostManager.hostJavaScriptRuntimeHost =
            Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
        hostManager.hostRuntimeTaskSchedulerHost =
            Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
        let proxy = LocalCoreProxy::new(OperitApplication::newWithContext(hostManager));
        let streamId = "generated-dispatch-core-stream".to_string();
        let source = CoreStreamSource::new(|request| {
            let (sender, receiver) = operit_rslink_runtime::core_event_stream_channel();
            sender
                .send(CoreEvent {
                    requestId: Some(request.requestId.clone()),
                    targetObjectId: request.targetObjectId,
                    propertyName: request.propertyName.clone(),
                    kind: CoreEventKind::Changed,
                    value: CoreValue::String("stream-pool-opened".to_string()),
                })
                .expect("test stream event receiver must stay open");
            sender
                .send(CoreEvent {
                    requestId: Some(request.requestId),
                    targetObjectId: request.targetObjectId,
                    propertyName: request.propertyName,
                    kind: CoreEventKind::Completed,
                    value: CoreValue::Null,
                })
                .expect("test stream completion receiver must stay open");
            Ok(receiver)
        });
        proxy.adoptCoreStreamAttachments(vec![CoreStreamAttachment {
            streamId: streamId.clone(),
            source: Arc::new(source),
        }]);
        let mut args = BTreeMap::new();
        args.insert("streamId".to_string(), CoreValue::String(streamId));

        let mut stream = generated_dispatch_core_proxy_watch_async(
            &proxy,
            CoreWatchRequest::new(
                "generated-dispatch-core-stream-watch",
                CORE_STREAM_POOL_OBJECT_ID,
                "openCoreStream",
                CoreValue::Map(args),
            ),
        )
        .await
        .expect("generated dispatch must open the core stream pool");
        let event = stream
            .recv()
            .await
            .expect("generated dispatch stream must produce one event");

        assert_eq!(event.kind, CoreEventKind::Changed);
        assert_eq!(
            event.value,
            CoreValue::String("stream-pool-opened".to_string())
        );
    }
}
