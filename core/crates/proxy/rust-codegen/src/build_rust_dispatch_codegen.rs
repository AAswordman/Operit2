use super::build_rust_codegen_utils::*;
use super::*;

pub(crate) fn render_object_call_dispatch(
    object: &SourceObject,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "async fn generated_dispatch_{}_call(object: &mut {}, request: operit_link::CoreCallRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    match request.methodName.as_str() {\n");
    for method in object
        .methods
        .iter()
        .filter(|method| method.is_async && method.call_protocol().is_some())
    {
        output.push_str(&render_async_call_arm(object, method));
    }
    output
        .push_str("        _ => Err(operit_link::CoreLinkError::methodNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    for method in object
        .methods
        .iter()
        .filter(|method| method.is_async && !method.returns_owned_future && method.call_protocol().is_some())
    {
        output.push('\n');
        output.push_str(&render_async_call_helper(object, method, error_types));
    }
    output
}

pub(crate) fn render_object_sync_call_dispatch(
    object: &SourceObject,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_call_sync(object: &mut {}, request: operit_link::CoreCallRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output
        .push_str("    let mut __core_args = operit_rslink_runtime::object_args(request.args)?;\n");
    output.push_str("    match request.methodName.as_str() {\n");
    for method in object
        .methods
        .iter()
        .filter(|method| !method.is_async && method.call_protocol().is_some())
    {
        output.push_str(&render_call_arm(method, error_types));
    }
    if object.schema_key == "application" {
        output.push_str("        \"coreProxySchema\" => Ok(generated_core_proxy_schema()),\n");
    }
    output
        .push_str("        _ => Err(operit_link::CoreLinkError::methodNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

pub(crate) fn render_object_watch_snapshot_dispatch(object: &SourceObject) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_watch_snapshot(object: &mut {}, request: &operit_link::CoreWatchRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str(
        "    let mut __core_args = operit_rslink_runtime::object_args(request.args.clone())?;\n",
    );
    output.push_str("    match request.propertyName.as_str() {\n");
    for method in object.methods.iter().filter(|method| {
        !method.is_async
            && method
                .watch_protocol()
                .and_then(|watch| watch.snapshot_type.as_ref())
                .is_some()
    }) {
        output.push_str(&render_watch_snapshot_arm(method, false));
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::watchNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

pub(crate) fn render_object_watch_snapshot_async_dispatch(object: &SourceObject) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "async fn generated_dispatch_{}_watch_snapshot_async(object: &mut {}, request: &operit_link::CoreWatchRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str(
        "    let mut __core_args = operit_rslink_runtime::object_args(request.args.clone())?;\n",
    );
    output.push_str("    match request.propertyName.as_str() {\n");
    for method in object.methods.iter().filter(|method| {
        method
            .watch_protocol()
            .and_then(|watch| watch.snapshot_type.as_ref())
            .is_some()
    }) {
        output.push_str(&render_watch_snapshot_arm(method, true));
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::watchNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

pub(crate) fn render_object_watch_dispatch(object: &SourceObject) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_watch(object: &mut {}, request: operit_link::CoreWatchRequest, attachmentAdopter: std::sync::Arc<dyn Fn(Vec<operit_link::CoreStreamAttachment>) + Send + Sync>) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str(
        "    let mut __core_args = operit_rslink_runtime::object_args(request.args.clone())?;\n",
    );
    output.push_str("    match request.propertyName.as_str() {\n");
    for method in object
        .methods
        .iter()
        .filter(|method| !method.is_async && method.watch_protocol().is_some())
    {
        output.push_str(&render_watch_stream_arm(method, false));
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::watchNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

pub(crate) fn render_object_watch_async_dispatch(object: &SourceObject) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "async fn generated_dispatch_{}_watch_async(object: &mut {}, request: operit_link::CoreWatchRequest, attachmentAdopter: std::sync::Arc<dyn Fn(Vec<operit_link::CoreStreamAttachment>) + Send + Sync>) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str(
        "    let mut __core_args = operit_rslink_runtime::object_args(request.args.clone())?;\n",
    );
    output.push_str("    match request.propertyName.as_str() {\n");
    for method in object
        .methods
        .iter()
        .filter(|method| method.watch_protocol().is_some())
    {
        output.push_str(&render_watch_stream_arm(method, true));
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::watchNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

/// Renders the generic source activation dispatcher for one generated watch object.
pub(crate) fn render_object_watch_transition_dispatch(object: &SourceObject) -> String {
    let _ = object;
    String::new()
}

/// Renders the canonical concrete-path predicate for every generated object.
pub(crate) fn render_object_path_matchers(objects: &[SourceObject]) -> String {
    let mut output = String::new();
    for object in objects {
        output.push_str(&format!(
            "/// Returns whether a concrete path resolves to generated object `{}`.\n",
            object.schema_key
        ));
        output.push_str(&format!(
            "fn generated_object_id_matches_{}(object_id: &str) -> bool {{\n",
            object.dispatch_name
        ));
        output.push_str(&format!("    {}\n", render_object_path_predicate(object)));
        output.push_str("}\n\n");
    }
    output
}

/// Renders the concrete object-id predicate implied by one dispatch access strategy.
fn render_object_path_predicate(object: &SourceObject) -> String {
    let _ = &object.path_match;
    format!("object_id == {}", format!("{:?}", object.object_id))
}

/// Renders remote watches through typed decoding and the proxy's attachment sink.
fn render_routed_watch_dispatch(objects: &[SourceObject], snapshot: bool) -> String {
    let mut output = String::from("    if let Some(runtime) = operit_link::coreRouteRuntime() {\n        if runtime.shouldRouteWatch(&request.propertyName, &request.args)? {\n            match (request.target.as_str(), request.propertyName.as_str()) {\n");
    for object in objects {
        for method in &object.methods {
            let Some(watch) = method.watch_protocol() else { continue; };
            if !method.is_async || !matches!(watch.stream, WatchStreamProtocol::JsonState { fallible: false }) {
                continue;
            }
            let value_type = watch.snapshot_type.as_ref().expect("StateFlow must have a snapshot type");
            output.push_str(&render_cfg_attrs(method));
            output.push_str(&format!(
                "                ({}, {:?}) => {{\n                    let mut stream = operit_rslink_runtime::core_route_proxy_watch::<{}>(runtime, request, proxy.streamAttachmentAdopter(), {}).await?;\n",
                format!("{:?}", object.object_id), method.name, value_type, snapshot,
            ));
            if snapshot {
                output.push_str("                    return stream.recv().await.ok_or_else(|| operit_link::CoreLinkError::new(\"WATCH_STREAM_EMPTY\", \"Routed snapshot stream is empty\"));\n");
            } else {
                output.push_str("                    return Ok(stream);\n");
            }
            output.push_str("                }\n");
        }
    }
    output.push_str("                _ => return Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),\n            }\n        }\n    }\n");
    output
}

/// Renders proxy dispatch and translates routed requests into the annotation namespace.
pub(crate) fn render_core_proxy_dispatch(objects: &[SourceObject]) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("async fn generated_dispatch_core_proxy_call(proxy: &LocalCoreProxy, request: operit_link::CoreCallRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {\n");
    output.push_str("    if let Some(__core_route_runtime) = operit_link::coreRouteRuntime() {\n        if __core_route_runtime.shouldRoute(&request.methodName, &request.args)? {\n            let request = operit_link::CoreCallRequest { target: operit_link::CORE_INTERNAL_TARGET.to_string(), ..request };\n            return __core_route_runtime.call(request).await.result;\n        }\n    }\n");
    output.push_str("    #[cfg(not(target_arch = \"wasm32\"))]\n");
    let application_id = objects
        .iter()
        .find(|object| object.schema_key == "application")
        .expect("application object must be generated")
        .object_id
        .clone();
    output.push_str(&format!("    if request.target == {application_id:?} && request.methodName == \"runCoreCommand\" {{\n"));
    output.push_str(
        "        let mut __core_args = operit_rslink_runtime::object_args(request.args)?;\n",
    );
    output.push_str(
        "        let args: Vec<String> = operit_rslink_runtime::decode_core_arg(&mut __core_args, \"args\")?;\n",
    );
    output.push_str("        let application = proxy.application.clone();\n");
    output.push_str(
        "        let (commandSender, commandReceiver) = tokio::sync::oneshot::channel();\n",
    );
    output.push_str(
        "        operit_host_api::HostRuntimeTaskSchedulerHost::scheduleHostRuntimeAsyncTask(\n",
    );
    output.push_str("            operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost().as_ref(),\n");
    output.push_str("            \"core-proxy-command\",\n");
    output.push_str("            Box::new(move || Box::pin(async move {\n");
    output.push_str("                let mut application = application.lock().await.sharedCoreCommandRuntime();\n");
    output.push_str("                let output = operit_command_core::run_core_command(&mut application, &args).await\n");
    output.push_str("                    .map_err(operit_link::CoreLinkError::command);\n");
    output.push_str("                let _ = commandSender.send(output);\n");
    output.push_str("            })),\n");
    output.push_str("        )\n");
    output.push_str(
        "        .map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n",
    );
    output.push_str("        let output = commandReceiver.await.map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))??;\n");
    output.push_str("        return operit_rslink_runtime::to_core_value(output);\n");
    output.push_str("    }\n");
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application && object.has_call_dispatch())
    {
        let owned_pattern = render_owned_call_method_pattern(application);
        if !owned_pattern.is_empty() {
            output.push_str(&format!(
                "    if request.target == {:?} && matches!(request.methodName.as_str(), {}) {{\n        let pending = {{\n            let mut application = proxy.application.lock().await;\n            generated_dispatch_{}_capture(&mut application, request)?\n        }};\n        return pending.await;\n    }}\n",
                application.object_id, owned_pattern, application.dispatch_name));
        }
        output.push_str(&format!(
            "    if request.target == {} {{\n        let mut application = proxy.application.lock().await;\n        return {};\n    }}\n",
            format!("{:?}", application.object_id),
            render_direct_call_dispatch_expression(application, "&mut application", "request", "        ")
        ));
    }
    for object in objects.iter().filter(|object| object.has_call_dispatch()) {
        let Some((holder_field, resolver_method)) = resolved_holder_metadata(&object.access) else {
            continue;
        };
        let holder_lookup = render_resolved_holder_lookup(
            object,
            resolver_method,
            "request.target.as_str()",
            "            ",
        );
        let owned_pattern = render_owned_call_method_pattern(object);
        if !owned_pattern.is_empty() {
            output.push_str(&format!(
                "    if generated_object_id_matches_{}(request.target.as_str()) && matches!(request.methodName.as_str(), {}) {{\n        let pending = {{\n            let mut holder = proxy.{holder_field}.lock().await;\n            let object = {holder_lookup}.ok_or_else(|| operit_link::CoreLinkError::methodNotFound(&request.registryKey()))?;\n            generated_dispatch_{}_capture(object, request)?\n        }};\n        return pending.await;\n    }}\n",
                object.dispatch_name, owned_pattern, object.dispatch_name));
        }
        output.push_str(&format!(
            "    if generated_object_id_matches_{}(request.target.as_str()) {{\n        let mut holder = proxy.{holder_field}.lock().await;\n        if let Some(object) = {holder_lookup} {{\n            return {};\n        }}\n    }}\n",
            object.dispatch_name,
            render_direct_call_dispatch_expression(object, "object", "request", "            ")
        ));
    }
    output.push_str("    match request.target.as_str() {\n");
    for object in objects
        .iter()
        .filter(|object| object.has_call_dispatch())
        .filter(|object| matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. }))
    {
        output.push_str(&render_factory_constructible_dispatch(
            object,
            DispatchMode::Call,
        ));
    }
    for object in objects
        .iter()
        .filter(|object| object.has_call_dispatch())
        .filter(|object| object.access == ObjectAccess::StringNewConstruct)
    {
        output.push_str(&render_string_constructible_dispatch(
            object,
            DispatchMode::Call,
        ));
    }
    for object in objects.iter().filter(|object| {
        object.has_call_dispatch()
            && object.access.is_constructible()
            && object.access != ObjectAccess::StringNewConstruct
            && !matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. })
    }) {
        output.push_str(&format!(
            "        {} => {{\n{}{}        }}\n",
            format!("{:?}", object.object_id),
            render_object_constructor(object, DispatchMode::Call),
            render_constructed_dispatch(object, DispatchMode::Call)
        ));
    }
    output.push_str(
        "        _ => Err(operit_link::CoreLinkError::methodNotFound(&request.registryKey())),\n",
    );
    output.push_str("    }\n}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("async fn generated_dispatch_core_proxy_watch_snapshot_async(proxy: &LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEvent, operit_link::CoreLinkError> {\n");
    output.push_str(&render_routed_watch_dispatch(objects, true));
    for object in objects {
        let Some((holder_field, resolver_method)) = resolved_holder_metadata(&object.access) else {
            continue;
        };
        let holder_lookup = render_resolved_holder_lookup(
            object,
            resolver_method,
            "request.target.as_str()",
            "        ",
        );
        output.push_str(&format!(
            "    if generated_object_id_matches_{}(request.target.as_str()) {{\n        let propertyName = request.propertyName.clone();\n        let mut holder = proxy.{holder_field}.lock().await;\n        if let Some(object) = {holder_lookup} {{\n            let value = generated_dispatch_{}_watch_snapshot_async(object, &request).await?;\n            return Ok(operit_link::CoreEvent {{ requestId: Some(request.requestId), target: request.target.clone(), propertyName, kind: operit_link::CoreEventKind::Snapshot, value }});\n        }}\n    }}\n",
            object.dispatch_name,
            object.dispatch_name
        ));
    }
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "    if request.target == {} {{\n        let propertyName = request.propertyName.clone();\n        let mut application = proxy.application.lock().await;\n        let value = generated_dispatch_{}_watch_snapshot_async(&mut application, &request).await?;\n        return Ok(operit_link::CoreEvent {{ requestId: Some(request.requestId), target: request.target.clone(), propertyName, kind: operit_link::CoreEventKind::Snapshot, value }});\n    }}\n",
            format!("{:?}", application.object_id), application.dispatch_name
        ));
    }
    for object in objects
        .iter()
        .filter(|object| matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. }))
    {
        output.push_str(&render_async_factory_watch(
            object,
            DispatchMode::WatchSnapshot,
        ));
    }
    output.push_str("    generated_dispatch_core_proxy_watch_snapshot(proxy, request)\n");
    output.push_str("}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("async fn generated_dispatch_core_proxy_watch_async(proxy: &LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {\n");
    output.push_str("    if request.target == operit_link::CORE_STREAM_TARGET {\n        return proxy.openCoreStreamWatch(request);\n    }\n");
    output.push_str(&render_routed_watch_dispatch(objects, false));
    for object in objects {
        let Some((holder_field, resolver_method)) = resolved_holder_metadata(&object.access) else {
            continue;
        };
        let holder_lookup = render_resolved_holder_lookup(
            object,
            resolver_method,
            "request.target.as_str()",
            "        ",
        );
        output.push_str(&format!(
            "    if generated_object_id_matches_{}(request.target.as_str()) {{\n        let mut holder = proxy.{holder_field}.lock().await;\n        if let Some(object) = {holder_lookup} {{\n            return generated_dispatch_{}_watch_async(object, request, proxy.streamAttachmentAdopter()).await;\n        }}\n    }}\n",
            object.dispatch_name,
            object.dispatch_name
        ));
    }
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "    if request.target == {} {{\n        let mut application = proxy.application.lock().await;\n        return generated_dispatch_{}_watch_async(&mut application, request, proxy.streamAttachmentAdopter()).await;\n    }}\n",
            format!("{:?}", application.object_id), application.dispatch_name
        ));
    }
    for object in objects
        .iter()
        .filter(|object| matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. }))
    {
        output.push_str(&render_async_factory_watch(object, DispatchMode::Watch));
    }
    output.push_str("    generated_dispatch_core_proxy_watch(proxy, request)\n");
    output.push_str("}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_dispatch_core_proxy_watch_snapshot(proxy: &LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEvent, operit_link::CoreLinkError> {\n");
    for object in objects {
        let Some((holder_field, resolver_method)) = resolved_holder_metadata(&object.access) else {
            continue;
        };
        let holder_lookup = render_resolved_holder_lookup(
            object,
            resolver_method,
            "request.target.as_str()",
            "        ",
        );
        output.push_str(&format!(
            "    if generated_object_id_matches_{}(request.target.as_str()) {{\n        let propertyName = request.propertyName.clone();\n        let mut holder = proxy.{holder_field}.try_lock().map_err(|_| operit_link::CoreLinkError::internal(\"Resolved holder is busy\"))?;\n        if let Some(object) = {holder_lookup} {{\n            let value = generated_dispatch_{}_watch_snapshot(object, &request)?;\n            return Ok(operit_link::CoreEvent {{ requestId: Some(request.requestId), target: request.target.clone(), propertyName, kind: operit_link::CoreEventKind::Snapshot, value }});\n        }}\n    }}\n",
            object.dispatch_name,
            object.dispatch_name
        ));
    }
    output.push_str("    let propertyName = request.propertyName.clone();\n");
    output.push_str("    let value = match request.target.as_str() {\n");
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "        {} => {{ let mut application = proxy.application.try_lock().map_err(|_| operit_link::CoreLinkError::internal(\"Application is busy\"))?; generated_dispatch_{}_watch_snapshot(&mut application, &request)? }},\n",
            format!("{:?}", application.object_id), application.dispatch_name
        ));
    }
    for object in objects
        .iter()
        .filter(|object| object.access == ObjectAccess::StringNewConstruct)
    {
        output.push_str(&render_string_constructible_dispatch(
            object,
            DispatchMode::WatchSnapshot,
        ));
    }
    for object in objects
        .iter()
        .filter(|object| matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. }))
    {
        output.push_str(&render_factory_constructible_dispatch(
            object,
            DispatchMode::WatchSnapshot,
        ));
    }
    for object in objects.iter().filter(|object| {
        object.access.is_constructible()
            && object.access != ObjectAccess::StringNewConstruct
            && !matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. })
    }) {
        output.push_str(&format!(
            "        {} => {{\n{}{}        }}\n",
            format!("{:?}", object.object_id),
            render_object_constructor(object, DispatchMode::WatchSnapshot),
            render_constructed_dispatch(object, DispatchMode::WatchSnapshot)
        ));
    }
    output.push_str("        _ => return Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),\n");
    output.push_str("    };\n");
    output.push_str("    Ok(operit_link::CoreEvent { requestId: Some(request.requestId), target: request.target.clone(), propertyName, kind: operit_link::CoreEventKind::Snapshot, value })\n");
    output.push_str("}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_dispatch_core_proxy_watch(proxy: &LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {\n    let attachmentAdopter = proxy.streamAttachmentAdopter();\n");
    output.push_str("    if request.target == operit_link::CORE_STREAM_TARGET {\n        return proxy.openCoreStreamWatch(request);\n    }\n");
    for object in objects {
        let Some((holder_field, resolver_method)) = resolved_holder_metadata(&object.access) else {
            continue;
        };
        let holder_lookup = render_resolved_holder_lookup(
            object,
            resolver_method,
            "request.target.as_str()",
            "        ",
        );
        output.push_str(&format!(
            "    if generated_object_id_matches_{}(request.target.as_str()) {{\n        let mut holder = proxy.{holder_field}.try_lock().map_err(|_| operit_link::CoreLinkError::internal(\"Resolved holder is busy\"))?;\n        if let Some(object) = {holder_lookup} {{\n            return generated_dispatch_{}_watch(object, request, attachmentAdopter.clone());\n        }}\n    }}\n",
            object.dispatch_name,
            object.dispatch_name
        ));
    }
    output.push_str("    match request.target.as_str() {\n");
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "        {} => {{ let mut application = proxy.application.try_lock().map_err(|_| operit_link::CoreLinkError::internal(\"Application is busy\"))?; generated_dispatch_{}_watch(&mut application, request, attachmentAdopter.clone()) }},\n",
            format!("{:?}", application.object_id), application.dispatch_name
        ));
    }
    for object in objects
        .iter()
        .filter(|object| object.access == ObjectAccess::StringNewConstruct)
    {
        output.push_str(&render_string_constructible_dispatch(
            object,
            DispatchMode::Watch,
        ));
    }
    for object in objects
        .iter()
        .filter(|object| matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. }))
    {
        output.push_str(&render_factory_constructible_dispatch(
            object,
            DispatchMode::Watch,
        ));
    }
    for object in objects.iter().filter(|object| {
        object.access.is_constructible()
            && object.access != ObjectAccess::StringNewConstruct
            && !matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. })
    }) {
        output.push_str(&format!(
            "        {} => {{\n{}{}        }}\n",
            format!("{:?}", object.object_id),
            render_object_constructor(object, DispatchMode::Watch),
            render_constructed_dispatch(object, DispatchMode::Watch)
        ));
    }
    output.push_str(
        "        _ => Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),\n",
    );
    output.push_str("    }\n}\n");
    output
}

