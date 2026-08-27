use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

static SHARED_CONCURRENCY_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    BrowserSessionCommand, BrowserSessionCommandResult, BrowserSessionHost, BrowserSessionInfo,
    BrowserSessionSnapshot, HostResult, HostRuntimeTaskSchedulerHost, HostSecretStore,
    RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeStorageEntry, RuntimeStorageHost,
};
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeRuntimeStorageHost,
    PosixFileSystemHost,
};
use operit_link::{
    fromCoreValue, toCoreValue, CoreCallRequest, CoreEvent, CoreEventKind, CoreEventStream,
    CoreLinkSharedClient, CoreStream, CoreStreamSource, CoreValue, CoreWatchRequest,
    CORE_STREAM_POOL_OBJECT_ID,
};
use operit_model::ChatMessage::ChatMessage;
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::services::RuntimeHostInteractionService::{
    requestOwnerBrowserSession, RuntimeHostInteractionBrowserSessionPayload,
};
use operit_util::MarkdownRenderStream::MarkdownStreamEvent;
use operit_util::RuntimeStorageLayout::{RUNTIME_ROOT_DIR_PATH, WORKSPACE_DIR_PATH};
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
use serde_json::{json, Value};

struct OwnerInteractionBrowserHost;

#[derive(Default)]
struct TestSecretStore {
    values: Mutex<BTreeMap<String, Vec<u8>>>,
}

struct TestRuntimeStorageHost {
    inner: NativeRuntimeStorageHost,
}

impl TestRuntimeStorageHost {
    /// Creates a test storage host around the native storage implementation.
    fn new(runtime_root: std::path::PathBuf, workspace_root: std::path::PathBuf) -> Self {
        Self {
            inner: NativeRuntimeStorageHost::new(runtime_root, workspace_root),
        }
    }
}

impl RuntimeStorageHost for TestRuntimeStorageHost {
    /// Returns the physical test runtime root.
    fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
        self.inner.runtimeRootDir()
    }

    /// Returns the physical test workspace root.
    fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
        self.inner.workspaceRootDir()
    }

    /// Reads bytes from the mapped test path.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.inner.readBytes(path)
    }

    /// Writes bytes to the mapped test path.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.writeBytes(path, content)
    }

    /// Appends bytes to the mapped test path.
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.inner.appendBytes(path, content)
    }

    /// Deletes an entry at the mapped test path.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.inner.delete(path, recursive)
    }

    /// Checks an entry at the mapped test path.
    fn exists(&self, path: &str) -> HostResult<bool> {
        self.inner.exists(path)
    }

    /// Lists entries below the mapped test path.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        self.inner.list(prefix)
    }
}

impl RuntimeSqliteHost for TestRuntimeStorageHost {
    /// Opens a SQLite database through the native test host.
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.inner.openSqliteDatabase(path)
    }
}

impl HostSecretStore for TestSecretStore {
    /// Reads one test secret from process memory.
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        Ok(self
            .values
            .lock()
            .expect("test secret store mutex must remain valid")
            .get(key)
            .cloned())
    }

    /// Writes one test secret into process memory.
    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        self.values
            .lock()
            .expect("test secret store mutex must remain valid")
            .insert(key.to_string(), content.to_vec());
        Ok(())
    }

    /// Deletes one test secret from process memory.
    fn deleteSecret(&self, key: &str) -> HostResult<()> {
        self.values
            .lock()
            .expect("test secret store mutex must remain valid")
            .remove(key);
        Ok(())
    }
}

impl BrowserSessionHost for OwnerInteractionBrowserHost {
    /// Lists sessions by requesting the runtime owner app through Core interaction.
    fn listBrowserSessions(&self) -> HostResult<Vec<BrowserSessionInfo>> {
        let response = requestOwnerBrowserSession(
            RuntimeHostInteractionBrowserSessionPayload {
                commandJson: json!({
                    "action": "list",
                    "sessionId": null,
                    "url": null,
                    "script": null,
                    "payloadJson": "",
                    "userAgent": null,
                    "headers": {}
                })
                .to_string(),
            },
            Duration::from_secs(2),
        )
        .expect("owner browser response must arrive");
        let result: BrowserSessionCommandResult =
            serde_json::from_str(&response.resultJson).expect("browser result must decode");
        Ok(result.sessions)
    }

