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
    output.push_str("fn generated_core_proxy_schema() -> serde_json::Value {\n");
    output.push_str("    serde_json::from_str(r#\"");
    output.push_str(&schema_json);
    output.push_str("\"#).expect(\"generated core proxy schema must be valid JSON\")\n");
    output.push_str("}\n\n");
    output.push_str(&render_generated_error_details(objects, error_types));
    for object in objects {
        if object_uses_arc_mutex_instance(&object.access)
            && object
                .methods
                .iter()
                .any(|method| method.is_async && method.call_protocol().is_some())
        {
            panic!(
                "Arc<Mutex<Self>> core proxy object exposes async call methods: {}",
                object.schema_key
            );
        }
        output.push_str(&render_object_call_dispatch(object, error_types));
        output.push('\n');
        output.push_str(&render_object_sync_call_dispatch(object, error_types));
        output.push('\n');
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
        render_error_type_helper_recursive(error_type, error_types, &mut generated_helpers, &mut output);
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
                render_error_type_helper_recursive(&field.ty, error_types, generated_helpers, output);
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
        "fn {}(error: &{}) -> serde_json::Value {{\n",
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
                "{:?}: {}",
                field.name,
                error_field_json_expr(&field.name, &field.ty, error_types)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "            {}::{}{} => serde_json::json!({{\n                \"errorType\": {:?},\n                \"variant\": {:?},\n                \"message\": error.to_string(),\n                \"fields\": {{ {} }},\n            }}),\n",
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
        name.to_string()
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
    output.push_str("fn generated_core_proxy_error_details_for_string(error: &String) -> serde_json::Value {\n");
    output.push_str("    serde_json::json!({\n");
    output.push_str("        \"errorType\": \"String\",\n");
    output.push_str("        \"message\": error,\n");
    output.push_str("        \"fields\": { \"value\": error },\n");
    output.push_str("    })\n");
    output.push_str("}\n\n");
    output
}

fn render_leaf_error_details_helper() -> String {
    let mut output = String::new();
    output.push_str("fn generated_core_proxy_error_leaf_details<E: std::fmt::Display>(error: &E, error_type: &str) -> serde_json::Value {\n");
    output.push_str("    serde_json::json!({\n");
    output.push_str("        \"errorType\": error_type,\n");
    output.push_str("        \"message\": error.to_string(),\n");
    output.push_str("    })\n");
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
