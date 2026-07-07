use super::build_rust_codegen_utils::*;
use super::*;

pub(crate) fn render_object_call_dispatch(
    object: &SourceObject,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "async fn generated_dispatch_{}_call(object: &mut {}, request: operit_link::CoreCallRequest) -> Result<serde_json::Value, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = object_args(request.args)?;\n");
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
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_call_sync(object: &mut {}, request: operit_link::CoreCallRequest) -> Result<serde_json::Value, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = object_args(request.args)?;\n");
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
        "fn generated_dispatch_{}_watch_snapshot(object: &mut {}, request: &operit_link::CoreWatchRequest) -> Result<serde_json::Value, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = object_args(request.args.clone())?;\n");
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
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_watch(object: &mut {}, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = object_args(request.args.clone())?;\n");
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

pub(crate) fn render_core_proxy_dispatch(objects: &[SourceObject]) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("async fn generated_dispatch_core_proxy_call(proxy: &mut LocalCoreProxy, request: operit_link::CoreCallRequest) -> Result<serde_json::Value, operit_link::CoreLinkError> {\n");
    output.push_str("    #[cfg(not(target_arch = \"wasm32\"))]\n");
    output.push_str("    if request.targetPath.key() == \"application\" && request.methodName == \"runCoreCommand\" {\n");
    output.push_str("        let mut __core_args = object_args(request.args)?;\n");
    output.push_str(
        "        let args: Vec<String> = decode_core_arg(&mut __core_args, \"args\")?;\n",
    );
    output.push_str("        let output = tokio::task::block_in_place(|| operit_command_core::run_core_command(&mut proxy.application, &args)).map_err(operit_link::CoreLinkError::command)?;\n");
    output.push_str("        return to_core_value(output);\n");
    output.push_str("    }\n");
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "    if request.targetPath.key() == {:?} {{\n        return generated_dispatch_{}_call(&mut proxy.application, request).await;\n    }}\n",
            application.schema_key, application.dispatch_name
        ));
    }
    if let Some(chat_runtime) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::ChatRuntimeMain)
    {
        output.push_str(&format!(
            "    if let Some(slot) = chat_runtime_slot(&request.targetPath) {{\n        let core = proxy.application.chatRuntimeHolder.getCore(slot);\n        return generated_dispatch_{}_call(core, request).await;\n    }}\n",
            chat_runtime.dispatch_name
        ));
    }
    output.push_str("    match request.targetPath.key().as_str() {\n");
    for object in objects
        .iter()
        .filter(|object| matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. }))
    {
        output.push_str(&render_factory_constructible_dispatch(
            object,
            DispatchMode::Call,
        ));
    }
    for object in objects
        .iter()
        .filter(|object| object.access == ObjectAccess::StringNewConstruct)
    {
        output.push_str(&render_string_constructible_dispatch(
            object,
            DispatchMode::Call,
        ));
    }
    for object in objects.iter().filter(|object| {
        object.access.is_constructible()
            && object.access != ObjectAccess::StringNewConstruct
            && !matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. })
    }) {
        output.push_str(&format!(
            "        {:?} => {{\n{}{}        }}\n",
            object.schema_key,
            render_object_constructor(object),
            render_constructed_dispatch(object, DispatchMode::Call)
        ));
    }
    output.push_str(
        "        _ => Err(operit_link::CoreLinkError::methodNotFound(&request.registryKey())),\n",
    );
    output.push_str("    }\n}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_dispatch_core_proxy_watch_snapshot(proxy: &mut LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEvent, operit_link::CoreLinkError> {\n");
    if let Some(chat_runtime) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::ChatRuntimeMain)
    {
        output.push_str(&format!(
            "    if let Some(slot) = chat_runtime_slot(&request.targetPath) {{\n        let propertyName = request.propertyName.clone();\n        let core = proxy.application.chatRuntimeHolder.getCore(slot);\n        let value = generated_dispatch_{}_watch_snapshot(core, &request)?;\n        return Ok(operit_link::CoreEvent {{ requestId: Some(request.requestId), targetPath: request.targetPath, propertyName, kind: operit_link::CoreEventKind::Snapshot, value }});\n    }}\n",
            chat_runtime.dispatch_name
        ));
    }
    output.push_str("    let propertyName = request.propertyName.clone();\n");
    output.push_str("    let value = match request.targetPath.key().as_str() {\n");
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "        {:?} => generated_dispatch_{}_watch_snapshot(&mut proxy.application, &request)?,\n",
            application.schema_key, application.dispatch_name
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
            "        {:?} => {{\n{}{}        }}\n",
            object.schema_key,
            render_object_constructor(object),
            render_constructed_dispatch(object, DispatchMode::WatchSnapshot)
        ));
    }
    output.push_str("        _ => return Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),\n");
    output.push_str("    };\n");
    output.push_str("    Ok(operit_link::CoreEvent { requestId: Some(request.requestId), targetPath: request.targetPath, propertyName, kind: operit_link::CoreEventKind::Snapshot, value })\n");
    output.push_str("}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_dispatch_core_proxy_watch(proxy: &mut LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {\n");
    if let Some(chat_runtime) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::ChatRuntimeMain)
    {
        output.push_str(&format!(
            "    if let Some(slot) = chat_runtime_slot(&request.targetPath) {{\n        let core = proxy.application.chatRuntimeHolder.getCore(slot);\n        return generated_dispatch_{}_watch(core, request);\n    }}\n",
            chat_runtime.dispatch_name
        ));
    }
    output.push_str("    match request.targetPath.key().as_str() {\n");
    if let Some(application) = objects
        .iter()
        .find(|object| object.access == ObjectAccess::Application)
    {
        output.push_str(&format!(
            "        {:?} => generated_dispatch_{}_watch(&mut proxy.application, request),\n",
            application.schema_key, application.dispatch_name
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
            "        {:?} => {{\n{}{}        }}\n",
            object.schema_key,
            render_object_constructor(object),
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
            DispatchMode::Call => format!(
                "{}            generated_dispatch_{}_call_sync(&mut object, request)\n",
                lock, object.dispatch_name
            ),
            DispatchMode::WatchSnapshot => format!(
                "{}            generated_dispatch_{}_watch_snapshot(&mut object, &request)?\n",
                lock, object.dispatch_name
            ),
            DispatchMode::Watch => format!(
                "{}            generated_dispatch_{}_watch(&mut object, request)\n",
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
            "            generated_dispatch_{}_watch(&mut object, request)\n",
            object.dispatch_name
        ),
    }
}

fn render_string_constructible_dispatch(object: &SourceObject, mode: DispatchMode) -> String {
    let base_segments = object.schema_key.split('.').collect::<Vec<_>>();
    let len = base_segments.len();
    let segment_checks = base_segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            format!(
                "request.targetPath.segments.get({index}).map(String::as_str) == Some({segment:?})"
            )
        })
        .collect::<Vec<_>>()
        .join(" && ");
    let dispatch = match mode {
        DispatchMode::Call => format!(
            "            return generated_dispatch_{}_call(&mut object, request).await;\n",
            object.dispatch_name
        ),
        DispatchMode::WatchSnapshot => format!(
            "            generated_dispatch_{}_watch_snapshot(&mut object, &request)?\n",
            object.dispatch_name
        ),
        DispatchMode::Watch => format!(
            "            return generated_dispatch_{}_watch(&mut object, request);\n",
            object.dispatch_name
        ),
    };
    format!(
        "        _ if request.targetPath.segments.len() == {} && {} => {{\n{}{}        }}\n",
        len + 1,
        segment_checks,
        render_object_constructor(object),
        dispatch
    )
}

