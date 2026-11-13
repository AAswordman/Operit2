use std::sync::Arc;

use operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{HostManager::HostManager, RuntimeStorageHost};
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_runtime::services::AttachmentTransferManager::AttachmentTransferManager;
use operit_store::FileSyncPolicy::{FileReference, REFERENCE_SUFFIX};
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_util::stream::ReverseStream::ReverseStream;

#[test]
fn new_attachments_use_portable_names_through_real_storage() {
    let root =
        std::env::temp_dir().join(format!("operit-attachment-names-{}", uuid::Uuid::new_v4()));
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "attachment_names_worker",
            "--ignored",
            "--nocapture",
        ])
        .env("OPERIT_ATTACHMENT_NAME_TEST_ROOT", &root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Releases runtime singletons before the parent removes its isolated storage.
#[tokio::test(flavor = "current_thread")]
#[ignore = "Run by the parent with isolated storage"]
async fn attachment_names_worker() {
    let root =
        std::path::PathBuf::from(std::env::var_os("OPERIT_ATTACHMENT_NAME_TEST_ROOT").unwrap());
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    setDefaultRuntimeStorageHost(storage.clone());
    setDefaultRuntimeSqliteHost(storage.clone());
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    let host = HostManager {
        fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
        runtimeStorageHost: Some(storage.clone()),
        runtimeStorageWriteHost: Some(storage.clone()),
        ..HostManager::default()
    };
    let transfer = AttachmentTransferManager::getInstance(&host).unwrap();
    let manager =
        operit_store::repository::ChatHistoryManager::ChatHistoryManager::default().unwrap();
    let draft = manager
        .newChatDraft(
            "Portable attachments".into(),
            Default::default(),
            None,
            None,
        )
        .unwrap();
    let chat = manager.commitChatDraft(draft, None).unwrap().id;
    let original = "_cgi-bin_mmwebwx-bin_webwxgetmsgimg??&MsgID=123&skey=example&mmweb_appid=wx_webfilehelper.jpg";
    let filesystem = host.vfs().unwrap();
    let bytes = [1, 2, 3];
    let mut paths = std::collections::BTreeSet::new();
    for name in [
        original.to_string(),
        "photo?.jpg".into(),
        "CON.jpg".into(),
        format!("{}.jpg", "图".repeat(90)),
        "需求文档.pdf".into(),
    ] {
        let id = transfer
            .beginAttachmentUpload(name.clone(), Some(3))
            .unwrap();
        let (mut sender, input) = ReverseStream::<Vec<u8>>::channel();
        let ((), written) = tokio::join!(
            async {
                sender.send(bytes.to_vec()).await.unwrap();
                sender.send(Vec::new()).await.unwrap();
                sender.close();
            },
            transfer.writeSelectedAttachment(
                chat.clone(),
                id.clone(),
                name.clone(),
                Some(3),
                input
            )
        );
        written.unwrap();
        let attachment = transfer
            .completeSelectedAttachment(chat.clone(), id, name.clone(), Some(3))
            .unwrap();
        let expected = if name == "需求文档.pdf" {
            "需求文档.pdf"
        } else {
            "attachment.jpg"
        };
        assert_eq!(attachment.fileName, expected);
        assert!(attachment
            .vfsPath
            .as_ref()
            .unwrap()
            .ends_with(&format!("/{expected}")));
        assert_eq!(
            filesystem
                .readFileBytesRange(attachment.vfsPath.as_ref().unwrap(), 0, 3)
                .unwrap(),
            bytes
        );
        assert!(
            paths.insert(attachment.vfsPath.unwrap()),
            "fallback names must not overwrite another upload"
        );
    }

    // Source-only uploads must use the same name for writing and the durable locator.
    let id = transfer
        .beginAttachmentUpload(original.into(), Some(3))
        .unwrap();
    let (mut sender, input) = ReverseStream::<Vec<u8>>::channel();
    let ((), written) = tokio::join!(
        async {
            sender.send(bytes.to_vec()).await.unwrap();
            sender.send(Vec::new()).await.unwrap();
            sender.close();
        },
        transfer.writeAttachmentSource(id.clone(), original.into(), Some(3), 1024, input)
    );
    written.unwrap();
    let reference: FileReference = serde_json::from_str(
        &transfer
            .completeAttachmentSource(id, original.into(), Some(3))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(reference.fileName, "attachment.jpg");
    assert!(reference.path.ends_with("/attachment.jpg"));
    assert_eq!(
        filesystem
            .readFileBytesRange(&reference.path, 0, 3)
            .unwrap(),
        bytes
    );

    // A locator may point at a source with its original name; only the new sidecar is renamed.
    for name in [original.to_string(), format!("{}.pdf", "a".repeat(170))] {
        let source_path = format!("/virtual/macos/files/source/{name}");
        let reference =
            FileReference::new(storage.clone(), source_path.clone(), name.clone(), 3, None)
                .unwrap();
        let attachment = transfer
            .createReferenceAttachment(
                chat.clone(),
                String::from_utf8(reference.bytes().unwrap()).unwrap(),
            )
            .unwrap();
        let expected = if name == original {
            "attachment.jpg"
        } else {
            "attachment.pdf"
        };
        assert_eq!(attachment.fileName, format!("{expected}{REFERENCE_SUFFIX}"));
        assert!(attachment
            .vfsPath
            .as_ref()
            .unwrap()
            .ends_with(&format!("/{}", attachment.fileName)));
        let content = filesystem
            .readFileBytesRange(
                attachment.vfsPath.as_ref().unwrap(),
                0,
                attachment.fileSize as usize,
            )
            .unwrap();
        let saved: FileReference = serde_json::from_slice(&content).unwrap();
        assert_eq!(
            saved.path, source_path,
            "the external original must remain at its real path"
        );
    }

    // Native import must rename its managed copy without renaming the original source.
    #[cfg(unix)]
    {
        let source_path = format!("runtime/data/{original}");
        storage.writeBytes(&source_path, &bytes).unwrap();
        let attachment = transfer
            .importNativeAttachment(chat.clone(), format!("/operitapp/data/data/{original}"))
            .unwrap();
        assert_eq!(attachment.fileName, "attachment.jpg");
        assert_eq!(
            filesystem
                .readFileBytesRange(attachment.vfsPath.as_ref().unwrap(), 0, 3)
                .unwrap(),
            bytes
        );
        assert_eq!(storage.readBytes(&source_path).unwrap(), bytes);
    }

    // Reusable sources keep their mount locator while their suggested name is normalized.
    let native_source = root.join("source.jpg");
    std::fs::write(&native_source, bytes).unwrap();
    let source = operit_vfs::MountRegistry::MountSource {
        namespace: "/virtual/macos/files".into(),
        backend: "native".into(),
        root: native_source.to_string_lossy().into_owned(),
        name: original.into(),
    };
    let reference: FileReference = serde_json::from_str(
        &transfer
            .registerAttachmentSource(serde_json::to_string(&source).unwrap(), original.into(), 3)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(reference.fileName, "attachment.jpg");
    assert_eq!(
        filesystem
            .readFileBytesRange(&reference.path, 0, 3)
            .unwrap(),
        bytes
    );
    assert_eq!(std::fs::read(native_source).unwrap(), bytes);
}
