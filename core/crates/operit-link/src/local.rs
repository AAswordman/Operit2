use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use serde_json::json;

use crate::client::CoreLinkClient;
use crate::protocol::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreLinkError, CoreObjectPath,
    CoreValue, CoreWatchRequest,
};
use crate::registry::{CoreMethodRegistry, CoreWatchRegistry};

pub struct LocalCoreProxy {
    application: OperitApplication,
    methods: CoreMethodRegistry,
    watches: CoreWatchRegistry,
    nextRequestId: u64,
}

impl LocalCoreProxy {
    pub fn new(application: OperitApplication) -> Self {
        let mut proxy = Self {
            application,
            methods: CoreMethodRegistry::new(),
            watches: CoreWatchRegistry::new(),
            nextRequestId: 1,
        };
        proxy.registerBuiltins();
        proxy
    }

    pub fn callValue(
        &mut self,
        targetPath: impl Into<CoreObjectPath>,
        methodName: impl Into<String>,
        args: CoreValue,
    ) -> Result<CoreValue, CoreLinkError> {
        let request = CoreCallRequest::new(self.nextRequestId(), targetPath, methodName, args);
        CoreLinkClient::call(self, request).result
    }

    pub fn watchValue(
        &mut self,
        targetPath: impl Into<CoreObjectPath>,
        propertyName: impl Into<String>,
        args: CoreValue,
    ) -> Result<CoreEvent, CoreLinkError> {
        let request = CoreWatchRequest::new(self.nextRequestId(), targetPath, propertyName, args);
        CoreLinkClient::watchSnapshot(self, request)
    }

    pub fn methodKeys(&self) -> Vec<String> {
        self.methods.methodKeys()
    }

    pub fn watchKeys(&self) -> Vec<String> {
        self.watches.watchKeys()
    }

    #[allow(non_snake_case)]
    pub fn localApplicationMut(&mut self) -> &mut OperitApplication {
        &mut self.application
    }

    fn nextRequestId(&mut self) -> String {
        let id = self.nextRequestId;
        self.nextRequestId += 1;
        format!("local-{id}")
    }

