use operit_runtime::services::ChatServiceCore::{ChatInputMenuSettings, ChatInputMenuSummary};
use operit_tools::tools::ToolPermissionSystem::AiPermissionMode;

/// A remote execution Core that records the full request, including its binding key.
struct MenuEndpoint {
    calls: StdMutex<Vec<CoreCallRequest>>,
}

#[async_trait]
impl TestRouteTarget for MenuEndpoint {
    async fn routedCall(
        &self,
        _: String,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse {
        assert_eq!(request.routeKind, RoutedCoreRequestKind::SpaceRoute);
        let payload = request.payload;
        let value = match payload.methodName.as_str() {
            "chatInputMenuSettings" => operit_link::toCoreValue(ChatInputMenuSettings {
                permissionMode: AiPermissionMode::Full,
                disableStreamOutput: true,
                disableUserPreferenceDescription: true,
                pluginChangeVersion: 42,
                pluginToggles: vec![],
            })
            .unwrap(),
            "chatInputMenuSummary" => operit_link::toCoreValue(ChatInputMenuSummary {
                currentWindowSize: 123,
                inputTokenCount: 456,
                outputTokenCount: 789,
                maxContextLength: 64.0,
            })
            .unwrap(),
            "triggerChatInputMenuToggle" => operit_link::toCoreValue(true).unwrap(),
            _ => CoreValue::Null,
        };
        let requestId = payload.requestId.clone();
        self.calls.lock().unwrap().push(payload);
        CoreCallResponse::ok(requestId, value)
    }

    async fn routedWatchSnapshot(
        &self,
        _: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError> {
        Err(CoreLinkError::watchNotFound(&request.payload.registryKey()))
    }
    async fn routedWatch(
        &self,
        _: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError> {
        Err(CoreLinkError::watchNotFound(&request.payload.registryKey()))
    }
    async fn routedOpenPush(
        &self,
        _: String,
        request: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        Err(CoreLinkError::new(
            "TEST_UNSUPPORTED_PUSH",
            request.payload.target,
        ))
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn chat_menu_reads_writes_and_permission_decisions_follow_remote_binding() {
    let _globalGuard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let phone = "menu-phone".to_string();
    let computer = "menu-computer".to_string();
    let chatId = "menu-computer-chat".to_string();
    let (router, holder, _) = testCoreNodeRouterInJoinedSpaceWithBindingRuntime(
        &phone,
        &computer,
        &chatId,
        &computer,
        CoreSpace {
            spaceId: "menu-space".into(),
            spaceName: "Menu Space".into(),
            spaceRevision: 2,
            members: vec![phone.clone(), computer.clone()],
        },
    );
    let remote = Arc::new(MenuEndpoint {
        calls: StdMutex::new(vec![]),
    });
    let peer = installTestPeer(&router, computer, remote.clone()).unwrap();
    let _routeGuard = installTestCoreRouteRuntime(Arc::new(router));
    {
        let mut holder = holder.lock().await;
        // This local Core has no enhanced AI service: any accidental local menu
        // settings read fails rather than silently matching the fixture.
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        let settings = core
            .chatInputMenuSettings(Some(chatId.clone()))
            .await
            .unwrap();
        assert_eq!(settings.permissionMode, AiPermissionMode::Full);
        assert!(settings.disableStreamOutput);
        assert_eq!(settings.pluginChangeVersion, 42);
        let summary = core
            .chatInputMenuSummary(Some(chatId.clone()))
            .await
            .unwrap();
        assert_eq!(
            (
                summary.currentWindowSize,
                summary.inputTokenCount,
                summary.outputTokenCount
            ),
            (123, 456, 789)
        );
        core.saveChatInputMenuSettings(
            Some(chatId.clone()),
            Some(AiPermissionMode::WorkspaceWrite),
            None,
            None,
        )
        .await
        .unwrap();
        assert!(
            core.triggerChatInputMenuToggle(Some(chatId.clone()), "remote-toggle".into())
                .await
        );
        for result in ["allow", "deny", "allow_session"] {
            core.respondChatToolPermission(chatId.clone(), "remote-request".into(), result.into())
                .await
                .unwrap();
        }
    }
    let calls = remote.calls.lock().unwrap();
    assert_eq!(calls.len(), 7);
    for call in calls.iter() {
        let CoreValue::Map(args) = &call.args else {
            panic!("route args must be a map")
        };
        assert_eq!(args.get("chatId"), Some(&CoreValue::String(chatId.clone())));
    }
    assert_eq!(
        calls
            .iter()
            .filter(|c| c.methodName == "respondChatToolPermission")
            .count(),
        3
    );
    let CoreValue::Map(args) = &calls[2].args else {
        panic!("route args must be a map")
    };
    assert!(!args.contains_key("enableMemoryAutoUpdate"));
    assert_eq!(args.get("disableStreamOutput"), Some(&CoreValue::Null));
    peer.close();
}
