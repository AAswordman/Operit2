#![allow(non_snake_case)]

use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreObjectPath, CoreWatchRequest,
};
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

include!(concat!(env!("OUT_DIR"), "/generated_core_dispatch.rs"));

pub struct LocalCoreProxy {
    application: OperitApplication,
}

impl LocalCoreProxy {
    pub fn new(application: OperitApplication) -> Self {
        Self { application }
    }

    #[allow(non_snake_case)]
    pub fn localApplicationMut(&mut self) -> &mut OperitApplication {
        &mut self.application
    }
}

#[async_trait]
impl CoreLinkClient for LocalCoreProxy {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        match self.dispatchCall(request).await {
            Ok(value) => CoreCallResponse::ok(requestId, value),
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(&mut self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        self.dispatchWatchSnapshot(request)
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.dispatchWatch(request)
    }
}

impl LocalCoreProxy {
    #[allow(non_snake_case)]
    async fn dispatchCall(&mut self, request: CoreCallRequest) -> Result<Value, CoreLinkError> {
        generated_dispatch_core_proxy_call(self, request).await
    }

    #[allow(non_snake_case)]
    fn dispatchWatchSnapshot(&mut self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        generated_dispatch_core_proxy_watch_snapshot(self, request)
    }

    #[allow(non_snake_case)]
    fn dispatchWatch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        generated_dispatch_core_proxy_watch(self, request)
    }
}

fn chat_runtime_slot(path: &CoreObjectPath) -> Option<ChatRuntimeSlot> {
    match path.key().as_str() {
        "chatRuntimeHolder.MAIN" | "chatRuntimeHolder.main" => Some(ChatRuntimeSlot::MAIN),
        "chatRuntimeHolder.FLOATING" | "chatRuntimeHolder.floating" => {
            Some(ChatRuntimeSlot::FLOATING)
        }
        "application.chatRuntimeHolder.MAIN" | "application.chatRuntimeHolder.main" => {
            Some(ChatRuntimeSlot::MAIN)
        }
        "application.chatRuntimeHolder.FLOATING" | "application.chatRuntimeHolder.floating" => {
            Some(ChatRuntimeSlot::FLOATING)
        }
        _ => None,
    }
}

fn object_args(args: Value) -> Result<Map<String, Value>, CoreLinkError> {
    match args {
        Value::Object(value) => Ok(value),
        Value::Null => Ok(Map::new()),
        _ => Err(CoreLinkError::new(
            "INVALID_ARGS",
            "core call args must be a JSON object",
        )),
    }
}

fn decode_core_arg<T: DeserializeOwned>(
    args: &mut Map<String, Value>,
    name: &str,
) -> Result<T, CoreLinkError> {
    let value = args.remove(name).unwrap_or(Value::Null);
    serde_json::from_value(value)
        .map_err(|error| CoreLinkError::new("INVALID_ARGS", format!("{name}: {error}")))
}

fn to_core_value(value: impl serde::Serialize) -> Result<Value, CoreLinkError> {
    serde_json::to_value(value).map_err(|error| CoreLinkError::internal(error.to_string()))
}

fn core_event_stream_channel() -> (tokio::sync::mpsc::UnboundedSender<CoreEvent>, CoreEventStream) {
    tokio::sync::mpsc::unbounded_channel()
}

fn generated_proxy_request_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis();
    format!("core-proxy-{millis}")
}