    /// Rejects session creation because this test only exercises listing.
    fn createBrowserSession(
        &self,
        _initialUrl: &str,
        _userAgent: Option<&str>,
        _headers: BTreeMap<String, String>,
    ) -> HostResult<BrowserSessionInfo> {
        panic!("createBrowserSession is not used by this test")
    }

    /// Rejects session updates because this test only exercises listing.
    fn updateBrowserSession(
        &self,
        _sessionId: &str,
        _userAgent: Option<&str>,
        _headers: BTreeMap<String, String>,
    ) -> HostResult<BrowserSessionInfo> {
        panic!("updateBrowserSession is not used by this test")
    }

    /// Rejects semantic commands because this test only exercises listing.
    fn submitBrowserCommand(
        &self,
        _command: BrowserSessionCommand,
    ) -> HostResult<BrowserSessionCommandResult> {
        panic!("submitBrowserCommand is not used by this test")
    }

    /// Rejects snapshots because this test only exercises listing.
    fn getBrowserSessionSnapshot(&self, _sessionId: &str) -> HostResult<BrowserSessionSnapshot> {
        panic!("getBrowserSessionSnapshot is not used by this test")
    }

    /// Rejects session closing because this test only exercises listing.
    fn closeBrowserSession(&self, _sessionId: &str) -> HostResult<BrowserSessionCommandResult> {
        panic!("closeBrowserSession is not used by this test")
    }
}

/// Builds an empty successful browser-session command result.
fn browser_result_json() -> String {
    json!({
        "success": true,
        "session": null,
        "sessions": [],
        "resultJson": "",
        "error": null
    })
    .to_string()
}

/// Registers isolated runtime roots required by application service constructors.
fn register_test_runtime_roots() -> Arc<TestRuntimeStorageHost> {
    let root = std::env::temp_dir().join(format!(
        "operit-proxy-local-shared-concurrency-{}",
        std::process::id()
    ));
    let runtime_root = root.join(RUNTIME_ROOT_DIR_PATH);
    let workspace_root = root.join(WORKSPACE_DIR_PATH);
    std::fs::create_dir_all(&runtime_root).expect("test runtime root must be created");
    std::fs::create_dir_all(&workspace_root).expect("test workspace root must be created");
    setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
        runtime_root.clone(),
        workspace_root.clone(),
    ));
    Arc::new(TestRuntimeStorageHost::new(runtime_root, workspace_root))
}

/// Creates a CoreStream source that emits one Markdown chunk.
fn proxy_test_markdown_stream_source(chat_id: String, text: String) -> Arc<CoreStreamSource> {
    Arc::new(CoreStreamSource::new(move |request| {
        let (sender, receiver) = CoreEventStream::channel();
        let event = MarkdownStreamEvent {
            chatId: chat_id.clone(),
            eventType: "chunk".to_string(),
            value: Some(text.clone()),
            id: None,
            blockId: None,
            inlineId: None,
            parentBlockId: None,
            nodeType: None,
            headerLevel: None,
        };
        let value = toCoreValue(event).expect("Markdown stream event must encode");
        let _ = sender.send(CoreEvent {
            requestId: Some(request.requestId.clone()),
            targetObjectId: request.targetObjectId,
            propertyName: request.propertyName.clone(),
            kind: CoreEventKind::Changed,
            value,
        });
        let _ = sender.send(CoreEvent {
            requestId: Some(request.requestId),
            targetObjectId: request.targetObjectId,
            propertyName: request.propertyName,
            kind: CoreEventKind::Completed,
            value: CoreValue::Null,
        });
        Ok(receiver)
    }))
}

