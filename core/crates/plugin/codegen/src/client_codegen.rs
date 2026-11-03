use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use operit_rslink_codegen::{
    CallProtocol, MethodProtocol, SerializableType, SerializableTypeKind, SourceArg, SourceObject,
};

/// Writes the four external SDK clients from the canonical Core proxy scan.
pub fn generate_plugin_sdk_client_bindings(
    objects: &[SourceObject],
    serializable_types: &HashMap<String, SerializableType>,
    output_root: &Path,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let objects = collect_objects(objects);
    let outputs = [
        (
            output_root.join("rust/src/generated.rs"),
            render_rust(&objects, serializable_types),
        ),
        (
            output_root.join("dart/lib/src/generated.dart"),
            render_dart(&objects, serializable_types),
        ),
        (
            output_root.join("dart/lib/src/models.dart"),
            render_dart_models(&objects, serializable_types),
        ),
        (
            output_root.join("kotlin/src/main/kotlin/OperitPluginSdkGenerated.kt"),
            render_kotlin(&objects, serializable_types),
        ),
        (
            output_root.join("typescript/src/generated.ts"),
            render_typescript(&objects, serializable_types),
        ),
        (
            output_root.join("typescript/src/models.ts"),
            render_typescript_models(&objects, serializable_types),
        ),
        (
            output_root.join("kotlin/src/main/kotlin/OperitPluginSdkModels.kt"),
            render_kotlin_models(&objects, serializable_types),
        ),
    ];
    let mut files = Vec::with_capacity(outputs.len());
    for (path, contents) in outputs {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_if_changed(&path, &contents)?;
        files.push(path);
    }
    Ok(files)
}

/// Renders the generated Core route allowlist consumed by the SDK IPC server.
pub fn generate_plugin_sdk_surface(
    objects: &[SourceObject],
    output_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut output = String::from("// GENERATED FILE. Source: explicit `sdk` route annotations.\n\nuse operit_plugin_sdk_ipc::PluginSdkSurface;\n\n/// Builds the externally exposed SDK route surface.\npub fn pluginSdkSurface() -> PluginSdkSurface {\n    let mut surface = PluginSdkSurface::new();\n");
    for object in objects {
        for method in &object.methods {
            if method.sdk_exposed
                && matches!(
                    &method.protocol,
                    MethodProtocol::Call(_)
                        | MethodProtocol::Factory(_)
                        | MethodProtocol::Watch(_)
                        | MethodProtocol::ReverseStream(_)
                )
            {
                output.push_str(&format!("    surface.expose({}, {:?});\n", format!("{:?}", object.object_id), method.name));
            }
        }
    }
    output.push_str("    surface\n}\n");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_if_changed(output_path, &output)
}

/// Keeps only methods represented by the canonical call/watch/push protocols.
fn collect_objects(objects: &[SourceObject]) -> Vec<SdkObject> {
    objects
        .iter()
        .filter_map(|object| {
            let mut names = BTreeSet::new();
            let methods = object
                .methods
                .iter()
                .filter_map(|method| {
                    if !method.sdk_exposed {
                        return None;
                    }
                    let mode = match &method.protocol {
                        MethodProtocol::Call(_) | MethodProtocol::Factory(_) => SdkMode::Call,
                        MethodProtocol::Watch(_) => SdkMode::Watch,
                        MethodProtocol::ReverseStream(_) => SdkMode::Push,
                        MethodProtocol::Unsupported(_) => return None,
                    };
                    if !names.insert(method.name.clone()) {
                        return None;
                    }
                    Some(SdkMethod {
                        name: method.name.clone(),
                        args: method.args.iter().filter(|arg| match &method.protocol {
                            MethodProtocol::ReverseStream(stream) => arg.name != stream.argument_name,
                            _ => true,
                        }).cloned().collect(),
                        mode,
                        return_type: call_return_type(&method.protocol),
                    })
                })
                .collect::<Vec<_>>();
            (!methods.is_empty()).then(|| SdkObject {
                object_id: object.object_id.clone(),
                schema_key: object.schema_key.clone(),
                class_name: class_name(&object.schema_key),
                methods,
            })
        })
        .collect()
}

#[derive(Clone, Copy)]
enum SdkMode {
    Call,
    Watch,
    Push,
}

struct SdkMethod {
    name: String,
    args: Vec<SourceArg>,
    mode: SdkMode,
    return_type: Option<String>,
}

/// Extracts the declared value type carried by one call protocol.
fn call_return_type(protocol: &MethodProtocol) -> Option<String> {
    match protocol {
        MethodProtocol::Call(CallProtocol::Value(value_type)) => Some(value_type.clone()),
        MethodProtocol::Call(CallProtocol::ResultValue { value_type, .. }) => {
            Some(value_type.clone())
        }
        MethodProtocol::Factory(_) => Some("Object".to_string()),
        MethodProtocol::Call(CallProtocol::Unit | CallProtocol::ResultUnit { .. })
        | MethodProtocol::ReverseStream(_)
        | MethodProtocol::Unsupported(_) => None,
        MethodProtocol::Watch(protocol) => Some(protocol.item_type.clone()),
    }
}

/// Maps the scanner's scalar Rust value types to Dart SDK value types.
fn dart_value_type(
    return_type: Option<&str>,
    serializable_types: &HashMap<String, SerializableType>,
) -> Option<String> {
    match return_type {
        None => None,
        Some(value) => Some(dart_type(value, serializable_types)),
    }
}

struct SdkObject {
    object_id: String,
    schema_key: String,
    class_name: String,
    methods: Vec<SdkMethod>,
}

/// Writes generated content only when bytes differ.
fn write_if_changed(path: &Path, content: &str) -> Result<(), Box<dyn Error>> {
    if fs::read_to_string(path).is_ok_and(|existing| existing == content) {
        return Ok(());
    }
    fs::write(path, content)?;
    Ok(())
}

/// Converts one schema key into a stable public client class name.
fn class_name(schema_key: &str) -> String {
    let mut value = String::from("Operit");
    for part in schema_key.split(['.', ':', '/', '-']) {
        if part.is_empty() {
            continue;
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            value.push(first.to_ascii_uppercase());
            value.extend(chars);
        }
    }
    value.push_str("Client");
    value
}

/// Converts one source method name into an identifier accepted by generated clients.
fn method_name(name: &str) -> String {
    let mut value = String::new();
    for (index, ch) in name.chars().enumerate() {
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            value.push('_');
        } else if index == 0 && ch.is_ascii_digit() {
            value.push('_');
            value.push(ch);
        } else {
            value.push(ch);
        }
    }
    if value.is_empty() {
        "method".to_string()
    } else {
        value
    }
}

/// Names the strongly typed language surface being rendered.
#[derive(Clone, Copy)]
enum SdkLanguage { Rust, Dart, Kotlin, TypeScript }