    fn registerBuiltins(&mut self) {
        self.methods
            .register("application", "onCreate", |application, _request| {
                application.onCreate().map_err(CoreLinkError::internal)?;
                Ok(json!({ "ok": true }))
            });
        self.methods
            .register("application", "initialized", |application, _request| {
                Ok(json!(application.initialized))
            });
        self.methods
            .register("link.registry", "methods", |_application, _request| {
                Ok(json!({ "note": "call LocalCoreProxy::methodKeys on embedded local proxy" }))
            });
        self.methods.register(
            "application.chatRuntimeHolder.MAIN",
            "cancelCurrentMessage",
            |application, _request| {
                let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
                core.cancelCurrentMessage();
                Ok(json!({ "ok": true }))
            },
        );
        self.methods.register(
            "application.chatRuntimeHolder.MAIN",
            "switchChat",
            |application, request| {
                let chatId = required_string_arg(&request.args, "chatId")?;
                let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
                core.switchChat(chatId);
                Ok(json!({ "ok": true }))
            },
        );
        self.methods.register(
            "application.chatRuntimeHolder.MAIN",
            "createNewChat",
            |application, request| {
                let characterCardName = optional_string_arg(&request.args, "characterCardName");
                let characterGroupId = optional_string_arg(&request.args, "characterGroupId");
                let group = optional_string_arg(&request.args, "group");
                let inheritGroupFromCurrent = optional_bool_arg(&request.args, "inheritGroupFromCurrent")
                    .unwrap_or(true);
                let setAsCurrentChat = optional_bool_arg(&request.args, "setAsCurrentChat")
                    .unwrap_or(true);
                let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
                core.createNewChat(
                    characterCardName,
                    group,
                    inheritGroupFromCurrent,
                    setAsCurrentChat,
                    characterGroupId,
                );
                Ok(json!({
                    "chatId": core.currentChatIdFlow().value()
                }))
            },
        );

        self.watches
            .register("application.chatRuntimeHolder.MAIN", "currentChatIdFlow", |application, request| {
                let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
                Ok(CoreEvent {
                    requestId: Some(request.requestId),
                    targetPath: CoreObjectPath::parse("application.chatRuntimeHolder.MAIN"),
                    propertyName: "currentChatIdFlow".to_string(),
                    kind: CoreEventKind::Snapshot,
                    value: json!(core.currentChatIdFlow().value()),
                })
            });
        self.watches
            .register("application.chatRuntimeHolder.MAIN", "chatHistoryFlow", |application, request| {
                let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
                serde_json::to_value(core.chatHistoryFlow().value())
                    .map(|value| CoreEvent {
                        requestId: Some(request.requestId),
                        targetPath: CoreObjectPath::parse("application.chatRuntimeHolder.MAIN"),
                        propertyName: "chatHistoryFlow".to_string(),
                        kind: CoreEventKind::Snapshot,
                        value,
                    })
                    .map_err(|error| CoreLinkError::internal(error.to_string()))
            });
        self.watches
            .register("application.chatRuntimeHolder.MAIN", "chatHistoriesFlow", |application, request| {
                let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
                serde_json::to_value(core.chatHistoriesFlow().value())
                    .map(|value| CoreEvent {
                        requestId: Some(request.requestId),
                        targetPath: CoreObjectPath::parse("application.chatRuntimeHolder.MAIN"),
                        propertyName: "chatHistoriesFlow".to_string(),
                        kind: CoreEventKind::Snapshot,
                        value,
                    })
                    .map_err(|error| CoreLinkError::internal(error.to_string()))
            });
        self.watches.register(
            "application.chatRuntimeHolder.MAIN",
            "inputProcessingStateByChatIdFlow",
            |application, request| {
                let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
                serde_json::to_value(core.inputProcessingStateByChatIdFlow().value())
                    .map(|value| CoreEvent {
                        requestId: Some(request.requestId),
                        targetPath: CoreObjectPath::parse("application.chatRuntimeHolder.MAIN"),
                        propertyName: "inputProcessingStateByChatIdFlow".to_string(),
                        kind: CoreEventKind::Snapshot,
                        value,
                    })
                    .map_err(|error| CoreLinkError::internal(error.to_string()))
            },
        );
        self.watches
            .register("application.chatRuntimeHolder.MAIN", "currentWindowSizeFlow", |application, request| {
                let core = application.chatRuntimeHolder.getCore(ChatRuntimeSlot::MAIN);
                Ok(CoreEvent {
                    requestId: Some(request.requestId),
                    targetPath: CoreObjectPath::parse("application.chatRuntimeHolder.MAIN"),
                    propertyName: "currentWindowSizeFlow".to_string(),
                    kind: CoreEventKind::Snapshot,
                    value: json!(core.currentWindowSizeFlow().value()),
                })
            });
    }
}

fn required_string_arg(args: &CoreValue, name: &str) -> Result<String, CoreLinkError> {
    optional_string_arg(args, name).ok_or_else(|| {
        CoreLinkError::new("INVALID_ARGUMENT", format!("missing string argument: {name}"))
    })
}

fn optional_string_arg(args: &CoreValue, name: &str) -> Option<String> {
    args.get(name)
        .and_then(|value| {
            if value.is_null() {
                None
            } else {
                value.as_str()
            }
        })
        .map(ToString::to_string)
        .filter(|value| !value.trim().is_empty())
}

fn optional_bool_arg(args: &CoreValue, name: &str) -> Option<bool> {
    args.get(name).and_then(|value| value.as_bool())
}

impl CoreLinkClient for LocalCoreProxy {
    fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        if request.targetPath.key() == "link.registry" && request.methodName == "methods" {
            return CoreCallResponse::ok(requestId, json!(self.methodKeys()));
        }
        if request.targetPath.key() == "link.registry" && request.methodName == "watches" {
            return CoreCallResponse::ok(requestId, json!(self.watchKeys()));
        }
        match self.methods.dispatch(&mut self.application, request) {
            Ok(value) => CoreCallResponse::ok(requestId, value),
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    #[allow(non_snake_case)]
    fn watchSnapshot(&mut self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        self.watches.snapshot(&mut self.application, request)
    }
}
