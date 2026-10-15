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
use operit_util::RuntimeStoreRoot::{RuntimeStoreRootConfig, setDefaultRuntimeStoreRootConfig};
use serde_json::json;

/// Exercise the same generated import boundary that Flutter uses, before chat-send routing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn imported_attachment_keeps_its_actual_node_and_ephemeral_storage() {
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
            setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
                runtime_root.clone(),
                workspace_root.clone(),
            ));
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
            let proxy = LocalCoreProxy::new(OperitApplication::newWithContext(host_manager));
            let target = LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain").unwrap();
            let expected_chat_id = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                holder.coreForTarget(target).unwrap().chatHistoryDelegate.currentChatIdFlow.value()
            };
            let transfer_target = LocalCoreProxy::generatedTargetForSchema("services.attachmentTransferManager").unwrap();
            let upload_id: String = fromCoreValue(CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new("begin-file", transfer_target, "beginAttachmentUpload",
                    toCoreValue(json!({"fileName": "文档.pdf", "expectedByteLength": 3})).unwrap()),
            ).await.result.unwrap()).unwrap();
            let mut stream = CoreLinkSharedClient::openPush(&proxy, CorePushRequest::new(
                "write-file", transfer_target, "writeAttachmentUpload",
            ).withArgs(toCoreValue(json!({
                "uploadId": upload_id, "fileName": "文档.pdf", "expectedByteLength": 3,
            })).unwrap())).await.unwrap();
            stream.send(toCoreValue(vec![0u8]).unwrap()).await.unwrap();
            stream.send(toCoreValue(vec![1u8, 255]).unwrap()).await.unwrap();
            stream.close().await.unwrap();
            let imported: AttachmentInfo = fromCoreValue(CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new("complete-file", transfer_target, "completeAttachmentUpload",
                    toCoreValue(json!({"uploadId": upload_id, "fileName": "文档.pdf", "expectedByteLength": 3})).unwrap()),
            ).await.result.unwrap()).unwrap();
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
            assert_eq!(attachment.nodeId.as_deref(), Some("core-source-b"));
            assert_eq!(std::fs::read(&attachment.filePath).unwrap(), [0, 1, 255]);
            assert!(std::path::Path::new(&attachment.filePath)
                .starts_with(runtime_root.join("temp/clean_on_exit")));
            assert!(attachment
                .fileToolPath()
                .starts_with("/app/data/temp/clean_on_exit/"));
            assert_eq!(
                runtimeStorageOwnership("runtime/temp/clean_on_exit/file.pdf").unwrap(),
                RuntimeStorageOwnership::Ephemeral
            );

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
            let _ = std::fs::remove_dir_all(root);
        })
        .await;
}