#[derive(Clone, Copy)]
enum DispatchMode {
    Call,
    WatchSnapshot,
    Watch,
}

/// Renders dispatch from a freshly constructed generated object.
/// Captures owned operations under the object lock and polls them only after unlocking.
fn render_constructed_dispatch(object: &SourceObject, mode: DispatchMode) -> String {
    if object_uses_arc_mutex_instance(&object.access) {
        for method in object
            .methods
            .iter()
            .filter(|method| method.is_async && method.call_protocol().is_some())
        {
            assert!(method.returns_owned_future,
                "{}.{} must return impl Future<Output = T> + 'static; proxy dispatch cannot clone a locked object or await its borrow",
                object.full_type, method.name);
        }
        let lock = "            let mut object = object.lock().expect(\"core proxy object mutex poisoned\");\n";
        return match mode {
            DispatchMode::Call if object.has_async_call_dispatch() => {
                let capture = format!(
                    "                let pending = {{\n    {}                    generated_dispatch_{}_capture(&mut object, request)?\n                }};\n                pending.await\n",
                    lock, object.dispatch_name);
                if object.has_sync_call_dispatch() {
                    format!("            if matches!(request.methodName.as_str(), {}) {{\n{}            }} else {{\n{}            generated_dispatch_{}_call_sync(&mut object, request)\n            }}\n",
                        render_async_call_method_pattern(object), capture, lock, object.dispatch_name)
                } else { capture }
            }
            DispatchMode::Call => format!("{}            generated_dispatch_{}_call_sync(&mut object, request)\n", lock, object.dispatch_name),
            DispatchMode::WatchSnapshot => format!("{}            generated_dispatch_{}_watch_snapshot(&mut object, &request)?\n", lock, object.dispatch_name),
            DispatchMode::Watch => format!("{}            generated_dispatch_{}_watch(&mut object, request, attachmentAdopter.clone())\n", lock, object.dispatch_name),
        };
    }
    match mode {
        DispatchMode::Call => format!("            {}\n",
            render_direct_call_dispatch_expression(object, "&mut object", "request", "            ")),
        DispatchMode::WatchSnapshot => format!("            generated_dispatch_{}_watch_snapshot(&mut object, &request)?\n", object.dispatch_name),
        DispatchMode::Watch => format!("            generated_dispatch_{}_watch(&mut object, request, attachmentAdopter.clone())\n", object.dispatch_name),
    }
}

