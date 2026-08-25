use super::build_rust_codegen_utils::*;
use super::*;

pub(crate) fn render_object_call_dispatch(
    object: &SourceObject,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let mut output = String::new();
    output.push_str(&render_object_item_cfg_attrs(object));
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "async fn generated_dispatch_{}_call(object: &mut {}, request: operit_link::CoreCallRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = operit_rslink_runtime::object_args(request.args)?;\n");
    output.push_str("    match request.methodName.as_str() {\n");
    for method in object
        .methods
        .iter()
        .filter(|method| method.call_protocol().is_some())
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

pub(crate) fn render_object_sync_call_dispatch(
    object: &SourceObject,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let mut output = String::new();
    output.push_str(&render_object_item_cfg_attrs(object));
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_call_sync(object: &mut {}, request: operit_link::CoreCallRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = operit_rslink_runtime::object_args(request.args)?;\n");
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
    output.push_str(&render_object_item_cfg_attrs(object));
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_watch_snapshot(object: &mut {}, request: &operit_link::CoreWatchRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = operit_rslink_runtime::object_args(request.args.clone())?;\n");
    output.push_str("    match request.propertyName.as_str() {\n");
    for method in object.methods.iter().filter(|method| {
        method
            .watch_protocol()
            .and_then(|watch| watch.snapshot_type.as_ref())
            .is_some()
    }) {
        output.push_str(&render_watch_snapshot_arm(method));
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::watchNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

pub(crate) fn render_object_watch_dispatch(object: &SourceObject) -> String {
    let mut output = String::new();
    output.push_str(&render_object_item_cfg_attrs(object));
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_watch(object: &mut {}, request: operit_link::CoreWatchRequest, attachmentAdopter: std::sync::Arc<dyn Fn(Vec<operit_link::CoreStreamAttachment>) + Send + Sync>) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = operit_rslink_runtime::object_args(request.args.clone())?;\n");
    output.push_str("    match request.propertyName.as_str() {\n");
    for method in object
        .methods
        .iter()
        .filter(|method| method.watch_protocol().is_some())
    {
        output.push_str(&render_watch_stream_arm(method));
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
            "fn generated_object_id_matches_{}(object_id: u32) -> bool {{\n",
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
    format!("object_id == {}", object.object_id)
}

pub(crate) fn render_core_proxy_dispatch(objects: &[SourceObject]) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("async fn generated_dispatch_core_proxy_call(proxy: &LocalCoreProxy, request: operit_link::CoreCallRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {\n");
    output.push_str("    #[cfg(not(target_arch = \"wasm32\"))]\n");
    let application_id = objects
        .iter()
        .find(|object| object.schema_key == "application")
        .expect("application object must be generated")
        .object_id;
    output.push_str(&format!("    if request.targetObjectId == {application_id} && request.methodName == \"runCoreCommand\" {{\n"));
    output.push_str("        let mut __core_args = operit_rslink_runtime::object_args(request.args)?;\n");
    output.push_str(
        "        let args: Vec<String> = operit_rslink_runtime::decode_core_arg(&mut __core_args, \"args\")?;\n",
    );
    output.push_str("        let application = proxy.application.clone();\n");
    output.push_str("        let (commandSender, commandReceiver) = tokio::sync::oneshot::channel();\n");
    output.push_str("        operit_host_api::HostRuntimeTaskSchedulerHost::scheduleHostRuntimeAsyncTask(\n");
    output.push_str("            operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost().as_ref(),\n");
    output.push_str("            \"core-proxy-command\",\n");
    output.push_str("            Box::new(move || Box::pin(async move {\n");
    output.push_str("                let mut application = application.lock().await;\n");
    output.push_str("                let output = operit_command_core::run_core_command(&mut application, &args)\n");
    output.push_str("                    .map_err(operit_link::CoreLinkError::command);\n");
    output.push_str("                let _ = commandSender.send(output);\n");
    output.push_str("            })),\n");
    output.push_str("        )\n");
    output.push_str("        .map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n");
    output.push_str("        let output = commandReceiver.await.map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))??;\n");
    output.push_str("        return operit_rslink_runtime::to_core_value(output);\n");
    output.push_str("    }\n");
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application && object.has_call_dispatch())
    {
        output.push_str(&format!(
            "    if request.targetObjectId == {} {{\n        let mut application = proxy.application.lock().await;\n        return generated_dispatch_{}_call(&mut application, request).await;\n    }}\n",
            application.object_id, application.dispatch_name
        ));
    }
    for object in objects.iter().filter(|object| object.has_call_dispatch()) {
        let Some((holder_field, resolver_method)) = resolved_holder_metadata(&object.access) else {
            continue;
        };
        output.push_str(&format!(
            "    if generated_object_id_matches_{}(request.targetObjectId) {{\n        let mut holder = proxy.{holder_field}.lock().await;\n        if let Some(object) = holder.{resolver_method}(request.targetObjectId) {{\n            return generated_dispatch_{}_call(object, request).await;\n        }}\n    }}\n",
            object.dispatch_name,
            object.dispatch_name
        ));
    }
    output.push_str("    match request.targetObjectId {\n");
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
            "{}        {} => {{\n{}{}        }}\n",
            render_object_match_arm_cfg_attrs(object),
            object.object_id,
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
    for object in objects {
        let Some((holder_field, resolver_method)) = resolved_holder_metadata(&object.access) else {
            continue;
        };
        output.push_str(&format!(
            "    if generated_object_id_matches_{}(request.targetObjectId) {{\n        let propertyName = request.propertyName.clone();\n        let mut holder = proxy.{holder_field}.lock().await;\n        if let Some(object) = holder.{resolver_method}(request.targetObjectId) {{\n            let value = generated_dispatch_{}_watch_snapshot(object, &request)?;\n            return Ok(operit_link::CoreEvent {{ requestId: Some(request.requestId), targetObjectId: request.targetObjectId, propertyName, kind: operit_link::CoreEventKind::Snapshot, value }});\n        }}\n    }}\n",
            object.dispatch_name,
            object.dispatch_name
        ));
    }
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "    if request.targetObjectId == {} {{\n        let propertyName = request.propertyName.clone();\n        let mut application = proxy.application.lock().await;\n        let value = generated_dispatch_{}_watch_snapshot(&mut application, &request)?;\n        return Ok(operit_link::CoreEvent {{ requestId: Some(request.requestId), targetObjectId: request.targetObjectId, propertyName, kind: operit_link::CoreEventKind::Snapshot, value }});\n    }}\n",
            application.object_id, application.dispatch_name
        ));
    }
    output.push_str("    generated_dispatch_core_proxy_watch_snapshot(proxy, request)\n");
    output.push_str("}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("async fn generated_dispatch_core_proxy_watch_async(proxy: &LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {\n");
    for object in objects {
        let Some((holder_field, resolver_method)) = resolved_holder_metadata(&object.access) else {
            continue;
        };
        output.push_str(&format!(
            "    if generated_object_id_matches_{}(request.targetObjectId) {{\n        let mut holder = proxy.{holder_field}.lock().await;\n        if let Some(object) = holder.{resolver_method}(request.targetObjectId) {{\n            return generated_dispatch_{}_watch(object, request, proxy.streamAttachmentAdopter());\n        }}\n    }}\n",
            object.dispatch_name,
            object.dispatch_name
        ));
    }
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "    if request.targetObjectId == {} {{\n        let mut application = proxy.application.lock().await;\n        return generated_dispatch_{}_watch(&mut application, request, proxy.streamAttachmentAdopter());\n    }}\n",
            application.object_id, application.dispatch_name
        ));
    }
    output.push_str("    generated_dispatch_core_proxy_watch(proxy, request)\n");
    output.push_str("}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_dispatch_core_proxy_watch_snapshot(proxy: &LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEvent, operit_link::CoreLinkError> {\n");
    for object in objects {
        let Some((holder_field, resolver_method)) = resolved_holder_metadata(&object.access) else {
            continue;
        };
        output.push_str(&format!(
            "    if generated_object_id_matches_{}(request.targetObjectId) {{\n        let propertyName = request.propertyName.clone();\n        let mut holder = proxy.{holder_field}.try_lock().map_err(|_| operit_link::CoreLinkError::internal(\"Resolved holder is busy\"))?;\n        if let Some(object) = holder.{resolver_method}(request.targetObjectId) {{\n            let value = generated_dispatch_{}_watch_snapshot(object, &request)?;\n            return Ok(operit_link::CoreEvent {{ requestId: Some(request.requestId), targetObjectId: request.targetObjectId, propertyName, kind: operit_link::CoreEventKind::Snapshot, value }});\n        }}\n    }}\n",
            object.dispatch_name,
            object.dispatch_name
        ));
    }
    output.push_str("    let propertyName = request.propertyName.clone();\n");
    output.push_str("    let value = match request.targetObjectId {\n");
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "        {} => {{ let mut application = proxy.application.try_lock().map_err(|_| operit_link::CoreLinkError::internal(\"Application is busy\"))?; generated_dispatch_{}_watch_snapshot(&mut application, &request)? }},\n",
            application.object_id, application.dispatch_name
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
            "{}        {} => {{\n{}{}        }}\n",
            render_object_match_arm_cfg_attrs(object),
            object.object_id,
            render_object_constructor(object, DispatchMode::WatchSnapshot),
            render_constructed_dispatch(object, DispatchMode::WatchSnapshot)
        ));
    }
    output.push_str("        _ => return Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),\n");
    output.push_str("    };\n");
    output.push_str("    Ok(operit_link::CoreEvent { requestId: Some(request.requestId), targetObjectId: request.targetObjectId, propertyName, kind: operit_link::CoreEventKind::Snapshot, value })\n");
    output.push_str("}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_dispatch_core_proxy_watch(proxy: &LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {\n    let attachmentAdopter = proxy.streamAttachmentAdopter();\n");
    for object in objects {
        let Some((holder_field, resolver_method)) = resolved_holder_metadata(&object.access) else {
            continue;
        };
        output.push_str(&format!(
            "    if generated_object_id_matches_{}(request.targetObjectId) {{\n        let mut holder = proxy.{holder_field}.try_lock().map_err(|_| operit_link::CoreLinkError::internal(\"Resolved holder is busy\"))?;\n        if let Some(object) = holder.{resolver_method}(request.targetObjectId) {{\n            return generated_dispatch_{}_watch(object, request, attachmentAdopter.clone());\n        }}\n    }}\n",
            object.dispatch_name,
            object.dispatch_name
        ));
    }
    output.push_str("    match request.targetObjectId {\n");
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "        {} => {{ let mut application = proxy.application.try_lock().map_err(|_| operit_link::CoreLinkError::internal(\"Application is busy\"))?; generated_dispatch_{}_watch(&mut application, request, attachmentAdopter.clone()) }},\n",
            application.object_id, application.dispatch_name
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
            "{}        {} => {{\n{}{}        }}\n",
            render_object_match_arm_cfg_attrs(object),
            object.object_id,
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