/// Rejects argument types that cannot be represented faithfully by generated SDK models.
fn validate_argument_type(ty: &str, all: &HashMap<String, SerializableType>) {
    if let Some(inner) = ty.strip_prefix('&') { validate_argument_type(inner, all); return; }
    for constructor in ["Option", "Vec", "BTreeMap", "std::collections::BTreeMap", "HashMap", "std::collections::HashMap"] {
        if let Some(args) = generic_args(ty, constructor) { for arg in args { validate_argument_type(arg, all); } return; }
    }
    if matches!(ty, "str" | "String" | "bool" | "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" | "f32" | "f64" | "serde_json::Value") { return; }
    assert!(matches!(all.get(ty).map(|model| &model.kind), Some(SerializableTypeKind::Struct { .. } | SerializableTypeKind::Enum { unit_only: true, .. })), "Unsupported Plugin SDK argument type: {ty}");
}

/// Renders concrete public parameters and their named Link argument map.
fn sdk_parameters(method: &SdkMethod, all: &HashMap<String, SerializableType>, language: SdkLanguage) -> (String, String) {
    let mut parameters = Vec::new();
    let mut arguments = Vec::new();
    for arg in &method.args {
        validate_argument_type(&arg.ty, all);
        let name = method_name(&arg.name);
        match language {
            SdkLanguage::Rust => {
                let ty = if arg.ty == "&str" { "&str".to_string() } else { rust_type(arg.ty.trim_start_matches('&'), all) };
                parameters.push(format!("{name}: {ty}"));
                arguments.push(format!("(\"{}\".to_string(), operit_link::toCoreValue(&{name}).map_err(|error| CoreLinkError::internal(error.to_string()))?)", arg.name));
            }
            SdkLanguage::Dart => {
                let ty = dart_type(&arg.ty, all);
                parameters.push(format!("required {ty} {name}"));
                arguments.push(format!("'{}': {}", arg.name, dart_messagepack_encode(&name, &ty)));
            }
            SdkLanguage::Kotlin => {
                let ty = kotlin_type(&arg.ty, all);
                parameters.push(format!("{name}: {ty}"));
                arguments.push(format!("\"{}\" to {}", arg.name, kotlin_encode_expr(&name, &ty)));
            }
            SdkLanguage::TypeScript => {
                parameters.push(format!("{name}: {}", ts_public_type(&arg.ty, all)));
                arguments.push(format!("'{}': {}", arg.name, ts_encode_expr(&name, &ts_type(&arg.ty, all))));
            }
        }
    }
    let joined = parameters.join(", ");
    match language {
        SdkLanguage::Rust => (if joined.is_empty() { joined } else { format!(", {joined}") }, format!("CoreValue::Map(std::collections::BTreeMap::from([{}]))", arguments.join(", "))),
        SdkLanguage::Dart => (if joined.is_empty() { joined } else { format!("{{{joined}}}") }, format!("<String, Object?>{{{}}}", arguments.join(", "))),
        SdkLanguage::Kotlin => (joined, format!("mapOf<String, Any?>({})", arguments.join(", "))),
        SdkLanguage::TypeScript => (joined, format!("{{{}}}", arguments.join(", "))),
    }
}

/// Encodes a typed Kotlin value into the canonical Link argument representation.
fn kotlin_encode_expr(value: &str, ty: &str) -> String {
    if matches!(ty, "Any?" | "Boolean" | "String" | "Int" | "Double") { return value.to_string(); }
    if let Some(inner) = ty.strip_suffix('?') { return format!("{value}?.let {{ {} }}", kotlin_encode_expr("it", inner)); }
    if ty == "ByteArray" { return format!("{value}.map {{ it.toInt() and 255 }}"); }
    if let Some(inner) = ty.strip_prefix("List<").and_then(|v| v.strip_suffix('>')) { return format!("{value}.map {{ item -> {} }}", kotlin_encode_expr("item", inner)); }
    if let Some(args) = generic_args(ty, "Map") { assert_eq!(args.len(), 2); return format!("{value}.entries.associate {{ entry -> entry.key to {} }}", kotlin_encode_expr("entry.value", args[1])); }
    format!("{value}.toMessagePackValue()")
}

/// Encodes a typed TypeScript value into the canonical Link argument representation.
fn ts_encode_expr(value: &str, ty: &str) -> String {
    if matches!(ty, "unknown" | "boolean" | "string" | "number") { return value.to_string(); }
    if let Some(inner) = ty.strip_suffix(" | null") { return format!("{value} === null ? null : {}", ts_encode_expr(value, inner)); }
    if ty == "Uint8Array" { return format!("Array.from({value})"); }
    if let Some(inner) = ty.strip_prefix("Array<").and_then(|v| v.strip_suffix('>')) { return format!("{value}.map(item => {})", ts_encode_expr("item", inner)); }
    if let Some(args) = generic_args(ty, "Record") { assert_eq!(args.len(), 2); return format!("Object.fromEntries(Object.entries({value}).map(([key, item]) => [key, {}]))", ts_encode_expr("item", args[1])); }
    format!("models.encode{ty}({value})")
}

/// Renders the Rust client wrappers over the canonical PluginSdkClient.
fn render_rust(objects: &[SdkObject], serializable_types: &HashMap<String, SerializableType>) -> String {
    let mut output = String::from(
        "// GENERATED FILE. Source: operit-proxy-scan.\n\nuse operit_link::{CoreEventStream, CoreLinkError, CoreLinkPushSession, CoreValue};\nuse operit_plugin_sdk_ipc::PluginSdkClient;\nuse crate::{decode_messagepack_value, OperitPluginSdkTypedEventStream};\n\n",
    );
    output.push_str(&render_rust_models(objects, serializable_types));
    for object in objects {
        output.push_str(&format!(
            "/// Generated client for Core object `{}`.\n#[derive(Clone)]\npub struct {} {{ client: PluginSdkClient }}\n\nimpl {} {{\n    /// Creates a generated object client over one connected SDK session.\n    pub fn new(client: PluginSdkClient) -> Self {{ Self {{ client }} }}\n",
            object.schema_key, object.class_name, object.class_name
        ));
        for method in &object.methods {
            let method_name = method_name(&method.name);
            let (parameters, arguments) = sdk_parameters(method, serializable_types, SdkLanguage::Rust);
            match method.mode {
                SdkMode::Call => output.push_str(&format!(
                        "    /// Calls `{}` through the Core Link route.\n    pub async fn {}(&self{parameters}) -> Result<{}, CoreLinkError> {{\n        let args = {arguments};\n        let response = self.client.call(operit_link::CoreCallRequest::new(self.client.nextRequestId()?, {}, \"{}\", args)).await;\n        let value = response.result?;\n        decode_messagepack_value(&value)\n    }}\n",
                    method.name, method_name, rust_method_type(method.return_type.as_deref(), serializable_types), format!("{:?}", object.object_id), method.name
                )),
                SdkMode::Watch => output.push_str(&format!(
                    "    /// Watches `{}` through the Core Link route.\n    pub async fn {}(&self{parameters}) -> Result<OperitPluginSdkTypedEventStream<{}>, CoreLinkError> {{\n        let args = {arguments};\n        let stream = self.client.watch(operit_link::CoreWatchRequest::new(self.client.nextRequestId()?, {}, \"{}\", args)).await?;\n        Ok(OperitPluginSdkTypedEventStream::new(stream))\n    }}\n",
                    method.name, method_name, rust_method_type(method.return_type.as_deref(), serializable_types), format!("{:?}", object.object_id), method.name
                )),
                SdkMode::Push => output.push_str(&format!(
                    "    /// Opens the caller-owned `{}` Core input stream.\n    pub async fn {}(&self{parameters}) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {{\n        let args = {arguments};\n        self.client.openPush(operit_link::CorePushRequest::new(self.client.nextRequestId()?, {}, \"{}\").withArgs(args)).await\n    }}\n",
                    method.name, method_name, format!("{:?}", object.object_id), method.name
                )),
            }
        }
        output.push_str("}\n\n");
    }
    output
}

