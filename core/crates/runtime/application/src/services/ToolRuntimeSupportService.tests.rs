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
        support.isChatProcessing("missing").unwrap_err(),
        "Tool runtime chat processing state is not bound",
    );
    support.bindChatProcessingState(state).unwrap();

    let mut locked = holder.lock().await;
    assert!(holder.try_lock().is_err());
    assert!(!support.isChatProcessing("missing").unwrap());
    for (slot, chat_id) in [
        (ChatRuntimeSlot::MAIN, "main-chat"),
        (ChatRuntimeSlot::FLOATING, "floating-chat"),
        (ChatRuntimeSlot::DETACHED("window".into()), "detached-chat"),
    ] {
        let delegate = &mut locked.getCore(slot).messageProcessingDelegate;
        for execution in [
            ChatExecutionState {
                isLoading: true,
                inputProcessingState: InputProcessingState::Receiving {
                    message: "streaming".into(),
                },
            },
            ChatExecutionState {
                isLoading: false,
                inputProcessingState: InputProcessingState::Summarizing {
                    message: "summarizing".into(),
                },
            },
        ] {
            delegate.setChatExecutionStateForTest(chat_id.into(), execution);
            assert!(support.isChatProcessing(chat_id).unwrap());
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
                    isLoading: false,
                    inputProcessingState: terminal,
                },
            );
            assert!(!support.isChatProcessing(chat_id).unwrap());
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
                    isLoading: true,
                    inputProcessingState: InputProcessingState::Idle,
                },
            );
    }
    locked
        .getCore(ChatRuntimeSlot::MAIN)
        .messageProcessingDelegate
        .setChatExecutionStateForTest("shared-chat".into(), ChatExecutionState::idle());
    assert!(support.isChatProcessing("shared-chat").unwrap());
    let mut stopped_worker = locked
        .getCore(ChatRuntimeSlot::FLOATING)
        .messageProcessingDelegate
        .clone_for_core();
    drop(locked);

    support.stopChatServices().unwrap();
    assert!(!support.isChatProcessing("shared-chat").unwrap());
    stopped_worker.setChatExecutionStateForTest(
        "shared-chat".into(),
        ChatExecutionState {
            isLoading: false,
            inputProcessingState: InputProcessingState::Summarizing {
                message: "obsolete worker".into(),
            },
        },
    );
    support.startChatServices().unwrap();
    let mut restarted = holder.lock().await;
    assert!(!support.isChatProcessing("shared-chat").unwrap());
    restarted
        .getCore(ChatRuntimeSlot::MAIN)
        .messageProcessingDelegate
        .setChatExecutionStateForTest(
            "shared-chat".into(),
            ChatExecutionState {
                isLoading: false,
                inputProcessingState: InputProcessingState::Connecting {
                    message: "new generation".into(),
                },
            },
        );
    assert!(support.isChatProcessing("shared-chat").unwrap());
    assert!(!support.isChatProcessing("missing").unwrap());
}
