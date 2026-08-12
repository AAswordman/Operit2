use super::build_rust_codegen_utils::*;
use super::build_rust_dispatch_codegen::{
    render_core_proxy_dispatch, render_object_call_dispatch, render_object_sync_call_dispatch,
    render_object_watch_dispatch, render_object_watch_snapshot_dispatch,
};
use super::build_rust_proxy_codegen::render_generated_proxy;
use super::*;

pub(crate) use super::build_rust_schema_codegen::render_schema;

pub(crate) fn render_generated(
    objects: &[SourceObject],
    schema_json: &str,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_core_proxy_schema() -> operit_link::CoreValue {\n");
    output.push_str("    to_core_value(serde_json::from_str::<serde_json::Value>(r#\"");
    output.push_str(&schema_json);
    output.push_str("\"#).expect(\"generated core proxy schema must be valid JSON\")).expect(\"generated core proxy schema must convert to CoreValue\")\n");
    output.push_str("}\n\n");
    output.push_str(&render_local_control_path_matcher(objects));
    output.push_str(&render_reverse_stream_dispatch(objects));
    output.push_str(&render_generated_error_details(objects, error_types));
    for object in objects {
        if object.has_call_dispatch()
            && (!object_uses_arc_mutex_instance(&object.access)
                || object
                    .methods
                    .iter()
                    .any(|method| method.is_async && method.call_protocol().is_some()))
        {
            output.push_str(&render_object_call_dispatch(object, error_types));
            output.push('\n');
        }
        if object_uses_arc_mutex_instance(&object.access) && object.has_sync_call_dispatch() {
            output.push_str(&render_object_sync_call_dispatch(object, error_types));
            output.push('\n');
        }
        output.push_str(&render_object_watch_snapshot_dispatch(object));
        output.push('\n');
        output.push_str(&render_object_watch_dispatch(object));
        output.push('\n');
    }
    output.push_str(&render_core_proxy_dispatch(objects));
    output.push('\n');
    output.push_str(&render_generated_proxy(objects));
    output
}

/// Renders runtime-owned opening logic for every generated reverse-stream method.
fn render_reverse_stream_dispatch(objects: &[SourceObject]) -> String {
    let mut output = String::new();
    output.push_str(
        "/// Returns whether one push request targets a schema-declared reverse stream.\n",
    );
    let patterns = objects
        .iter()
        .flat_map(|object| {
            object
                .methods
                .iter()
                .filter(|method| method.reverse_stream_protocol().is_some())
                .map(move |method| format!("({:?}, {:?})", object.schema_key, method.name))
        })
        .collect::<Vec<_>>()
        .join(" | ");
    output.push_str("fn generated_is_reverse_stream_request(request: &operit_link::CorePushRequest) -> bool {\n");
    if patterns.is_empty() {
        output.push_str("    false\n");
    } else {
        output.push_str(
            "    matches!((request.targetPath.key().as_str(), request.methodName.as_str()), ",
        );
        output.push_str(&patterns);
        output.push_str(")\n");
    }
    output.push_str("}\n\n");
    output.push_str(
        "/// Opens one schema-declared reverse stream without exposing Link details to services.\n",
    );
    output.push_str("fn generated_open_reverse_stream(proxy: &LocalCoreProxy, request: operit_link::CorePushRequest) -> Result<CoreReverseStreamSession, operit_link::CoreLinkError> {\n");
    output
        .push_str("    match (request.targetPath.key().as_str(), request.methodName.as_str()) {\n");
    for object in objects {
        for method in &object.methods {
            let Some(reverse) = method.reverse_stream_protocol() else {
                continue;
            };
            let construct_object = match object.access {
                ObjectAccess::ContextRefGetInstanceConstruct => format!(
                    "                let object = Ok::<{}, operit_link::CoreLinkError>({}::getInstance(&hostManager));\n",
                    object.full_type, object.full_type
                ),
                ObjectAccess::ResultContextRefGetInstanceConstruct => format!(
                    "                let object = {}::getInstance(&hostManager).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()));\n",
                    object.full_type
                ),
                _ => panic!(
                    "reverse stream method uses an unsupported object access: {}",
                    object.schema_key
                ),
            };
            let normal_args = method
                .args
                .iter()
                .filter(|arg| arg.name != reverse.argument_name)
                .collect::<Vec<_>>();
            let decode_args = normal_args
                .iter()
                .map(|arg| {
                    format!(
                        "            let {}: {} = decode_core_arg(&mut __core_args, {:?})?;\n",
                        arg.name, arg.ty, arg.name
                    )
                })
                .collect::<String>();
            let call_args = method
                .args
                .iter()
                .map(|arg| {
                    if arg.name == reverse.argument_name {
                        "input".to_string()
                    } else {
                        arg.name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            output.push_str(&format!("        ({:?}, {:?}) => {{\n            let mut __core_args = object_args(request.args)?;\n{}            let (sender, input) = operit_util::stream::ReverseStream::ReverseStream::<{}>::channel();\n            let (completionSender, completionReceiver) = tokio::sync::oneshot::channel();\n            let hostManager = proxy.hostManager.clone();\n            operit_host_api::HostRuntimeTaskSchedulerHost::scheduleHostRuntimeAsyncTask(operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost().as_ref(), \"core-proxy-reverse-stream\", Box::new(move || Box::pin(async move {{\n{}                let result = match object {{\n                    Ok(object) => object.{}({}).await.map_err(|error| operit_link::CoreLinkError::internal(error.to_string())),\n                    Err(error) => Err(error),\n                }};\n                let _ = completionSender.send(result);\n            }}))).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n            Ok(CoreReverseStreamSession::new(sender, completionReceiver))\n        }}\n", object.schema_key, method.name, decode_args, reverse.item_type, construct_object, method.name, call_args));
        }
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::new(\"REVERSE_STREAM_NOT_FOUND\", \"reverse stream method is not declared by this proxy\")),\n    }\n}\n\n");
    output
}

/// Renders the generated routing classification for locally handled control objects.
fn render_local_control_path_matcher(objects: &[SourceObject]) -> String {
    let paths = objects
        .iter()
        .filter(|object| object.route_scope == ObjectRouteScope::LocalControl)
        .map(|object| format!("{:?}", object.schema_key))
        .collect::<Vec<_>>()
        .join(" | ");
    let mut output = String::new();
    output.push_str(
        "/// Returns whether a generated object is handled by the local runtime control plane.\n",
    );
    output.push_str(
        "pub(crate) fn generated_is_local_runtime_control_path(target_path: &operit_link::CoreObjectPath) -> bool {\n",
    );
    if paths.is_empty() {
        output.push_str("    false\n");
    } else {
        output.push_str("    matches!(target_path.key().as_str(), ");
        output.push_str(&paths);
        output.push_str(")\n");
    }
    output.push_str("}\n\n");
    output
}

fn render_generated_error_details(
    objects: &[SourceObject],
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let mut used = objects
        .iter()
        .flat_map(|object| object.methods.iter())
        .filter_map(|method| match method.call_protocol()? {
            CallProtocol::ResultUnit { error_type } => Some(error_type.as_str()),
            CallProtocol::ResultValue { error_type, .. } => Some(error_type.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    used.sort();
    used.dedup();

    let mut output = String::new();
    output.push_str(&render_string_error_details_helper());
    output.push_str(&render_leaf_error_details_helper());

    let mut generated_helpers = HashSet::new();
    for error_type in &used {
        let error_type = *error_type;
        render_error_type_helper_recursive(
            error_type,
            error_types,
            &mut generated_helpers,
            &mut output,
        );
    }
    output
}

fn render_error_type_helper_recursive(
    error_type: &str,
    error_types: &HashMap<String, ErrorTypeDefinition>,
    generated_helpers: &mut HashSet<String>,
    output: &mut String,
) {
    if !generated_helpers.insert(error_type.to_string()) {
        return;
    }
    let Some(definition) = error_types.get(error_type) else {
        return;
    };
    for variant in &definition.variants {
        for field in &variant.fields {
            if error_types.contains_key(&field.ty) {
                render_error_type_helper_recursive(
                    &field.ty,
                    error_types,
                    generated_helpers,
                    output,
                );
            }
        }
    }
    output.push_str(&render_error_type_helper(definition, error_types));
}

fn render_error_type_helper(
    definition: &ErrorTypeDefinition,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "fn {}(error: &{}) -> operit_link::CoreValue {{\n",
        error_details_fn_name(&definition.full_type),
        definition.full_type
    ));
    output.push_str("        match error {\n");
    for variant in &definition.variants {
        output.push_str(&render_error_variant_arm(definition, variant, error_types));
    }
    output.push_str("        }\n");
    output.push_str("}\n\n");
    output
}

fn render_error_variant_arm(
    definition: &ErrorTypeDefinition,
    variant: &ErrorEnumVariant,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    let pattern = match variant.fields_kind {
        ErrorFieldsKind::Unit => String::new(),
        ErrorFieldsKind::Named => {
            let bindings = variant
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!(" {{ {bindings} }}")
        }
        ErrorFieldsKind::Unnamed => {
            let bindings = variant
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("({bindings})")
        }
    };
    let fields = variant
        .fields
        .iter()
        .map(|field| {
            format!(
                "({:?}.to_string(), {})",
                field.name,
                error_field_json_expr(&field.name, &field.ty, error_types)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "            {}::{}{} => core_value_map(vec![(\"errorType\".to_string(), CoreValue::String({:?}.to_string())), (\"variant\".to_string(), CoreValue::String({:?}.to_string())), (\"message\".to_string(), CoreValue::String(error.to_string())), (\"fields\".to_string(), core_value_map(vec![{}]))]),\n",
        definition.full_type,
        variant.name,
        pattern,
        definition.full_type,
        variant.name,
        fields
    )
}

fn error_field_json_expr(
    name: &str,
    ty: &str,
    error_types: &HashMap<String, ErrorTypeDefinition>,
) -> String {
    if is_json_direct_error_field_type(ty) {
        format!("to_core_value({name}).expect(\"error field must convert\")")
    } else if error_types.contains_key(ty) {
        format!("{}({name})", error_details_fn_name(ty))
    } else {
        format!(
            "generated_core_proxy_error_leaf_details({name}, {})",
            json_string(ty)
        )
    }
}

fn render_string_error_details_helper() -> String {
    let mut output = String::new();
    output.push_str(
        "fn generated_core_proxy_error_details_for_string(error: &String) -> operit_link::CoreValue {\n",
    );
    output.push_str("    core_value_map(vec![(\"errorType\".to_string(), CoreValue::String(\"String\".to_string())), (\"message\".to_string(), CoreValue::String(error.clone())), (\"fields\".to_string(), core_value_map(vec![(\"value\".to_string(), CoreValue::String(error.clone()))]))])\n");
    output.push_str("}\n\n");
    output
}

fn render_leaf_error_details_helper() -> String {
    let mut output = String::new();
    output.push_str("fn generated_core_proxy_error_leaf_details<E: std::fmt::Display>(error: &E, error_type: &str) -> operit_link::CoreValue {\n");
    output.push_str("    core_value_map(vec![(\"errorType\".to_string(), CoreValue::String(error_type.to_string())), (\"message\".to_string(), CoreValue::String(error.to_string()))])\n");
    output.push_str("}\n\n");
    output
}

fn is_json_direct_error_field_type(ty: &str) -> bool {
    matches!(
        ty,
        "String"
            | "&str"
            | "bool"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "usize"
            | "f32"
            | "f64"
            | "serde_json::Value"
    )
}