/// Maps one scanned return type to a Rust SDK model type.
fn rust_method_type(return_type: Option<&str>, serializable_types: &HashMap<String, SerializableType>) -> String {
    return_type.map(|value| rust_type(value, serializable_types)).unwrap_or_else(|| "()".to_string())
}

/// Maps one portable Rust protocol type into the generated SDK model namespace.
fn rust_type(ty: &str, serializable_types: &HashMap<String, SerializableType>) -> String {
    if let Some(inner) = generic_arg(ty, "Option") { return format!("Option<{}>", rust_type(inner, serializable_types)); }
    if let Some(inner) = generic_arg(ty, "Vec") { return format!("Vec<{}>", rust_type(inner, serializable_types)); }
    if let Some(args) = generic_args(ty, "BTreeMap").or_else(|| generic_args(ty, "std::collections::BTreeMap")) { if args.len() == 2 { return format!("std::collections::BTreeMap<{}, {}>", rust_type(args[0], serializable_types), rust_type(args[1], serializable_types)); } }
    match ty {
        "serde_json::Value" => "operit_link::CoreValue".to_string(),
        _ => match serializable_types.get(ty) {
            Some(SerializableType { kind: SerializableTypeKind::Struct { .. }, .. })
            | Some(SerializableType { kind: SerializableTypeKind::Enum { unit_only: true, .. }, .. }) => format!("Sdk{}", dart_class_name(ty)),
            Some(_) => "operit_link::CoreValue".to_string(),
            None => ty.to_string(),
        },
    }
}

/// Renders SDK-owned Rust models with serde MessagePack decoding.
fn render_rust_models(objects: &[SdkObject], serializable_types: &HashMap<String, SerializableType>) -> String {
    let reachable = reachable_sdk_types(objects, serializable_types);
    let mut output = String::new();
    for name in reachable {
        let Some(ty) = serializable_types.get(&name) else { continue; };
        if let SerializableTypeKind::Struct { fields } = &ty.kind {
            let class_name = format!("Sdk{}", dart_class_name(&ty.full_type));
            output.push_str(&format!("/// Generated SDK model for `{}`.\n#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]\npub struct {} {{\n", ty.full_type, class_name));
            for field in fields { output.push_str(&format!("    #[serde(rename = \"{}\")]\n    pub {}: {},\n", field.json_name, dart_field_name(&field.name), rust_type(&field.ty, serializable_types))); }
            output.push_str("}\n\n");
        }
        if let SerializableTypeKind::Enum { variants, unit_only: true } = &ty.kind {
            let class_name = format!("Sdk{}", dart_class_name(&ty.full_type));
            output.push_str(&format!("/// Generated SDK enum for `{}`.\n#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]\n#[serde(rename_all = \"SCREAMING_SNAKE_CASE\")]\npub enum {} {{\n", ty.full_type, class_name));
            for variant in variants { output.push_str(&format!("    {},\n", dart_field_name(&variant.name))); }
            output.push_str("}\n\n");
        }
    }
    output
}

/// Renders Dart wrappers over the package's concrete IPC client.
fn render_dart(
    objects: &[SdkObject],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let mut output = String::from(
        "// GENERATED FILE. Source: operit-proxy-scan.\n\nimport 'client.dart';\nimport 'models.dart';\n\n",
    );
    for object in objects {
        output.push_str(&format!(
            "/// Generated client for Core object `{}`.\nfinal class {} {{\n  const {}(this._client);\n  final OperitPluginSdkClient _client;\n",
            object.schema_key, object.class_name, object.class_name
        ));
        for method in &object.methods {
            let method_name = method_name(&method.name);
            let (parameters, arguments) = sdk_parameters(method, serializable_types, SdkLanguage::Dart);
            match method.mode {
                SdkMode::Call => match dart_value_type(method.return_type.as_deref(), serializable_types) {
                    None => output.push_str(&format!(
                        "  /// Calls `{}` through the Core Link route.\n  Future<void> {}({parameters}) async {{ await _client.call({}, '{}', {arguments}); }}\n",
                        method.name, method_name, format!("{:?}", object.object_id), method.name
                    )),
                    Some(return_type) => output.push_str(&format!(
                        "  /// Calls `{}` through the Core Link route.\n  Future<{}> {}({parameters}) async {{ return await _client.callTyped<{}>({}, '{}', {arguments}, {}); }}\n",
                        method.name,
                        return_type,
                        method_name,
                        return_type,
                        format!("{:?}", object.object_id),
                        method.name,
                        dart_decoder_expr(method.return_type.as_deref().expect("typed return"), serializable_types)
                    )),
                },
                SdkMode::Watch => {
                    let item_type = dart_value_type(method.return_type.as_deref(), serializable_types).unwrap_or_else(|| "Object?".to_string());
                    output.push_str(&format!(
                        "  /// Watches `{}` through the Core Link route.\n  Stream<{}> {}({parameters}) => _client.watchTyped<{}>({}, '{}', {arguments}, {});\n",
                        method.name,
                        item_type,
                        method_name,
                        item_type,
                        format!("{:?}", object.object_id),
                        method.name,
                        dart_decoder_expr(method.return_type.as_deref().expect("watch item type"), serializable_types)
                    ));
                },
                SdkMode::Push => output.push_str(&format!(
                    "  /// Opens the caller-owned `{}` Core input stream.\n  Future<PluginSdkPushSink> {}({parameters}) => _client.push({}, '{}', {arguments});\n",
                    method.name, method_name, format!("{:?}", object.object_id), method.name
                )),
            }
        }
        output.push_str("}\n\n");
    }
    output
}

