/// Resolves one annotation-generated Space route by its wire route ID.
pub fn generated_space_route_for_id(routeId: u32, methodName: &str) -> Option<GeneratedSpaceRoute> {
    match (routeId, methodName) {
        (0, "afterChangeRoute") => Some(GeneratedSpaceRoute { routeId: 0, methodName: "afterChangeRoute", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::AfterChangeRoute }),
        (1, "beforeChangeRoute") => Some(GeneratedSpaceRoute { routeId: 1, methodName: "beforeChangeRoute", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::BeforeChangeRoute }),
        (2, "cancelMessage") => Some(GeneratedSpaceRoute { routeId: 2, methodName: "cancelMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (3, "chatMessagesFlow") => Some(GeneratedSpaceRoute { routeId: 3, methodName: "chatMessagesFlow", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (4, "chatStateFlow") => Some(GeneratedSpaceRoute { routeId: 4, methodName: "chatStateFlow", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (5, "clearPendingQueueAutoDequeueSuppression") => Some(GeneratedSpaceRoute { routeId: 5, methodName: "clearPendingQueueAutoDequeueSuppression", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (6, "deleteMessage") => Some(GeneratedSpaceRoute { routeId: 6, methodName: "deleteMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (7, "deleteMessages") => Some(GeneratedSpaceRoute { routeId: 7, methodName: "deleteMessages", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (8, "deleteMessagesFrom") => Some(GeneratedSpaceRoute { routeId: 8, methodName: "deleteMessagesFrom", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (9, "deletePendingQueueMessage") => Some(GeneratedSpaceRoute { routeId: 9, methodName: "deletePendingQueueMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (10, "enqueuePendingQueueMessage") => Some(GeneratedSpaceRoute { routeId: 10, methodName: "enqueuePendingQueueMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (11, "previewWorkspaceChangesForMessage") => Some(GeneratedSpaceRoute { routeId: 11, methodName: "previewWorkspaceChangesForMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (12, "regenerateSingleAiMessage") => Some(GeneratedSpaceRoute { routeId: 12, methodName: "regenerateSingleAiMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (13, "restorePendingQueueMessage") => Some(GeneratedSpaceRoute { routeId: 13, methodName: "restorePendingQueueMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (14, "resume") => Some(GeneratedSpaceRoute { routeId: 14, methodName: "resume", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (15, "rewindAndResendMessage") => Some(GeneratedSpaceRoute { routeId: 15, methodName: "rewindAndResendMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (16, "rewindWorkspaceForMessage") => Some(GeneratedSpaceRoute { routeId: 16, methodName: "rewindWorkspaceForMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (17, "rollbackToMessage") => Some(GeneratedSpaceRoute { routeId: 17, methodName: "rollbackToMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (18, "routeProbeChatMessagesFlow") => Some(GeneratedSpaceRoute { routeId: 18, methodName: "routeProbeChatMessagesFlow", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (19, "sendUserMessage") => Some(GeneratedSpaceRoute { routeId: 19, methodName: "sendUserMessage", bindingArgument: "chatIdOverride", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (20, "setPendingQueueExpanded") => Some(GeneratedSpaceRoute { routeId: 20, methodName: "setPendingQueueExpanded", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (21, "takeNextPendingQueueMessageIfReady") => Some(GeneratedSpaceRoute { routeId: 21, methodName: "takeNextPendingQueueMessageIfReady", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (22, "takePendingQueueMessage") => Some(GeneratedSpaceRoute { routeId: 22, methodName: "takePendingQueueMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        (23, "updateMessage") => Some(GeneratedSpaceRoute { routeId: 23, methodName: "updateMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        _ => None,
    }
}

/// Resolves one internal annotation route without a Proxy object address.
pub fn generated_space_route_for_method(methodName: &str) -> Option<GeneratedSpaceRoute> {
    match methodName {
        "afterChangeRoute" => Some(GeneratedSpaceRoute { routeId: 0, methodName: "afterChangeRoute", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::AfterChangeRoute }),
        "beforeChangeRoute" => Some(GeneratedSpaceRoute { routeId: 1, methodName: "beforeChangeRoute", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::BeforeChangeRoute }),
        "cancelMessage" => Some(GeneratedSpaceRoute { routeId: 2, methodName: "cancelMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "chatMessagesFlow" => Some(GeneratedSpaceRoute { routeId: 3, methodName: "chatMessagesFlow", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "chatStateFlow" => Some(GeneratedSpaceRoute { routeId: 4, methodName: "chatStateFlow", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "clearPendingQueueAutoDequeueSuppression" => Some(GeneratedSpaceRoute { routeId: 5, methodName: "clearPendingQueueAutoDequeueSuppression", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "deleteMessage" => Some(GeneratedSpaceRoute { routeId: 6, methodName: "deleteMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "deleteMessages" => Some(GeneratedSpaceRoute { routeId: 7, methodName: "deleteMessages", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "deleteMessagesFrom" => Some(GeneratedSpaceRoute { routeId: 8, methodName: "deleteMessagesFrom", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "deletePendingQueueMessage" => Some(GeneratedSpaceRoute { routeId: 9, methodName: "deletePendingQueueMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "enqueuePendingQueueMessage" => Some(GeneratedSpaceRoute { routeId: 10, methodName: "enqueuePendingQueueMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "previewWorkspaceChangesForMessage" => Some(GeneratedSpaceRoute { routeId: 11, methodName: "previewWorkspaceChangesForMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "regenerateSingleAiMessage" => Some(GeneratedSpaceRoute { routeId: 12, methodName: "regenerateSingleAiMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "restorePendingQueueMessage" => Some(GeneratedSpaceRoute { routeId: 13, methodName: "restorePendingQueueMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "resume" => Some(GeneratedSpaceRoute { routeId: 14, methodName: "resume", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "rewindAndResendMessage" => Some(GeneratedSpaceRoute { routeId: 15, methodName: "rewindAndResendMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "rewindWorkspaceForMessage" => Some(GeneratedSpaceRoute { routeId: 16, methodName: "rewindWorkspaceForMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "rollbackToMessage" => Some(GeneratedSpaceRoute { routeId: 17, methodName: "rollbackToMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "routeProbeChatMessagesFlow" => Some(GeneratedSpaceRoute { routeId: 18, methodName: "routeProbeChatMessagesFlow", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "sendUserMessage" => Some(GeneratedSpaceRoute { routeId: 19, methodName: "sendUserMessage", bindingArgument: "chatIdOverride", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "setPendingQueueExpanded" => Some(GeneratedSpaceRoute { routeId: 20, methodName: "setPendingQueueExpanded", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "takeNextPendingQueueMessageIfReady" => Some(GeneratedSpaceRoute { routeId: 21, methodName: "takeNextPendingQueueMessageIfReady", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "takePendingQueueMessage" => Some(GeneratedSpaceRoute { routeId: 22, methodName: "takePendingQueueMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        "updateMessage" => Some(GeneratedSpaceRoute { routeId: 23, methodName: "updateMessage", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::Normal }),
        _ => None,
    }
}

/// Resolves the generated Space route registered for one lifecycle hook.
pub fn generated_space_lifecycle_route(lifecycle: GeneratedRouteLifecycle) -> Option<GeneratedSpaceRoute> {
    match lifecycle {
        GeneratedRouteLifecycle::AfterChangeRoute => Some(GeneratedSpaceRoute { routeId: 0, methodName: "afterChangeRoute", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::AfterChangeRoute }),
        GeneratedRouteLifecycle::BeforeChangeRoute => Some(GeneratedSpaceRoute { routeId: 1, methodName: "beforeChangeRoute", bindingArgument: "chatId", targetType: "operit_runtime::services::ChatServiceCore::ChatServiceCore", lifecycle: GeneratedRouteLifecycle::BeforeChangeRoute }),
        GeneratedRouteLifecycle::Normal => None,
    }
}

/// Resolves one annotation-generated Space route from a standard Link call request.
pub fn generated_space_call_route(request: &operit_link::CoreCallRequest) -> Option<GeneratedSpaceRoute> { if request.targetObjectId == operit_link::CORE_INTERNAL_ROUTE_OBJECT_ID { generated_space_route_for_method(&request.methodName) } else { generated_space_route_for_id(request.targetObjectId, &request.methodName) } }

/// Resolves one annotation-generated Space route from a standard Link watch request.
pub fn generated_space_watch_route(request: &operit_link::CoreWatchRequest) -> Option<GeneratedSpaceRoute> { if request.targetObjectId == operit_link::CORE_INTERNAL_ROUTE_OBJECT_ID { generated_space_route_for_method(&request.propertyName) } else { generated_space_route_for_id(request.targetObjectId, &request.propertyName) } }

/// Resolves one annotation-generated Space route from a standard Link push request.
pub fn generated_space_push_route(request: &operit_link::CorePushRequest) -> Option<GeneratedSpaceRoute> { if request.targetObjectId == operit_link::CORE_INTERNAL_ROUTE_OBJECT_ID { generated_space_route_for_method(&request.methodName) } else { generated_space_route_for_id(request.targetObjectId, &request.methodName) } }

/// Dispatches one generated Space call on the runtime's main ChatServiceCore.
pub async fn generated_space_call_on_chat_core(core: &mut operit_runtime::services::ChatServiceCore::ChatServiceCore, request: operit_link::CoreCallRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {
    match request.methodName.as_str() {
        "afterChangeRoute" => core.__operit_core_route_call_afterChangeRoute(request).await,
        "beforeChangeRoute" => core.__operit_core_route_call_beforeChangeRoute(request).await,
        "cancelMessage" => core.__operit_core_route_call_cancelMessage(request).await,
        "clearPendingQueueAutoDequeueSuppression" => core.__operit_core_route_call_clearPendingQueueAutoDequeueSuppression(request).await,
        "deleteMessage" => core.__operit_core_route_call_deleteMessage(request).await,
        "deleteMessages" => core.__operit_core_route_call_deleteMessages(request).await,
        "deleteMessagesFrom" => core.__operit_core_route_call_deleteMessagesFrom(request).await,
        "deletePendingQueueMessage" => core.__operit_core_route_call_deletePendingQueueMessage(request).await,
        "enqueuePendingQueueMessage" => core.__operit_core_route_call_enqueuePendingQueueMessage(request).await,
        "previewWorkspaceChangesForMessage" => core.__operit_core_route_call_previewWorkspaceChangesForMessage(request).await,
        "regenerateSingleAiMessage" => core.__operit_core_route_call_regenerateSingleAiMessage(request).await,
        "restorePendingQueueMessage" => core.__operit_core_route_call_restorePendingQueueMessage(request).await,
        "resume" => core.__operit_core_route_call_resume(request).await,
        "rewindAndResendMessage" => core.__operit_core_route_call_rewindAndResendMessage(request).await,
        "rewindWorkspaceForMessage" => core.__operit_core_route_call_rewindWorkspaceForMessage(request).await,
        "rollbackToMessage" => core.__operit_core_route_call_rollbackToMessage(request).await,
        "sendUserMessage" => core.__operit_core_route_call_sendUserMessage(request).await,
        "setPendingQueueExpanded" => core.__operit_core_route_call_setPendingQueueExpanded(request).await,
        "takeNextPendingQueueMessageIfReady" => core.__operit_core_route_call_takeNextPendingQueueMessageIfReady(request).await,
        "takePendingQueueMessage" => core.__operit_core_route_call_takePendingQueueMessage(request).await,
        "updateMessage" => core.__operit_core_route_call_updateMessage(request).await,
        _ => Err(operit_link::CoreLinkError::methodNotFound(&request.registryKey())),
    }
}

/// Reads one generated Space watch snapshot on the runtime's main ChatServiceCore.
pub async fn generated_space_watch_snapshot_on_chat_core(core: &mut operit_runtime::services::ChatServiceCore::ChatServiceCore, request: &operit_link::CoreWatchRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {
    match request.propertyName.as_str() {
        "chatMessagesFlow" => core.__operit_core_route_watch_snapshot_chatMessagesFlow(request).await,
        "chatStateFlow" => core.__operit_core_route_watch_snapshot_chatStateFlow(request).await,
        "routeProbeChatMessagesFlow" => core.__operit_core_route_watch_snapshot_routeProbeChatMessagesFlow(request).await,
        _ => Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),
    }
}

/// Opens one generated Space watch on the runtime's main ChatServiceCore.
pub async fn generated_space_watch_on_chat_core(core: &mut operit_runtime::services::ChatServiceCore::ChatServiceCore, request: operit_link::CoreWatchRequest, attachmentAdopter: std::sync::Arc<dyn Fn(Vec<operit_link::CoreStreamAttachment>) + Send + Sync>) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {
    match request.propertyName.as_str() {
        "chatMessagesFlow" => core.__operit_core_route_watch_chatMessagesFlow(request, attachmentAdopter).await,
        "chatStateFlow" => core.__operit_core_route_watch_chatStateFlow(request, attachmentAdopter).await,
        "routeProbeChatMessagesFlow" => core.__operit_core_route_watch_routeProbeChatMessagesFlow(request, attachmentAdopter).await,
        _ => Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),
    }
}

/// Resolves one request using route declarations from runtime annotations.
fn generated_route_for_request(methodName: &str, args: &operit_link::CoreValue) -> Result<GeneratedCoreRoute, operit_link::CoreLinkError> {
    let bindingArgument = match methodName {
        "afterChangeRoute" => Some("chatId"),
        "beforeChangeRoute" => Some("chatId"),
        "cancelMessage" => Some("chatId"),
        "chatMessagesFlow" => Some("chatId"),
        "chatStateFlow" => Some("chatId"),
        "clearPendingQueueAutoDequeueSuppression" => Some("chatId"),
        "deleteMessage" => Some("chatId"),
        "deleteMessages" => Some("chatId"),
        "deleteMessagesFrom" => Some("chatId"),
        "deletePendingQueueMessage" => Some("chatId"),
        "enqueuePendingQueueMessage" => Some("chatId"),
        "previewWorkspaceChangesForMessage" => Some("chatId"),
        "regenerateSingleAiMessage" => Some("chatId"),
        "restorePendingQueueMessage" => Some("chatId"),
        "resume" => Some("chatId"),
        "rewindAndResendMessage" => Some("chatId"),
        "rewindWorkspaceForMessage" => Some("chatId"),
        "rollbackToMessage" => Some("chatId"),
        "routeProbeChatMessagesFlow" => Some("chatId"),
        "sendUserMessage" => Some("chatIdOverride"),
        "setPendingQueueExpanded" => Some("chatId"),
        "takeNextPendingQueueMessageIfReady" => Some("chatId"),
        "takePendingQueueMessage" => Some("chatId"),
        "updateMessage" => Some("chatId"),
        _ => None,
    };
    let Some(bindingArgument) = bindingArgument else { return Ok(GeneratedCoreRoute::Local); };
    let operit_link::CoreValue::Map(arguments) = args else { return Err(operit_link::CoreLinkError::new("INVALID_ARGS", "Binding request arguments must be a map")); };
    let Some(value) = arguments.get(bindingArgument) else { return Err(operit_link::CoreLinkError::new("CORE_BINDING_KEY_REQUIRED", "Binding request does not include its required key")); };
    let key = match value {
        operit_link::CoreValue::String(key) => key,
        operit_link::CoreValue::Null => return Ok(GeneratedCoreRoute::Local),
        _ => return Err(operit_link::CoreLinkError::new("CORE_BINDING_KEY_INVALID", "Binding key must be a string")),
    };
    if key.trim().is_empty() { return Err(operit_link::CoreLinkError::new("CORE_BINDING_KEY_REQUIRED", "Binding requires a non-empty key")); }
    Ok(GeneratedCoreRoute::Binding { scope: 0, key: key.clone() })
}

/// Resolves one call request using route declarations from runtime annotations.
pub fn generated_core_call_route(request: &operit_link::CoreCallRequest) -> Result<GeneratedCoreRoute, operit_link::CoreLinkError> { generated_route_for_request(&request.methodName, &request.args) }

/// Resolves one watch request using route declarations from runtime annotations.
pub fn generated_core_watch_route(request: &operit_link::CoreWatchRequest) -> Result<GeneratedCoreRoute, operit_link::CoreLinkError> { generated_route_for_request(&request.propertyName, &request.args) }

/// Resolves one push request using route declarations from runtime annotations.
pub fn generated_core_push_route(request: &operit_link::CorePushRequest) -> Result<GeneratedCoreRoute, operit_link::CoreLinkError> { generated_route_for_request(&request.methodName, &request.args) }