fn render_string_constructible_dispatch(object: &SourceObject, mode: DispatchMode) -> String {
    let dispatch = render_constructed_dispatch(object, mode);
    format!(
        "        {} => {{\n{}{}        }}\n",
        format!("{:?}", object.object_id),
        render_object_constructor(object, mode),
        dispatch
    )
}

/// Resolves factory parents asynchronously before establishing a synchronous watch.
fn render_async_factory_watch(object: &SourceObject, mode: DispatchMode) -> String {
    let constructor = render_object_constructor(object, DispatchMode::Call);
    let dispatch = render_constructed_dispatch(object, mode);
    let body = match mode {
        DispatchMode::WatchSnapshot => format!(
            "        let propertyName = request.propertyName.clone();\n        let value = {{\n{dispatch}        }};\n        return Ok(operit_link::CoreEvent {{ requestId: Some(request.requestId), target: request.target.clone(), propertyName, kind: operit_link::CoreEventKind::Snapshot, value }});\n"
        ),
        DispatchMode::Watch => format!(
            "        let attachmentAdopter = proxy.streamAttachmentAdopter();\n        return {{\n{dispatch}        }};\n"
        ),
        DispatchMode::Call => unreachable!("factory watch requires a watch dispatch mode"),
    };
    format!(
        "    if request.target == {} {{\n{constructor}{body}    }}\n",
        format!("{:?}", object.object_id)
    )
}