fn render_constructed_dispatch(object: &SourceObject, mode: DispatchMode) -> String {
    if object_uses_arc_mutex_instance(&object.access) {
        let lock = "            let mut object = object.lock().expect(\"core proxy object mutex poisoned\");\n";
        return match mode {
            DispatchMode::Call
                if object
                    .methods
                    .iter()
                    .any(|method| method.is_async && method.call_protocol().is_some()) =>
            {
                let async_methods = object
                    .methods
                    .iter()
                    .filter(|method| method.is_async && method.call_protocol().is_some())
                    .map(|method| format!("{:?}", method.name))
                    .collect::<Vec<_>>()
                    .join(" | ");
                format!(
                    "            if matches!(request.methodName.as_str(), {async_methods}) {{\n                let mut object = object.lock().expect(\"core proxy object mutex poisoned\").clone();\n                generated_dispatch_{}_call(&mut object, request).await\n            }} else {{\n{}            generated_dispatch_{}_call_sync(&mut object, request)\n            }}\n",
                    object.dispatch_name, lock, object.dispatch_name
                )
            }
            DispatchMode::Call => format!(
                "{}            generated_dispatch_{}_call_sync(&mut object, request)\n",
                lock, object.dispatch_name
            ),
            DispatchMode::WatchSnapshot => format!(
                "{}            generated_dispatch_{}_watch_snapshot(&mut object, &request)?\n",
                lock, object.dispatch_name
            ),
            DispatchMode::Watch => format!(
                "{}            generated_dispatch_{}_watch(&mut object, request, attachmentAdopter.clone())\n",
                lock, object.dispatch_name
            ),
        };
    }
    match mode {
        DispatchMode::Call => format!(
            "            generated_dispatch_{}_call(&mut object, request).await\n",
            object.dispatch_name
        ),
        DispatchMode::WatchSnapshot => format!(
            "            generated_dispatch_{}_watch_snapshot(&mut object, &request)?\n",
            object.dispatch_name
        ),
        DispatchMode::Watch => format!(
            "            generated_dispatch_{}_watch(&mut object, request, attachmentAdopter.clone())\n",
            object.dispatch_name
        ),
    }
}