/// Maps one scanner Rust type into a concrete SDK Dart type.
fn dart_type(ty: &str, serializable_types: &HashMap<String, SerializableType>) -> String {
    if ty == "str" { return "String".to_string(); }
    if let Some(inner) = ty.strip_prefix('&') { return dart_type(inner, serializable_types); }
    if ty == "Vec<u8>" { return "Uint8List".to_string(); }
    if let Some(inner) = generic_arg(ty, "Option") { return nullable_type(dart_type(inner, serializable_types)); }
    if let Some(inner) = generic_arg(ty, "Vec") { return format!("List<{}>", dart_type(inner, serializable_types)); }
    if let Some(args) = generic_args(ty, "BTreeMap").or_else(|| generic_args(ty, "std::collections::BTreeMap")).or_else(|| generic_args(ty, "HashMap")).or_else(|| generic_args(ty, "std::collections::HashMap")) {
        if args.len() == 2 { return format!("Map<{}, {}>", dart_type(args[0], serializable_types), dart_type(args[1], serializable_types)); }
    }
    match ty {
        "()" => "void".to_string(),
        "bool" => "bool".to_string(),
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => "int".to_string(),
        "f32" | "f64" => "double".to_string(),
        "String" | "&str" => "String".to_string(),
        "serde_json::Value" => "Object?".to_string(),
        _ => match serializable_types.get(ty) {
            Some(SerializableType { kind: SerializableTypeKind::Struct { .. }, .. })
            | Some(SerializableType { kind: SerializableTypeKind::Enum { unit_only: true, .. }, .. }) => dart_class_name(ty),
            _ => "Object?".to_string(),
        },
    }
}

/// Returns a nullable Dart type without duplicating a nullable marker.
fn nullable_type(value: String) -> String { if value.ends_with('?') { value } else { format!("{value}?") } }

/// Returns the public Dart class name for a Rust fully qualified type.
fn dart_class_name(ty: &str) -> String {
    let raw = ty.rsplit("::").next().unwrap_or(ty);
    let mut out = String::new();
    for part in raw.split(['_', '-']) {
        if part.is_empty() { continue; }
        let mut chars = part.chars();
        if let Some(first) = chars.next() { out.push(first.to_ascii_uppercase()); out.extend(chars); }
    }
    if out.is_empty() { "PluginSdkModel".to_string() } else { out }
}

/// Splits one generic type into its top-level arguments.
fn generic_args<'a>(ty: &'a str, name: &str) -> Option<Vec<&'a str>> {
    let prefix = format!("{name}<");
    if !ty.starts_with(&prefix) || !ty.ends_with('>') { return None; }
    let body = &ty[prefix.len()..ty.len() - 1];
    let mut args = Vec::new(); let mut start = 0usize; let mut depth = 0i32;
    for (index, ch) in body.char_indices() {
        match ch { '<' => depth += 1, '>' => depth -= 1, ',' if depth == 0 => { args.push(body[start..index].trim()); start = index + 1; }, _ => {} }
    }
    args.push(body[start..].trim()); Some(args)
}

/// Returns the single generic argument for a type constructor.
fn generic_arg<'a>(ty: &'a str, name: &str) -> Option<&'a str> { generic_args(ty, name).and_then(|args| (args.len() == 1).then_some(args[0])) }

/// Builds a decoder callback expression for one generated call result.
fn dart_decoder_expr(rust_type: &str, serializable_types: &HashMap<String, SerializableType>) -> String {
    let dart = dart_type(rust_type, serializable_types);
    dart_decoder_for_dart_type(&dart)
}

/// Builds a decoder callback from a concrete Dart type.
fn dart_decoder_for_dart_type(dart: &str) -> String {
    format!("(value) => {}", dart_decode_value_expr("value", dart))
}

/// Renders a nested SDK decoder expression for one value variable.
fn dart_decode_value_expr(value: &str, dart: &str) -> String {
    if dart == "Uint8List" { return format!("Uint8List.fromList((({value} as List).cast<int>()))"); }
    if dart == "bool" || dart == "String" { return format!("{value} as {dart}"); }
    if dart == "int" { return format!("({value} as num).toInt()"); }
    if dart == "double" { return format!("({value} as num).toDouble()"); }
    if dart == "Object?" { return value.to_string(); }
    if let Some(inner) = dart.strip_suffix('?') { return format!("{value} == null ? null : {}", dart_decode_value_expr(value, inner)); }
    if let Some(inner) = dart.strip_prefix("List<").and_then(|v| v.strip_suffix('>')) { return format!("({value} as List<Object?>).map((item) => {}).toList(growable: false)", dart_decode_value_expr("item", inner)); }
    if let Some(args) = generic_args(dart, "Map") {
        if args.len() == 2 { return format!("({value} as Map).map((key, item) => MapEntry({}, {}))", dart_decode_value_expr("key", args[0]), dart_decode_value_expr("item", args[1])); }
    }
    format!("{}.fromMessagePackValue({value} as Map<String, Object?>)", dart)
}

/// Renders the SDK-owned serializable model classes reachable from exposed methods.
fn render_dart_models(objects: &[SdkObject], serializable_types: &HashMap<String, SerializableType>) -> String {
    let reachable = reachable_sdk_types(objects, serializable_types);
    let mut output = String::from("// GENERATED FILE. Source: operit-proxy-scan.\n\nimport 'dart:typed_data';\n\n");
    for name in reachable {
        let Some(ty) = serializable_types.get(&name) else { continue; };
        if let SerializableTypeKind::Struct { fields } = &ty.kind { output.push_str(&render_sdk_struct(ty, fields, serializable_types)); }
        if let SerializableTypeKind::Enum { variants, unit_only: true } = &ty.kind { output.push_str(&render_sdk_enum(ty, variants)); }
    }
    output
}

/// Computes recursively reachable serializable model names.
fn collect_reachable(ty: &str, all: &HashMap<String, SerializableType>, out: &mut BTreeSet<String>) {
    if let Some(inner) = ty.strip_prefix('&') { collect_reachable(inner, all, out); return; }
    for constructor in ["Option", "Vec", "BTreeMap", "std::collections::BTreeMap", "HashMap", "std::collections::HashMap"] {
        if let Some(args) = generic_args(ty, constructor) { for arg in args { collect_reachable(arg, all, out); } return; }
    }
    if !out.insert(ty.to_string()) { return; }
    if let Some(SerializableType { kind: SerializableTypeKind::Struct { fields }, .. }) = all.get(ty) { for field in fields { collect_reachable(&field.ty, all, out); } }
}

