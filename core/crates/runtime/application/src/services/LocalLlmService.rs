#![allow(non_snake_case)]
use operit_host_api::{HostManager::HostManager, RuntimeStorageHost};
use operit_link::CoreCallRequest;
use operit_local_models::LocalInference::LocalChatResponse;
use operit_local_models::LocalModelProvider::LocalModelProvider;
use operit_model::ModelConfigData::LocalModelRuntimeSettings;
use operit_store::CoreNodeBindingStore::CoreNodeBindingStore;
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use serde_json::Value;
use std::sync::Arc;

/// Allocates a stable inference Binding on the device holding these weights.
pub(crate) fn ensureLocalLlmBinding(
    storage: Arc<dyn RuntimeStorageHost>,
    modelBinding: &str,
) -> Result<String, String> {
    let node = CoreNodeIdentityStore::new(storage.clone())
        .initialize()?
        .nodeId;
    let key = format!("local-llm:{node}:{modelBinding}");
    let store = CoreNodeBindingStore::new(storage)?;
    match store.bindingOptional(&key)? {
        Some(binding) if binding.nodeId != node => {
            return Err("Local inference Binding no longer belongs to this device".into())
        }
        Some(_) => {}
        None => {
            store.create(&key, &node)?;
        }
    }
    Ok(key)
}

/// Uses the same Binding route and target execution permission as other Core calls.
pub(crate) async fn generateRoutedLocalLlm(
    context: HostManager,
    bindingKey: String,
    modelBinding: String,
    requestJson: String,
    settingsJson: String,
) -> Result<LocalChatResponse, String> {
    generateWithRoute(
        context,
        bindingKey,
        modelBinding,
        requestJson,
        settingsJson,
        operit_link::coreRouteRuntime(),
    )
    .await
}

/// Resolves the owning device before executing a local inference request.
async fn generateWithRoute(
    context: HostManager,
    bindingKey: String,
    modelBinding: String,
    requestJson: String,
    settingsJson: String,
    route: Option<Arc<dyn operit_link::CoreRouteRuntime>>,
) -> Result<LocalChatResponse, String> {
    let storage = context
        .runtimeStorageHost
        .clone()
        .ok_or("Local inference requires runtime storage")?;
    let request = CoreCallRequest::new(
        operit_link::nextCoreRouteRequestId("generateLocalLlm"),
        operit_link::CORE_INTERNAL_TARGET,
        "generateLocalLlm",
        operit_link::toCoreValue(
            serde_json::json!({ "bindingKey": bindingKey, "modelBinding": modelBinding,
            "requestJson": requestJson, "settingsJson": settingsJson }),
        )
        .map_err(|error| error.to_string())?,
    );
    if let Some(runtime) = route {
        if runtime
            .shouldRoute("generateLocalLlm", &request.args)
            .map_err(|error| error.to_string())?
        {
            return operit_rslink_runtime::decode_core_call_response::<LocalChatResponse>(
                runtime.call(request).await,
            )
            .map_err(|error| error.to_string());
        }
    }
    if !bindingKey.is_empty() {
        let node = CoreNodeIdentityStore::new(storage.clone())
            .initialize()?
            .nodeId;
        let binding = CoreNodeBindingStore::new(storage.clone())?.binding(&bindingKey)?;
        if binding.nodeId != node {
            return Err(
                "The model's inference device is unavailable; start its Core route connection"
                    .into(),
            );
        }
    }
    executeLocalLlm(context, modelBinding, requestJson, settingsJson).await
}