fn render_string_constructible_dispatch(object: &SourceObject, mode: DispatchMode) -> String {
    let dispatch = render_constructed_dispatch(object, mode);
    format!(
        "{}        {} => {{\n{}{}        }}\n",
        render_object_match_arm_cfg_attrs(object),
        object.object_id,
        render_object_constructor(object, mode),
        dispatch
    )
}

fn render_factory_constructible_dispatch(object: &SourceObject, mode: DispatchMode) -> String {
    if !matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. }) {
        return String::new();
    }
    let dispatch = render_constructed_dispatch(object, mode);
    format!(
        "{}        {} => {{\n{}{}        }}\n",
        render_object_match_arm_cfg_attrs(object),
        object.object_id,
        render_object_constructor(object, mode),
        dispatch
    )
}

/// Returns item attributes for generated objects that require native server routing.
fn render_object_item_cfg_attrs(object: &SourceObject) -> String {
    if object_requires_native_dispatch(object) {
        "#[cfg(not(target_arch = \"wasm32\"))]\n".to_string()
    } else {
        String::new()
    }
}

/// Returns match-arm attributes for generated objects that require native server routing.
fn render_object_match_arm_cfg_attrs(object: &SourceObject) -> String {
    if object_requires_native_dispatch(object) {
        "        #[cfg(not(target_arch = \"wasm32\"))]\n".to_string()
    } else {
        String::new()
    }
}