/// Renders a factory dispatch arm for the selected protocol operation.
fn render_factory_constructible_dispatch(object: &SourceObject, mode: DispatchMode) -> String {
    if !matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. }) {
        return String::new();
    }
    let dispatch = render_constructed_dispatch(object, mode);
    format!(
        "        {} => {{\n{}{}        }}\n",
        format!("{:?}", object.object_id),
        render_object_constructor(object, mode),
        dispatch
    )
}

fn render_object_constructor(object: &SourceObject, mode: DispatchMode) -> String {
    match &object.access {
        ObjectAccess::DefaultConstruct => {
            format!(
                "            let mut object = {}::default();\n",
                object.full_type
            )
        }
        ObjectAccess::GetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance();\n",
                object.full_type
            )
        }
        ObjectAccess::ResultGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n",
                object.full_type
            )
        }
        ObjectAccess::NewConstruct => {
            format!(
                "            let mut object = {}::new();\n",
                object.full_type
            )
        }
        ObjectAccess::StringNewConstruct => {
            format!(
                "            let mut __core_constructor_args = operit_rslink_runtime::object_args(request.args.clone())?;\n            let __core_instance_id: String = operit_rslink_runtime::decode_core_arg(&mut __core_constructor_args, \"__core_instance_id\")?;\n            let mut object = {}::new(__core_instance_id);\n",
                object.full_type
            )
        }
        ObjectAccess::ContextGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance(proxy.hostManager.clone());\n",
                object.full_type
            )
        }
        ObjectAccess::ContextRefGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance(&proxy.hostManager);\n",
                object.full_type
            )
        }
        ObjectAccess::ResultContextGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance(proxy.hostManager.clone()).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n",
                object.full_type
            )
        }
        ObjectAccess::ResultContextRefGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance(&proxy.hostManager).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n",
                object.full_type
            )
        }
        ObjectAccess::ContextGetInstanceArcMutexConstruct => {
            format!(
                "            let object = {}::getInstance(proxy.hostManager.clone());\n",
                object.full_type
            )
        }
        ObjectAccess::ContextRefGetInstanceArcMutexConstruct => {
            format!(
                "            let object = {}::getInstance(&proxy.hostManager);\n",
                object.full_type
            )
        }
        ObjectAccess::CoreProxyConstruct => {
            format!(
                "            let mut object = {}::new(proxy.clone());\n",
                object.full_type
            )
        }
        ObjectAccess::CoreNodeLocalRuntimeConstruct => {
            render_core_node_local_runtime_constructor("object", &object.full_type)
        }
        ObjectAccess::ResultCoreNodeLocalRuntimeConstruct => {
            format!("            let mut object = {}::new(proxy.coreNodeLocalRuntime()).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n", object.full_type)
        }
        ObjectAccess::StorePathsConstruct => {
            format!(
                "            let mut object = {}::new(operit_store::RuntimeStorePaths::RuntimeStorePaths::default());\n",
                object.full_type
            )
        }
        ObjectAccess::ResultStorePathsConstruct => {
            format!(
                "            let mut object = {}::new(operit_store::RuntimeStorePaths::RuntimeStorePaths::default()).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n",
                object.full_type
            )
        }
        ObjectAccess::FactoryMethodConstruct {
            parent_full_type,
            parent_access,
            factory_method,
            factory_arg_types,
            returns_result,
            returns_arc_mutex,
            ..
        } => render_factory_object_constructor(
            object,
            parent_full_type,
            parent_access,
            factory_method,
            factory_arg_types,
            *returns_result,
            *returns_arc_mutex,
            mode,
        ),
        ObjectAccess::Application | ObjectAccess::ResolvedHolder { .. } => String::new(),
    }
}

