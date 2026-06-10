use super::build_rust_codegen_utils::*;
use super::*;

pub(crate) fn render_schema(
    objects: &[SourceObject],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    format!(
        "{{\"objects\":{},\"types\":{}}}",
        render_schema_objects(objects),
        render_schema_types(serializable_types)
    )
}

fn render_schema_objects(objects: &[SourceObject]) -> String {
    let entries = objects
        .iter()
        .map(|object| {
            format!(
                "{}:{}",
                json_string(&object.schema_key),
                render_schema_methods(&object.methods)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{entries}}}")
}

fn render_schema_methods(methods: &[SourceMethod]) -> String {
    let entries = methods
        .iter()
        .map(|method| {
            let args = method
                .args
                .iter()
                .map(|arg| {
                    format!(
                        "{{\"name\":{},\"type\":{}}}",
                        json_string(&arg.name),
                        json_string(&arg.ty)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"name\":{},\"args\":[{}],\"async\":{},\"callable\":{},\"watchable\":{},\"returnType\":{},\"protocol\":{},\"unsupportedReason\":{}}}",
                json_string(&method.name),
                args,
                method.is_async,
                method.call_protocol().is_some(),
                method.watch_protocol().is_some(),
                json_string(&method.rust_return_type),
                render_schema_protocol(&method.protocol),
                option_json_string(method.unsupported_reason())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_schema_types(serializable_types: &HashMap<String, SerializableType>) -> String {
    let mut types = serializable_types.values().collect::<Vec<_>>();
    types.sort_by(|left, right| left.full_type.cmp(&right.full_type));
    let entries = types
        .iter()
        .map(|ty| match &ty.kind {
            SerializableTypeKind::Struct { fields } => {
                let fields_json = fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{{\"name\":{},\"type\":{}}}",
                            json_string(&field.name),
                            json_string(&field.ty)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{}:{{\"kind\":\"struct\",\"fields\":[{}]}}",
                    json_string(&ty.full_type),
                    fields_json
                )
            }
            SerializableTypeKind::Enum { variants } => {
                let variants_json = variants
                    .iter()
                    .map(|variant| json_string(variant))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{}:{{\"kind\":\"enum\",\"variants\":[{}]}}",
                    json_string(&ty.full_type),
                    variants_json
                )
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{entries}}}")
}

fn render_schema_protocol(protocol: &MethodProtocol) -> String {
    match protocol {
        MethodProtocol::Call(_) => {
            "{\"mode\":\"Call\",\"payload\":\"Json\",\"initial\":\"None\"}".to_string()
        }
        MethodProtocol::Watch(watch) => {
            let payload = match watch.stream {
                WatchStreamProtocol::JsonFlow { .. } | WatchStreamProtocol::JsonState { .. } => {
                    "Json"
                }
                WatchStreamProtocol::StringStream => "String",
                WatchStreamProtocol::TextEvent { .. } => "TextStreamEvent",
            };
            let initial = if watch.snapshot_type.is_some() {
                "Snapshot"
            } else {
                "None"
            };
            format!("{{\"mode\":\"Watch\",\"payload\":\"{payload}\",\"initial\":\"{initial}\"}}")
        }
        MethodProtocol::Factory(factory) => format!(
            "{{\"mode\":\"Factory\",\"target\":{}}}",
            json_string(&factory.target_schema_key)
        ),
        MethodProtocol::Unsupported(_) => "null".to_string(),
    }
}
