use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::HostManager::HostManager;
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeRuntimeStorageHost,
    PosixFileSystemHost,
};
use operit_link::{
    CoreCallRequest, CoreLinkSharedClient, CorePushRequest, fromCoreValue, toCoreValue,
};
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_util::RuntimeStorageLayout::{RuntimeStorageOwnership, runtimeStorageOwnership};

use serde_json::json;

/// Exercise the same generated import boundary that Flutter uses, before chat-send routing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn imported_attachment_keeps_its_actual_node_and_persistent_storage() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let root = std::env::temp_dir().join(format!(
                "operit-attachment-origin-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let runtime_root = root.join("runtime");
            let workspace_root = root.join("workspaces");
            std::fs::create_dir_all(&runtime_root).unwrap();
            std::fs::create_dir_all(&workspace_root).unwrap();

            let storage = Arc::new(NativeRuntimeStorageHost::new(
                runtime_root.clone(),
                workspace_root,
            ));
            CoreNodeIdentityStore::new(storage.clone())
                .writeNodeId("core-source-b".into())
                .unwrap();
            let host_manager = HostManager {
                fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
                runtimeStorageHost: Some(storage.clone()),
                runtimeStorageWriteHost: Some(storage.clone()),
                runtimeSqliteHost: Some(storage),
                hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
                hostRuntimeTaskSchedulerHost: Some(Arc::new(
                    NativeHostRuntimeTaskSchedulerHost::new(),
                )),
                ..HostManager::default()
            };
            let filesystem = host_manager.vfs().unwrap();
            let proxy = LocalCoreProxy::new(OperitApplication::newWithContext(host_manager));
            let target = LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain").unwrap();
            let expected_chat_id = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder.coreForTarget(target).unwrap();
                let manager = &core.chatHistoryDelegate.chatHistoryManager;
                let draft = manager.newChatDraft("Attachment origin".into(), Default::default(), None, None).unwrap();
                let id = manager.commitChatDraft(draft, None).unwrap().id;
                core.switchChatLocal(id.clone()).await.unwrap();
                id
            };
            let transfer_target = LocalCoreProxy::generatedTargetForSchema("services.attachmentTransferManager").unwrap();
            // Removed upload entry points must disappear from the generated runtime surface too.
            assert!(!proxy.isReverseStreamRequest(&CorePushRequest::new(
                "removed-upload", transfer_target, "writeAttachmentUpload",
            )));
            for method in ["completeAttachmentUpload", "discardAttachmentUpload"] {
                let error = CoreLinkSharedClient::call(&proxy, CoreCallRequest::new(
                    format!("removed-{method}"), transfer_target, method,
                    toCoreValue(json!({"uploadId": "00000000000000000000000000000001", "fileName": "old.bin", "expectedByteLength": 0})).unwrap(),
                )).await.result.unwrap_err();
                assert_eq!(error.code, "METHOD_NOT_FOUND", "{method}: {error}");
            }
            let application_target = LocalCoreProxy::generatedTargetForSchema("application").unwrap();
            let error = CoreLinkSharedClient::call(&proxy, CoreCallRequest::new(
                "removed-directory", application_target, "chatAttachmentsPath", toCoreValue(json!({})).unwrap(),
            )).await.result.unwrap_err();
            assert_eq!(error.code, "METHOD_NOT_FOUND");
            let upload_id: String = fromCoreValue(CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new("begin-file", transfer_target, "beginAttachmentUpload",
                    toCoreValue(json!({"fileName": "文档.pdf", "expectedByteLength": 3})).unwrap()),
            ).await.result.unwrap()).unwrap();
            let mut stream = CoreLinkSharedClient::openPush(&proxy, CorePushRequest::new(
                "write-file", transfer_target, "writeSelectedAttachment",
            ).withArgs(toCoreValue(json!({
                "chatId": expected_chat_id, "uploadId": upload_id, "fileName": "文档.pdf", "expectedByteLength": 3,
            })).unwrap())).await.unwrap();
            stream.send(toCoreValue(vec![0u8]).unwrap()).await.unwrap();
            stream.send(toCoreValue(vec![1u8, 255]).unwrap()).await.unwrap();
            stream.send(toCoreValue(Vec::<u8>::new()).unwrap()).await.unwrap();
            stream.close().await.unwrap();
            let imported: AttachmentInfo = fromCoreValue(CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new("complete-file", transfer_target, "completeSelectedAttachment",
                    toCoreValue(json!({"chatId": expected_chat_id, "uploadId": upload_id, "fileName": "文档.pdf", "expectedByteLength": 3})).unwrap()),
            ).await.result.unwrap()).unwrap();
            let committed_path = imported.filePath.clone();
            assert!(filesystem.isWithin("/operitapp/data/data/chat_outputs", &committed_path).unwrap());
            CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new("attach-file", target, "attachUploadedFile",
                    toCoreValue(json!({"attachment": imported, "expectedChatId": expected_chat_id})).unwrap()),
            ).await.result.expect("committed upload must register through the generated chat boundary");
            let attachments = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                holder.coreForTarget(target).unwrap().attachments()
            };
            assert_eq!(attachments.len(), 1);
            let attachment = &attachments[0];
            assert!(attachment.vfsPath.as_deref().unwrap().contains("/.operit/attachments/"));
            assert_eq!(attachment.nodeId.as_deref(), Some("core-source-b"));
            assert_eq!(filesystem.readFileBytes(&attachment.filePath).unwrap(), [0, 1, 255]);
            assert!(filesystem.isWithin("/operitapp/data/data/chat_outputs", &attachment.filePath).unwrap());
            assert_eq!(std::path::Path::new(&attachment.filePath).file_name().unwrap(), "文档.pdf");
            assert_eq!(attachment.filePath, committed_path);
            assert!(filesystem.fileExists(&committed_path).unwrap().exists, "registration must retain the directly committed file");
            assert!(attachment
                .fileToolPath()
                .starts_with("/operitapp/outputs/"));
            assert_eq!(
                runtimeStorageOwnership("runtime/data/chat_attachments/chat/attachment/file.pdf").unwrap(),
                RuntimeStorageOwnership::Space
            );
            CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("reattach-file", target, "attachUploadedFile",
                toCoreValue(json!({"attachment": attachment, "expectedChatId": expected_chat_id})).unwrap()))
                .await.result.expect("registering an existing final attachment must be idempotent");
            assert!(filesystem.fileExists(&committed_path).unwrap().exists);

            let stale_chat = CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new("stale-chat", target, "attachUploadedFile", toCoreValue(json!({
                    "attachment": attachment, "expectedChatId": "changed-chat",
                })).unwrap()),
            ).await;
            assert!(stale_chat.result.is_err(), "an upload must not attach to a changed chat");
            let mut wrong_source = attachment.clone();
            wrong_source.nodeId = Some("different-node".into());
            assert!(CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new("wrong-source", target, "attachUploadedFile", toCoreValue(json!({
                    "attachment": wrong_source, "expectedChatId": expected_chat_id,
                })).unwrap()),
            ).await.result.is_err(), "source metadata must belong to the receiving runtime");

            // A later identity/send context must not rewrite B's attachment to the receiving node.
            CoreNodeIdentityStore::native()
                .writeNodeId("core-execution-a".into())
                .unwrap();
            let forwarded: Vec<AttachmentInfo> =
                fromCoreValue(toCoreValue(&attachments).unwrap()).unwrap();
            assert_eq!(forwarded[0].nodeId.as_deref(), Some("core-source-b"));
            assert!(!forwarded[0].isLocalToNode(CoreNodeIdentityStore::localNodeId().as_deref()));

            CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new(
                    "import-text",
                    target,
                    "handleAttachment",
                    toCoreValue(json!({"_filePath": "pasted_text:inline content"})).unwrap(),
                ),
            )
            .await
            .result
            .expect("inline attachment import must succeed");
            let holder = proxy.chatRuntimeHolder();
            let mut holder = holder.lock().await;
            let attachments = holder.coreForTarget(target).unwrap().attachments();
            assert_eq!(attachments[1].nodeId, None);
            assert_eq!(attachments[1].content, "inline content");
            let source = root.join("report.pdf");
            std::fs::write(&source, [1, 2, 3]).unwrap();
            let core = holder.coreForTarget(target).unwrap();
            core.handleAttachment(source.to_string_lossy().into_owned()).await;
            core.handleAttachment(source.to_string_lossy().into_owned()).await;
            let attachments = core.attachments();
            assert_eq!(attachments.len(), 4);
            assert_ne!(attachments[2].filePath, attachments[3].filePath);
            for attachment in &attachments[2..] {
                assert!(filesystem.isWithin("/operitapp/data/data/chat_outputs", &attachment.filePath).unwrap());
                assert!(attachment.vfsPath.as_deref().unwrap().starts_with("/operitapp/outputs/"));
                assert!(attachment.vfsPath.as_deref().unwrap().contains("/.operit/attachments/"));
                assert_eq!(std::path::Path::new(&attachment.filePath).file_name().unwrap(), "report.pdf");
                assert_eq!(filesystem.readFileBytes(&attachment.filePath).unwrap(), [1, 2, 3]);
            }
            drop(holder);
            let pasted = json!({"fileName": "pasted.png", "mimeType": "image/png", "fileSize": 3, "base64Content": "AAH/"});
            CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("paste-image", target, "handleAttachment",
                toCoreValue(json!({"_filePath": format!("pasted_image:{}", pasted)})).unwrap()))
                .await.result.expect("pasted images must use the shared final attachment pipeline");
            let pasted = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                holder.coreForTarget(target).unwrap().attachments().last().cloned().unwrap()
            };
            assert_eq!(pasted.fileName, "pasted.png");
            assert_eq!(filesystem.readFileBytes(&pasted.filePath).unwrap(), [0, 1, 255]);
            assert!(pasted.vfsPath.as_deref().unwrap().starts_with("/operitapp/outputs/"));
            assert!(pasted.vfsPath.as_deref().unwrap().contains("/.operit/attachments/"));
            // The browser and manual terminal receive the exact same persisted output base.
            let directory = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder.coreForTarget(target).unwrap();
                let state = core.localChatStateFlow(expected_chat_id.clone()).value();
                assert!(state.currentWorkspacePath.is_none());
                let expected = core.chatHistoryDelegate.chatHistoryManager.fileDirectoryForChat(&expected_chat_id).unwrap();
                assert_eq!(state.currentFileDirectory.as_deref(), Some(expected.as_str()));
                expected
            };
            let workspace_target = LocalCoreProxy::generatedTargetForSchema("services.workspaceService").unwrap();
            let entries: serde_json::Value = fromCoreValue(CoreLinkSharedClient::call(&proxy,
                CoreCallRequest::new("list-outputs", workspace_target, "listWorkspaceFiles", toCoreValue(json!({
                    "chatId": expected_chat_id, "relativePath": "",
                })).unwrap())).await.result.unwrap()).unwrap();
            assert!(entries.as_array().unwrap().iter().any(|entry| entry["name"] == ".operit"));
            CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("write-output", workspace_target, "writeWorkspaceTextFile",
                toCoreValue(json!({"chatId": expected_chat_id, "relativePath": "report.md", "content": "# Generated report"})).unwrap()))
                .await.result.expect("unbound output files must be editable through the existing browser service");
            let report: String = fromCoreValue(CoreLinkSharedClient::call(&proxy,
                CoreCallRequest::new("read-output", workspace_target, "readWorkspaceTextFile", toCoreValue(json!({
                    "chatId": expected_chat_id, "relativePath": "report.md",
                })).unwrap())).await.result.unwrap()).unwrap();
            assert_eq!(report, "# Generated report");
            assert!(CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("escape-outputs", workspace_target, "readWorkspaceTextFile",
                toCoreValue(json!({"chatId": expected_chat_id, "relativePath": "../report.md"})).unwrap())).await.result.is_err());

            // Large ephemeral inputs keep their source once and publish the locator directly into the final directory.
            let policy = operit_store::FileSyncPolicy::FileSyncPolicy {
                maxFileBytes: 512, maxTotalBytes: 8192, ..Default::default()
            };
            CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("write-policy", workspace_target, "writeWorkspaceTextFile",
                toCoreValue(json!({"chatId": expected_chat_id, "relativePath": ".operit/sync.json",
                    "content": serde_json::to_string(&policy).unwrap()})).unwrap())).await.result.unwrap();
            // CLI callers use this same native import boundary. Binary text input must not be read as UTF-8.
            let native_source = root.join("large-native.txt");
            std::fs::write(&native_source, vec![255u8; 1024]).unwrap();
            let native_attachment: AttachmentInfo = fromCoreValue(CoreLinkSharedClient::call(&proxy,
                CoreCallRequest::new("import-native-large", transfer_target, "importNativeAttachment",
                    toCoreValue(json!({"chatId": expected_chat_id, "filePath": native_source.to_string_lossy()})).unwrap()))
                    .await.result.unwrap()).unwrap();
            assert_eq!(native_attachment.fileName, "large-native.txt.operit-ref.json");
            assert!(native_attachment.content.is_empty());
            let native_reference: operit_store::FileSyncPolicy::FileReference =
                serde_json::from_slice(&filesystem.readFileBytes(&native_attachment.filePath).unwrap()).unwrap();
            assert_eq!(native_reference.path, operit_vfs::VfsPath::resolveFileResourcePath(None, &native_source.to_string_lossy()).unwrap());
            assert_eq!(filesystem.readFileBytes(&native_reference.path).unwrap(), vec![255u8; 1024]);
            assert_eq!(native_reference.fileSize, 1024);
            assert_eq!(std::fs::read(&native_source).unwrap(), vec![255u8; 1024]);
            let upload_id: String = fromCoreValue(CoreLinkSharedClient::call(&proxy,
                CoreCallRequest::new("begin-large", transfer_target, "beginAttachmentUpload",
                    toCoreValue(json!({"fileName": "large.bin", "expectedByteLength": 1024})).unwrap()))
                    .await.result.unwrap()).unwrap();
            let mut stream = CoreLinkSharedClient::openPush(&proxy, CorePushRequest::new("write-large", transfer_target, "writeAttachmentSource")
                .withArgs(toCoreValue(json!({"uploadId": upload_id, "fileName": "large.bin", "expectedByteLength": 1024, "maxBytes": 8192})).unwrap())).await.unwrap();
            stream.send(toCoreValue(vec![7u8; 1024]).unwrap()).await.unwrap();
            stream.send(toCoreValue(Vec::<u8>::new()).unwrap()).await.unwrap();
            stream.close().await.unwrap();
            let source_reference: String = fromCoreValue(CoreLinkSharedClient::call(&proxy,
                CoreCallRequest::new("complete-large", transfer_target, "completeAttachmentSource",
                    toCoreValue(json!({"uploadId": upload_id, "fileName": "large.bin", "expectedByteLength": 1024})).unwrap()))
                    .await.result.unwrap()).unwrap();
            let imported: AttachmentInfo = fromCoreValue(CoreLinkSharedClient::call(&proxy,
                CoreCallRequest::new("locate-large", transfer_target, "createReferenceAttachment",
                    toCoreValue(json!({"chatId": expected_chat_id, "referenceJson": source_reference})).unwrap()))
                    .await.result.unwrap()).unwrap();
            CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("attach-large", target, "attachUploadedFile",
                toCoreValue(json!({"attachment": imported, "expectedChatId": expected_chat_id})).unwrap()))
                .await.result.expect("a directly committed locator must attach without moving files");
            let locator = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                holder.coreForTarget(target).unwrap().attachments().last().cloned().unwrap()
            };
            assert_eq!(locator.fileName, "large.bin.operit-ref.json");
            assert_eq!(locator.mimeType, "application/json");
            assert!(locator.vfsPath.as_deref().unwrap().starts_with(&format!("{directory}/.operit/attachments/")));
            let reference: operit_store::FileSyncPolicy::FileReference = serde_json::from_slice(&filesystem.readFileBytes(&locator.filePath).unwrap()).unwrap();
            reference.validate().unwrap();
            assert_eq!(reference.fileName, "large.bin");
            assert_eq!(reference.fileSize, 1024);
            assert!(reference.path.starts_with("/operitapp/data/data/attachment_sources/"));
            assert_eq!(reference.nodeId, "core-execution-a");
            let source = runtime_root.join(reference.path.strip_prefix("/operitapp/data/").unwrap());
            assert_eq!(std::fs::read(&source).unwrap(), vec![7u8; 1024]);

            // Paste uses the same source/locator path if the user's small-file limit is lower.
            let pasted = json!({"fileName": "large.png", "mimeType": "image/png", "fileSize": 1024,
                "base64Content": format!("{}BQ==", "BQUF".repeat(341))});
            CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("paste-large", target, "handleAttachment",
                toCoreValue(json!({"_filePath": format!("pasted_image:{}", pasted)})).unwrap())).await.result.unwrap();
            let locator = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                holder.coreForTarget(target).unwrap().attachments().last().cloned().unwrap()
            };
            assert_eq!(locator.fileName, "large.png.operit-ref.json");
            assert_eq!(locator.mimeType, "application/json", "a locator must not be sent to image decoding");
            let reference: operit_store::FileSyncPolicy::FileReference = serde_json::from_slice(&filesystem.readFileBytes(&locator.filePath).unwrap()).unwrap();
            let source = runtime_root.join(reference.path.strip_prefix("/operitapp/data/").unwrap());
            assert_eq!(std::fs::read(source).unwrap(), vec![5u8; 1024]);

            // Registration cannot import an arbitrary native path outside the selected directory.
            let too_large = root.join("unimported/limit.bin");
            std::fs::create_dir_all(too_large.parent().unwrap()).unwrap();
            std::fs::write(&too_large, [9u8; 1024]).unwrap();
            CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("lower-retention", workspace_target, "writeWorkspaceTextFile",
                toCoreValue(json!({"chatId": expected_chat_id, "relativePath": ".operit/sync.json", "content":
                    serde_json::to_string(&operit_store::FileSyncPolicy::FileSyncPolicy {
                        maxFileBytes: 512, maxTotalBytes: 512, ..Default::default()
                    }).unwrap()})).unwrap())).await.result.unwrap();
            let uploaded = AttachmentInfo { filePath: too_large.to_string_lossy().into_owned(), fileName: "limit.bin".into(),
                mimeType: "application/octet-stream".into(), fileSize: 1024, content: String::new(),
                nodeId: Some("core-execution-a".into()), vfsPath: None };
            assert!(CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("reject-over-limit", target, "attachUploadedFile",
                toCoreValue(json!({"attachment": uploaded, "expectedChatId": expected_chat_id})).unwrap())).await.result.is_err());
            assert!(too_large.exists(), "registration must not move or delete an unimported source");
            assert!(!runtime_root.join("data/attachment_sources/00000000000000000000000000000001").exists());

            // Binding and unbinding keep the workspace browser's folder aliases and saved output binding intact.
            let workspace = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let manager = &holder.coreForTarget(target).unwrap().chatHistoryDelegate.chatHistoryManager;
                let workspace = manager.createWorkspace("Project".into(), vec![operit_model::Workspace::WorkspaceFolder {
                    name: "main".into(), path: "/operitapp/workspaces/project".into(),
                }]).unwrap();
                manager.updateChatWorkspaceId(expected_chat_id.clone(), Some(workspace.id.clone())).unwrap();
                workspace
            };
            let entries: serde_json::Value = fromCoreValue(CoreLinkSharedClient::call(&proxy,
                CoreCallRequest::new("list-bound", workspace_target, "listWorkspaceFiles",
                    toCoreValue(json!({"chatId": expected_chat_id, "relativePath": ""})).unwrap())).await.result.unwrap()).unwrap();
            assert_eq!(entries.as_array().unwrap().len(), 1);
            assert_eq!(entries[0]["relativePath"], "main");
            CoreLinkSharedClient::call(&proxy, CoreCallRequest::new("write-bound", workspace_target, "writeWorkspaceTextFile",
                toCoreValue(json!({"chatId": expected_chat_id, "relativePath": "main/bound.md", "content": "Bound file"})).unwrap()))
                .await.result.unwrap();
            let content: String = fromCoreValue(CoreLinkSharedClient::call(&proxy,
                CoreCallRequest::new("read-bound", workspace_target, "readWorkspaceTextFile",
                    toCoreValue(json!({"chatId": expected_chat_id, "relativePath": "main/bound.md"})).unwrap())).await.result.unwrap()).unwrap();
            assert_eq!(content, "Bound file");
            {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                let core = holder.coreForTarget(target).unwrap();
                let state = core.localChatStateFlow(expected_chat_id.clone()).value();
                assert_eq!(state.currentFileDirectory.as_deref(), Some(workspace.primaryFolder().path.as_str()));
                assert_eq!(state.currentWorkspacePath.as_deref(), Some(workspace.primaryFolder().path.as_str()));
                core.chatHistoryDelegate.chatHistoryManager.updateChatWorkspaceId(expected_chat_id.clone(), None).unwrap();
                let state = core.localChatStateFlow(expected_chat_id.clone()).value();
                assert!(state.currentWorkspacePath.is_none());
                assert_eq!(state.currentFileDirectory.as_deref(), Some(directory.as_str()));
            }
            let _ = std::fs::remove_dir_all(root);
        })
        .await;
}