fn render_factory_object_constructor(
    object: &SourceObject,
    parent_full_type: &str,
    parent_access: &ObjectAccess,
    factory_method: &str,
    factory_arg_types: &[String],
    returns_result: bool,
    returns_arc_mutex: bool,
    mode: DispatchMode,
) -> String {
    let mut output = String::new();
    for (index, _) in factory_arg_types.iter().enumerate() {
        if index == 0 {
            output.push_str(
                "            let mut __core_constructor_args = operit_rslink_runtime::object_args(request.args.clone())?;\n",
            );
        }
        output.push_str(&format!(
            "            let __core_factory_arg_{index}: String = operit_rslink_runtime::decode_core_arg(&mut __core_constructor_args, \"__core_factory_arg_{index}\")?;\n"
        ));
    }
    if returns_arc_mutex {
        output.push_str("            let object = {\n");
    } else {
        output.push_str("            let mut object = {\n");
    }
    output.push_str(&render_object_constructor_for_access(
        "__core_parent_object",
        parent_full_type,
        parent_access,
        mode,
    ));
    let factory_args = factory_arg_types
        .iter()
        .enumerate()
        .map(|(index, ty)| match ty.as_str() {
            "&str" => format!("&__core_factory_arg_{index}"),
            "String" => format!("__core_factory_arg_{index}.clone()"),
            _ => format!("__core_factory_arg_{index}.clone()"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    if object_uses_arc_mutex_instance(parent_access) {
        output.push_str("            let mut __core_parent_object = __core_parent_object.lock().expect(\"core proxy object mutex poisoned\");\n");
    }
    output.push_str(&format!(
        "                __core_parent_object.{factory_method}({factory_args})"
    ));
    if returns_result {
        output
            .push_str(".map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?");
    }
    output.push('\n');
    output.push_str("            };\n");
    output
}

fn render_object_constructor_for_access(
    variable_name: &str,
    full_type: &str,
    access: &ObjectAccess,
    mode: DispatchMode,
) -> String {
    match access {
        ObjectAccess::Application => match mode {
            DispatchMode::Call => format!(
                "            let mut __core_application = proxy.application.lock().await;\n            let {variable_name} = &mut *__core_application;\n"
            ),
            DispatchMode::WatchSnapshot | DispatchMode::Watch => format!(
                "            let mut __core_application = proxy.application.try_lock().map_err(|_| operit_link::CoreLinkError::internal(\"Application is busy\"))?;\n            let {variable_name} = &mut *__core_application;\n"
            ),
        },
        ObjectAccess::DefaultConstruct => {
            format!("            let mut {variable_name} = {full_type}::default();\n")
        }
        ObjectAccess::GetInstanceConstruct => {
            format!("            let mut {variable_name} = {full_type}::getInstance();\n")
        }
        ObjectAccess::ResultGetInstanceConstruct => {
            format!(
                "            let mut {variable_name} = {full_type}::getInstance().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n"
            )
        }
        ObjectAccess::NewConstruct => {
            format!("            let mut {variable_name} = {full_type}::new();\n")
        }
        ObjectAccess::ContextGetInstanceConstruct => {
            format!(
                "            let mut {variable_name} = {full_type}::getInstance(proxy.hostManager.clone());\n"
            )
        }
        ObjectAccess::ContextRefGetInstanceConstruct => {
            format!(
                "            let mut {variable_name} = {full_type}::getInstance(&proxy.hostManager);\n"
            )
        }
        ObjectAccess::ResultContextGetInstanceConstruct => {
            format!(
                "            let mut {variable_name} = {full_type}::getInstance(proxy.hostManager.clone()).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n"
            )
        }
        ObjectAccess::ResultContextRefGetInstanceConstruct => {
            format!(
                "            let mut {variable_name} = {full_type}::getInstance(&proxy.hostManager).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n"
            )
        }
        ObjectAccess::ContextGetInstanceArcMutexConstruct => {
            format!(
                "            let {variable_name} = {full_type}::getInstance(proxy.hostManager.clone());\n"
            )
        }
        ObjectAccess::ContextRefGetInstanceArcMutexConstruct => {
            format!(
                "            let {variable_name} = {full_type}::getInstance(&proxy.hostManager);\n"
            )
        }
        ObjectAccess::CoreProxyConstruct => {
            format!("            let mut {variable_name} = {full_type}::new(proxy.clone());\n")
        }
        ObjectAccess::CoreNodeLocalRuntimeConstruct => {
            render_core_node_local_runtime_constructor(variable_name, full_type)
        }
        ObjectAccess::ResultCoreNodeLocalRuntimeConstruct => {
            format!("            let mut {variable_name} = {full_type}::new(proxy.coreNodeLocalRuntime()).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n")
        }
        ObjectAccess::StorePathsConstruct => {
            format!(
                "            let mut {variable_name} = {full_type}::new(operit_store::RuntimeStorePaths::RuntimeStorePaths::default());\n"
            )
        }
        ObjectAccess::ResultStorePathsConstruct => {
            format!(
                "            let mut {variable_name} = {full_type}::new(operit_store::RuntimeStorePaths::RuntimeStorePaths::default()).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n"
            )
        }
        ObjectAccess::StringNewConstruct
        | ObjectAccess::FactoryMethodConstruct { .. }
        | ObjectAccess::ResolvedHolder { .. } => String::new(),
    }
}

/// Builds the local server capability container required by server-owned services.
fn render_core_node_local_runtime_constructor(variable_name: &str, full_type: &str) -> String {
    format!(
        "            let mut {variable_name} = {full_type}::new(proxy.coreNodeLocalRuntime());\n"
    )
}

/// Returns the holder field and resolver declared by one generic holder-backed access strategy.
fn resolved_holder_metadata(access: &ObjectAccess) -> Option<(&str, &str)> {
    match access {
        ObjectAccess::ResolvedHolder {
            holder_field,
            resolver_method,
            ..
        } => Some((holder_field.as_str(), resolver_method.as_str())),
        _ => None,
    }
}

/// Renders the holder lookup used by generated proxy dispatch.
///
/// Detached chat windows bind the same generated object schema as the main
/// window, so their slot identity is carried in `__core_instance_id` rather
/// than in the generated target.
pub(super) fn render_resolved_holder_lookup(
    object: &SourceObject,
    resolver_method: &str,
    object_id: &str,
    indent: &str,
) -> String {
    if object.schema_key != "chatRuntimeHolderMain" {
        return format!("holder.{resolver_method}({object_id})");
    }
    format!(
        "{{\n{indent}    let mut __core_args = operit_rslink_runtime::object_args(request.args.clone())?;\n{indent}    let __core_instance_id: Option<String> = operit_rslink_runtime::decode_core_arg(&mut __core_args, \"__core_instance_id\")?;\n{indent}    match __core_instance_id {{\n{indent}        Some(instance_id) => holder.coreForInstanceId(instance_id),\n{indent}        None => holder.{resolver_method}({object_id}),\n{indent}    }}\n{indent}}}"
    )
}

/// Renders the generated call expression for one already resolved object.
fn render_direct_call_dispatch_expression(
    object: &SourceObject,
    object_ref: &str,
    request: &str,
    indent: &str,
) -> String {
    match (
        object.has_async_call_dispatch(),
        object.has_sync_call_dispatch(),
    ) {
        (true, true) => format!(
            "if matches!({request}.methodName.as_str(), {}) {{\n{}    Box::pin(generated_dispatch_{}_call({}, {})).await\n{}}} else {{\n{}    generated_dispatch_{}_call_sync({}, {})\n{}}}",
            render_async_call_method_pattern(object),
            indent,
            object.dispatch_name,
            object_ref,
            request,
            indent,
            indent,
            object.dispatch_name,
            object_ref,
            request,
            indent
        ),
        (true, false) => format!(
            "Box::pin(generated_dispatch_{}_call({}, {})).await",
            object.dispatch_name, object_ref, request
        ),
        (false, true) => format!(
            "generated_dispatch_{}_call_sync({}, {})",
            object.dispatch_name, object_ref, request
        ),
        (false, false) => {
            format!("Err(operit_link::CoreLinkError::methodNotFound(&{request}.registryKey()))")
        }
    }
}

/// Renders the match pattern covering every async call method on one object.
fn render_async_call_method_pattern(object: &SourceObject) -> String {
    object
        .methods
        .iter()
        .filter(|method| method.is_async && method.call_protocol().is_some())
        .map(|method| format!("{:?}", method.name))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Selects capture methods independently of the object's other async methods.
fn render_owned_call_method_pattern(object: &SourceObject) -> String {
    object
        .methods
        .iter()
        .filter(|method| method.returns_owned_future && method.call_protocol().is_some())
        .map(|method| format!("{:?}", method.name))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Renders one async call match arm that delegates to its method-sized helper.
fn render_async_call_arm(object: &SourceObject, method: &SourceMethod) -> String {
    if method.returns_owned_future {
        return format!(
            "        {:?} => generated_dispatch_{}_capture(object, request)?.await,\n",
            method.name, object.dispatch_name
        )
        .prepend_with(render_cfg_attrs(method));
    }
    format!(
        "        {:?} => Box::pin(generated_dispatch_{}_call_{}(object, request)).await,\n",
        method.name, object.dispatch_name, method.name
    )
    .prepend_with(render_cfg_attrs(method))
}

/// Renders one method-sized async call helper for generated proxy dispatch.
fn render_async_call_helper(
    object: &SourceObject,
    method: &SourceMethod,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let body = render_call_body(method, error_types, "    ");
    format!(
        "/// Dispatches generated async call `{}` for `{}`.\n{}#[allow(unused_mut, unused_variables)]\nasync fn generated_dispatch_{}_call_{}(object: &mut {}, request: operit_link::CoreCallRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {{\n    let mut __core_args = operit_rslink_runtime::object_args(request.args)?;\n{}{}\n}}\n",
        method.name,
        object.schema_key,
        render_item_cfg_attrs(method),
        object.dispatch_name,
        method.name,
        object.full_type,
        body,
        if body.ends_with('\n') { "" } else { "\n" }
    )
}

/// Generates synchronous argument capture with a compiler-enforced owned response future.
pub(crate) fn render_owned_call_dispatch(
    object: &SourceObject,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let methods = object
        .methods
        .iter()
        .filter(|method| method.returns_owned_future && method.call_protocol().is_some())
        .collect::<Vec<_>>();
    if methods.is_empty() {
        return String::new();
    }
    let mut output = format!(
        "/// Captures an operation without retaining the receiver or argument borrows.\n#[allow(unused_mut, unused_variables)]\nfn generated_dispatch_{}_capture(object: &mut {}, request: operit_link::CoreCallRequest) -> Result<std::pin::Pin<Box<dyn std::future::Future<Output = Result<operit_link::CoreValue, operit_link::CoreLinkError>> + 'static>>, operit_link::CoreLinkError> {{\n    let registryKey = request.registryKey();\n    match request.methodName.as_str() {{\n",
        object.dispatch_name, object.full_type);
    for method in methods {
        let conversion = match method.call_protocol().expect("call protocol") {
            CallProtocol::Unit => "pending.await; Ok(operit_link::CoreValue::Null)".to_string(),
            CallProtocol::ResultUnit { error_type } => format!("pending.await.map_err(|error| operit_rslink_runtime::core_call_error(error.to_string(), {}(&error)))?; Ok(operit_link::CoreValue::Null)", error_details_converter(error_type, error_types)),
            CallProtocol::Value(value_type) => render_core_value_result(value_type, "pending.await"),
            CallProtocol::ResultValue { value_type, error_type } => render_core_value_result(value_type, &format!("pending.await.map_err(|error| operit_rslink_runtime::core_call_error(error.to_string(), {}(&error)))?", error_details_converter(error_type, error_types))),
        };
        output.push_str(&render_cfg_attrs(method));
        output.push_str(&format!("        {:?} => {{\n            let mut __core_args = operit_rslink_runtime::object_args(request.args)?;\n{}            let pending = object.{}({});\n            Ok(Box::pin(async move {{ {} }}))\n        }}\n",
            method.name, render_arg_decoders_with_indent(method, "            "), method.name, render_arg_call_list(method), conversion));
    }
    output.push_str(
        "        _ => Err(operit_link::CoreLinkError::methodNotFound(&registryKey)),\n    }\n}\n",
    );
    output
}

/// Renders one synchronous call match arm with inline method execution.
fn render_call_arm(
    method: &SourceMethod,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let body = render_call_body(method, error_types, "            ");
    let arm = format!("        {:?} => {{\n{}        }}\n", method.name, body);
    render_cfg_attrs(method) + &arm
}

/// Renders the decode, invoke, and encode body for one generated method call.
fn render_call_body(
    method: &SourceMethod,
    error_types: &HashMap<String, ErrorTypeDefinition>,
    indent: &str,
) -> String {
    let args = render_arg_decoders_with_indent(method, indent);
    let call_args = render_arg_call_list(method);
    match method.call_protocol() {
        Some(CallProtocol::Unit) => format!(
            "{}{}object.{}({}){};\n{}Ok(operit_link::CoreValue::Null)\n",
            args,
            indent,
            method.name,
            call_args,
            await_suffix(method),
            indent
        ),
        Some(CallProtocol::ResultUnit { error_type }) => format!(
            "{}{}object.{}({}){}.map_err(|error| operit_rslink_runtime::core_call_error(error.to_string(), {}(&error)))?;\n{}Ok(operit_link::CoreValue::Null)\n",
            args,
            indent,
            method.name,
            call_args,
            await_suffix(method),
            error_details_converter(error_type, error_types),
            indent
        ),
        Some(CallProtocol::Value(value_type)) => {
            let value = format!(
                "object.{}({}){}",
                method.name,
                call_args,
                await_suffix(method)
            );
            format!(
                "{}{}{}\n",
                args,
                indent,
                render_core_value_result(value_type, &value)
            )
        }
        Some(CallProtocol::ResultValue {
            value_type,
            error_type,
        }) => {
            let value = format!(
                "object.{}({}){}.map_err(|error| operit_rslink_runtime::core_call_error(error.to_string(), {}(&error)))?",
                method.name,
                call_args,
                await_suffix(method),
                error_details_converter(error_type, error_types)
            );
            format!(
                "{}{}{}\n",
                args,
                indent,
                render_core_value_result(value_type, &value)
            )
        }
        None => String::new(),
    }
}

/// Renders a typed runtime value as a native CoreValue result.
fn render_core_value_result(value_type: &str, value: &str) -> String {
    if value_type == "Vec<u8>" {
        format!("Ok(operit_link::CoreValue::Bytes(({value}).into()))")
    } else {
        format!("operit_rslink_runtime::to_core_value({value})")
    }
}

fn error_details_converter(
    error_type: &str,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    if error_type == "String" {
        return "generated_core_proxy_error_details_for_string".to_string();
    }
    let Some(definition) = error_types.get(error_type) else {
        panic!("core proxy error type is not generated: {error_type}");
    };
    error_details_fn_name(&definition.full_type)
}

fn render_watch_snapshot_arm(method: &SourceMethod, async_dispatch: bool) -> String {
    let Some(watch) = method.watch_protocol() else {
        return String::new();
    };
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let method_await = watch_await_suffix(method, async_dispatch);
    let value_expr = match watch.stream {
        WatchStreamProtocol::JsonFlow { fallible: true } => format!(
            "object.{}({}){}.map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?.first().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
            method.name, call_args, method_await
        ),
        WatchStreamProtocol::JsonFlow { fallible: false } => format!(
            "object.{}({}){}.first().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
            method.name, call_args, method_await
        ),
        WatchStreamProtocol::JsonState { fallible: true } => format!(
            "object.{}({}){}.map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?.value()",
            method.name, call_args, method_await
        ),
        WatchStreamProtocol::JsonState { fallible: false } => {
            format!("object.{}({}){}.value()", method.name, call_args, method_await)
        }
        WatchStreamProtocol::JsonStream => return String::new(),
        WatchStreamProtocol::StringStream => return String::new(),
    };
    format!(
        "        {:?} => {{\n{}            {}\n        }}\n",
        method.name,
        args,
        render_core_value_result(
            watch.snapshot_type.as_deref().expect("watch snapshot type"),
            &value_expr,
        )
    )
    .prepend_with(render_cfg_attrs(method))
}

fn render_watch_stream_arm(method: &SourceMethod, async_dispatch: bool) -> String {
    let Some(watch) = method.watch_protocol() else {
        return String::new();
    };
    match watch.stream {
        WatchStreamProtocol::JsonFlow { fallible } => {
            render_json_flow_watch_stream_arm(method, fallible, async_dispatch)
        }
        WatchStreamProtocol::JsonState { fallible } => {
            render_json_state_watch_stream_arm(method, fallible, async_dispatch)
        }
        WatchStreamProtocol::JsonStream => render_json_watch_stream_arm(method, async_dispatch),
        WatchStreamProtocol::StringStream => render_string_watch_stream_arm(method, async_dispatch),
    }
}

fn render_json_flow_watch_stream_arm(
    method: &SourceMethod,
    fallible: bool,
    async_dispatch: bool,
) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let method_await = watch_await_suffix(method, async_dispatch);
    let flow_expr = if fallible {
        format!(
            "object.{}({}){}.map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
            method.name, call_args, method_await
        )
    } else {
        format!("object.{}({}){}", method.name, call_args, method_await)
    };
    format!(
        "        {:?} => {{\n{}            let flow = {};\n            operit_rslink_runtime::core_flow_event_stream(flow, request, attachmentAdopter.clone())\n        }}\n",
        method.name, args, flow_expr
    )
    .prepend_with(render_cfg_attrs(method))
}

fn render_json_state_watch_stream_arm(
    method: &SourceMethod,
    fallible: bool,
    async_dispatch: bool,
) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let method_await = watch_await_suffix(method, async_dispatch);
    let state_expr = if fallible {
        format!(
            "object.{}({}){}.map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
            method.name, call_args, method_await
        )
    } else {
        format!("object.{}({}){}", method.name, call_args, method_await)
    };
    format!(
        "        {:?} => {{\n{}            let stateFlow = {};\n            operit_rslink_runtime::core_state_flow_event_stream(stateFlow, request, attachmentAdopter.clone())\n        }}\n",
        method.name, args, state_expr
    )
    .prepend_with(render_cfg_attrs(method))
}

fn render_string_watch_stream_arm(method: &SourceMethod, async_dispatch: bool) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let method_await = watch_await_suffix(method, async_dispatch);
    format!(
        "        {:?} => {{\n{}            let stream = object.{}({}){};\n            Ok(operit_rslink_runtime::core_string_event_stream(stream, request))\n        }}\n",
        method.name, args, method.name, call_args, method_await
    )
    .prepend_with(render_cfg_attrs(method))
}

fn render_json_watch_stream_arm(method: &SourceMethod, async_dispatch: bool) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let method_await = watch_await_suffix(method, async_dispatch);
    format!(
        "        {:?} => {{\n{}            let stream = object.{}({}){};\n            Ok(operit_rslink_runtime::core_json_event_stream(stream, request))\n        }}\n",
        method.name, args, method.name, call_args, method_await
    )
    .prepend_with(render_cfg_attrs(method))
}

/// Returns the await suffix required by one generated async watch dispatcher.
fn watch_await_suffix(method: &SourceMethod, async_dispatch: bool) -> &'static str {
    if async_dispatch && method.is_async {
        ".await"
    } else {
        ""
    }
}