/// Receives one event from a Core stream within a short test deadline.
async fn receive_proxy_test_event(stream: &mut CoreEventStream) -> CoreEvent {
    tokio::time::timeout(Duration::from_millis(500), stream.recv())
        .await
        .expect("proxy test event must arrive")
        .expect("proxy test stream must stay open")
}

/// Verifies unrelated generated watches never wait for a resolved holder lock.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn preferences_watch_bypasses_unrelated_resolved_holder() {
    let _testGuard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    tokio::task::LocalSet::new()
        .run_until(async {
            let storage_host = register_test_runtime_roots();
            let mut host_manager =
                HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
            host_manager.runtimeStorageHost = Some(storage_host.clone());
            host_manager.runtimeSqliteHost = Some(storage_host);
            host_manager.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
            host_manager.hostJavaScriptRuntimeHost =
                Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
            host_manager.hostRuntimeTaskSchedulerHost =
                Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
            let application = OperitApplication::newWithContext(host_manager);
            let holder = application.chatRuntimeHolder.clone();
            let proxy = LocalCoreProxy::new(application);
            let _holderGuard = holder.lock().await;
            let character_group_manager_id =
                LocalCoreProxy::generatedObjectIdForSchema("preferences.characterGroupCardManager")
                    .expect("character group manager object id must be generated");

            let event = tokio::time::timeout(
                Duration::from_millis(500),
                CoreLinkSharedClient::watchSnapshot(
                    &proxy,
                    CoreWatchRequest::new(
                        "preferences-watch",
                        character_group_manager_id,
                        "allCharacterGroupCardsFlow",
                        toCoreValue(json!({})).unwrap(),
                    ),
                ),
            )
            .await
            .expect("preferences watch must not wait for the chat runtime holder")
            .expect("preferences watch snapshot must succeed");
            let value: Value = fromCoreValue(event.value).expect("watch value must decode");
            assert!(value.is_array());
        })
        .await;
}