/// Returns whether one generated object depends on native Core server routing.
fn object_requires_native_dispatch(object: &SourceObject) -> bool {
    access_requires_native_dispatch(&object.access)
}

/// Returns whether one object access strategy depends on native Core server routing.
fn access_requires_native_dispatch(access: &ObjectAccess) -> bool {
    match access {
        ObjectAccess::CoreNodeLocalRuntimeConstruct => true,
        ObjectAccess::FactoryMethodConstruct { parent_access, .. } => {
            access_requires_native_dispatch(parent_access)
        }
        _ => false,
    }
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

fn render_call_arm(
    method: &SourceMethod,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let arm = match method.call_protocol() {
        Some(CallProtocol::Unit) => format!(
            "        {:?} => {{\n{}            object.{}({}){};\n            Ok(operit_link::CoreValue::Null)\n        }}\n",
            method.name,
            args,
            method.name,
            call_args,
            await_suffix(method)
        ),
        Some(CallProtocol::ResultUnit { error_type }) => format!(
            "        {:?} => {{\n{}            object.{}({}){}.map_err(|error| operit_rslink_runtime::core_call_error(error.to_string(), {}(&error)))?;\n            Ok(operit_link::CoreValue::Null)\n        }}\n",
            method.name,
            args,
            method.name,
            call_args,
            await_suffix(method),
            error_details_converter(error_type, error_types)
        ),
        Some(CallProtocol::Value(value_type)) => {
            let value = format!(
                "object.{}({}){}",
                method.name,
                call_args,
                await_suffix(method)
            );
            format!(
                "        {:?} => {{\n{}            {}\n        }}\n",
                method.name,
                args,
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
                "        {:?} => {{\n{}            {}\n        }}\n",
                method.name,
                args,
                render_core_value_result(value_type, &value)
            )
        }
        None => String::new(),
    };
    render_cfg_attrs(method) + &arm
}

/// Renders a typed runtime value as a native CoreValue result.
fn render_core_value_result(value_type: &str, value: &str) -> String {
    if value_type == "Vec<u8>" {
        format!("Ok(operit_link::CoreValue::Bytes({value}))")
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

fn render_watch_snapshot_arm(method: &SourceMethod) -> String {
    let Some(watch) = method.watch_protocol() else {
        return String::new();
    };
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let value_expr = match watch.stream {
        WatchStreamProtocol::JsonFlow { fallible: true } => format!(
            "object.{}({}).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?.first().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
            method.name, call_args
        ),
        WatchStreamProtocol::JsonFlow { fallible: false } => format!(
            "object.{}({}).first().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
            method.name, call_args
        ),
        WatchStreamProtocol::JsonState { fallible: true } => format!(
            "object.{}({}).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?.value()",
            method.name, call_args
        ),
        WatchStreamProtocol::JsonState { fallible: false } => {
            format!("object.{}({}).value()", method.name, call_args)
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

fn render_watch_stream_arm(method: &SourceMethod) -> String {
    let Some(watch) = method.watch_protocol() else {
        return String::new();
    };
    match watch.stream {
        WatchStreamProtocol::JsonFlow { fallible } => {
            render_json_flow_watch_stream_arm(method, fallible)
        }
        WatchStreamProtocol::JsonState { fallible } => {
            render_json_state_watch_stream_arm(method, fallible)
        }
        WatchStreamProtocol::JsonStream => render_json_watch_stream_arm(method),
        WatchStreamProtocol::StringStream => render_string_watch_stream_arm(method),
    }
}

fn render_json_flow_watch_stream_arm(method: &SourceMethod, fallible: bool) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let flow_expr = if fallible {
        format!(
            "object.{}({}).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
            method.name, call_args
        )
    } else {
        format!("object.{}({})", method.name, call_args)
    };
    format!(
        "        {:?} => {{\n{}            let flow = {};\n            operit_rslink_runtime::core_flow_event_stream(flow, request, attachmentAdopter.clone())\n        }}\n",
        method.name, args, flow_expr
    )
    .prepend_with(render_cfg_attrs(method))
}

fn render_json_state_watch_stream_arm(method: &SourceMethod, fallible: bool) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let state_expr = if fallible {
        format!(
            "object.{}({}).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
            method.name, call_args
        )
    } else {
        format!("object.{}({})", method.name, call_args)
    };
    format!(
        "        {:?} => {{\n{}            let stateFlow = {};\n            operit_rslink_runtime::core_state_flow_event_stream(stateFlow, request, attachmentAdopter.clone())\n        }}\n",
        method.name, args, state_expr
    )
    .prepend_with(render_cfg_attrs(method))
}

fn render_string_watch_stream_arm(method: &SourceMethod) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    format!(
        "        {:?} => {{\n{}            let stream = object.{}({});\n            Ok(operit_rslink_runtime::core_string_event_stream(stream, request))\n        }}\n",
        method.name, args, method.name, call_args
    )
    .prepend_with(render_cfg_attrs(method))
}

fn render_json_watch_stream_arm(method: &SourceMethod) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    format!(
        "        {:?} => {{\n{}            let stream = object.{}({});\n            Ok(operit_rslink_runtime::core_json_event_stream(stream, request))\n        }}\n",
        method.name, args, method.name, call_args
    )
    .prepend_with(render_cfg_attrs(method))
}

fn render_cfg_attrs(method: &SourceMethod) -> String {
    method
        .cfg_attrs
        .iter()
        .map(|attr| format!("        {attr}\n"))
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

fn render_arg_decoders(method: &SourceMethod) -> String {
    method
        .args
        .iter()
        .map(|arg| {
            format!(
                "            let {}: {} = operit_rslink_runtime::decode_core_arg(&mut __core_args, {:?})?;\n",
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
