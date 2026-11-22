#![cfg(not(target_arch = "wasm32"))]
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, HostEnvironmentDescriptor, HostError, HostResult, HostRuntimeAsyncTask,
    HostRuntimeTask, HostRuntimeTaskSchedulerHost, HostRuntimeTurnFuture,
};
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeRuntimeStorageHost, NativeHttpHost,
    PosixFileSystemHost,
};
use operit_link::{
    fromCoreValue, toCoreValue, CoreCallRequest, CoreEventStream, CoreLinkSharedClient,
    CoreValue, CoreWatchRequest,
};
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::services::RuntimeHostInteractionService::{
    requestOwnerFileOpen, RuntimeHostInteractionRequest,
    RuntimeHostInteractionResponse, RuntimeHostInteractionFileOpenPayload,
    RuntimeHostInteractionFileOperationResponse,
};
use operit_runtime::services::WorkspaceService::{WorkspaceFileEntry, WorkspaceService};
use serde_json::{json, Value};

const SAMPLE_NAME: &str = "sample.zip";

/// Uses real filesystem metadata but makes opening wait for the production owner broker.
/// The small sample tests path-only transport; archive validity/large-file seeks are device tests.
struct OwnerOpenFileSystem {
    inner: PosixFileSystemHost,
    timeoutMillis: AtomicU64,
    openCalls: AtomicUsize,
    completedCalls: AtomicUsize,
    contentReads: AtomicUsize,
    workerNames: Mutex<Vec<String>>,
}

impl OwnerOpenFileSystem {
    fn new() -> Self {
        Self {
            inner: PosixFileSystemHost::new(),
            timeoutMillis: AtomicU64::new(5_000),
            openCalls: AtomicUsize::new(0),
            completedCalls: AtomicUsize::new(0),
            contentReads: AtomicUsize::new(0),
            workerNames: Mutex::new(Vec::new()),
        }
    }

    fn observeRead(&self, path: &str) {
        if Path::new(path).file_name().and_then(|name| name.to_str()) == Some(SAMPLE_NAME) {
            self.contentReads.fetch_add(1, Ordering::SeqCst);
        }
    }
}

macro_rules! delegate_filesystem {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $result:ty;)*) => {
        $(fn $name(&self, $($arg: $ty),*) -> $result {
            self.inner.$name($($arg),*)
        })*
    };
}

impl FileSystemHost for OwnerOpenFileSystem {
    fn readFile(&self, path: &str) -> HostResult<String> {
        self.observeRead(path);
        self.inner.readFile(path)
    }

    fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> HostResult<String> {
        self.observeRead(path);
        self.inner.readFileWithLimit(path, maxBytes)
    }

    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.observeRead(path);
        self.inner.readFileBytes(path)
    }

    fn readFileBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        self.observeRead(path);
        self.inner.readFileBytesRange(path, offset, length)
    }

    fn openFile(&self, path: &str) -> HostResult<()> {
        self.inner.validatePath(path, "path")?;
        if !Path::new(path).is_file() {
            return Err(HostError::new("test opener requires a regular sample file"));
        }
        self.openCalls.fetch_add(1, Ordering::SeqCst);
        self.workerNames.lock().unwrap().push(
            std::thread::current().name().unwrap_or("unnamed").to_string(),
        );
        let result = requestOwnerFileOpen(
            RuntimeHostInteractionFileOpenPayload {
                path: path.to_string(),
            },
            Duration::from_millis(self.timeoutMillis.load(Ordering::SeqCst)),
        );
        self.completedCalls.fetch_add(1, Ordering::SeqCst);
        let response = result.map_err(HostError::new)?;
        if response.success {
            Ok(())
        } else {
            Err(HostError::new(response.error.expect("failed open must include an error")))
        }
    }

    delegate_filesystem! {
        envLabel() -> &str;
        environmentDescriptor() -> HostEnvironmentDescriptor;
        validatePath(path: &str, paramName: &str) -> HostResult<()>;
        listFiles(path: &str) -> HostResult<Vec<FileEntry>>;
        writeFile(path: &str, content: &str, append: bool) -> HostResult<()>;
        writeFileBytes(path: &str, content: &[u8]) -> HostResult<()>;
        deleteFile(path: &str, recursive: bool) -> HostResult<()>;
        fileExists(path: &str) -> HostResult<FileExistence>;
        moveFile(source: &str, destination: &str) -> HostResult<()>;
        copyFile(source: &str, destination: &str, recursive: bool) -> HostResult<()>;
        makeDirectory(path: &str, createParents: bool) -> HostResult<()>;
        findFiles(request: FindFilesRequest) -> HostResult<Vec<String>>;
        fileInfo(path: &str) -> HostResult<FileInfo>;
        grepCode(request: GrepCodeRequest) -> HostResult<GrepCodeResult>;
        zipFiles(source: &str, destination: &str) -> HostResult<()>;
        unzipFiles(source: &str, destination: &str) -> HostResult<()>;
        shareFile(path: &str, title: &str) -> HostResult<()>;
    }
}

