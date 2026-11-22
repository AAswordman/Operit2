use super::*;
use crate::services::core::MessageProcessingDelegate::ChatExecutionState;
use operit_host_api::HostManager::HostManager;
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::InputProcessingState::InputProcessingState;
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};

/// Keeps status queries live under holder contention and across every core's stop/restart lifecycle.
#[tokio::test]
async fn chat_status_reads_live_execution_states_without_locking_the_holder() {
    let root = std::env::temp_dir().join(format!("operit-chat-status-{}", uuid::Uuid::new_v4()));
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    setDefaultRuntimeStorageHost(storage.clone());
    setDefaultRuntimeSqliteHost(storage.clone());
    operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(Arc::new(
        NativeHostRuntimeTaskSchedulerHost::new(),
    ));
    let host = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()))
        .withRuntimeStorageHost(storage);
    let runtime = ChatRuntimeHolder::new(Arc::new(host.vfs().unwrap()));
    let state = runtime.processingState();
    let holder = Arc::new(AsyncMutex::new(runtime));
    let support = ToolRuntimeSupportService::create(host, holder.clone());
    assert_eq!(
        support.chatAgentStatus("missing").err().unwrap(),
        "Tool runtime chat processing state is not bound",
    );
    support.bindChatProcessingState(state).unwrap();

    let mut locked = holder.lock().await;
    assert!(holder.try_lock().is_err());
    assert!(!support.chatAgentStatus("missing").unwrap().isProcessing);
    for (slot, chat_id) in [
        (ChatRuntimeSlot::MAIN, "main-chat"),
        (ChatRuntimeSlot::FLOATING, "floating-chat"),
        (ChatRuntimeSlot::DETACHED("window".into()), "detached-chat"),
    ] {
        let delegate = &mut locked.getCore(slot).messageProcessingDelegate;
        for execution in [
            ChatExecutionState {
                taskTitle: Some("整理文档".into()),
                currentRound: 3,
                isLoading: true,
                inputProcessingState: InputProcessingState::Receiving {
                    message: "streaming".into(),
                },
            },
            ChatExecutionState {
                taskTitle: Some("整理文档".into()),
                currentRound: 3,
                isLoading: false,
                inputProcessingState: InputProcessingState::Summarizing {
                    message: "summarizing".into(),
                },
            },
        ] {
            delegate.setChatExecutionStateForTest(chat_id.into(), execution);
            let status = support.chatAgentStatus(chat_id).unwrap();
            assert!(status.isProcessing);
            assert_eq!(status.currentRound, 3);
            assert_eq!(status.taskTitle.as_deref(), Some("整理文档"));
        }
        for terminal in [
            InputProcessingState::Completed,
            InputProcessingState::Idle,
            InputProcessingState::Error {
                message: "failed".into(),
            },
        ] {
            delegate.setChatExecutionStateForTest(
                chat_id.into(),
                ChatExecutionState {
                    taskTitle: None,
                    currentRound: 0,
                    isLoading: false,
                    inputProcessingState: terminal,
                },
            );
            assert!(!support.chatAgentStatus(chat_id).unwrap().isProcessing);
        }
    }

    // Occupancy is true while any live slot still owns work for the same chat.
    for slot in [ChatRuntimeSlot::MAIN, ChatRuntimeSlot::FLOATING] {
        locked
            .getCore(slot)
            .messageProcessingDelegate
            .setChatExecutionStateForTest(
                "shared-chat".into(),
                ChatExecutionState {
                    taskTitle: None,
                    currentRound: 0,
                    isLoading: true,
                    inputProcessingState: InputProcessingState::Idle,
                },
            );
    }
    locked
        .getCore(ChatRuntimeSlot::MAIN)
        .messageProcessingDelegate
        .setChatExecutionStateForTest("shared-chat".into(), ChatExecutionState::idle());
    assert!(support.chatAgentStatus("shared-chat").unwrap().isProcessing);
    let mut stopped_worker = locked
        .getCore(ChatRuntimeSlot::FLOATING)
        .messageProcessingDelegate
        .clone_for_core();
    drop(locked);

    support.stopChatServices().unwrap();
    assert!(!support.chatAgentStatus("shared-chat").unwrap().isProcessing);
    stopped_worker.setChatExecutionStateForTest(
        "shared-chat".into(),
        ChatExecutionState {
            taskTitle: None,
            currentRound: 0,
            isLoading: false,
            inputProcessingState: InputProcessingState::Summarizing {
                message: "obsolete worker".into(),
            },
        },
    );
    support.startChatServices().unwrap();
    let mut restarted = holder.lock().await;
    assert!(!support.chatAgentStatus("shared-chat").unwrap().isProcessing);
    restarted
        .getCore(ChatRuntimeSlot::MAIN)
        .messageProcessingDelegate
        .setChatExecutionStateForTest(
            "shared-chat".into(),
            ChatExecutionState {
                taskTitle: None,
                currentRound: 0,
                isLoading: false,
                inputProcessingState: InputProcessingState::Connecting {
                    message: "new generation".into(),
                },
            },
        );
    assert!(support.chatAgentStatus("shared-chat").unwrap().isProcessing);
    assert!(!support.chatAgentStatus("missing").unwrap().isProcessing);
}