/// Renders one SDK struct model with MessagePack value decoding and encoding.
fn render_sdk_struct(ty: &SerializableType, fields: &[operit_rslink_codegen::SerializableField], all: &HashMap<String, SerializableType>) -> String {
    let name = dart_class_name(&ty.full_type); let mut out = format!("/// Generated SDK model for Rust type `{}`.\nclass {} {{\n", ty.full_type, name);
    out.push_str(&format!("  const {}({{\n", name)); for field in fields { out.push_str(&format!("    required this.{},\n", dart_field_name(&field.name))); } out.push_str("  });\n\n");
    out.push_str(&format!("  /// Decodes `{}` from a MessagePack value map.\n  factory {}.fromMessagePackValue(Map<String, Object?> value) => {}(\n", ty.full_type, name, name));
    for field in fields { let dt = dart_type(&field.ty, all); out.push_str(&format!("    {}: {},\n", dart_field_name(&field.name), dart_messagepack_decode(&format!("value['{}']", field.json_name), &dt))); } out.push_str("  );\n\n");
    out.push_str("  /// Encodes this model into a MessagePack-compatible value map.\n  Map<String, Object?> toMessagePackValue() => <String, Object?>{\n"); for field in fields { out.push_str(&format!("    '{}': {},\n", field.json_name, dart_messagepack_encode(&dart_field_name(&field.name), &dart_type(&field.ty, all)))); } out.push_str("  };\n\n");
    for field in fields { out.push_str(&format!("  final {} {};\n", dart_type(&field.ty, all), dart_field_name(&field.name))); } out.push_str("}\n\n"); out
}

/// Renders one unit enum model.
fn render_sdk_enum(ty: &SerializableType, variants: &[operit_rslink_codegen::SerializableEnumVariant]) -> String {
    let name = dart_class_name(&ty.full_type); let mut out = format!("enum {} {{\n", name); for variant in variants { out.push_str(&format!("  {},\n", dart_field_name(&variant.name))); } out.push_str("  ;\n\n");
    out.push_str(&format!("  /// Decodes `{}` from its MessagePack scalar value.\n  factory {}.fromMessagePackValue(Object? value) => switch (value) {{\n", ty.full_type, name));
    for variant in variants { out.push_str(&format!("    '{}' => {}.{},\n", variant.json_name, name, dart_field_name(&variant.name))); }
    out.push_str(&format!("    _ => throw ArgumentError('Unknown {}: $value'),\n  }};\n\n", name));
    out.push_str("  /// Encodes this enum into a MessagePack scalar value.\n  String toMessagePackValue() => switch (this) {\n");
    for variant in variants { out.push_str(&format!("    {}.{} => '{}',\n", name, dart_field_name(&variant.name), variant.json_name)); }
    out.push_str("  };\n}\n\n"); out
}

/// Converts a Rust field name into a Dart identifier.
fn dart_field_name(name: &str) -> String { name.trim_start_matches("r#").replace('-', "_") }

/// Renders a MessagePack-decoded value conversion expression.
fn dart_messagepack_decode(value: &str, dart: &str) -> String { if dart == "Object?" { return value.to_string(); } if dart == "Uint8List" { return format!("Uint8List.fromList((({value} as List).cast<int>()))"); } if dart == "int" { return format!("({value} as num).toInt()"); } if dart == "double" { return format!("({value} as num).toDouble()"); } if dart == "bool" || dart == "String" { return format!("{value} as {dart}"); } if let Some(inner) = dart.strip_suffix('?') { return format!("{value} == null ? null : {}", dart_messagepack_decode(value, inner)); } if let Some(inner) = dart.strip_prefix("List<").and_then(|v| v.strip_suffix('>')) { return format!("({value} as List<Object?>).map((item) => {}).toList(growable: false)", dart_messagepack_decode("item", inner)); } if let Some(args) = generic_args(dart, "Map") { if args.len() == 2 { return format!("({value} as Map).map((key, item) => MapEntry({}, {}))", dart_messagepack_decode("key", args[0]), dart_messagepack_decode("item", args[1])); } } format!("{}.fromMessagePackValue({} as Map<String, Object?>)", dart, value) }

/// Renders a MessagePack-compatible value encoding expression.
fn dart_messagepack_encode(value: &str, dart: &str) -> String {
    if matches!(dart, "Object?" | "bool" | "String" | "int" | "double") { return value.to_string(); }
    if let Some(inner) = dart.strip_suffix('?') { return format!("{value} == null ? null : {}", dart_messagepack_encode(&format!("{value}!"), inner)); }
    if dart == "Uint8List" { return format!("{value}.toList(growable: false)"); }
    if let Some(inner) = dart.strip_prefix("List<").and_then(|v| v.strip_suffix('>')) { return format!("{value}.map((item) => {}).toList(growable: false)", dart_messagepack_encode("item", inner)); }
    if let Some(args) = generic_args(dart, "Map") { assert_eq!(args.len(), 2); return format!("{value}.map((key, item) => MapEntry(key, {}))", dart_messagepack_encode("item", args[1])); }
    format!("{value}.toMessagePackValue()")
}

/// Renders Kotlin wrappers over the package's concrete IPC client.
fn render_kotlin(objects: &[SdkObject], serializable_types: &HashMap<String, SerializableType>) -> String {
    let mut output = String::from(
        "// GENERATED FILE. Source: operit-proxy-scan.\n\npackage operit.plugin.sdk\n\n",
    );
    for object in objects {
        output.push_str(&format!(
            "/** Generated client for Core object `{}`. */\nclass {}(private val client: OperitPluginSdkClient) {{\n",
            object.schema_key, object.class_name
        ));
        for method in &object.methods {
            let method_name = method_name(&method.name);
            let (parameters, arguments) = sdk_parameters(method, serializable_types, SdkLanguage::Kotlin);
            match method.mode {
                SdkMode::Call => {
                    let return_type = method.return_type.as_deref().map(|value| kotlin_type(value, serializable_types)).unwrap_or_else(|| "Unit".to_string());
                    output.push_str(&format!("    /** Calls `{}` through the Core Link route. */\n    suspend fun {}({parameters}): {} = client.callTyped({}, \"{}\", {arguments}, ::decode{})\n", method.name, method_name, return_type, format!("{:?}", object.object_id), method.name, kotlin_decoder_name(&return_type)));
                },
                SdkMode::Watch => output.push_str(&format!(
                    "    /** Watches `{}` through the Core Link route. */\n    fun {}({parameters}): kotlinx.coroutines.flow.Flow<{}> = client.watchTyped({}, \"{}\", {arguments}, ::decode{})\n",
                    method.name, method_name, method.return_type.as_deref().map(|value| kotlin_type(value, serializable_types)).unwrap_or_else(|| "Any?".to_string()), format!("{:?}", object.object_id), method.name, method.return_type.as_deref().map(|value| kotlin_decoder_name(&kotlin_type(value, serializable_types))).unwrap_or_else(|| "Any".to_string())
                )),
                SdkMode::Push => output.push_str(&format!(
                    "    /** Opens the caller-owned `{}` Core input stream. */\n    suspend fun {}({parameters}): OperitPluginSdkPushSink = client.push({}, \"{}\", {arguments})\n",
                    method.name, method_name, format!("{:?}", object.object_id), method.name
                )),
            }
        }
        output.push_str("}\n\n");
    }
    output
}