#[derive(Clone, Copy)]
enum SchedulerFault {
    Reject,
    DropAcceptedTask,
}

/// Preserves all other Host scheduling while failing just the workspace blocking task.
struct FaultScheduler {
    inner: NativeHostRuntimeTaskSchedulerHost,
    fault: SchedulerFault,
}

impl HostRuntimeTaskSchedulerHost for FaultScheduler {
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        self.inner.monotonicTimeMillis()
    }

    fn scheduleHostRuntimeTask(&self, taskName: &str, task: HostRuntimeTask) -> HostResult<()> {
        self.inner.scheduleHostRuntimeTask(taskName, task)
    }

    fn scheduleHostRuntimeBlockingAsyncTask(&self, taskName: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        if taskName != "workspace-file-open" {
            return self.inner.scheduleHostRuntimeBlockingAsyncTask(taskName, task);
        }
        match self.fault {
            SchedulerFault::Reject => Err(HostError::new("workspace test scheduler rejected task")),
            SchedulerFault::DropAcceptedTask => {
                drop(task);
                Ok(())
            }
        }
    }

    fn scheduleHostRuntimeAsyncTask(&self, name: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        self.inner.scheduleHostRuntimeAsyncTask(name, task)
    }

    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self, name: &str, task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.inner.scheduleHostRuntimeCooperativeAsyncTask(name, task)
    }

    fn scheduleDelayedHostRuntimeTask(
        &self, name: &str, delayMs: u64, task: HostRuntimeTask,
    ) -> HostResult<()> {
        self.inner.scheduleDelayedHostRuntimeTask(name, delayMs, task)
    }

    fn waitForHostRuntimeDelay(&self, delayMs: u64) -> HostRuntimeTurnFuture {
        self.inner.waitForHostRuntimeDelay(delayMs)
    }
}

fn workspace_request(method: &str, args: Value) -> CoreCallRequest {
    CoreCallRequest::new(
        format!("workspace-external-{method}"),
        LocalCoreProxy::generatedTargetForSchema("application.workspaceService").unwrap(),
        method,
        toCoreValue(args).unwrap(),
    )
}

fn open_request(chatId: &str) -> CoreCallRequest {
    workspace_request(
        "openWorkspaceFile",
        json!({"chatId": chatId, "relativePath": format!("project/{SAMPLE_NAME}")}),
    )
}

async fn next_owner_request(events: &mut CoreEventStream) -> RuntimeHostInteractionRequest {
    let event = tokio::time::timeout(Duration::from_secs(2), events.recv())
        .await
        .expect("owner event must arrive while the generated open call remains pending")
        .expect("owner stream must remain open");
    fromCoreValue(event.value).expect("owner request must decode")
}

async fn respond_owner(proxy: &LocalCoreProxy, requestId: &str, error: Option<&str>) {
    let response = serde_json::to_value(RuntimeHostInteractionResponse::fileOpen(
        RuntimeHostInteractionFileOperationResponse {
            success: error.is_none(),
            error: error.map(str::to_string),
        },
    )).unwrap();
    let result = CoreLinkSharedClient::call(
        proxy,
        CoreCallRequest::new(
            "workspace-respond-owner",
            LocalCoreProxy::generatedTargetForSchema("services.runtimeHostInteractionService").unwrap(),
            "respondOwnerHostInteraction",
            toCoreValue(json!({"requestId": requestId, "response": response})).unwrap(),
        ),
    );
    tokio::time::timeout(Duration::from_secs(1), result)
        .await.expect("owner response must not wait for the workspace service")
        .result.expect("owner response must be accepted");
}