/// Verifies an owner response can enter Core while the originating Core call is waiting.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shared_core_accepts_nested_owner_response() {
    let _testGuard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    tokio::task::LocalSet::new()
        .run_until(async {
            let storage_host = register_test_runtime_roots();
            let mut host_manager =
                HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()))
                    .withBrowserSessionHost(Arc::new(OwnerInteractionBrowserHost));
            host_manager.runtimeStorageHost = Some(storage_host.clone());
            host_manager.runtimeSqliteHost = Some(storage_host);
            host_manager.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
            host_manager.hostJavaScriptRuntimeHost =
                Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
            host_manager.hostRuntimeTaskSchedulerHost =
                Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
            let proxy = Arc::new(LocalCoreProxy::new(OperitApplication::newWithContext(
                host_manager,
            )));
            let runtime_host_interaction_service_id = LocalCoreProxy::generatedObjectIdForSchema(
                "services.runtimeHostInteractionService",
            )
            .expect("runtime host interaction service object id must be generated");
            let runtime_browser_service_id =
                LocalCoreProxy::generatedObjectIdForSchema("services.runtimeBrowserService")
                    .expect("runtime browser service object id must be generated");
            let mut owner_events = CoreLinkSharedClient::watch(
                proxy.as_ref(),
                CoreWatchRequest::new(
                    "owner-events",
                    runtime_host_interaction_service_id,
                    "ownerHostInteractionEvents",
                    toCoreValue(json!({"kinds": ["browser_session"]})).unwrap(),
                ),
            )
            .await
            .expect("owner event stream must open");

            let list_proxy = proxy.clone();
            let (list_result_sender, list_result_receiver) = tokio::sync::oneshot::channel();
            let list_thread = std::thread::spawn(move || {
                let scheduler = NativeHostRuntimeTaskSchedulerHost::new();
                let (response_sender, response_receiver) = std::sync::mpsc::channel();
                scheduler
                    .scheduleHostRuntimeAsyncTask(
                        "shared-concurrency-list-call",
                        Box::new(move || {
                            Box::pin(async move {
                                let response = CoreLinkSharedClient::call(
                                    list_proxy.as_ref(),
                                    CoreCallRequest::new(
                                        "list-browser-sessions",
                                        runtime_browser_service_id,
                                        "listBrowserSessions",
                                        toCoreValue(json!({})).unwrap(),
                                    ),
                                )
                                .await;
                                let _ = response_sender.send(response);
                            })
                        }),
                    )
                    .expect("list call runtime task must be scheduled");
                let response = response_receiver
                    .recv()
                    .expect("list call response must be delivered");
                let _ = list_result_sender.send(response);
            });

            let owner_event = tokio::time::timeout(Duration::from_millis(500), owner_events.recv())
                .await
                .expect("owner request must be published without waiting for the list timeout")
                .expect("owner event stream must remain open");
            let owner_request_value: Value = fromCoreValue(owner_event.value).unwrap();
            let owner_request = owner_request_value
                .as_object()
                .expect("owner request must be an object");
            let owner_request_id = owner_request
                .get("requestId")
                .and_then(Value::as_str)
                .expect("owner request id must be present");

            let response = CoreLinkSharedClient::call(
                proxy.as_ref(),
                CoreCallRequest::new(
                    "respond-owner",
                    runtime_host_interaction_service_id,
                    "respondOwnerHostInteraction",
                    toCoreValue(json!({
                        "requestId": owner_request_id,
                        "response": {
                            "browserAutomation": null,
                            "browserSession": {"resultJson": browser_result_json()},
                            "webVisit": null,
                            "composeWebViewController": null,
                            "systemCaptureScreenshot": null,
                            "systemLanguageCode": null,
                            "systemRecognizeText": null,
                            "audioPlay": null,
                            "musicPlayback": null,
                            "bluetooth": null,
                            "ttsSynthesis": null,
                            "ttsPlayback": null,
                            "toolPermission": null
                        }
                    }))
                    .unwrap(),
                ),
            )
            .await;
            assert!(
                response.result.is_ok(),
                "owner response call failed: {response:?}"
            );

            let list_response =
                tokio::time::timeout(Duration::from_millis(500), list_result_receiver)
                    .await
                    .expect("browser list call must complete after the owner response")
                    .expect("browser list thread must return its response");
            list_thread
                .join()
                .expect("browser list thread must not panic");
            assert!(
                list_response.result.is_ok(),
                "browser list call failed: {list_response:?}"
            );
        })
        .await;
}

/// Verifies Flutter-style LocalCoreProxy watches open the local chat Flow.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generated_proxy_chat_messages_flow_opens_local_flow() {
    let _testGuard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    tokio::task::LocalSet::new()
        .run_until(async {
            let storage_host = register_test_runtime_roots();
            let mut host_manager =
                HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
            host_manager.runtimeStorageHost = Some(storage_host.clone());
            host_manager.runtimeSqliteHost = Some(storage_host);
            host_manager.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
            host_manager.hostJavaScriptRuntimeHost =
                Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
            host_manager.hostRuntimeTaskSchedulerHost =
                Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
            let proxy = LocalCoreProxy::new(OperitApplication::newWithContext(host_manager));
            let chat_object_id =
                LocalCoreProxy::generatedObjectIdForSchema("chatRuntimeHolderMain")
                    .expect("chatRuntimeHolderMain object id must be generated");
            let mut messages_stream = CoreLinkSharedClient::watch(
                &proxy,
                CoreWatchRequest::new(
                    "proxy-chat-messages-flow-local",
                    chat_object_id,
                    "chatMessagesFlow",
                    toCoreValue(json!({ "chatId": "proxy-chat-local" })).unwrap(),
                ),
            )
            .await
            .expect("proxy chatMessagesFlow watch must open locally");
            let messages_event = receive_proxy_test_event(&mut messages_stream).await;
            let messages: Vec<ChatMessage> =
                fromCoreValue(messages_event.value).expect("proxy chat messages must decode");
            assert!(messages.is_empty());
        })
        .await;
}

