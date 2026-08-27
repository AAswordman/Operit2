use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use operit_rslink_codegen::{
    core_stream_inner, generic_args, single_generic_arg, split_top_level_args, CallProtocol,
    FactoryProtocol, MethodProtocol, ObjectAccess, ReverseStreamProtocol, SerializableEnumVariant,
    SerializableField, SerializableType, SerializableTypeKind, SourceArg, SourceMethod,
    SourceObject, WatchStreamProtocol,
};

pub fn dart_default_value(dart_ty: &str) -> &'static str {
    match dart_ty {
        "String" => "''",
        "int" => "0",
        "double" => "0.0",
        "bool" => "false",
        "Uint8List" => "Uint8List(0)",
        _ if dart_ty.starts_with("List<") => "const []",
        _ if dart_ty.starts_with("Map<") => "const {}",
        _ if dart_ty.ends_with('?') => "null",
        _ => "null",
    }
}

/// Writes generated Flutter Dart proxy models and clients from one proxy scan result.
pub fn write_dart_proxy_artifacts(
    manifest_dir: &Path,
    objects: &[SourceObject],
    serializable_types: &HashMap<String, SerializableType>,
) {
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("proxy/local must live under core/crates/proxy");
    let dart_dir = repo_root.join("apps/flutter/app/lib/core/proxy/generated");
    fs::create_dir_all(&dart_dir).expect("create generated dart proxy directory");
    write_generated_file(
        &dart_dir.join("CoreProxyModels.g.dart"),
        &render_dart_models(objects, serializable_types),
    );
    write_generated_file(
        &dart_dir.join("CoreProxyClients.g.dart"),
        &render_dart_clients(objects, serializable_types),
    );
}

/// Writes generated content only when its bytes differ from the existing file.
fn write_generated_file(path: &Path, contents: &str) {
    if fs::read(path).is_ok_and(|current| current == contents.as_bytes()) {
        return;
    }

    fs::write(path, contents)
        .unwrap_or_else(|error| panic!("write generated file {}: {error}", path.display()));
}

fn render_dart_models(
    objects: &[SourceObject],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let mut reachable = reachable_serializable_types(objects, serializable_types);
    include_source_tree_model_types(&mut reachable, serializable_types);
    let mut types = reachable
        .iter()
        .filter_map(|name| serializable_types.get(name))
        .collect::<Vec<_>>();
    types.sort_by(|left, right| left.full_type.cmp(&right.full_type));

    let mut output = generated_header();
    output.push_str("import 'dart:async';\nimport 'dart:typed_data';\n\n");
    output.push_str("import '../../link/CoreLinkCodec.dart';\n");
    output.push_str("import '../../link/CoreLinkProtocol.dart';\n\n");
    output.push_str(&render_core_proxy_error_details());
    for ty in types {
        match &ty.kind {
            SerializableTypeKind::Struct { fields } => {
                output.push_str(&render_dart_struct(ty, fields, serializable_types));
            }
            SerializableTypeKind::Enum {
                variants,
                unit_only: true,
            } => {
                output.push_str(&render_dart_enum(ty, variants, serializable_types));
            }
            SerializableTypeKind::TaggedEnum {
                externally_tagged,
                tag_name,
                content_name,
                variants,
            } => {
                output.push_str(&render_dart_tagged_enum(
                    ty,
                    variants,
                    *externally_tagged,
                    tag_name.as_deref(),
                    content_name.as_deref(),
                    serializable_types,
                ));
            }
            SerializableTypeKind::Enum {
                unit_only: false, ..
            } => {}
        }
    }
    output
}

fn render_core_proxy_error_details() -> String {
    let mut output = String::new();
    output.push_str("class CoreProxyErrorDetails {\n");
    output.push_str("  const CoreProxyErrorDetails({\n");
    output.push_str("    required this.errorType,\n");
    output.push_str("    required this.message,\n");
    output.push_str("    this.variant,\n");
    output.push_str("    this.kind,\n");
    output.push_str("    this.httpStatus,\n");
    output.push_str("    this.remoteMessage,\n");
    output.push_str("    this.fields = const <String, Object?>{},\n");
    output.push_str("  });\n\n");
    output.push_str("  factory CoreProxyErrorDetails.fromCoreLinkError(CoreLinkError error) {\n");
    output.push_str("    final details = error.details;\n");
    output.push_str("    if (details is Map<String, Object?>) {\n");
    output.push_str(
        "      return CoreProxyErrorDetails.fromJson(details, message: error.message);\n",
    );
    output.push_str("    }\n");
    output.push_str(
        "    return CoreProxyErrorDetails(errorType: error.code, message: error.message);\n",
    );
    output.push_str("  }\n\n");
    output.push_str("  factory CoreProxyErrorDetails.fromJson(Map<String, Object?> json, {String? message}) {\n");
    output.push_str("    final classification = json['classification'];\n");
    output.push_str("    final fields = json['fields'];\n");
    output.push_str("    return CoreProxyErrorDetails(\n");
    output.push_str("      errorType: _stringValue(json['errorType']) ?? 'unknown',\n");
    output.push_str("      message: _stringValue(json['message']) ?? message ?? '',\n");
    output.push_str("      variant: _stringValue(json['variant']),\n");
    output.push_str("      kind: classification is Map<String, Object?> ? _stringValue(classification['kind']) : _stringValue(json['kind']),\n");
    output.push_str("      httpStatus: _intValue(json['httpStatus']),\n");
    output.push_str("      remoteMessage: _stringValue(json['remoteMessage']),\n");
    output.push_str(
        "      fields: fields is Map<String, Object?> ? fields : const <String, Object?>{},\n",
    );
    output.push_str("    );\n");
    output.push_str("  }\n\n");
    output.push_str("  final String errorType;\n");
    output.push_str("  final String message;\n");
    output.push_str("  final String? variant;\n");
    output.push_str("  final String? kind;\n");
    output.push_str("  final int? httpStatus;\n");
    output.push_str("  final String? remoteMessage;\n");
    output.push_str("  final Map<String, Object?> fields;\n\n");
    output.push_str("  String? stringField(String name) => _stringValue(fields[name]);\n\n");
    output.push_str("  static String? _stringValue(Object? value) {\n");
    output.push_str("    if (value is String && value.trim().isNotEmpty) {\n");
    output.push_str("      return value.trim();\n");
    output.push_str("    }\n");
    output.push_str("    return null;\n");
    output.push_str("  }\n\n");
    output.push_str("  static int? _intValue(Object? value) {\n");
    output.push_str("    if (value is int) {\n");
    output.push_str("      return value;\n");
    output.push_str("    }\n");
    output.push_str("    return null;\n");
    output.push_str("  }\n");
    output.push_str("}\n\n");
    output
}

fn render_dart_clients(
    objects: &[SourceObject],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let mut output = generated_header();
    output.push_str("import 'dart:async';\nimport 'dart:typed_data';\n\n");
    output.push_str("import '../../bridge/OperitRuntimeBridge.dart';\n");
    output.push_str("import '../../link/CoreLinkCodec.dart';\n");
    output.push_str("import '../../link/CoreLinkProtocol.dart';\n");
    output.push_str("import 'CoreProxyModels.g.dart';\n\n");
    output.push_str(
        "String _coreProxyRequestId() => 'flutter-${DateTime.now().microsecondsSinceEpoch}';\nMap<String, Object?> _coreProxyArgs(Map<String, Object?> args, Map<String, Object?> objectArgs) => <String, Object?>{...objectArgs, ...args};\n\n",
    );
    output.push_str("class GeneratedCoreProxyClients {\n");
    output.push_str("  const GeneratedCoreProxyClients(this.bridge);\n\n");
    output.push_str("  final OperitRuntimeBridge bridge;\n\n");
    let namespace_prefixes = dart_namespace_prefixes(objects);
    for object in objects
        .iter()
        .filter(|object| !matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. }))
    {
        let getter_name = dart_schema_getter_name(&object.schema_key);
        let class_name = dart_proxy_class_name(&object.schema_key);
        output.push_str(&format!(
            "  /// Returns a generated proxy client for `{}`.\n",
            object.schema_key
        ));
        output.push_str(&format!(
            "  {class_name} get {getter_name} => {class_name}._(bridge, {});\n",
            object.object_id
        ));
        if object.schema_key == "repository.memoryRepository" {
            output.push_str(&format!(
                "  /// Returns the generated proxy for one memory owner.\n  {class_name} repositoryMemoryRepositoryForOwner(String ownerKey) => {class_name}._(bridge, {}, objectArgs: <String, Object?>{{'__core_instance_id': ownerKey}});\n",
                object.object_id
            ));
        }
    }
    for namespace in namespace_prefixes.iter().filter(|path| {
        !path.contains('.') && !objects.iter().any(|object| object.schema_key == **path)
    }) {
        let getter_name = dart_identifier(namespace);
        let class_name = dart_namespace_class_name(namespace);
        output.push_str(&format!(
            "  /// Returns the generated proxy namespace for `{namespace}`.\n  {class_name} get {getter_name} => {class_name}._(bridge);\n"
        ));
    }
    output.push_str("}\n\n");

    for namespace in &namespace_prefixes {
        output.push_str(&render_dart_namespace_class(
            namespace,
            &namespace_prefixes,
            objects,
        ));
    }

    for object in objects {
        output.push_str(&render_dart_client_class(
            objects,
            object,
            serializable_types,
        ));
    }
    output
}