async fn bound_chat(proxy: &LocalCoreProxy) -> String {
    let holder = proxy.chatRuntimeHolder();
    let mut holder = holder.lock().await;
    let target = LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain").unwrap();
    let core = holder.coreForTarget(target).unwrap();
    let manager = &core.chatHistoryDelegate.chatHistoryManager;
    let draft = manager.newChatDraft("External file open".into(), Default::default(), None, None).unwrap();
    let chat = manager.commitChatDraft(draft, None).unwrap();
    let workspace = manager.createWorkspace("External open project".into(), vec![
        operit_model::Workspace::WorkspaceFolder {
            name: "project".into(), path: "/operitapp/workspaces/project".into(),
        },
    ]).unwrap();
    manager.updateChatWorkspaceId(chat.id.clone(), Some(workspace.id)).unwrap();
    core.chatHistoryDelegate.currentChatIdFlow.set_value(Some(chat.id.clone()));
    chat.id
}

/// A current-thread caller cannot poll owner events if generated dispatch blocks inline.
/// This exercises the real capture dispatcher, owner broker and its String error mapping.
#[test]
fn generated_workspace_open_releases_service_and_preserves_real_outcomes() {
    let root: PathBuf = std::env::temp_dir().join(format!(
        "operit-workspace-open-{}-{}", std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos(),
    ));
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "workspace_open_worker", "--ignored", "--nocapture"])
        .env("OPERIT_WORKSPACE_OPEN_TEST_ROOT", &root)
        .output().unwrap();
    assert!(output.status.success(), "workspace worker failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    // The worker owns application singletons; exit releases their database handles on Windows.
    let resolved = root.canonicalize().unwrap();
    assert_eq!(resolved.parent(), Some(std::env::temp_dir().canonicalize().unwrap().as_path()));
    std::fs::remove_dir_all(resolved).unwrap();
}