fn render_factory_constructible_dispatch(object: &SourceObject, mode: DispatchMode) -> String {
    let ObjectAccess::FactoryMethodConstruct {
        factory_arg_types, ..
    } = &object.access
    else {
        return String::new();
    };
    let base_segments = object.schema_key.split('.').collect::<Vec<_>>();
    let len = base_segments.len();
    let segment_checks = base_segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            format!(
                "request.targetPath.segments.get({index}).map(String::as_str) == Some({segment:?})"
            )
        })
        .collect::<Vec<_>>()
        .join(" && ");
    let dispatch = match mode {
        DispatchMode::Call => format!(
            "            return generated_dispatch_{}_call(&mut object, request).await;\n",
            object.dispatch_name
        ),
        DispatchMode::WatchSnapshot => format!(
            "            generated_dispatch_{}_watch_snapshot(&mut object, &request)?\n",
            object.dispatch_name
        ),
        DispatchMode::Watch => format!(
            "            return generated_dispatch_{}_watch(&mut object, request);\n",
            object.dispatch_name
        ),
    };
    format!(
        "        _ if request.targetPath.segments.len() == {} && {} => {{\n{}{}        }}\n",
        len + factory_arg_types.len(),
        segment_checks,
        render_object_constructor(object),
        dispatch
    )
}