/// Returns every dotted schema prefix that should be exposed as a Dart namespace.
fn dart_namespace_prefixes(objects: &[SourceObject]) -> std::collections::BTreeSet<String> {
    let mut prefixes = std::collections::BTreeSet::new();
    for object in objects {
        let mut parts = object.schema_key.split('.');
        let Some(first) = parts.next() else {
            continue;
        };
        let mut prefix = first.to_string();
        for part in parts {
            prefixes.insert(prefix.clone());
            prefix.push('.');
            prefix.push_str(part);
        }
    }
    prefixes
}

/// Renders one generated namespace that mirrors dotted schema segments.
fn render_dart_namespace_class(
    namespace: &str,
    prefixes: &std::collections::BTreeSet<String>,
    objects: &[SourceObject],
) -> String {
    let class_name = dart_namespace_class_name(namespace);
    let mut children = std::collections::BTreeSet::new();
    let prefix = format!("{namespace}.");
    for path in prefixes {
        if let Some(rest) = path.strip_prefix(&prefix) {
            if let Some(child) = rest.split('.').next() {
                children.insert(child.to_string());
            }
        }
    }
    for object in objects {
        if let Some(rest) = object.schema_key.strip_prefix(&prefix) {
            if let Some(child) = rest.split('.').next() {
                children.insert(child.to_string());
            }
        }
    }

    let mut output = String::new();
    output.push_str(&format!("class {class_name} {{\n"));
    output.push_str(&format!("  const {class_name}._(this.bridge);\n\n"));
    output.push_str("  final OperitRuntimeBridge bridge;\n\n");
    for child in children {
        let child_path = format!("{namespace}.{child}");
        if prefixes.contains(&child_path)
            && !objects.iter().any(|object| object.schema_key == child_path)
        {
            let child_class = dart_namespace_class_name(&child_path);
            let child_getter = dart_identifier(&child);
            output.push_str(&format!(
                "  /// Returns the generated proxy namespace for `{child_path}`.\n  {child_class} get {child_getter} => {child_class}._(bridge);\n"
            ));
            continue;
        }
        if let Some(object) = objects
            .iter()
            .find(|object| object.schema_key == child_path)
        {
            if matches!(object.access, ObjectAccess::FactoryMethodConstruct { .. }) {
                continue;
            }
            let getter_name = dart_identifier(&child);
            let proxy_class = dart_proxy_class_name(&object.schema_key);
            output.push_str(&format!(
                "  /// Returns a generated proxy client for `{child_path}`.\n  {proxy_class} get {getter_name} => {proxy_class}._(bridge, {});\n",
                object.object_id
            ));
        }
    }
    output.push_str("}\n\n");
    output
}

/// Includes serializable models owned by the independent Link and Core server source trees.
fn include_source_tree_model_types(
    reachable: &mut HashSet<String>,
    serializable_types: &HashMap<String, SerializableType>,
) {
    for type_name in serializable_types.keys() {
        let crate_name = type_name.split("::").next().unwrap_or_default();
        if !matches!(crate_name, "operit_access_runtime" | "operit_node_runtime") {
            continue;
        }
        collect_reachable_type(type_name, serializable_types, reachable);
    }
}

fn render_dart_client_class(
    objects: &[SourceObject],
    object: &SourceObject,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let class_name = dart_proxy_class_name(&object.schema_key);
    let mut output = String::new();
    output.push_str(&format!("class {class_name} {{\n"));
    output.push_str(&format!(
        "  const {class_name}._(this.bridge, this.objectId, {{this.objectArgs = const <String, Object?>{{}}}});\n\n"
    ));
    output.push_str("  final OperitRuntimeBridge bridge;\n\n");
    output.push_str("  final int objectId;\n\n");
    output.push_str("  final Map<String, Object?> objectArgs;\n\n");
    for method in &object.methods {
        if method.factory_protocol().is_some() {
            output.push_str(&render_dart_factory_method(
                objects,
                object,
                method,
                serializable_types,
            ));
        }
        if method.call_protocol().is_some() {
            output.push_str(&render_dart_call_method(object, method, serializable_types));
        }
        if method.watch_protocol().is_some() {
            output.push_str(&render_dart_watch_method(
                object,
                method,
                serializable_types,
            ));
        }
        if method.reverse_stream_protocol().is_some() {
            output.push_str(&render_dart_reverse_stream_method(
                object,
                method,
                serializable_types,
            ));
        }
    }
    if object.schema_key == "application" {
        output.push_str(
            "  /// Runs one CLI-style Core command through the generated application proxy.\n  Future<Object?> runCoreCommand({required List<String> args}) {\n    return bridge.callApplication('runCoreCommand', args: <String, Object?>{'args': args});\n  }\n\n",
        );
    }
    output.push_str("}\n\n");
    output
}