/// Renders method cfg attributes for a generated match arm.
fn render_cfg_attrs(method: &SourceMethod) -> String {
    method
        .cfg_attrs
        .iter()
        .map(|attr| format!("        {attr}\n"))
        .collect()
}

/// Renders method cfg attributes for a generated item.
fn render_item_cfg_attrs(method: &SourceMethod) -> String {
    method
        .cfg_attrs
        .iter()
        .map(|attr| format!("{attr}\n"))
        .collect()
}

trait GeneratedStringExt {
    fn prepend_with(self, prefix: String) -> String;
}

impl GeneratedStringExt for String {
    fn prepend_with(self, prefix: String) -> String {
        prefix + &self
    }
}

/// Renders argument decode statements for a generated match arm.
fn render_arg_decoders(method: &SourceMethod) -> String {
    render_arg_decoders_with_indent(method, "            ")
}

/// Renders argument decode statements using the requested indentation.
fn render_arg_decoders_with_indent(method: &SourceMethod, indent: &str) -> String {
    method
        .args
        .iter()
        .map(|arg| {
            format!(
                "{indent}let {}: {} = operit_rslink_runtime::decode_core_arg(&mut __core_args, {:?})?;\n",
                arg.name,
                render_arg_decode_type(arg),
                arg.name
            )
        })
        .collect::<String>()
}