/// Verifies a live local chat Flow can expose a newly inserted embedded message stream.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_chat_messages_flow_update_opens_embedded_stream() {
    let _testGuard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    tokio::task::LocalSet::new()
        .run_until(async {
            let storage_host = register_test_runtime_roots();
            let mut host_manager =
                HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
            host_manager.runtimeStorageHost = Some(storage_host.clone());
            host_manager.runtimeSqliteHost = Some(storage_host);
            host_manager.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
            host_manager.hostJavaScriptRuntimeHost =
                Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
            host_manager.hostRuntimeTaskSchedulerHost =
                Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
            let proxy = LocalCoreProxy::new(OperitApplication::newWithContext(host_manager));
            let chat_object_id =
                LocalCoreProxy::generatedObjectIdForSchema("chatRuntimeHolderMain")
                    .expect("chatRuntimeHolderMain object id must be generated");
            let chat_id = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder
                    .coreForObjectId(chat_object_id)
                    .expect("chat runtime holder main core must exist");
                core.createNewChat(None, None, false, true, None);
                core.chatHistoryDelegate
                    .currentChatIdFlow()
                    .value()
                    .expect("test chat must become current")
            };
            let mut messages_stream = CoreLinkSharedClient::watch(
                &proxy,
                CoreWatchRequest::new(
                    "proxy-local-live-chat-messages-flow",
                    chat_object_id,
                    "chatMessagesFlow",
                    toCoreValue(json!({ "chatId": chat_id.clone() })).unwrap(),
                ),
            )
            .await
            .expect("local proxy chatMessagesFlow watch must open");
            let initial_event = receive_proxy_test_event(&mut messages_stream).await;
            let initial_messages: Vec<ChatMessage> = fromCoreValue(initial_event.value)
                .expect("initial local chat messages must decode");
            assert!(initial_messages.is_empty());

            let stream_id = "proxy-local-live-chat-message-stream";
            let source = proxy_test_markdown_stream_source(
                chat_id.clone(),
                "hello from local live Flow".to_string(),
            );
            let mut message = ChatMessage::new("ai".to_string());
            message.contentStream =
                Some(CoreStream::fromSourceWithId(stream_id.to_string(), source));
            {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder
                    .coreForObjectId(chat_object_id)
                    .expect("chat runtime holder main core must exist");
                core.chatHistoryDelegate
                    .addMessageToChat(message, Some(chat_id.clone()));
            }

            let updated_event = receive_proxy_test_event(&mut messages_stream).await;
            let updated_messages: Vec<ChatMessage> = fromCoreValue(updated_event.value)
                .expect("updated local chat messages must decode");
            let stream = updated_messages
                .iter()
                .find_map(|message| message.contentStream.clone())
                .expect("updated local message must carry a content stream");
            let mut content_stream = CoreLinkSharedClient::watch(
                &proxy,
                CoreWatchRequest::new(
                    "proxy-local-live-chat-content-stream",
                    CORE_STREAM_POOL_OBJECT_ID,
                    "openCoreStream",
                    stream.descriptor.args.clone(),
                ),
            )
            .await
            .expect("updated local embedded content stream must open");
            let chunk = receive_proxy_test_event(&mut content_stream).await;
            let event: MarkdownStreamEvent =
                fromCoreValue(chunk.value).expect("local Markdown event must decode");
            assert_eq!(event.eventType, "chunk");
            assert_eq!(event.value, Some("hello from local live Flow".to_string()));

            let mut completed_message = updated_messages
                .first()
                .expect("first local message must exist")
                .clone();
            completed_message.contentStream = None;
            {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder
                    .coreForObjectId(chat_object_id)
                    .expect("chat runtime holder main core must exist");
                core.chatHistoryDelegate
                    .addMessageToChat(completed_message, Some(chat_id.clone()));
            }
            let completed_event = receive_proxy_test_event(&mut messages_stream).await;
            let completed_messages: Vec<ChatMessage> = fromCoreValue(completed_event.value)
                .expect("completed local chat messages must decode");
            assert!(completed_messages
                .iter()
                .all(|message| message.contentStream.is_none()));

            let second_stream_id = "proxy-local-live-chat-message-stream-second";
            let second_source = proxy_test_markdown_stream_source(
                chat_id.clone(),
                "hello from second local live Flow".to_string(),
            );
            let mut second_message = ChatMessage::new("ai".to_string());
            second_message.contentStream = Some(CoreStream::fromSourceWithId(
                second_stream_id.to_string(),
                second_source,
            ));
            {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder
                    .coreForObjectId(chat_object_id)
                    .expect("chat runtime holder main core must exist");
                core.chatHistoryDelegate
                    .addMessageToChat(second_message, Some(chat_id.clone()));
            }
            let second_event = receive_proxy_test_event(&mut messages_stream).await;
            let second_messages: Vec<ChatMessage> =
                fromCoreValue(second_event.value).expect("second local chat messages must decode");
            let second_stream = second_messages
                .iter()
                .find_map(|message| message.contentStream.clone())
                .expect("second local message must carry a content stream");
            let mut second_content_stream = CoreLinkSharedClient::watch(
                &proxy,
                CoreWatchRequest::new(
                    "proxy-local-live-chat-content-stream-second",
                    CORE_STREAM_POOL_OBJECT_ID,
                    "openCoreStream",
                    second_stream.descriptor.args.clone(),
                ),
            )
            .await
            .expect("second local embedded content stream must open");
            let second_chunk = receive_proxy_test_event(&mut second_content_stream).await;
            let second_event: MarkdownStreamEvent =
                fromCoreValue(second_chunk.value).expect("second Markdown event must decode");
            assert_eq!(second_event.eventType, "chunk");
            assert_eq!(
                second_event.value,
                Some("hello from second local live Flow".to_string())
            );
        })
        .await;
}

