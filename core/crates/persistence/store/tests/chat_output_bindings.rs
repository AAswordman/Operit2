//! Stable output ownership and historical relative links across chat lifecycle changes.
use operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost;
use operit_host_api::RuntimeStorageHost;
use operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::{
    ChatHistory::ChatHistory, ChatMessage::ChatMessage, Workspace::WorkspaceFolder,
};
use operit_store::{
    repository::ChatHistoryManager::ChatHistoryManager,
    ChatOutputStore::ChatOutputStore,
    CoreNodeIdentityStore::CoreNodeIdentityStore,
    RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost},
};

use std::sync::Arc;

fn draft(
    manager: &ChatHistoryManager,
    title: &str,
    time: i64,
    parent: Option<&str>,
) -> ChatHistory {
    let mut draft = manager
        .newChatDraft(
            title.into(),
            Default::default(),
            None,
            parent.map(str::to_string),
        )
        .unwrap();
    draft.createdAt = time.to_string();
    draft.updatedAt = time.to_string();
    draft
}

/// One process owns repository singletons; each scenario uses distinct chats.
#[test]
fn output_bindings_survive_branches_promotion_deletion_and_workspace_changes() {
    let root =
        std::env::temp_dir().join(format!("operit-output-bindings-{}", uuid::Uuid::new_v4()));
    let host = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));

    setDefaultRuntimeStorageHost(host.clone());
    setDefaultRuntimeSqliteHost(host.clone());
    CoreNodeIdentityStore::new(host.clone())
        .writeNodeId("output-source".into())
        .unwrap();
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    let manager = ChatHistoryManager::default().unwrap();
    let outputs = ChatOutputStore::new(host.clone());
    let time = 1793740800123;
    let parent = manager
        .commitChatDraft(draft(&manager, "parent", time, None), None)
        .unwrap();
    let base = manager.fileDirectoryForChat(&parent.id).unwrap();
    assert!(base.starts_with("/operitapp/outputs/"));
    assert!(manager.getWorkspaceForChat(&parent.id).unwrap().is_none());
    let storage_base = ChatOutputStore::storagePath(&base).unwrap();
    assert!(host
        .exists(&format!("{storage_base}/.operit/sync.json"))
        .unwrap());
    host.writeBytes(&format!("{storage_base}/report.md"), b"persistent output")
        .unwrap();

    let child = manager
        .commitChatDraft(
            draft(&manager, "child", time + 86_400_000, Some(&parent.id)),
            None,
        )
        .unwrap();
    let grandchild = manager
        .commitChatDraft(
            draft(&manager, "grandchild", time + 172_800_000, Some(&child.id)),
            None,
        )
        .unwrap();
    for chat in [&child, &grandchild] {
        assert_eq!(manager.fileDirectoryForChat(&chat.id).unwrap(), base);
    }
    // Equal creation timestamps intentionally share the same output area.
    let overlapping = manager
        .commitChatDraft(draft(&manager, "overlap", time, None), None)
        .unwrap();
    assert_eq!(manager.fileDirectoryForChat(&overlapping.id).unwrap(), base);

    let first = manager
        .addMessage(
            parent.id.clone(),
            ChatMessage::new_with_markdown_timestamp(
                "ai".into(),
                "![chart](images/chart.png)".into(),
                time + 1,
            ),
        )
        .unwrap();
    assert_eq!(first.parts[0].attributes["file_base_path"], base);
    assert_eq!(first.parts[0].attributes["file_node_id"], "output-source");

    // Terminal assistant commits receive fresh stream parts, without the saved attributes.
    let terminal = ChatMessage::new_with_markdown_timestamp(
        "ai".into(), "![](operit_art.jpg)".into(), first.timestamp,
    );
    manager.commitAssistantMessageSegment(parent.id.clone(), terminal, None).unwrap();
    let committed = manager.loadChatMessageVariant(&parent.id, first.timestamp, 0).unwrap();
    assert_eq!(committed.parts[0].attributes["file_base_path"], base);
    assert_eq!(committed.parts[0].attributes["file_node_id"], "output-source");

    let branch = manager
        .commitChatDraft(
            draft(&manager, "branch", time + 259_200_000, None),
            Some((&parent.id, None)),
        )
        .unwrap();
    assert_eq!(manager.fileDirectoryForChat(&branch.id).unwrap(), base);
    assert_eq!(
        manager.loadChatMessages(&branch.id).unwrap()[0].parts[0].attributes["file_base_path"],
        base
    );

    let workspace = manager
        .createWorkspace(
            "project".into(),
            vec![WorkspaceFolder {
                name: "project".into(),
                path: "/operitapp/workspaces/project".into(),
            }],
        )
        .unwrap();
    manager
        .updateChatWorkspaceId(parent.id.clone(), Some(workspace.id))
        .unwrap();
    assert_eq!(
        manager.fileDirectoryForChat(&parent.id).unwrap(),
        "/operitapp/workspaces/project"
    );
    let edited = manager
        .addMessage(
            parent.id.clone(),
            ChatMessage::new_with_markdown_timestamp(
                "ai".into(),
                "![edited](images/chart.png)".into(),
                first.timestamp,
            ),
        )
        .unwrap();
    assert_eq!(
        edited.parts[0].attributes["file_base_path"], base,
        "edits keep the historical base"
    );
    let next = manager
        .addMessage(
            parent.id.clone(),
            ChatMessage::new_with_markdown_timestamp("ai".into(), "new output".into(), time + 2),
        )
        .unwrap();
    assert_eq!(
        next.parts[0].attributes["file_base_path"],
        "/operitapp/workspaces/project"
    );
    manager
        .updateChatWorkspaceId(parent.id.clone(), None)
        .unwrap();
    assert_eq!(manager.fileDirectoryForChat(&parent.id).unwrap(), base);

    manager.promoteChatToRoot(child.id.clone()).unwrap();
    assert_eq!(manager.fileDirectoryForChat(&child.id).unwrap(), base);
    let new_child = manager
        .commitChatDraft(
            draft(
                &manager,
                "after promotion",
                time + 345_600_000,
                Some(&child.id),
            ),
            None,
        )
        .unwrap();
    assert_eq!(manager.fileDirectoryForChat(&new_child.id).unwrap(), base);
    assert!(manager.deleteChatHistory(parent.id.clone()).unwrap());
    for chat in [&child, &grandchild, &branch, &new_child] {
        assert_eq!(manager.fileDirectoryForChat(&chat.id).unwrap(), base);
    }
    assert_eq!(
        host.readBytes(&format!("{storage_base}/report.md"))
            .unwrap(),
        b"persistent output"
    );
    // A reopened store must use the persisted binding, even with a different time.
    assert_eq!(
        ChatOutputStore::new(host.clone())
            .bind(&grandchild.id, time + 999_999_999, None)
            .unwrap(),
        base
    );

    // Migrate a legacy child before removing its unbound parent metadata.
    let legacy_parent = manager
        .commitChatDraft(
            draft(&manager, "legacy parent", time + 432_000_000, None),
            None,
        )
        .unwrap();
    let legacy_child = manager
        .commitChatDraft(
            draft(
                &manager,
                "legacy child",
                time + 518_400_000,
                Some(&legacy_parent.id),
            ),
            None,
        )
        .unwrap();
    let legacy_base = outputs.directory(&legacy_parent.id).unwrap().unwrap();
    host.delete(
        &format!("runtime/data/chat_output_bindings/{}.json", legacy_child.id),
        false,
    )
    .unwrap();
    assert!(outputs.directory(&legacy_child.id).unwrap().is_none());
    manager.deleteChatHistory(legacy_parent.id).unwrap();
    assert_eq!(
        manager.fileDirectoryForChat(&legacy_child.id).unwrap(),
        legacy_base
    );
    std::fs::remove_dir_all(root).unwrap();
}
