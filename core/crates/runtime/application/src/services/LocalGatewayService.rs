#![allow(non_snake_case)]
//! Executes configured HTTP gateways on their Binding owner through the platform HTTP host.
use operit_host_api::{HostManager::HostManager, HttpRequestData, RuntimeStorageHost};
use operit_link::{
    CoreEvent, CoreEventKind, CoreEventStream, CoreStream, CoreStreamSource, CoreValue,
};
use operit_model::ModelConfigData::{ApiProviderType, ProviderProfile};
use operit_providers::chat::llmprovider::EndpointCompleter::EndpointCompleter;
use operit_providers::chat::llmprovider::LocalGatewayTransport::{
    event_value, protocol, GatewayEvent, GatewayRequest,
};
use operit_store::{
    CoreNodeBindingStore::CoreNodeBindingStore, CoreNodeIdentityStore::CoreNodeIdentityStore,
};
use std::sync::Arc;

/// Pins a newly created gateway to the device on which it was configured.
pub(crate) fn createBinding(
    storage: Arc<dyn RuntimeStorageHost>,
    provider: &ProviderProfile,
) -> Result<(), String> {
    protocol(&provider.endpoint)?;
    let node = CoreNodeIdentityStore::new(storage.clone())
        .initialize()?
        .nodeId;
    let store = CoreNodeBindingStore::new(storage)?;
    match store.bindingOptional(&provider.id)? {
        Some(binding) if binding.nodeId != node => {
            Err("Local gateway already belongs to another device".into())
        }
        Some(_) => Ok(()),
        None => store.create(&provider.id, &node).map(|_| ()),
    }
}

/// Requires the routed device to own a configured gateway before opening localhost traffic.
pub(crate) fn requireOwner(
    context: &HostManager,
    binding: &str,
) -> Result<ProviderProfile, String> {
    let storage = context
        .runtimeStorageHost
        .clone()
        .ok_or("Local gateway requires runtime storage")?;
    let node = CoreNodeIdentityStore::new(storage.clone())
        .initialize()?
        .nodeId;
    if CoreNodeBindingStore::new(storage)?.binding(binding)?.nodeId != node {
        return Err("Local gateway device is unavailable; start its Core route connection".into());
    }
    let provider = crate::data::preferences::ModelConfigManager::ModelConfigManager::default()
        .getProviderProfile(binding)
        .map_err(|error| error.to_string())?;
    if provider.providerType != ApiProviderType::LOCAL_GATEWAY {
        return Err("Binding does not identify a local gateway provider".into());
    }
    Ok(provider)
}

/// Opens a single-use HTTP response source whose cancellation closes the owning Host request.
pub(crate) fn response(
    context: HostManager,
    binding: String,
    requestJson: String,
) -> Result<CoreStream<serde_json::Value>, String> {
    let provider = requireOwner(&context, &binding)?;
    let request: GatewayRequest =
        serde_json::from_str(&requestJson).map_err(|error| error.to_string())?;
    let endpoint = EndpointCompleter::completeEndpointForProviderType(
        &provider.endpoint,
        protocol(&provider.endpoint)?,
    );
    if request.url != endpoint {
        return Err("Local gateway request endpoint differs from its configured endpoint".into());
    }
    let host = context
        .httpHost
        .clone()
        .ok_or("Local gateway requires HttpHost")?;
    let request = std::sync::Mutex::new(Some(request));
    let source = CoreStreamSource::new(move |watch| {
        let request = request
            .lock()
            .map_err(|_| {
                operit_link::CoreLinkError::internal("Local gateway request lock poisoned")
            })?
            .take()
            .ok_or_else(|| {
                operit_link::CoreLinkError::new(
                    "GATEWAY_STREAM_ALREADY_OPENED",
                    "Local gateway responses can only be opened once",
                )
            })?;
        let (sender, stream) = CoreEventStream::channel();
        let requestId = watch.requestId;
        let target = watch.target;
        let property = watch.propertyName;
        let emit = Arc::new(move |value: GatewayEvent| {
            let done = matches!(&value, GatewayEvent::Closed(_));
            let value = event_value(value).expect("gateway lifecycle values are serializable");
            let _ = sender.send(CoreEvent {
                requestId: Some(requestId.clone()),
                target: target.clone(),
                propertyName: property.clone(),
                kind: CoreEventKind::Changed,
                value,
            });
            if done {
                let _ = sender.send(CoreEvent {
                    requestId: Some(requestId.clone()),
                    target: target.clone(),
                    propertyName: property.clone(),
                    kind: CoreEventKind::Completed,
                    value: CoreValue::Null,
                });
            }
        });
        let id = format!("local-gateway-{}", uuid::Uuid::new_v4());
        let cancelHost = host.clone();
        let cancelId = id.clone();
        let stream = stream.withOnClose(move || {
            let _ = cancelHost.closeHttpByteStream(&cancelId);
        });
        let head = emit.clone();
        let bytes = emit.clone();
        host.openHttpResponseStream(
            id,
            HttpRequestData {
                url: request.url.clone(),
                method: "POST".into(),
                headers: request.headers.clone(),
                body: request.body.clone(),
                formFields: Vec::new(),
                fileParts: Vec::new(),
                connectTimeoutSeconds: 60,
                readTimeoutSeconds: 1000,
                followRedirects: true,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            },
            Arc::new(move |value| {
                head(GatewayEvent::Head {
                    url: value.finalUrl,
                    status: value.statusCode,
                    message: value.statusMessage,
                    headers: value.headers,
                })
            }),
            Arc::new(move |value| bytes(GatewayEvent::Bytes(value))),
            Arc::new(move |value| emit(GatewayEvent::Closed(value))),
        )
        .map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;
        Ok(stream)
    });
    Ok(CoreStream::fromSource(Arc::new(source)))
}