/// Renders TypeScript wrappers over the package's concrete IPC client.
fn render_typescript(
    objects: &[SdkObject],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let mut output = String::from(
        "// GENERATED FILE. Source: operit-proxy-scan.\n\nimport { OperitPluginSdkClient, OperitPluginSdkEvent, OperitPluginSdkPushSink } from './client.js';\nimport * as models from './models.js';\n\n",
    );
    for object in objects {
        output.push_str(&format!(
            "/** Generated client for Core object `{}`. */\nexport class {} {{\n  public constructor(private readonly client: OperitPluginSdkClient) {{}}\n",
            object.schema_key, object.class_name
        ));
        for method in &object.methods {
            let method_name = method_name(&method.name);
            let (parameters, arguments) = sdk_parameters(method, serializable_types, SdkLanguage::TypeScript);
            match method.mode {
                SdkMode::Call => {
                    let return_type = method.return_type.as_deref().map(|value| ts_public_type(value, serializable_types)).unwrap_or_else(|| "void".to_string());
                    if return_type == "void" {
                        output.push_str(&format!("  /** Calls `{}` through the Core Link route. */\n  public {}({parameters}): Promise<void> {{ return this.client.call({}, '{}', {arguments}).then(() => undefined); }}\n", method.name, method_name, format!("{:?}", object.object_id), method.name));
                    } else {
                        let decode_type = method.return_type.as_deref().map(|value| ts_type(value, serializable_types)).unwrap_or_else(|| "unknown".to_string());
                        output.push_str(&format!("  /** Calls `{}` through the Core Link route. */\n  public {}({parameters}): Promise<{}> {{ return this.client.callTyped<{}>({}, '{}', {arguments}, {}); }}\n", method.name, method_name, return_type, return_type, format!("{:?}", object.object_id), method.name, ts_decoder_expr(&decode_type)));
                    }
                },
                SdkMode::Watch => output.push_str(&format!(
                    "  /** Watches `{}` through the Core Link route. */\n  public {}({parameters}): AsyncIterable<{}> {{ return this.client.watchTyped<{}>({}, '{}', {arguments}, {}); }}\n",
                    method.name, method_name, method.return_type.as_deref().map(|value| ts_public_type(value, serializable_types)).unwrap_or_else(|| "unknown".to_string()), method.return_type.as_deref().map(|value| ts_public_type(value, serializable_types)).unwrap_or_else(|| "unknown".to_string()), format!("{:?}", object.object_id), method.name, method.return_type.as_deref().map(|value| ts_decoder_expr(&ts_type(value, serializable_types))).unwrap_or_else(|| "(value) => value".to_string())
                )),
                SdkMode::Push => output.push_str(&format!(
                    "  /** Opens the caller-owned `{}` Core input stream. */\n  public {}({parameters}): Promise<OperitPluginSdkPushSink> {{ return this.client.push({}, '{}', {arguments}); }}\n",
                    method.name, method_name, format!("{:?}", object.object_id), method.name
                )),
            }
        }
        output.push_str("}\n\n");
    }
    output
}

/// Maps one Rust protocol type to a TypeScript type.
fn ts_type(ty: &str, serializable_types: &HashMap<String, SerializableType>) -> String {
    let mapped = dart_type(ty, serializable_types);
    mapped.replace("Uint8List", "__BYTES__").replace("Object?", "unknown").replace("bool", "boolean").replace("int", "number").replace("double", "number").replace("String", "string").replace("?", " | null").replace("List<", "Array<").replace("Map<", "Record<").replace("__BYTES__", "Uint8Array")
}

/// Qualifies generated model types from the TypeScript models module.
fn ts_public_type(ty: &str, serializable_types: &HashMap<String, SerializableType>) -> String {
    let mut result = ts_type(ty, serializable_types);
    for name in serializable_types.keys().map(|value| dart_class_name(value)).collect::<BTreeSet<_>>() {
        result = replace_type_identifier(&result, &name, &format!("models.{name}"));
    }
    result
}

/// Replaces a whole TypeScript identifier without touching longer names or imports.
fn replace_type_identifier(value: &str, name: &str, replacement: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let name_chars: Vec<char> = name.chars().collect();
    let mut result = String::with_capacity(value.len());
    let mut index = 0;
    while index < chars.len() {
        let boundary_before = index == 0
            || (!chars[index - 1].is_alphanumeric() && chars[index - 1] != '_' && chars[index - 1] != '.');
        let end = index + name_chars.len();
        let boundary_after = end <= chars.len()
            && (end == chars.len()
                || (!chars[end].is_alphanumeric() && chars[end] != '_'));
        if boundary_before && boundary_after && chars[index..end] == name_chars[..] {
            result.push_str(replacement);
            index = end;
        } else {
            result.push(chars[index]);
            index += 1;
        }
    }
    result
}

/// Returns a generated TypeScript decoder expression for one type.
fn ts_decoder_expr(ty: &str) -> String {
    format!("(value) => {}", ts_decode_body("value", ty))
}

/// Produces a TypeScript expression that restores one nested MessagePack value.
fn ts_decode_body(value: &str, ty: &str) -> String {
    if ty == "Uint8Array" { return format!("new Uint8Array({value} as number[])"); }
    if ty == "boolean" || ty == "string" || ty == "number" || ty == "unknown" { return format!("{value} as {ty}"); }
    if let Some(inner) = ty.strip_suffix(" | null") { return format!("{value} == null ? null : {}", ts_decode_body(value, inner)); }
    if let Some(inner) = ty.strip_prefix("Array<").and_then(|v| v.strip_suffix('>')) { return format!("({value} as unknown[]).map((item) => {})", ts_decode_body("item", inner)); }
    if let Some(inner) = ty.strip_prefix("Record<").and_then(|v| v.strip_suffix('>')) { let map_type = format!("Map<{inner}>"); let args = generic_args(&map_type, "Map"); if let Some(args) = args { if args.len() == 2 { return format!("Object.fromEntries(Object.entries({value} as Record<string, unknown>).map(([key, item]) => [key, {}]))", ts_decode_body("item", args[1])); } } return value.to_string(); }
    format!("models.decode{}({value})", ty.replace(['<', '>', ',', ' '], ""))
}