/// Runs CPU-intensive native inference away from the runtime event loop.
pub(crate) async fn executeLocalLlm(
    context: HostManager,
    modelBinding: String,
    requestJson: String,
    settingsJson: String,
) -> Result<LocalChatResponse, String> {
    let selection =
        super::LocalProviderService::LocalProviderService::parseModelBinding(modelBinding)?;
    let storage = context
        .runtimeStorageHost
        .clone()
        .ok_or("Local inference requires runtime storage")?;
    let provider =
        LocalModelProvider::forRuntimeStorage(storage).map_err(|error| error.to_string())?;
    let runtime = provider
        .resolve(
            &selection,
            operit_local_models::LocalModelManifest::LocalModelKind::Chat,
        )
        .map_err(|error| error.to_string())?;
    let (engine, relative) = match runtime.model.manifest.driver.as_ref() {
        Some(operit_local_models::LocalModelManifest::LocalModelDriver::LlamaCpp { model }) => {
            ("llama-cpp", model)
        }
        Some(operit_local_models::LocalModelManifest::LocalModelDriver::Mnn { config }) => {
            ("mnn", config)
        }
        _ => return Err("Local model has no supported chat driver".into()),
    };
    if relative.starts_with(['/', '\\']) || relative.as_bytes().get(1) == Some(&b':') {
        return Err("Local model driver entry must be relative to its model directory".into());
    }
    let modelPath = operit_vfs::VfsPath::joinVfsPath(&runtime.modelDirectory, relative)?;
    let model = context.vfs()?.file(&modelPath)?;
    let host = context
        .localInferenceHost
        .clone()
        .ok_or("LocalInferenceHost is not configured")?;
    let scheduler = context
        .hostRuntimeTaskSchedulerHost
        .clone()
        .ok_or("Local inference requires a task scheduler")?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    scheduler
        .scheduleHostRuntimeTask(
            "local-llm-inference",
            Box::new(move || {
                let result = host
                    .generateLocalChat(engine, &model, &requestJson, &settingsJson)
                    .map_err(|error| error.to_string())
                    .and_then(|result| {
                        serde_json::from_str(&result).map_err(|error| error.to_string())
                    });
                let _ = sender.send(result);
            }),
        )
        .map_err(|error| error.to_string())?;
    receiver.await.map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_native_storage::NativeRuntimeStorageHost;
    use operit_link::{
        CoreCallResponse, CoreEventStream, CoreLinkError, CoreRouteRuntime, CoreValue,
        CoreWatchRequest,
    };
    use std::{future::Future, pin::Pin, sync::Mutex};

    #[derive(Default)]
    struct InferenceRoute {
        requests: Mutex<Vec<CoreCallRequest>>,
    }
    impl CoreRouteRuntime for InferenceRoute {
        fn shouldRoute(&self, method: &str, _args: &CoreValue) -> Result<bool, CoreLinkError> {
            assert_eq!(method, "generateLocalLlm");
            Ok(true)
        }
        fn call(
            &self,
            request: CoreCallRequest,
        ) -> Pin<Box<dyn Future<Output = CoreCallResponse>>> {
            let response = CoreCallResponse::ok(request.requestId.clone(), operit_link::toCoreValue(
                serde_json::json!({"text":"remote inference", "metadata":{"inputTokens":3,"outputTokens":2}})).unwrap());
            self.requests.lock().unwrap().push(request);
            Box::pin(async move { response })
        }
        fn watch(
            &self,
            _: CoreWatchRequest,
        ) -> Pin<Box<dyn Future<Output = Result<CoreEventStream, CoreLinkError>>>> {
            Box::pin(async { Err(CoreLinkError::new("UNUSED", "unused")) })
        }
    }

    #[tokio::test]
    async fn inference_routes_without_local_weights_or_chat_migration() {
        let root = std::env::temp_dir().join(format!("operit-llm-route-{}", uuid::Uuid::new_v4()));
        let storage: Arc<dyn RuntimeStorageHost> = Arc::new(NativeRuntimeStorageHost::new(
            root.clone(),
            root.join("workspace"),
        ));
        let route = Arc::new(InferenceRoute::default());
        let context = HostManager::default().withRuntimeStorageHost(storage);
        let response = generateWithRoute(
            context,
            "remote-model-binding".into(),
            "qwen@revision".into(),
            r#"{"messages":[{"role":"user","content":"hello"}]}"#.into(),
            "{}".into(),
            Some(route.clone()),
        )
        .await
        .unwrap();
        assert_eq!(response.text, "remote inference");
        let requests = route.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].target, operit_link::CORE_INTERNAL_TARGET);
        let args: Value = operit_link::fromCoreValue(requests[0].args.clone()).unwrap();
        assert_eq!(args["bindingKey"], "remote-model-binding");
        assert_eq!(args["modelBinding"], "qwen@revision");
        assert_eq!(args.as_object().unwrap().len(), 4);
        assert!(!root.join("models").exists());
    }

    #[tokio::test]
    async fn unavailable_remote_inference_never_falls_back_to_this_device() {
        let root =
            std::env::temp_dir().join(format!("operit-llm-offline-{}", uuid::Uuid::new_v4()));
        let storage: Arc<dyn RuntimeStorageHost> = Arc::new(NativeRuntimeStorageHost::new(
            root.clone(),
            root.join("workspace"),
        ));
        CoreNodeBindingStore::new(storage.clone())
            .unwrap()
            .create("remote-model", "another-node")
            .unwrap();
        let context = HostManager::default().withRuntimeStorageHost(storage);
        let error = generateWithRoute(
            context,
            "remote-model".into(),
            "qwen@revision".into(),
            "{}".into(),
            "{}".into(),
            None,
        )
        .await
        .unwrap_err();
        assert!(error.contains("inference device is unavailable"), "{error}");
        std::fs::remove_dir_all(root).unwrap();
    }
}