/// Runs the real application broker in a process whose database lifetime the parent owns.
#[tokio::test(flavor = "current_thread")]
#[ignore = "Executed by the parent with an isolated storage directory"]
async fn workspace_open_worker() {
    let root = PathBuf::from(std::env::var_os("OPERIT_WORKSPACE_OPEN_TEST_ROOT")
        .expect("workspace worker requires its parent-owned storage directory"));
    let runtimeRoot = root.join("runtime");
    let workspaceRoot = root.join("workspaces");
    let project = workspaceRoot.join("project");
    std::fs::create_dir_all(&runtimeRoot).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    let sample = project.join(SAMPLE_NAME);
    let original: &[u8] = b"path-only external-open test sample";
    std::fs::write(&sample, original).unwrap();

    tokio::task::LocalSet::new().run_until(async {
        let filesystem = Arc::new(OwnerOpenFileSystem::new());
        let storage = Arc::new(NativeRuntimeStorageHost::new(runtimeRoot, workspaceRoot));
        let context = HostManager {
            httpHost: Some(Arc::new(NativeHttpHost::new())),
            fileSystemHost: Some(filesystem.clone()),
            runtimeStorageHost: Some(storage.clone()),
            runtimeSqliteHost: Some(storage),
            hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
            hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new())),
            ..HostManager::default()
        };
        let application = OperitApplication::newWithContext(context.clone());
        let handler = application.aiToolHandler();
        let proxy = Arc::new(LocalCoreProxy::new(application));
        let chatId = bound_chat(proxy.as_ref()).await;
        let mut events = CoreLinkSharedClient::watch(
            proxy.as_ref(),
            CoreWatchRequest::new(
                "workspace-owner-events",
                LocalCoreProxy::generatedTargetForSchema("services.runtimeHostInteractionService").unwrap(),
                "ownerHostInteractionEvents",
                toCoreValue(json!({"kinds": ["file_open"]})).unwrap(),
            ),
        ).await.unwrap();

        let pendingProxy = proxy.clone();
        let request = open_request(&chatId);
        let pending = tokio::task::spawn_local(async move {
            CoreLinkSharedClient::call(pendingProxy.as_ref(), request).await
        });
        let owner = next_owner_request(&mut events).await;
        assert!(!pending.is_finished(), "open must await the actual owner response");
        assert_eq!(filesystem.openCalls.load(Ordering::SeqCst), 1);
        assert_eq!(filesystem.completedCalls.load(Ordering::SeqCst), 0);
        assert!(filesystem.workerNames.lock().unwrap()[0].starts_with("operit-blk-"));
        let payload = owner.fileOpen.as_ref().unwrap();
        assert_eq!(Path::new(&payload.path), sample.as_path());

        let listing = tokio::time::timeout(
            Duration::from_secs(1),
            CoreLinkSharedClient::call(proxy.as_ref(), workspace_request(
                "listWorkspaceFiles", json!({"chatId": chatId, "relativePath": "project"}),
            )),
        ).await.expect("listing must proceed while the opener waits").result.unwrap();
        let entries: Vec<WorkspaceFileEntry> = fromCoreValue(listing).unwrap();
        assert!(entries.iter().any(|entry| entry.name == SAMPLE_NAME));
        assert!(!pending.is_finished(), "listing must not complete the pending open");
        respond_owner(proxy.as_ref(), &owner.requestId, None).await;
        let result = tokio::time::timeout(Duration::from_secs(1), pending)
            .await.unwrap().unwrap().result.unwrap();
        assert!(matches!(result, CoreValue::Null));
        assert_eq!(filesystem.completedCalls.load(Ordering::SeqCst), 1);

        let pendingProxy = proxy.clone();
        let request = open_request(&chatId);
        let failed = tokio::task::spawn_local(async move {
            CoreLinkSharedClient::call(pendingProxy.as_ref(), request).await
        });
        let owner = next_owner_request(&mut events).await;
        respond_owner(proxy.as_ref(), &owner.requestId, Some("owner file opener failed")).await;
        let error = tokio::time::timeout(Duration::from_secs(1), failed)
            .await.unwrap().unwrap().result.unwrap_err();
        assert_eq!(error.code, "COMMAND_ERROR");
        assert_eq!(error.message, "owner file opener failed");
        assert_eq!(filesystem.completedCalls.load(Ordering::SeqCst), 2);

        // A short test-only broker timeout must still remove the pending capability.
        filesystem.timeoutMillis.store(250, Ordering::SeqCst);
        let pendingProxy = proxy.clone();
        let request = open_request(&chatId);
        let expired = tokio::task::spawn_local(async move {
            CoreLinkSharedClient::call(pendingProxy.as_ref(), request).await
        });
        let owner = next_owner_request(&mut events).await;
        let error = tokio::time::timeout(Duration::from_secs(2), expired)
            .await.unwrap().unwrap().result.unwrap_err();
        assert_eq!(error.code, "COMMAND_ERROR");
        assert_eq!(error.message, format!("host interaction timed out: {}", owner.requestId));
        let lateResponse = CoreLinkSharedClient::call(
            proxy.as_ref(),
            CoreCallRequest::new(
                "workspace-late-owner-response",
                LocalCoreProxy::generatedTargetForSchema("services.runtimeHostInteractionService").unwrap(),
                "respondOwnerHostInteraction",
                toCoreValue(json!({"requestId": owner.requestId, "response": {"error": "late"}})).unwrap(),
            ),
        ).await.result.unwrap_err();
        assert_eq!(lateResponse.code, "COMMAND_ERROR");
        assert!(lateResponse.message.starts_with("host interaction request not found:"));
        assert_eq!(filesystem.completedCalls.load(Ordering::SeqCst), 3);
        assert_eq!(filesystem.contentReads.load(Ordering::SeqCst), 0);
        assert_eq!(std::fs::read(&sample).unwrap().as_slice(), original);

        let mut noScheduler = context.clone();
        noScheduler.hostRuntimeTaskSchedulerHost = None;
        let service = WorkspaceService::new(&noScheduler, handler.clone());
        let owned = service.openWorkspaceFile(chatId.clone(), format!("project/{SAMPLE_NAME}"));
        drop(service);
        assert_eq!(owned.await.unwrap_err(),
            "HostRuntimeTaskSchedulerHost is not registered for workspace file opening");

        for (fault, expected) in [
            (SchedulerFault::Reject, "workspace test scheduler rejected task"),
            (SchedulerFault::DropAcceptedTask, "workspace file opener task stopped:"),
        ] {
            let mut failingContext = context.clone();
            failingContext.hostRuntimeTaskSchedulerHost = Some(Arc::new(FaultScheduler {
                inner: NativeHostRuntimeTaskSchedulerHost::new(), fault,
            }));
            let service = WorkspaceService::new(&failingContext, handler.clone());
            let owned = service.openWorkspaceFile(chatId.clone(), format!("project/{SAMPLE_NAME}"));
            drop(service);
            let error = tokio::time::timeout(Duration::from_secs(1), owned)
                .await.unwrap().unwrap_err();
            assert!(error.starts_with(expected), "unexpected scheduler error: {error}");
        }
        assert_eq!(filesystem.openCalls.load(Ordering::SeqCst), 3,
            "failed scheduling must not enter the file opener");
    }).await;
}