fn render_arg_decode_type(arg: &SourceArg) -> String {
    if arg.ty == "&str" {
        "String".to_string()
    } else if arg.ty == "Option<&str>" {
        "Option<String>".to_string()
    } else if let Some(inner) =
        single_generic_arg(&arg.ty, "Option").and_then(|inner| inner.strip_prefix('&'))
    {
        format!("Option<{inner}>")
    } else if arg.ty == "&std::path::Path" {
        "String".to_string()
    } else if let Some(inner) = borrowed_slice_inner(&arg.ty) {
        match inner {
            "std::path::PathBuf" => "Vec<std::path::PathBuf>".to_string(),
            "i64" => "Vec<i64>".to_string(),
            "String" => "Vec<String>".to_string(),
            _ => arg.ty.clone(),
        }
    } else if let Some(inner) = arg.ty.strip_prefix('&') {
        inner.to_string()
    } else {
        arg.ty.clone()
    }
}

fn render_arg_call_list(method: &SourceMethod) -> String {
    method
        .args
        .iter()
        .map(render_arg_call_expr)
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_arg_call_expr(arg: &SourceArg) -> String {
    if arg.ty == "&str" {
        format!("{}.as_str()", arg.name)
    } else if arg.ty == "Option<&str>" {
        format!("{}.as_deref()", arg.name)
    } else if single_generic_arg(&arg.ty, "Option")
        .and_then(|inner| inner.strip_prefix('&'))
        .is_some()
    {
        format!("{}.as_ref()", arg.name)
    } else if arg.ty == "&std::path::Path" {
        format!("std::path::Path::new(&{})", arg.name)
    } else if borrowed_slice_inner(&arg.ty).is_some() {
        format!("{}.as_slice()", arg.name)
    } else if arg.ty.strip_prefix('&').is_some() {
        format!("&{}", arg.name)
    } else {
        arg.name.clone()
    }
}

fn await_suffix(method: &SourceMethod) -> &'static str {
    if method.is_async {
        ".await"
    } else {
        ""
    }
}

#[cfg(test)]
#[path = "owned_dispatch_tests.rs"]
mod owned_dispatch_tests;