/// Renders SDK-owned TypeScript models and MessagePack value decoders.
fn render_typescript_models(objects: &[SdkObject], serializable_types: &HashMap<String, SerializableType>) -> String {
    let reachable = reachable_sdk_types(objects, serializable_types);
    let mut output = String::from("// GENERATED FILE. Source: operit-proxy-scan.\n\n");
    for name in reachable {
        let Some(ty) = serializable_types.get(&name) else { continue; };
        if let SerializableTypeKind::Struct { fields } = &ty.kind {
            let class_name = dart_class_name(&ty.full_type);
            output.push_str(&format!("export interface {} {{\n", class_name));
            for field in fields { output.push_str(&format!("  readonly {}: {};\n", dart_field_name(&field.name), ts_type(&field.ty, serializable_types))); }
            output.push_str("}\n\n");
            output.push_str(&format!("export function decode{}(value: unknown): {} {{\n  const input = value as Record<string, unknown>;\n  return {{\n", class_name, class_name));
            for field in fields { output.push_str(&format!("    {}: {} as {},\n", dart_field_name(&field.name), ts_decode_expr(&format!("input['{}']", field.json_name), &ts_type(&field.ty, serializable_types)), ts_type(&field.ty, serializable_types))); }
            output.push_str("  };\n}\n\n");
            output.push_str(&format!("/** Encodes a typed SDK model into its Link argument representation. */\nexport function encode{}(value: {}): Record<string, unknown> {{\n  return {{\n", class_name, class_name));
            for field in fields {
                output.push_str(&format!("    '{}': {},\n", field.json_name, ts_encode_expr(&format!("value.{}", dart_field_name(&field.name)), &ts_type(&field.ty, serializable_types)).replace("models.", "")));
            }
            output.push_str("  };\n}\n\n");
        }
        if let SerializableTypeKind::Enum { variants, unit_only: true } = &ty.kind {
            let class_name = dart_class_name(&ty.full_type);
            output.push_str(&format!("export type {} = {};\n", class_name, variants.iter().map(|variant| format!("'{}'", variant.json_name)).collect::<Vec<_>>().join(" | ")));
            output.push_str(&format!("export function decode{}(value: unknown): {} {{ return value as {}; }}\n\n", class_name, class_name, class_name));
            output.push_str(&format!("/** Encodes the declared Link enum scalar. */\nexport function encode{}(value: {}): string {{ return value; }}\n\n", class_name, class_name));
        }
    }
    output
}

/// Produces a TypeScript value decoder expression for one concrete type.
fn ts_decode_expr(value: &str, ty: &str) -> String {
    if ty == "Uint8Array" { return format!("new Uint8Array({value} as number[])"); }
    if ty == "boolean" || ty == "string" || ty == "number" || ty == "unknown" { return value.to_string(); }
    if let Some(inner) = ty.strip_suffix(" | null") { return format!("{} == null ? null : {}", value, ts_decode_expr(value, inner)); }
    if let Some(inner) = ty.strip_prefix("Array<").and_then(|v| v.strip_suffix('>')) { return format!("({value} as unknown[]).map((item) => {})", ts_decode_expr("item", inner)); }
    if ty.starts_with("Record<") { return value.to_string(); }
    format!("decode{}({value})", ty)
}

/// Renders SDK-owned Kotlin models and MessagePack value decoders.
fn render_kotlin_models(objects: &[SdkObject], serializable_types: &HashMap<String, SerializableType>) -> String {
    let reachable = reachable_sdk_types(objects, serializable_types);
    let mut output = String::from("// GENERATED FILE. Source: operit-proxy-scan.\n\npackage operit.plugin.sdk\n\nfun decodeBoolean(value: Any?): Boolean = value as Boolean\nfun decodeString(value: Any?): String = value as String\nfun decodeInt(value: Any?): Int = (value as Number).toInt()\nfun decodeDouble(value: Any?): Double = (value as Number).toDouble()\nfun decodeUnit(value: Any?): Unit = Unit\n\n");
    let mut decoder_types = BTreeSet::new();
    for object in objects { for method in &object.methods { if let Some(value) = &method.return_type { decoder_types.insert(kotlin_type(value, serializable_types)); } } }
    for ty in decoder_types {
        let name = kotlin_decoder_name(&ty);
        if let Some(inner) = ty.strip_suffix('?') {
            output.push_str(&format!("fun decode{}(value: Any?): {}? = value?.let {{ decode{}(it) }}\n", name, inner, kotlin_decoder_name(inner)));
        } else if let Some(inner) = ty.strip_prefix("List<").and_then(|v| v.strip_suffix('>')) {
            output.push_str(&format!("fun decode{}(value: Any?): List<{}> = (value as List<*>).map {{ decode{}(it) }}\n", name, inner, kotlin_decoder_name(inner)));
        } else if ty.starts_with("Map<") {
            output.push_str(&format!("/** Decodes a typed SDK map from its Link representation. */\nfun decode{}(value: Any?): {} = {}\n", name, ty, kotlin_decode_expr("value", &ty)));
        }
    }
    output.push('\n');
    for name in reachable {
        let Some(ty) = serializable_types.get(&name) else { continue; };
        if let SerializableTypeKind::Struct { fields } = &ty.kind {
            let class_name = dart_class_name(&ty.full_type);
            output.push_str("data class "); output.push_str(&class_name); output.push_str("(\n");
            for (index, field) in fields.iter().enumerate() { output.push_str(&format!("    val {}: {}{}\n", dart_field_name(&field.name), kotlin_type(&field.ty, serializable_types), if index + 1 == fields.len() { "" } else { "," })); }
            output.push_str(")\n\n");
            output.push_str(&format!("fun decode{}(value: Any?): {} {{\n    val input = value as Map<*, *>\n    return {}(\n", class_name, class_name, class_name));
            for field in fields { output.push_str(&format!("        {} = {} as {},\n", dart_field_name(&field.name), kotlin_decode_expr(&format!("input[\"{}\"]", field.json_name), &kotlin_type(&field.ty, serializable_types)), kotlin_type(&field.ty, serializable_types))); }
            output.push_str("    )\n}\n\n");
            output.push_str(&format!("/** Encodes a typed SDK model into its Link argument representation. */\nfun {}.toMessagePackValue(): Map<String, Any?> = mapOf(\n", class_name));
            for field in fields {
                output.push_str(&format!("    \"{}\" to {},\n", field.json_name, kotlin_encode_expr(&format!("this.{}", dart_field_name(&field.name)), &kotlin_type(&field.ty, serializable_types))));
            }
            output.push_str(")\n\n");
        }
        if let SerializableTypeKind::Enum { variants, unit_only: true } = &ty.kind {
            let class_name = dart_class_name(&ty.full_type);
            output.push_str(&format!("enum class {} {{ {} }}\n", class_name, variants.iter().map(|variant| dart_field_name(&variant.name)).collect::<Vec<_>>().join(", ")));
            output.push_str(&format!("fun decode{}(value: Any?): {} = {}.valueOf(value.toString())\n\n", class_name, class_name, class_name));
            output.push_str(&format!("/** Encodes the declared Link enum scalar. */\nfun {}.toMessagePackValue(): String = when (this) {{\n", class_name));
            for variant in variants { output.push_str(&format!("    {}.{} -> \"{}\"\n", class_name, dart_field_name(&variant.name), variant.json_name)); }
            output.push_str("}\n\n");
        }
    }
    output
}