fn render_object_constructor(object: &SourceObject) -> String {
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
            let segment_index = object.schema_key.split('.').count();
            format!(
                "            let __core_instance_id = request.targetPath.segments.get({segment_index}).cloned().ok_or_else(|| operit_link::CoreLinkError::internal(\"missing object instance id\"))?;\n            let mut object = {}::new(__core_instance_id);\n",
                object.full_type
            )
        }
        ObjectAccess::ContextGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance(proxy.application.applicationContext.clone());\n",
                object.full_type
            )
        }
        ObjectAccess::ContextRefGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance(&proxy.application.applicationContext);\n",
                object.full_type
            )
        }
        ObjectAccess::ResultContextGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance(proxy.application.applicationContext.clone()).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n",
                object.full_type
            )
        }
        ObjectAccess::ResultContextRefGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance(&proxy.application.applicationContext).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n",
                object.full_type
            )
        }
        ObjectAccess::ContextGetInstanceArcMutexConstruct => {
            format!(
                "            let object = {}::getInstance(proxy.application.applicationContext.clone());\n",
                object.full_type
            )
        }
        ObjectAccess::ContextRefGetInstanceArcMutexConstruct => {
            format!(
                "            let object = {}::getInstance(&proxy.application.applicationContext);\n",
                object.full_type
            )
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
            ..
        } => render_factory_object_constructor(
            object,
            parent_full_type,
            parent_access,
            factory_method,
            factory_arg_types,
        ),
        ObjectAccess::Application | ObjectAccess::ChatRuntimeMain => String::new(),
    }
}

fn render_factory_object_constructor(
    object: &SourceObject,
    parent_full_type: &str,
    parent_access: &ObjectAccess,
    factory_method: &str,
    factory_arg_types: &[String],
) -> String {
    let base_index = object.schema_key.split('.').count();
    let mut output = String::new();
    for (index, _) in factory_arg_types.iter().enumerate() {
        let segment_index = base_index + index;
        output.push_str(&format!(
            "            let __core_factory_arg_{index} = request.targetPath.segments.get({segment_index}).cloned().ok_or_else(|| operit_link::CoreLinkError::internal(\"missing object factory argument\"))?;\n"
        ));
    }
    output.push_str("            let mut object = {\n");
    output.push_str(&render_object_constructor_for_access(
        "__core_parent_object",
        parent_full_type,
        parent_access,
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
        "                __core_parent_object.{factory_method}({factory_args})\n"
    ));
    output.push_str("            };\n");
    output
}