/// Verifies live chat Flow stream sources stay attached during an outer async call capture.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_chat_messages_flow_update_inside_async_capture_opens_embedded_stream_immediately() {
    let _testGuard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    tokio::task::LocalSet::new()
        .run_until(async {
            let storage_host = register_test_runtime_roots();
            let mut host_manager =
                HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
            host_manager.runtimeStorageHost = Some(storage_host.clone());
            host_manager.runtimeSqliteHost = Some(storage_host);
            host_manager.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
            host_manager.hostJavaScriptRuntimeHost =
                Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
            host_manager.hostRuntimeTaskSchedulerHost =
                Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
            let proxy = LocalCoreProxy::new(OperitApplication::newWithContext(host_manager));
            let chat_object_id =
                LocalCoreProxy::generatedObjectIdForSchema("chatRuntimeHolderMain")
                    .expect("chatRuntimeHolderMain object id must be generated");
            let chat_id = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder
                    .coreForObjectId(chat_object_id)
                    .expect("chat runtime holder main core must exist");
                core.createNewChat(None, None, false, true, None);
                core.chatHistoryDelegate
                    .currentChatIdFlow()
                    .value()
                    .expect("test chat must become current")
            };
            let mut messages_stream = CoreLinkSharedClient::watch(
                &proxy,
                CoreWatchRequest::new(
                    "proxy-local-live-chat-messages-flow-nested-capture",
                    chat_object_id,
                    "chatMessagesFlow",
                    toCoreValue(json!({ "chatId": chat_id.clone() })).unwrap(),
                ),
            )
            .await
            .expect("local proxy chatMessagesFlow watch must open");
            let initial_event = receive_proxy_test_event(&mut messages_stream).await;
            let initial_messages: Vec<ChatMessage> = fromCoreValue(initial_event.value)
                .expect("initial local chat messages must decode");
            assert!(initial_messages.is_empty());

            let stream_id = "proxy-local-live-chat-message-stream-nested-capture";
            let source = proxy_test_markdown_stream_source(
                chat_id.clone(),
                "hello while outer capture is active".to_string(),
            );
            let mut message = ChatMessage::new("ai".to_string());
            message.contentStream =
                Some(CoreStream::fromSourceWithId(stream_id.to_string(), source));

            let ((), outer_attachments) = operit_link::withCoreStreamCapture(async {
                {
                    let holder = proxy.chatRuntimeHolder();
                    let mut holder = holder.lock().await;
                    let core = holder
                        .coreForObjectId(chat_object_id)
                        .expect("chat runtime holder main core must exist");
                    core.chatHistoryDelegate
                        .addMessageToChat(message, Some(chat_id.clone()));
                }

                let updated_event = receive_proxy_test_event(&mut messages_stream).await;
                let updated_messages: Vec<ChatMessage> = fromCoreValue(updated_event.value)
                    .expect("updated local chat messages must decode");
                let stream = updated_messages
                    .iter()
                    .find_map(|message| message.contentStream.clone())
                    .expect("updated local message must carry a content stream");
                let mut content_stream = CoreLinkSharedClient::watch(
                    &proxy,
                    CoreWatchRequest::new(
                        "proxy-local-live-chat-content-stream-nested-capture",
                        CORE_STREAM_POOL_OBJECT_ID,
                        "openCoreStream",
                        stream.descriptor.args.clone(),
                    ),
                )
                .await
                .expect("embedded content stream must open before outer capture is adopted");
                let chunk = receive_proxy_test_event(&mut content_stream).await;
                let event: MarkdownStreamEvent =
                    fromCoreValue(chunk.value).expect("Markdown event must decode");
                assert_eq!(event.eventType, "chunk");
                assert_eq!(
                    event.value,
                    Some("hello while outer capture is active".to_string())
                );
            })
            .await;

            assert_eq!(outer_attachments.len(), 1);
        })
        .await;
}