/// Maps one Rust protocol type to a Kotlin type.
fn kotlin_type(ty: &str, serializable_types: &HashMap<String, SerializableType>) -> String {
    let mapped = dart_type(ty, serializable_types);
    mapped.replace("Uint8List", "ByteArray").replace("Object?", "Any?").replace("bool", "Boolean").replace("int", "Int").replace("double", "Double")
}

/// Returns a stable Kotlin decoder function name.
fn kotlin_decoder_name(ty: &str) -> String { let nullable = ty.ends_with('?'); let base = ty.trim_end_matches('?'); let name = if base == "Boolean" { "Boolean".to_string() } else if base == "String" { "String".to_string() } else if base == "Int" { "Int".to_string() } else if base == "Double" { "Double".to_string() } else if base == "Unit" { "Unit".to_string() } else { base.replace(['<', '>', ',', ' '], "") }; if nullable { format!("Nullable{name}") } else { name } }

/// Produces a Kotlin value decoder expression for one generated type.
fn kotlin_decode_expr(value: &str, ty: &str) -> String {
    if ty == "Any?" { return value.to_string(); }
    if let Some(inner) = ty.strip_suffix('?') { return format!("{value}?.let {{ {} }}", kotlin_decode_expr("it", inner)); }
    if ty == "Int" { return format!("({value} as Number).toInt()"); }
    if ty == "Double" { return format!("({value} as Number).toDouble()"); }
    if matches!(ty, "Boolean" | "String") { return format!("{value} as {ty}"); }
    if ty == "ByteArray" { return format!("({value} as List<*>).map {{ (it as Number).toByte() }}.toByteArray()"); }
    if let Some(inner) = ty.strip_prefix("List<").and_then(|v| v.strip_suffix('>')) {
        return format!("({value} as List<*>).map {{ item -> {} }}", kotlin_decode_expr("item", inner));
    }
    if let Some(args) = generic_args(ty, "Map") {
        if args.len() == 2 {
            return format!("({value} as Map<*, *>).entries.associate {{ entry -> {} to {} }}", kotlin_decode_expr("entry.key", args[0]), kotlin_decode_expr("entry.value", args[1]));
        }
    }
    format!("decode{}({value})", kotlin_decoder_name(ty))
}

/// Collects serializable types reachable from the exposed SDK methods.
fn reachable_sdk_types(objects: &[SdkObject], all: &HashMap<String, SerializableType>) -> BTreeSet<String> {
    let mut reachable = BTreeSet::new();
    for object in objects { for method in &object.methods {
        if let Some(ty) = &method.return_type { collect_reachable(ty, all, &mut reachable); }
        for arg in &method.args { collect_reachable(&arg.ty, all, &mut reachable); }
    } }
    reachable
}

#[cfg(test)]
mod parameter_tests {
    use super::*;

    /// Builds a route with real string and boolean input metadata.
    fn objects() -> Vec<SdkObject> {
        vec![SdkObject { object_id: "core/application.packageManager".to_string(), schema_key: "application.packageManager".to_string(), class_name: "PackageManagerClient".to_string(), methods: vec![SdkMethod {
            name: "getToolPkgContainerDetails".to_string(), mode: SdkMode::Call, return_type: Some("String".to_string()),
            args: vec![SourceArg { name: "packageName".to_string(), ty: "&str".to_string() }, SourceArg { name: "useEnglish".to_string(), ty: "bool".to_string() }],
        }] }]
    }

    /// Ensures each public language client retains required parameter types and Link keys.
    #[test]
    fn all_languages_preserve_route_parameters() {
        let objects = objects();
        let models = HashMap::new();
        let dart = render_dart(&objects, &models);
        for source in [render_rust(&objects, &models), render_dart(&objects, &models), render_kotlin(&objects, &models), render_typescript(&objects, &models)] {
            assert!(source.contains("\"core/application.packageManager\""));
        }
        assert!(dart.contains("getToolPkgContainerDetails({required String packageName, required bool useEnglish})"));
        assert!(dart.contains("'packageName': packageName, 'useEnglish': useEnglish"));
        assert!(render_kotlin(&objects, &models).contains("getToolPkgContainerDetails(packageName: String, useEnglish: Boolean)"));
        assert!(render_typescript(&objects, &models).contains("getToolPkgContainerDetails(packageName: string, useEnglish: boolean)"));
        assert!(render_rust(&objects, &models).contains("getToolPkgContainerDetails(&self, packageName: &str, useEnglish: bool)"));
    }

    /// Ensures input-only DTOs are generated and explicitly serialized as argument values.
    #[test]
    fn input_only_models_are_reachable() {
        let mut objects = objects();
        objects[0].methods[0].args = vec![SourceArg { name: "options".to_string(), ty: "dto::Options".to_string() }];
        let models = HashMap::from([("dto::Options".to_string(), SerializableType {
            full_type: "dto::Options".to_string(), supports_serialize: true, supports_deserialize: true,
            kind: SerializableTypeKind::Struct { fields: vec![operit_rslink_codegen::SerializableField { name: "enabled".to_string(), json_name: "is_enabled".to_string(), ty: "bool".to_string(), has_serde_default: false }] },
        })]);
        assert!(render_dart(&objects, &models).contains("required Options options"));
        assert!(render_dart_models(&objects, &models).contains("class Options"));
        assert!(render_dart(&objects, &models).contains("options.toMessagePackValue()"));
        assert!(render_typescript_models(&objects, &models).contains("'is_enabled': value.enabled"));
        assert!(render_kotlin_models(&objects, &models).contains("\"is_enabled\" to this.enabled"));
    }

    /// Prevents a short DTO name from being replaced inside a longer DTO name.
    #[test]
    fn typescript_model_names_qualify_as_whole_identifiers() {
        let model = |full_type: &str| SerializableType {
            full_type: full_type.to_string(),
            supports_serialize: true,
            supports_deserialize: true,
            kind: SerializableTypeKind::Struct { fields: Vec::new() },
        };
        let models = HashMap::from([
            ("dto::Snapshot".to_string(), model("dto::Snapshot")),
            ("dto::ExtensionCatalogSnapshot".to_string(), model("dto::ExtensionCatalogSnapshot")),
        ]);
        assert_eq!(
            ts_public_type("dto::ExtensionCatalogSnapshot", &models),
            "models.ExtensionCatalogSnapshot"
        );
    }
}