fn render_object_constructor_for_access(
    variable_name: &str,
    full_type: &str,
    access: &ObjectAccess,
) -> String {
    match access {
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
                "            let mut {variable_name} = {full_type}::getInstance(proxy.application.applicationContext.clone());\n"
            )
        }
        ObjectAccess::ContextRefGetInstanceConstruct => {
            format!(
                "            let mut {variable_name} = {full_type}::getInstance(&proxy.application.applicationContext);\n"
            )
        }
        ObjectAccess::ResultContextGetInstanceConstruct => {
            format!(
                "            let mut {variable_name} = {full_type}::getInstance(proxy.application.applicationContext.clone()).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n"
            )
        }
        ObjectAccess::ResultContextRefGetInstanceConstruct => {
            format!(
                "            let mut {variable_name} = {full_type}::getInstance(&proxy.application.applicationContext).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n"
            )
        }
        ObjectAccess::ContextGetInstanceArcMutexConstruct => {
            format!(
                "            let {variable_name} = {full_type}::getInstance(proxy.application.applicationContext.clone());\n"
            )
        }
        ObjectAccess::ContextRefGetInstanceArcMutexConstruct => {
            format!(
                "            let {variable_name} = {full_type}::getInstance(&proxy.application.applicationContext);\n"
            )
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
        | ObjectAccess::Application
        | ObjectAccess::ChatRuntimeMain => String::new(),
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
            "        {:?} => {{\n{}            object.{}({}){};\n            Ok(serde_json::Value::Null)\n        }}\n",
            method.name,
            args,
            method.name,
            call_args,
            await_suffix(method)
        ),
        Some(CallProtocol::ResultUnit { error_type }) => format!(
            "        {:?} => {{\n{}            object.{}({}){}.map_err(|error| core_call_error(error.to_string(), {}(&error)))?;\n            Ok(serde_json::Value::Null)\n        }}\n",
            method.name,
            args,
            method.name,
            call_args,
            await_suffix(method),
            error_details_converter(error_type, error_types)
        ),
        Some(CallProtocol::Value(_)) => format!(
            "        {:?} => {{\n{}            to_core_value(object.{}({}){})\n        }}\n",
            method.name,
            args,
            method.name,
            call_args,
            await_suffix(method)
        ),
        Some(CallProtocol::ResultValue { error_type, .. }) => format!(
            "        {:?} => {{\n{}            to_core_value(object.{}({}){}.map_err(|error| core_call_error(error.to_string(), {}(&error)))?)\n        }}\n",
            method.name,
            args,
            method.name,
            call_args,
            await_suffix(method),
            error_details_converter(error_type, error_types)
        ),
        None => String::new(),
    };
    render_cfg_attrs(method) + &arm
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
        WatchStreamProtocol::TextEvent { .. } => return String::new(),
    };
    format!(
        "        {:?} => {{\n{}            to_core_value({})\n        }}\n",
        method.name, args, value_expr
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
        WatchStreamProtocol::TextEvent { optional } => {
            render_text_event_watch_stream_arm(method, optional)
        }
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
        "        {:?} => {{\n{}            let flow = {};\n            let (sender, receiver) = core_event_stream_channel();\n            let requestId = request.requestId;\n            let targetPath = request.targetPath;\n            let propertyName = request.propertyName;\n            let isFirstEvent = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));\n            let subscription = flow.subscribeWithCancellation(\n                operit_store::PreferencesDataStore::FlowCancellation::new(),\n                move |value| {{\n                    let kind = if isFirstEvent.swap(false, std::sync::atomic::Ordering::SeqCst) {{\n                        operit_link::CoreEventKind::Snapshot\n                    }} else {{\n                        operit_link::CoreEventKind::Changed\n                    }};\n                    if let Ok(value) = serde_json::to_value(value) {{\n                        let _ = sender.send(operit_link::CoreEvent {{\n                            requestId: Some(requestId.clone()),\n                            targetPath: targetPath.clone(),\n                            propertyName: propertyName.clone(),\n                            kind,\n                            value,\n                        }});\n                    }}\n                }},\n            ).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n            Ok(receiver.withOnClose(move || subscription.cancel()))\n        }}\n",
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
        "        {:?} => {{\n{}            let stateFlow = {};\n            let (sender, receiver) = core_event_stream_channel();\n            let requestId = request.requestId;\n            let targetPath = request.targetPath;\n            let propertyName = request.propertyName;\n            let isFirstEvent = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));\n            let isFirstEventForSubscriber = isFirstEvent.clone();\n            let subscriptionId = stateFlow.subscribe(move |value| {{\n                let kind = if isFirstEventForSubscriber.swap(false, std::sync::atomic::Ordering::SeqCst) {{\n                    operit_link::CoreEventKind::Snapshot\n                }} else {{\n                    operit_link::CoreEventKind::Changed\n                }};\n                if let Ok(value) = serde_json::to_value(value) {{\n                    let _ = sender.send(operit_link::CoreEvent {{\n                        requestId: Some(requestId.clone()),\n                        targetPath: targetPath.clone(),\n                        propertyName: propertyName.clone(),\n                        kind,\n                        value,\n                    }});\n                }}\n            }});\n            Ok(receiver.withOnClose(move || stateFlow.unsubscribe(subscriptionId)))\n        }}\n",
        method.name, args, state_expr
    )
    .prepend_with(render_cfg_attrs(method))
}

fn render_text_event_watch_stream_arm(method: &SourceMethod, optional: bool) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    let chat_id_expr = method
        .args
        .iter()
        .find(|arg| arg.name == "chatId" || arg.name == "chat_id")
        .map(|arg| arg.name.clone())
        .unwrap_or_else(|| "\"\".to_string()".to_string());
    let stream_expr = if optional {
        format!(
            "object.{}({}).ok_or_else(|| operit_link::CoreLinkError::watchNotFound(&registryKey))?",
            method.name, call_args
        )
    } else {
        format!("object.{}({})", method.name, call_args)
    };
    format!(
        "        {:?} => {{\n{}            let streamChatId = {}.clone();\n            let stream = {};\n            Ok(core_text_event_stream(streamChatId, stream, request))\n        }}\n",
        method.name, args, chat_id_expr, stream_expr
    )
    .prepend_with(render_cfg_attrs(method))
}

fn render_string_watch_stream_arm(method: &SourceMethod) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    format!(
        "        {:?} => {{\n{}            let stream = object.{}({});\n            Ok(core_string_event_stream(stream, request))\n        }}\n",
        method.name, args, method.name, call_args
    )
    .prepend_with(render_cfg_attrs(method))
}

fn render_json_watch_stream_arm(method: &SourceMethod) -> String {
    let args = render_arg_decoders(method);
    let call_args = render_arg_call_list(method);
    format!(
        "        {:?} => {{\n{}            let stream = object.{}({});\n            Ok(core_json_event_stream(stream, request))\n        }}\n",
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
                "            let {}: {} = decode_core_arg(&mut __core_args, {:?})?;\n",
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