/// Renders one generated caller-to-runtime reverse stream method.
fn render_dart_reverse_stream_method(
    _object: &SourceObject,
    method: &SourceMethod,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let protocol = method
        .reverse_stream_protocol()
        .expect("reverse stream protocol");
    let item_type = dart_type(&protocol.item_type, serializable_types);
    let params = method
        .args
        .iter()
        .map(|argument| {
            if argument.name == protocol.argument_name {
                format!(
                    "required Stream<{item_type}> {}",
                    dart_identifier(&argument.name)
                )
            } else {
                format!(
                    "required {} {}",
                    dart_type(&argument.ty, serializable_types),
                    dart_identifier(&argument.name)
                )
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let open_args = method
        .args
        .iter()
        .filter(|argument| argument.name != protocol.argument_name)
        .map(|argument| {
            format!(
                "'{}': {}",
                argument.name,
                dart_encode_expr(
                    &dart_identifier(&argument.name),
                    &dart_type(&argument.ty, serializable_types),
                    serializable_types
                )
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let method_name = dart_identifier(&method.name);
    let input_name = dart_identifier(&protocol.argument_name);
    let mut output = render_dart_doc_comments(method, "  ");
    output.push_str(&format!(
        "  Future<void> {method_name}({{{params}}}) async {{\n"
    ));
    output.push_str("    final sink = await bridge.push(\n");
    output.push_str("      CorePushRequest(\n");
    output.push_str("        requestId: _coreProxyRequestId(),\n");
    output.push_str("        targetObjectId: objectId,\n");
    output.push_str(&format!("        methodName: '{}',\n", method.name));
    output.push_str(&format!(
        "        args: _coreProxyArgs(<String, Object?>{{{open_args}}}, objectArgs),\n"
    ));
    output.push_str("      ),\n    );\n");
    output.push_str("    try {\n");
    output.push_str(&format!(
        "      await for (final item in {input_name}) {{\n"
    ));
    let encoded_item = dart_encode_expr("item", &item_type, serializable_types);
    output.push_str(&format!("        await sink.add({encoded_item});\n"));
    output.push_str("      }\n");
    output.push_str("    } finally {\n      await sink.close();\n    }\n  }\n\n");
    output
}

fn render_dart_call_method(
    object: &SourceObject,
    method: &SourceMethod,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let return_type = match method.call_protocol().expect("call protocol") {
        CallProtocol::Unit | CallProtocol::ResultUnit { .. } => "void".to_string(),
        CallProtocol::Value(ty) => dart_type(ty, serializable_types),
        CallProtocol::ResultValue { value_type, .. } => dart_type(value_type, serializable_types),
    };
    let params = render_dart_params(&method.args, serializable_types);
    let args = render_dart_args_map(&method.args, serializable_types);
    let mut output = String::new();
    let method_name = dart_identifier(&method.name);
    output.push_str(&render_dart_doc_comments(method, "  "));
    output.push_str(&format!(
        "  Future<{return_type}> {method_name}({params}) async {{\n"
    ));
    let bridge_method = if control_call_registry()
        .iter()
        .any(|(schema_key, method_name)| {
            *schema_key == object.schema_key && *method_name == method.name
        }) {
        "callControlBytes"
    } else {
        "callBytes"
    };
    output.push_str(&format!(
        "    final responseBytes = await bridge.{bridge_method}(\n"
    ));
    output.push_str("      CoreCallRequest(\n");
    output.push_str("        requestId: _coreProxyRequestId(),\n");
    output.push_str("        targetObjectId: objectId,\n");
    output.push_str(&format!("        methodName: '{}',\n", method.name));
    output.push_str(&format!(
        "        args: _coreProxyArgs({args}, objectArgs),\n"
    ));
    output.push_str("      ),\n");
    output.push_str("    );\n");
    if return_type == "void" {
        output.push_str("    decodeNativeCoreVoidResult(responseBytes);\n");
    } else {
        output.push_str(&format!(
            "    return decodeNativeCoreResult<{}>(responseBytes, decode: (reader) => {}, targetObjectId: objectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream);\n",
            return_type,
            dart_message_pack_decode_expr("reader", &return_type, serializable_types)
        ));
    }
    output.push_str("  }\n\n");
    output
}

/// Returns the generated Core methods that must bypass serialized runtime work.
fn control_call_registry() -> &'static [(&'static str, &'static str)] {
    &[(
        "services.runtimeHostInteractionService",
        "respondOwnerHostInteraction",
    )]
}

fn render_dart_factory_method(
    objects: &[SourceObject],
    object: &SourceObject,
    method: &SourceMethod,
    _serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let factory = method.factory_protocol().expect("factory protocol");
    let class_name = dart_proxy_class_name(&factory.target_schema_key);
    let params = method
        .args
        .iter()
        .map(|arg| format!("required String {}", dart_identifier(&arg.name)))
        .collect::<Vec<_>>()
        .join(", ");
    let params = if params.is_empty() {
        String::new()
    } else {
        format!("{{{params}}}")
    };
    let constructor_args = method
        .args
        .iter()
        .enumerate()
        .map(|(index, arg)| {
            format!(
                "'__core_factory_arg_{index}': {}",
                dart_identifier(&arg.name)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let factory_method_name = dart_identifier(&method.name);
    let object_args_expr = if constructor_args.is_empty() {
        "const <String, Object?>{}".to_string()
    } else {
        format!("<String, Object?>{{{constructor_args}}}")
    };
    let target_object_id = objects
        .iter()
        .find(|candidate| candidate.schema_key == factory.target_schema_key)
        .expect("factory target object must be generated")
        .object_id;
    let mut output = render_dart_doc_comments(method, "  ");
    output.push_str(&format!(
        "  {class_name} {factory_method_name}({params}) {{\n    return {class_name}._(bridge, {target_object_id}, objectArgs: {object_args_expr});\n  }}\n\n"
    ));
    output
}

fn render_dart_watch_method(
    _object: &SourceObject,
    method: &SourceMethod,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let watch = method.watch_protocol().expect("watch protocol");
    let value_type = dart_type(&watch.item_type, serializable_types);
    let params = render_dart_params(&method.args, serializable_types);
    let args = render_dart_args_map(&method.args, serializable_types);
    let mut output = String::new();
    let method_name = dart_identifier(&method.name);
    output.push_str(&render_dart_doc_comments(method, "  "));
    output.push_str(&format!(
        "  Stream<{value_type}> {method_name}({params}) {{\n"
    ));
    output.push_str("    final eventValueDecoder = CoreLinkEventValueDecoder();\n");
    output.push_str("    return bridge\n");
    output.push_str(&format!(
        "        .watchStream(CoreWatchRequest(requestId: _coreProxyRequestId(), targetObjectId: objectId, propertyName: '{}', args: _coreProxyArgs({args}, objectArgs)))\n",
        method.name
    ));
    output.push_str("        .where((event) => event.kind != 'Completed')\n");
    output.push_str("        .map((event) {\n");
    output.push_str(&format!(
            "          return eventValueDecoder.decode<{}>(event, decode: (valueBytes) => decodeCoreLink<{}>(valueBytes, decode: (reader) => {}, targetObjectId: event.targetObjectId, embeddedStreamFactory: bridge.openEmbeddedCoreStream));\n",
        value_type,
        value_type,
        dart_message_pack_decode_expr("reader", &value_type, serializable_types)
    ));
    output.push_str("        });\n");
    output.push_str("  }\n\n");
    output
}

fn render_dart_struct(
    ty: &SerializableType,
    fields: &[SerializableField],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let class_name = dart_class_name(&ty.full_type, serializable_types);
    let mut output = String::new();
    output.push_str(&format!(
        "/// Generated Dart model for Rust type `{}`.\n",
        ty.full_type
    ));
    output.push_str(&format!("class {class_name} {{\n"));
    if fields.is_empty() {
        output.push_str(&format!("  const {class_name}();\n\n"));
    } else {
        output.push_str(&format!("  const {class_name}({{\n"));
        for field in fields {
            output.push_str(&format!(
                "    required this.{},\n",
                dart_identifier(&field.name)
            ));
        }
        output.push_str("  });\n\n");
    }
    output.push_str(&format!(
        "  factory {class_name}.fromJson(Map<String, Object?> json) {{\n"
    ));
    if fields.is_empty() {
        output.push_str(&format!("    return {class_name}();\n"));
    } else {
        output.push_str(&format!("    return {class_name}(\n"));
        for field in fields {
            let field_type = dart_type(&field.ty, serializable_types);
            output.push_str(&format!(
                "      {}: {},\n",
                dart_identifier(&field.name),
                dart_decode_expr(
                    &format!("json['{}']", field.json_name),
                    &field_type,
                    serializable_types
                )
            ));
        }
        output.push_str("    );\n");
    }
    output.push_str("  }\n\n");
    output.push_str(&render_dart_message_pack_struct_decoder(
        &class_name,
        fields,
        serializable_types,
    ));
    output.push_str("  Map<String, Object?> toJson() {\n");
    output.push_str("    return <String, Object?>{\n");
    for field in fields {
        let field_type = dart_type(&field.ty, serializable_types);
        output.push_str(&format!(
            "      '{}': {},\n",
            field.json_name,
            dart_encode_expr(
                &dart_identifier(&field.name),
                &field_type,
                serializable_types
            )
        ));
    }
    output.push_str("    };\n");
    output.push_str("  }\n\n");
    for field in fields {
        output.push_str(&format!(
            "  /// Rust field `{}` serialized as `{}`.\n",
            field.name, field.json_name
        ));
        output.push_str(&format!(
            "  final {} {};\n",
            dart_type(&field.ty, serializable_types),
            dart_identifier(&field.name)
        ));
    }
    output.push_str("}\n\n");
    output
}

/// Renders direct MessagePack decoding for one generated struct.
fn render_dart_message_pack_struct_decoder(
    class_name: &str,
    fields: &[SerializableField],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "  factory {class_name}.fromMessagePack(CoreLinkValueReader reader) {{\n"
    ));
    output.push_str("    final fieldCount = reader.readMapLength();\n");
    for field in fields {
        let field_type = dart_type(&field.ty, serializable_types);
        let field_name = dart_identifier(&field.name);
        output.push_str(&format!("    late {field_type} {field_name};\n"));
        output.push_str(&format!("    var has_{field_name} = false;\n"));
    }
    output.push_str("    for (var index = 0; index < fieldCount; index += 1) {\n");
    output.push_str("      switch (reader.readString()) {\n");
    for field in fields {
        let field_type = dart_type(&field.ty, serializable_types);
        let field_name = dart_identifier(&field.name);
        output.push_str(&format!(
            "        case '{}':\n          {field_name} = {};\n          has_{field_name} = true;\n          break;\n",
            dart_string_literal(&field.json_name),
            dart_message_pack_decode_expr("reader", &field_type, serializable_types),
        ));
    }
    output.push_str("        default:\n          reader.skipValue();\n      }\n    }\n");
    for field in fields {
        let field_name = dart_identifier(&field.name);
        output.push_str(&format!(
            "    if (!has_{field_name}) {{\n      throw FormatException('Missing {class_name}.{}');\n    }}\n",
            dart_string_literal(&field.json_name),
        ));
    }
    if fields.is_empty() {
        output.push_str(&format!("    return {class_name}();\n"));
    } else {
        output.push_str(&format!("    return {class_name}(\n"));
        for field in fields {
            let field_name = dart_identifier(&field.name);
            output.push_str(&format!("      {field_name}: {field_name},\n"));
        }
        output.push_str("    );\n");
    }
    output.push_str("  }\n\n");
    output
}

fn render_dart_enum(
    ty: &SerializableType,
    variants: &[SerializableEnumVariant],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let enum_name = dart_class_name(&ty.full_type, serializable_types);
    let mut output = String::new();
    output.push_str(&format!(
        "/// Generated Dart enum for Rust type `{}`.\n",
        ty.full_type
    ));
    output.push_str(&format!("enum {enum_name} {{\n"));
    for variant in variants {
        output.push_str(&format!(
            "  {}('{}'),\n",
            dart_identifier(&variant.name),
            dart_string_literal(&variant.json_name)
        ));
    }
    output.push_str(&format!(
        "  ;\n\n  const {enum_name}(this.value);\n\n  final String value;\n\n"
    ));
    output.push_str(&format!(
        "  factory {enum_name}.fromJson(Object? value) {{\n"
    ));
    output.push_str("    return switch (value) {\n");
    for variant in variants {
        output.push_str(&format!(
            "      '{}' => {}.{},\n",
            dart_string_literal(&variant.json_name),
            enum_name,
            dart_identifier(&variant.name)
        ));
    }
    output.push_str(&format!(
        "      _ => throw ArgumentError('Unknown {enum_name}: $value'),\n"
    ));
    output.push_str("    };\n  }\n\n");
    output.push_str(&format!(
        "  factory {enum_name}.fromMessagePack(CoreLinkValueReader reader) => {enum_name}.fromJson(reader.readString());\n\n"
    ));
    output.push_str("  String toJson() => value;\n");
    output.push_str("}\n\n");
    output
}

fn render_dart_tagged_enum(
    ty: &SerializableType,
    variants: &[SerializableEnumVariant],
    externally_tagged: bool,
    tag_name: Option<&str>,
    content_name: Option<&str>,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let enum_name = dart_class_name(&ty.full_type, serializable_types);
    let mut output = String::new();
    output.push_str(&format!(
        "/// Generated Dart tagged enum model for Rust type `{}`.\n",
        ty.full_type
    ));
    output.push_str(&format!("class {enum_name} {{\n"));
    output.push_str(&format!("  const {enum_name}._({{\n"));
    let mut seen_fields: Vec<String> = Vec::new();
    for variant in variants {
        for field in &variant.fields {
            let name = dart_identifier(&field.name);
            if !seen_fields.contains(&name) {
                let field_type =
                    dart_tagged_enum_field_type(&field.name, variants, serializable_types);
                let default_val = dart_default_value(&field_type);
                output.push_str(&format!("    this.{} = {},\n", name, default_val));
                seen_fields.push(name);
            }
        }
    }
    output.push_str("    required this.tag,\n");
    output.push_str("  });\n\n");
    for variant in variants {
        let variant_name = dart_identifier(&variant.name);
        if variant.fields.is_empty() {
            output.push_str(&format!(
                "  factory {enum_name}.{variant_name}() => {enum_name}._(tag: '{}');\n",
                dart_string_literal(&variant.json_name)
            ));
        } else {
            let mut factory_params = String::from("{");
            let mut forward_args = String::new();
            for field in &variant.fields {
                let name = dart_identifier(&field.name);
                let field_type = dart_type(&field.ty, serializable_types);
                factory_params.push_str(&format!("required {} {}, ", field_type, name));
                if !forward_args.is_empty() {
                    forward_args.push_str(", ");
                }
                forward_args.push_str(&format!("{}: {}", name, name));
            }
            factory_params.push_str("}");
            output.push_str(&format!(
                "  factory {enum_name}.{variant_name}({}) => {enum_name}._(tag: '{}', {});\n",
                factory_params,
                dart_string_literal(&variant.json_name),
                forward_args
            ));
        }
    }
    output.push_str("  final String tag;\n");
    let mut seen_field_decls: Vec<String> = Vec::new();
    for variant in variants {
        for field in &variant.fields {
            let name = dart_identifier(&field.name);
            if !seen_field_decls.contains(&name) {
                let field_type =
                    dart_tagged_enum_field_type(&field.name, variants, serializable_types);
                output.push_str(&format!("  final {} {};\n", field_type, name));
                seen_field_decls.push(name);
            }
        }
    }
    let has_external_unit_variants =
        externally_tagged && variants.iter().any(|variant| variant.fields_are_unit);
    if has_external_unit_variants {
        output.push_str(&format!(
            "\n  factory {enum_name}.fromJson(Object? json) {{\n"
        ));
        output.push_str("    switch (json) {\n");
        for variant in variants.iter().filter(|variant| variant.fields_are_unit) {
            output.push_str(&format!(
                "      case '{}':\n        return {enum_name}.{}();\n",
                dart_string_literal(&variant.json_name),
                dart_identifier(&variant.name)
            ));
        }
        output.push_str("      case Map<String, Object?> map:\n");
        output.push_str("        final tag = map.keys.single;\n");
        output.push_str("        final data = map[tag] as Map<String, Object?>;\n");
        output.push_str("        return switch (tag) {\n");
        for variant in variants.iter().filter(|variant| !variant.fields_are_unit) {
            let variant_name = dart_identifier(&variant.name);
            output.push_str(&format!(
                "          '{}' => {enum_name}.{variant_name}(",
                dart_string_literal(&variant.json_name)
            ));
            for field in &variant.fields {
                output.push_str(&format!(
                    "{}: {}, ",
                    dart_identifier(&field.name),
                    dart_decode_expr(
                        &format!("data['{}']", field.json_name),
                        &dart_type(&field.ty, serializable_types),
                        serializable_types
                    )
                ));
            }
            output.push_str("),\n");
        }
        output.push_str(&format!(
            "          _ => throw ArgumentError('Unknown {enum_name} tag: $tag'),\n"
        ));
        output.push_str("        };\n");
        output.push_str(&format!(
            "      default:\n        throw ArgumentError('Unknown {enum_name} representation: $json');\n"
        ));
        output.push_str("    }\n  }\n\n");
        output.push_str("  Object toJson() {\n");
        output.push_str("    return switch (tag) {\n");
        for variant in variants {
            if variant.fields_are_unit {
                output.push_str(&format!(
                    "      '{}' => '{}',\n",
                    dart_string_literal(&variant.json_name),
                    dart_string_literal(&variant.json_name)
                ));
            } else {
                output.push_str(&format!(
                    "      '{}' => <String, Object?>{{'{}': <String, Object?>{{\n",
                    dart_string_literal(&variant.json_name),
                    dart_string_literal(&variant.json_name)
                ));
                for field in &variant.fields {
                    let field_type = dart_type(&field.ty, serializable_types);
                    output.push_str(&format!(
                        "        '{}': {},\n",
                        field.json_name,
                        dart_encode_expr(
                            &dart_tagged_enum_encode_value(
                                &field.name,
                                &field_type,
                                variants,
                                serializable_types
                            ),
                            &field_type,
                            serializable_types,
                        )
                    ));
                }
                output.push_str("      }},\n");
            }
        }
        output.push_str(&format!(
            "      _ => throw StateError('Unknown {enum_name} tag: $tag'),\n"
        ));
        output.push_str("    };\n  }\n");
    } else {
        output.push_str(&format!(
            "\n  factory {enum_name}.fromJson(Object? json) {{\n"
        ));
        output.push_str(&format!("    final map = json as Map<String, Object?>;\n"));
        // externally tagged: {{\"CharacterCard\": {{\"id\": \"...\"}}}}
        output.push_str("    final tag = map.keys.first;\n");
        output.push_str(
            "    final data = map[tag] as Map<String, Object?>? ?? <String, Object?>{};\n",
        );
        output.push_str("    return switch (tag) {\n");
        for variant in variants {
            let variant_name = dart_identifier(&variant.name);
            output.push_str(&format!(
                "      '{}' => {enum_name}.{variant_name}(",
                dart_string_literal(&variant.json_name)
            ));
            for field in &variant.fields {
                output.push_str(&format!(
                    "{}: {}, ",
                    dart_identifier(&field.name),
                    dart_decode_expr(
                        &format!("data['{}']", field.json_name),
                        &dart_type(&field.ty, serializable_types),
                        serializable_types
                    )
                ));
            }
            output.push_str("),\n");
        }
        output.push_str(&format!(
            "      _ => throw ArgumentError('Unknown {enum_name} tag: $tag'),\n"
        ));
        output.push_str("    };\n  }\n\n");
        output.push_str("  Map<String, Object?> toJson() {\n");
        output.push_str("    final data = <String, Object?>{\n");
        for variant in variants {
            output.push_str(&format!(
                "      if (tag == '{}') ...<String, Object?>{{\n",
                dart_string_literal(&variant.json_name)
            ));
            for field in &variant.fields {
                let field_type = dart_type(&field.ty, serializable_types);
                output.push_str(&format!(
                    "        '{}': {},\n",
                    field.json_name,
                    dart_encode_expr(
                        &dart_tagged_enum_encode_value(
                            &field.name,
                            &field_type,
                            variants,
                            serializable_types
                        ),
                        &field_type,
                        serializable_types,
                    )
                ));
            }
            output.push_str("      },\n");
        }
        output.push_str("    };\n");
        output.push_str("    return <String, Object?>{tag: data};\n");
        output.push_str("  }\n");
    }
    output.push_str(&render_dart_tagged_enum_message_pack_decoder(
        &enum_name,
        variants,
        externally_tagged,
        tag_name,
        content_name,
        serializable_types,
    ));
    output.push_str("}\n\n");
    output
}

/// Renders direct MessagePack decoding for one tagged enum.
fn render_dart_tagged_enum_message_pack_decoder(
    enum_name: &str,
    variants: &[SerializableEnumVariant],
    externally_tagged: bool,
    tag_name: Option<&str>,
    content_name: Option<&str>,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let mut output =
        format!("\n  factory {enum_name}.fromMessagePack(CoreLinkValueReader reader) {{\n");
    if externally_tagged {
        output.push_str("    if (reader.isNextString()) {\n");
        output.push_str("      final tag = reader.readString();\n");
        output.push_str("      return switch (tag) {\n");
        for variant in variants.iter().filter(|variant| variant.fields_are_unit) {
            output.push_str(&format!(
                "        '{}' => {enum_name}.{}(),\n",
                dart_string_literal(&variant.json_name),
                dart_identifier(&variant.name),
            ));
        }
        output.push_str(&format!(
            "        _ => throw ArgumentError('Unknown {enum_name} tag: $tag'),\n"
        ));
        output.push_str("      };\n    }\n");
        output.push_str("    final entryCount = reader.readMapLength();\n");
        output.push_str(&format!(
            "    if (entryCount != 1) {{ throw FormatException('External {enum_name} payload must contain one entry'); }}\n"
        ));
        output.push_str("    final tag = reader.readString();\n");
        output.push_str("    return switch (tag) {\n");
        for variant in variants.iter().filter(|variant| !variant.fields_are_unit) {
            output.push_str(&format!(
                "      '{}' => {},\n",
                dart_string_literal(&variant.json_name),
                dart_tagged_enum_variant_payload(
                    "reader",
                    enum_name,
                    variant,
                    serializable_types,
                    None,
                ),
            ));
        }
        output.push_str(&format!(
            "      _ => throw ArgumentError('Unknown {enum_name} tag: $tag'),\n"
        ));
        output.push_str("    };\n  }\n\n");
        return output;
    }

    let tag_name = tag_name.expect("tagged enum tag metadata is missing");
    output.push_str("    final fieldCount = reader.readMapLength();\n");
    output.push_str(&format!(
        "    if (fieldCount < 1) {{ throw FormatException('Tagged {enum_name} payload is empty'); }}\n"
    ));
    output.push_str("    final tagKey = reader.readString();\n");
    output.push_str(&format!(
        "    if (tagKey != '{}') {{ throw FormatException('Tagged {enum_name} tag key is invalid'); }}\n",
        dart_string_literal(tag_name),
    ));
    output.push_str("    final tag = reader.readString();\n");
    if let Some(content_name) = content_name {
        output.push_str("    if (fieldCount == 1) {\n");
        output.push_str("      return switch (tag) {\n");
        for variant in variants.iter().filter(|variant| variant.fields_are_unit) {
            output.push_str(&format!(
                "        '{}' => {enum_name}.{}(),\n",
                dart_string_literal(&variant.json_name),
                dart_identifier(&variant.name),
            ));
        }
        output.push_str(&format!(
            "        _ => throw ArgumentError('Unknown {enum_name} tag: $tag'),\n"
        ));
        output.push_str("      };\n    }\n");
        output.push_str(&format!(
            "    if (fieldCount != 2) {{ throw FormatException('Adjacent {enum_name} payload must contain tag and content'); }}\n"
        ));
        output.push_str("    final contentKey = reader.readString();\n");
        output.push_str(&format!(
            "    if (contentKey != '{}') {{ throw FormatException('Adjacent {enum_name} content key is invalid'); }}\n",
            dart_string_literal(content_name),
        ));
        output.push_str("    return switch (tag) {\n");
        for variant in variants.iter().filter(|variant| !variant.fields_are_unit) {
            output.push_str(&format!(
                "      '{}' => {},\n",
                dart_string_literal(&variant.json_name),
                dart_tagged_enum_variant_payload(
                    "reader",
                    enum_name,
                    variant,
                    serializable_types,
                    None,
                ),
            ));
        }
        output.push_str(&format!(
            "      _ => throw ArgumentError('Unknown {enum_name} tag: $tag'),\n"
        ));
        output.push_str("    };\n  }\n\n");
        return output;
    }
    output.push_str("    return switch (tag) {\n");
    for variant in variants {
        output.push_str(&format!(
            "      '{}' => {},\n",
            dart_string_literal(&variant.json_name),
            dart_tagged_enum_variant_payload(
                "reader",
                enum_name,
                variant,
                serializable_types,
                Some("fieldCount - 1"),
            ),
        ));
    }
    output.push_str(&format!(
        "      _ => throw ArgumentError('Unknown {enum_name} tag: $tag'),\n"
    ));
    output.push_str("    };\n  }\n\n");
    output
}

/// Renders direct decoding of one tagged enum variant payload.
fn dart_tagged_enum_variant_payload(
    reader: &str,
    enum_name: &str,
    variant: &SerializableEnumVariant,
    serializable_types: &HashMap<String, SerializableType>,
    known_field_count: Option<&str>,
) -> String {
    let variant_name = dart_identifier(&variant.name);
    if variant.fields_are_unit {
        if let Some(field_count) = known_field_count {
            return format!(
                "(() {{ for (var index = 0; index < {field_count}; index += 1) {{ {reader}.readString(); {reader}.skipValue(); }} return {enum_name}.{variant_name}(); }})()"
            );
        }
        return format!("{enum_name}.{variant_name}()");
    }
    if !variant.fields_are_named {
        if let Some(field_count) = known_field_count {
            if variant.fields.len() == 1 {
                let field = &variant.fields[0];
                if let Some(SerializableType {
                    kind:
                        SerializableTypeKind::Struct {
                            fields: nested_fields,
                        },
                    ..
                }) = serializable_types.get(&field.ty)
                {
                    let nested_class = dart_class_name(&field.ty, serializable_types);
                    let mut output = String::from("(() { ");
                    for nested_field in nested_fields {
                        let nested_name = dart_identifier(&nested_field.name);
                        let nested_type = dart_type(&nested_field.ty, serializable_types);
                        output.push_str(&format!(
                            "late {nested_type} {nested_name}; var has_{nested_name} = false; "
                        ));
                    }
                    output.push_str(&format!(
                        "for (var index = 0; index < {field_count}; index += 1) {{"
                    ));
                    output.push_str(&format!("switch ({reader}.readString()) {{"));
                    for nested_field in nested_fields {
                        let nested_name = dart_identifier(&nested_field.name);
                        let nested_type = dart_type(&nested_field.ty, serializable_types);
                        output.push_str(&format!(
                            "case '{}': {nested_name} = {}; has_{nested_name} = true; break;",
                            dart_string_literal(&nested_field.json_name),
                            dart_message_pack_decode_expr(reader, &nested_type, serializable_types),
                        ));
                    }
                    output.push_str(&format!("default: {reader}.skipValue(); }} }}"));
                    for nested_field in nested_fields {
                        let nested_name = dart_identifier(&nested_field.name);
                        output.push_str(&format!(
                            " if (!has_{nested_name}) {{ throw FormatException('Missing {nested_class}.{}'); }}",
                            dart_string_literal(&nested_field.json_name),
                        ));
                    }
                    output.push_str(&format!(
                        " return {enum_name}.{variant_name}(value: {nested_class}("
                    ));
                    for nested_field in nested_fields {
                        let nested_name = dart_identifier(&nested_field.name);
                        output.push_str(&format!("{nested_name}: {nested_name}, "));
                    }
                    output.push_str(")); })()");
                    return output;
                }
            }
            return format!(
                "throw StateError('Internally tagged tuple variant {enum_name}.{variant_name} requires a struct payload')"
            );
        }
        let mut output = format!("(() {{ final itemCount = {reader}.readArrayLength(); if (itemCount != {}) {{ throw FormatException('Invalid {enum_name}.{variant_name} item count'); }} return {enum_name}.{variant_name}(", variant.fields.len());
        for field in &variant.fields {
            let field_type = dart_type(&field.ty, serializable_types);
            output.push_str(&format!(
                "{}: {}, ",
                dart_identifier(&field.name),
                dart_message_pack_decode_expr(reader, &field_type, serializable_types)
            ));
        }
        output.push_str("); })()");
        return output;
    }
    let mut output = String::from("(() { ");
    for field in &variant.fields {
        let field_name = dart_identifier(&field.name);
        let field_type = dart_type(&field.ty, serializable_types);
        output.push_str(&format!(
            "late {field_type} {field_name}; var has_{field_name} = false; "
        ));
    }
    if let Some(field_count) = known_field_count {
        output.push_str(&format!(
            "for (var index = 0; index < {field_count}; index += 1) {{"
        ));
    } else {
        output.push_str(&format!("final fieldCount = {reader}.readMapLength(); for (var index = 0; index < fieldCount; index += 1) {{"));
    }
    output.push_str(&format!("switch ({reader}.readString()) {{"));
    for field in &variant.fields {
        let field_name = dart_identifier(&field.name);
        let field_type = dart_type(&field.ty, serializable_types);
        output.push_str(&format!(
            "case '{}': {field_name} = {}; has_{field_name} = true; break;",
            dart_string_literal(&field.json_name),
            dart_message_pack_decode_expr(reader, &field_type, serializable_types)
        ));
    }
    output.push_str(&format!("default: {reader}.skipValue(); }} }}"));
    for field in &variant.fields {
        let field_name = dart_identifier(&field.name);
        output.push_str(&format!(" if (!has_{field_name}) {{ throw FormatException('Missing {enum_name}.{variant_name}.{}'); }}", dart_string_literal(&field.json_name)));
    }
    output.push_str(&format!(" return {enum_name}.{variant_name}("));
    for field in &variant.fields {
        let field_name = dart_identifier(&field.name);
        output.push_str(&format!("{field_name}: {field_name}, "));
    }
    output.push_str("); })()");
    output
}

fn dart_tagged_enum_field_type(
    field_name: &str,
    variants: &[SerializableEnumVariant],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let mut field_types = variants
        .iter()
        .flat_map(|variant| variant.fields.iter())
        .filter(|field| field.name == field_name)
        .map(|field| dart_type(&field.ty, serializable_types))
        .collect::<Vec<_>>();
    field_types.sort();
    field_types.dedup();
    if field_types.len() == 1 {
        field_types.remove(0)
    } else {
        "Object?".to_string()
    }
}

fn dart_tagged_enum_encode_value(
    field_name: &str,
    field_type: &str,
    variants: &[SerializableEnumVariant],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let field_value = dart_identifier(field_name);
    if dart_tagged_enum_field_type(field_name, variants, serializable_types) == field_type {
        field_value
    } else {
        format!("({field_value} as {field_type})")
    }
}

fn render_dart_params(
    args: &[SourceArg],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    if args.is_empty() {
        return String::new();
    }
    let params = args
        .iter()
        .map(|arg| {
            format!(
                "required {} {}",
                dart_type(&arg.ty, serializable_types),
                dart_parameter_name(&arg.name)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{params}}}")
}

fn render_dart_args_map(
    args: &[SourceArg],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    if args.is_empty() {
        return "const <String, Object?>{}".to_string();
    }
    let entries = args
        .iter()
        .map(|arg| {
            let arg_type = dart_type(&arg.ty, serializable_types);
            format!(
                "'{}': {}",
                arg.name,
                dart_encode_expr(
                    &dart_parameter_name(&arg.name),
                    &arg_type,
                    serializable_types
                )
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("<String, Object?>{{{entries}}}")
}

fn reachable_serializable_types(
    objects: &[SourceObject],
    serializable_types: &HashMap<String, SerializableType>,
) -> HashSet<String> {
    let mut out = HashSet::new();
    for object in objects {
        for method in &object.methods {
            for arg in &method.args {
                collect_reachable_type(&arg.ty, serializable_types, &mut out);
            }
            match &method.protocol {
                MethodProtocol::Call(CallProtocol::Value(ty)) => {
                    collect_reachable_type(ty, serializable_types, &mut out);
                }
                MethodProtocol::Call(CallProtocol::ResultValue { value_type, .. }) => {
                    collect_reachable_type(value_type, serializable_types, &mut out);
                }
                MethodProtocol::Watch(watch) => {
                    if let Some(snapshot_type) = &watch.snapshot_type {
                        collect_reachable_type(snapshot_type, serializable_types, &mut out);
                    }
                    collect_reachable_type(&watch.item_type, serializable_types, &mut out);
                }
                _ => {}
            }
        }
    }
    out
}

fn collect_reachable_type(
    ty: &str,
    serializable_types: &HashMap<String, SerializableType>,
    out: &mut HashSet<String>,
) {
    if serializable_types.contains_key(ty) && out.insert(ty.to_string()) {
        if let Some(SerializableType { kind, .. }) = serializable_types.get(ty) {
            match kind {
                SerializableTypeKind::Struct { fields } => {
                    for field in fields {
                        collect_reachable_type(&field.ty, serializable_types, out);
                    }
                }
                SerializableTypeKind::TaggedEnum { variants, .. } => {
                    for variant in variants {
                        for field in &variant.fields {
                            collect_reachable_type(&field.ty, serializable_types, out);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(inner) = single_generic_arg(ty, "Option")
        .or_else(|| single_generic_arg(ty, "Vec"))
        .or_else(|| single_generic_arg(ty, "HashSet"))
        .or_else(|| single_generic_arg(ty, "std::collections::HashSet"))
    {
        collect_reachable_type(inner, serializable_types, out);
    }
    if let Some(inner) = core_stream_inner(ty) {
        collect_reachable_type(inner, serializable_types, out);
    }
    if let Some(args) = generic_args(ty, "HashMap")
        .or_else(|| generic_args(ty, "std::collections::HashMap"))
        .or_else(|| generic_args(ty, "BTreeMap"))
        .or_else(|| generic_args(ty, "std::collections::BTreeMap"))
    {
        for arg in args {
            collect_reachable_type(arg, serializable_types, out);
        }
    }
}

fn dart_type(ty: &str, serializable_types: &HashMap<String, SerializableType>) -> String {
    if ty == "Vec<u8>" {
        return "Uint8List".to_string();
    }
    if let Some(inner) = single_generic_arg(ty, "Option") {
        let inner_type = dart_type(inner, serializable_types);
        if inner_type.ends_with('?') {
            return inner_type;
        }
        return format!("{inner_type}?");
    }
    if let Some(inner) = core_stream_inner(ty) {
        return format!("Stream<{}>", dart_type(inner, serializable_types));
    }
    if let Some(inner) = single_generic_arg(ty, "Vec")
        .or_else(|| single_generic_arg(ty, "HashSet"))
        .or_else(|| single_generic_arg(ty, "std::collections::HashSet"))
    {
        return format!("List<{}>", dart_type(inner, serializable_types));
    }
    if let Some(args) = generic_args(ty, "HashMap")
        .or_else(|| generic_args(ty, "std::collections::HashMap"))
        .or_else(|| generic_args(ty, "BTreeMap"))
        .or_else(|| generic_args(ty, "std::collections::BTreeMap"))
    {
        if args.len() == 2 {
            return format!(
                "Map<{}, {}>",
                dart_type(args[0], serializable_types),
                dart_type(args[1], serializable_types)
            );
        }
    }
    match ty {
        "()" => "void".to_string(),
        "bool" => "bool".to_string(),
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
            "int".to_string()
        }
        "f32" | "f64" => "double".to_string(),
        "String" | "&str" => "String".to_string(),
        "serde_json::Value" => "Object?".to_string(),
        _ => match serializable_types.get(ty) {
            Some(SerializableType {
                kind: SerializableTypeKind::Struct { .. },
                ..
            }) => dart_class_name(ty, serializable_types),
            Some(SerializableType {
                kind:
                    SerializableTypeKind::Enum {
                        unit_only: true, ..
                    },
                ..
            }) => dart_class_name(ty, serializable_types),
            Some(SerializableType {
                kind: SerializableTypeKind::TaggedEnum { .. },
                ..
            }) => dart_class_name(ty, serializable_types),
            Some(SerializableType {
                kind:
                    SerializableTypeKind::Enum {
                        unit_only: false, ..
                    },
                ..
            }) => dart_class_name(ty, serializable_types),
            None => "Object?".to_string(),
        },
    }
}

/// Renders a direct MessagePack reader expression for one generated Dart type.
fn dart_message_pack_decode_expr(
    reader: &str,
    dart_type: &str,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    if dart_type == "Object?" {
        return format!("{reader}.readValue()");
    }
    if let Some(inner) = dart_type.strip_suffix('?') {
        return format!(
            "{reader}.readNullable<{}>(() => {})",
            inner,
            dart_message_pack_decode_expr(reader, inner, serializable_types)
        );
    }
    match dart_type {
        "void" => "null".to_string(),
        "bool" => format!("{reader}.readBool()"),
        "int" => format!("{reader}.readInt()"),
        "double" => format!("{reader}.readDouble()"),
        "String" => format!("{reader}.readString()"),
        "Uint8List" => format!("{reader}.readBytes()"),
        _ => {
            if let Some(inner) = list_inner(dart_type) {
                return format!(
                    "(() {{ final length = {reader}.readArrayLength(); return List<{}>.generate(length, (_) => {}, growable: false); }})()",
                    inner,
                    dart_message_pack_decode_expr(reader, inner, serializable_types)
                );
            }
            if let Some((key, value_type)) = map_inner(dart_type) {
                return format!(
                    "(() {{ final length = {reader}.readMapLength(); final result = <{}, {}>{{}}; for (var index = 0; index < length; index += 1) {{ result[{}] = {}; }} return result; }})()",
                    key,
                    value_type,
                    dart_message_pack_decode_expr(reader, key, serializable_types),
                    dart_message_pack_decode_expr(reader, value_type, serializable_types),
                );
            }
            if let Some(inner) = stream_inner(dart_type) {
                return format!(
                    "{reader}.readEmbeddedStream<{}>((item) => {})",
                    inner,
                    dart_message_pack_decode_expr("item", inner, serializable_types),
                );
            }
            if dart_is_unit_enum_type(dart_type, serializable_types)
                || dart_is_tagged_enum_type(dart_type, serializable_types)
            {
                return format!("{dart_type}.fromMessagePack({reader})");
            }
            format!("{dart_type}.fromMessagePack({reader})")
        }
    }
}

fn dart_decode_expr(
    value: &str,
    dart_type: &str,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    if dart_type == "Object?" {
        return value.to_string();
    }
    if let Some(inner) = dart_type.strip_suffix('?') {
        return format!(
            "{value} == null ? null : {}",
            dart_decode_expr(value, inner, serializable_types)
        );
    }
    if dart_type == "void" {
        return "null".to_string();
    }
    if matches!(dart_type, "bool" | "int" | "String") {
        return format!("{value} as {dart_type}");
    }
    if dart_type == "double" {
        return format!("({value} as num).toDouble()");
    }
    if dart_type == "Uint8List" {
        return format!("{value} as Uint8List");
    }
    if let Some(inner) = stream_inner(dart_type) {
        return format!(
            "({value} as Stream).map((item) => {})",
            dart_decode_expr("item", inner, serializable_types)
        );
    }
    if let Some(inner) = list_inner(dart_type) {
        return format!(
            "({value} as List<Object?>).map((item) => {}).toList(growable: false)",
            dart_decode_expr("item", inner, serializable_types)
        );
    }
    if let Some((key, value_type)) = map_inner(dart_type) {
        return format!(
            "({value} as Map<Object?, Object?>).map((key, value) => MapEntry({}, {}))",
            dart_decode_expr("key", key, serializable_types),
            dart_decode_expr("value", value_type, serializable_types)
        );
    }
    if dart_is_unit_enum_type(dart_type, serializable_types) {
        return format!("{dart_type}.fromJson({value})");
    }
    if dart_is_tagged_enum_type(dart_type, serializable_types) {
        return format!("{dart_type}.fromJson({value})");
    }
    format!("{dart_type}.fromJson({value} as Map<String, Object?>)")
}

fn dart_encode_expr(
    value: &str,
    dart_type: &str,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    if dart_type == "Object?" {
        return value.to_string();
    }
    if let Some(inner) = dart_type.strip_suffix('?') {
        if inner == "Object?"
            || matches!(
                inner,
                "bool" | "int" | "double" | "String" | "Uint8List" | "void"
            )
        {
            return value.to_string();
        }
        if stream_inner(inner).is_some() {
            return format!(
                "{value} == null ? null : (throw UnsupportedError('Embedded Core streams are read-only'))"
            );
        }
        if dart_is_unit_enum_type(inner, serializable_types) {
            return format!("{value}?.toJson()");
        }
        if let Some(list_inner) = list_inner(inner) {
            return format!(
                "{value}?.map((item) => {}).toList(growable: false)",
                dart_encode_expr("item", list_inner, serializable_types)
            );
        }
        if let Some((key, value_type)) = map_inner(inner) {
            return format!(
                "{value}?.map((key, value) => MapEntry({}, {}))",
                dart_encode_expr("key", key, serializable_types),
                dart_encode_expr("value", value_type, serializable_types)
            );
        }
        return format!("{value}?.toJson()");
    }
    if matches!(
        dart_type,
        "bool" | "int" | "double" | "String" | "Uint8List" | "void"
    ) {
        return value.to_string();
    }
    if stream_inner(dart_type).is_some() {
        return format!("throw UnsupportedError('Embedded Core streams are read-only')");
    }
    if let Some(inner) = list_inner(dart_type) {
        return format!(
            "{value}.map((item) => {}).toList(growable: false)",
            dart_encode_expr("item", inner, serializable_types)
        );
    }
    if let Some((key, value_type)) = map_inner(dart_type) {
        return format!(
            "{value}.map((key, value) => MapEntry({}, {}))",
            dart_encode_expr("key", key, serializable_types),
            dart_encode_expr("value", value_type, serializable_types)
        );
    }
    if dart_is_unit_enum_type(dart_type, serializable_types) {
        return format!("{value}.toJson()");
    }
    if dart_is_tagged_enum_type(dart_type, serializable_types) {
        return format!("{value}.toJson()");
    }
    format!("{value}.toJson()")
}

/// Returns the Dart item type for an embedded Core stream.
fn stream_inner(dart_type: &str) -> Option<&str> {
    dart_type
        .strip_prefix("Stream<")
        .and_then(|value| value.strip_suffix('>'))
}

fn dart_is_unit_enum_type(
    dart_type: &str,
    serializable_types: &HashMap<String, SerializableType>,
) -> bool {
    serializable_types.values().any(|ty| {
        matches!(
            &ty.kind,
            SerializableTypeKind::Enum {
                unit_only: true,
                ..
            }
        ) && dart_class_name(&ty.full_type, serializable_types) == dart_type
    })
}

fn dart_is_tagged_enum_type(
    dart_type: &str,
    serializable_types: &HashMap<String, SerializableType>,
) -> bool {
    serializable_types.values().any(|ty| {
        matches!(&ty.kind, SerializableTypeKind::TaggedEnum { .. })
            && dart_class_name(&ty.full_type, serializable_types) == dart_type
    })
}

fn list_inner(dart_type: &str) -> Option<&str> {
    dart_type
        .strip_prefix("List<")
        .and_then(|value| value.strip_suffix('>'))
}

fn map_inner(dart_type: &str) -> Option<(&str, &str)> {
    let inner = dart_type
        .strip_prefix("Map<")
        .and_then(|value| value.strip_suffix('>'))?;
    let args = split_top_level_args(inner);
    if args.len() == 2 {
        Some((args[0], args[1]))
    } else {
        None
    }
}

fn dart_class_name(
    full_type: &str,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let final_segment = full_type
        .rsplit("::")
        .next()
        .expect("full type must have a final segment")
        .split('<')
        .next()
        .expect("type segment must exist")
        .to_string();
    let duplicate_count = serializable_types
        .keys()
        .filter(|candidate| {
            candidate
                .rsplit("::")
                .next()
                .map(|segment| segment == final_segment)
                .unwrap_or(false)
        })
        .count();
    if duplicate_count <= 1 {
        return dart_type_name(&final_segment);
    }
    let mut out = String::from("Core");
    for part in full_type
        .strip_prefix("operit_runtime::")
        .unwrap_or(full_type)
        .split("::")
    {
        let type_part = dart_type_name(part);
        out.push_str(&type_part);
    }
    out
}

fn dart_proxy_class_name(schema_key: &str) -> String {
    let mut out = String::from("Generated");
    out.push_str(&upper_camel_from_words(&identifier_words(schema_key)));
    out.push_str("CoreProxy");
    out
}

/// Builds a stable Dart class name for one generated schema namespace.
fn dart_namespace_class_name(namespace: &str) -> String {
    let mut out = String::from("Generated");
    out.push_str(&upper_camel_from_words(&identifier_words(namespace)));
    out.push_str("CoreProxyNamespace");
    out
}

fn dart_parameter_name(name: &str) -> String {
    dart_identifier(name.trim_start_matches('_'))
}

fn dart_schema_getter_name(schema_key: &str) -> String {
    lower_camel_from_words(&identifier_words(schema_key))
}

fn dart_identifier(name: &str) -> String {
    let raw = name.trim_start_matches("r#");
    let mut out = lower_camel_from_words(&identifier_words(raw));
    if out.is_empty() {
        out.push_str("value");
    }
    if out
        .chars()
        .next()
        .map(|ch| ch.is_ascii_digit())
        .unwrap_or(false)
    {
        out.insert(0, '_');
    }
    if dart_reserved_word(&out) {
        out.push_str("Value");
    }
    out
}

fn dart_type_name(name: &str) -> String {
    let mut out = upper_camel_identifier(name.trim_start_matches("r#"));
    if out.is_empty() {
        out.push_str("Value");
    }
    if out
        .chars()
        .next()
        .map(|ch| ch.is_ascii_digit())
        .unwrap_or(false)
    {
        out.insert(0, 'T');
    }
    out
}

fn upper_camel_identifier(name: &str) -> String {
    upper_camel_from_words(&identifier_words(name))
}

fn identifier_words(name: &str) -> Vec<String> {
    let mut words = Vec::new();
    for segment in name.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        if segment.is_empty() {
            continue;
        }
        words.extend(split_identifier_segment(segment));
    }
    collapse_duplicate_words(merge_acronym_words(words))
}

fn split_identifier_segment(segment: &str) -> Vec<String> {
    let chars = segment.chars().collect::<Vec<_>>();
    let mut words = Vec::new();
    let mut start = 0usize;
    for index in 1..chars.len() {
        let previous = chars[index - 1];
        let current = chars[index];
        let next = chars.get(index + 1).copied();
        let lower_to_upper = previous.is_ascii_lowercase() && current.is_ascii_uppercase();
        let acronym_to_word = previous.is_ascii_uppercase()
            && current.is_ascii_uppercase()
            && next.map(|ch| ch.is_ascii_lowercase()).unwrap_or(false);
        let digit_boundary = previous.is_ascii_digit() != current.is_ascii_digit();
        if lower_to_upper || acronym_to_word || digit_boundary {
            words.push(chars[start..index].iter().collect::<String>());
            start = index;
        }
    }
    words.push(chars[start..].iter().collect::<String>());
    words
}

fn merge_acronym_words(words: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < words.len() {
        if index + 1 < words.len()
            && words[index].len() == 1
            && words[index].chars().all(|ch| ch.is_ascii_lowercase())
            && words[index + 1].chars().all(|ch| ch.is_ascii_uppercase())
        {
            out.push(format!(
                "{}{}",
                words[index].to_ascii_uppercase(),
                words[index + 1]
            ));
            index += 2;
        } else {
            out.push(words[index].clone());
            index += 1;
        }
    }
    out
}

fn collapse_duplicate_words(words: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for word in words {
        let duplicate = out
            .last()
            .map(|previous: &String| previous.eq_ignore_ascii_case(&word))
            .unwrap_or(false);
        if !duplicate {
            out.push(word);
        }
    }
    out
}

fn lower_camel_from_words(words: &[String]) -> String {
    let mut out = String::new();
    for (index, word) in words.iter().enumerate() {
        if index == 0 {
            out.push_str(&word.to_ascii_lowercase());
        } else {
            push_title_word(&mut out, word);
        }
    }
    out
}

fn upper_camel_from_words(words: &[String]) -> String {
    let mut out = String::new();
    for word in words {
        push_title_word(&mut out, word);
    }
    out
}

fn push_title_word(out: &mut String, word: &str) {
    let lower = word.to_ascii_lowercase();
    let mut chars = lower.chars();
    if let Some(first) = chars.next() {
        out.push(first.to_ascii_uppercase());
        out.extend(chars);
    }
}

fn dart_reserved_word(value: &str) -> bool {
    matches!(
        value,
        "abstract"
            | "as"
            | "assert"
            | "async"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "covariant"
            | "default"
            | "deferred"
            | "do"
            | "dynamic"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "extension"
            | "external"
            | "factory"
            | "false"
            | "final"
            | "finally"
            | "for"
            | "Function"
            | "get"
            | "hide"
            | "if"
            | "implements"
            | "import"
            | "in"
            | "interface"
            | "is"
            | "late"
            | "library"
            | "mixin"
            | "new"
            | "null"
            | "on"
            | "operator"
            | "part"
            | "required"
            | "rethrow"
            | "return"
            | "sealed"
            | "set"
            | "show"
            | "static"
            | "super"
            | "switch"
            | "sync"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typedef"
            | "var"
            | "void"
            | "when"
            | "while"
            | "with"
            | "yield"
    )
}

fn generated_header() -> String {
    "// GENERATED CODE - DO NOT MODIFY BY HAND\n\n".to_string()
}

fn dart_string_literal(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('$', "\\$")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Renders Dart doc comments for a generated proxy method.
fn render_dart_doc_comments(method: &SourceMethod, indent: &str) -> String {
    if method.doc_lines.is_empty() {
        return format!("{indent}/// Generated proxy for `{}`.\n", method.name);
    }
    method
        .doc_lines
        .iter()
        .map(|line| {
            if line.is_empty() {
                format!("{indent}///\n")
            } else {
                format!("{indent}/// {line}\n")
            }
        })
        .collect()
}