/// Verifies concurrent Application child calls wait for the shared application lock.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shared_core_serializes_application_child_access() {
    let _testGuard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    tokio::task::LocalSet::new()
        .run_until(async {
            let storage_host = register_test_runtime_roots();
            let mut host_manager =
                HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
            host_manager.runtimeStorageHost = Some(storage_host.clone());
            host_manager.runtimeSqliteHost = Some(storage_host);
            host_manager.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
            host_manager.hostJavaScriptRuntimeHost =
                Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
            host_manager.hostRuntimeTaskSchedulerHost =
                Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
            let proxy = Arc::new(LocalCoreProxy::new(OperitApplication::newWithContext(
                host_manager,
            )));
            let package_manager_id =
                LocalCoreProxy::generatedObjectIdForSchema("application.packageManager")
                    .expect("package manager object id must be generated");
            let barrier = Arc::new(tokio::sync::Barrier::new(16));
            let mut calls = Vec::with_capacity(16);

            for index in 0..16 {
                let call_proxy = proxy.clone();
                let call_barrier = barrier.clone();
                calls.push(tokio::task::spawn_local(async move {
                    call_barrier.wait().await;
                    let (method_name, args) = if index % 2 == 0 {
                        (
                            "getToolPkgUiRoutes",
                            json!({"runtime": "compose_dsl", "useEnglish": false}),
                        )
                    } else {
                        ("getToolPkgNavigationEntries", json!({"useEnglish": false}))
                    };
                    CoreLinkSharedClient::call(
                        call_proxy.as_ref(),
                        CoreCallRequest::new(
                            format!("application-child-{index}"),
                            package_manager_id,
                            method_name,
                            toCoreValue(args).unwrap(),
                        ),
                    )
                    .await
                }));
            }

            for call in calls {
                let response = call.await.expect("application child call must not panic");
                assert!(
                    response.result.is_ok(),
                    "application child call failed: {response:?}"
                );
            }
        })
        .await;
}
