use super::*;

pub(crate) fn write_dart_proxy_artifacts(
    manifest_dir: &Path,
    schema_json: &str,
    objects: &[SourceObject],
    serializable_types: &HashMap<String, SerializableType>,
) {
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("operit-core-proxy must live under core/crates");
    let schema_dir = repo_root.join("core/generated");
    fs::create_dir_all(&schema_dir).expect("create generated schema directory");
    fs::write(schema_dir.join("core_proxy_schema.json"), schema_json)
        .expect("write core_proxy_schema.json");

    let dart_dir = repo_root.join("apps/flutter/app/lib/core/proxy/generated");
    fs::create_dir_all(&dart_dir).expect("create generated dart proxy directory");
    fs::write(
        dart_dir.join("CoreProxyModels.g.dart"),
        render_dart_models(objects, serializable_types),
    )
    .expect("write CoreProxyModels.g.dart");
    fs::write(
        dart_dir.join("CoreProxyClients.g.dart"),
        render_dart_clients(objects, serializable_types),
    )
    .expect("write CoreProxyClients.g.dart");
}

fn render_dart_models(
    objects: &[SourceObject],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let reachable = reachable_serializable_types(objects, serializable_types);
    let mut types = reachable
        .iter()
        .filter_map(|name| serializable_types.get(name))
        .collect::<Vec<_>>();
    types.sort_by(|left, right| left.full_type.cmp(&right.full_type));

    let mut output = generated_header();
    for ty in types {
        if let SerializableTypeKind::Struct { fields } = &ty.kind {
            output.push_str(&render_dart_struct(ty, fields, serializable_types));
        }
    }
    output
}

fn render_dart_clients(
    objects: &[SourceObject],
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let mut output = generated_header();
    output.push_str("import '../../bridge/OperitRuntimeBridge.dart';\n");
    output.push_str("import '../../link/CoreLinkProtocol.dart';\n");
    output.push_str("import 'CoreProxyModels.g.dart';\n\n");
    output.push_str(
        "String _coreProxyRequestId() => 'flutter-${DateTime.now().microsecondsSinceEpoch}';\n\n",
    );
    output.push_str("class GeneratedCoreProxyClients {\n");
    output.push_str("  const GeneratedCoreProxyClients(this.bridge);\n\n");
    output.push_str("  final OperitRuntimeBridge bridge;\n\n");
    for object in objects {
        let getter_name = dart_schema_getter_name(&object.schema_key);
        let class_name = dart_proxy_class_name(&object.schema_key);
        output.push_str(&format!(
            "  {class_name} get {getter_name} => {class_name}(bridge);\n"
        ));
    }
    output.push_str("}\n\n");

    for object in objects {
        output.push_str(&render_dart_client_class(object, serializable_types));
    }
    output
}

fn render_dart_client_class(
    object: &SourceObject,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let class_name = dart_proxy_class_name(&object.schema_key);
    let mut output = String::new();
    output.push_str(&format!("class {class_name} {{\n"));
    output.push_str(&format!("  const {class_name}(this.bridge);\n\n"));
    output.push_str("  final OperitRuntimeBridge bridge;\n\n");
    for method in &object.methods {
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
    }
    output.push_str("}\n\n");
    output
}

fn render_dart_call_method(
    object: &SourceObject,
    method: &SourceMethod,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let return_type = match method.call_protocol().expect("call protocol") {
        CallProtocol::Unit | CallProtocol::ResultUnit => "void".to_string(),
        CallProtocol::Value(ty) | CallProtocol::ResultValue(ty) => {
            dart_type(ty, serializable_types)
        }
    };
    let params = render_dart_params(&method.args, serializable_types);
    let args = render_dart_args_map(&method.args, serializable_types);
    let mut output = String::new();
    let method_name = dart_identifier(&method.name);
    output.push_str(&format!(
        "  Future<{return_type}> {method_name}({params}) async {{\n"
    ));
    if return_type == "void" {
        output.push_str("    await bridge.call(\n");
    } else {
        output.push_str("    final value = await bridge.call(\n");
    }
    output.push_str("      CoreCallRequest(\n");
    output.push_str("        requestId: _coreProxyRequestId(),\n");
    output.push_str(&format!(
        "        targetPath: CoreObjectPath.parse('{}'),\n",
        object.schema_key
    ));
    output.push_str(&format!("        methodName: '{}',\n", method.name));
    output.push_str(&format!("        args: {args},\n"));
    output.push_str("      ),\n");
    output.push_str("    );\n");
    if return_type != "void" {
        output.push_str(&format!(
            "    return {};\n",
            dart_decode_expr("value", &return_type)
        ));
    }
    output.push_str("  }\n\n");
    output
}

fn render_dart_watch_method(
    object: &SourceObject,
    method: &SourceMethod,
    serializable_types: &HashMap<String, SerializableType>,
) -> String {
    let watch = method.watch_protocol().expect("watch protocol");
    let value_type = watch
        .snapshot_type
        .as_ref()
        .map(|ty| dart_type(ty, serializable_types))
        .unwrap_or_else(|| "Object?".to_string());
    let params = render_dart_params(&method.args, serializable_types);
    let args = render_dart_args_map(&method.args, serializable_types);
    let mut output = String::new();
    let method_name = dart_identifier(&method.name);
    output.push_str(&format!(
        "  Future<{value_type}> {method_name}Snapshot({params}) async {{\n"
    ));
    output.push_str("    final event = await bridge.watch(\n");
    output.push_str(&format!("      '{}',\n", object.schema_key));
    output.push_str(&format!("      '{}',\n", method.name));
    output.push_str(&format!("      args: {args},\n"));
    output.push_str("    );\n");
    output.push_str(&format!(
        "    return {};\n",
        dart_decode_expr("event.value", &value_type)
    ));
    output.push_str("  }\n\n");
    output.push_str(&format!(
        "  Stream<{value_type}> {method_name}Changes({params}) {{\n"
    ));
    output.push_str("    return bridge\n");
    output.push_str(&format!(
        "        .watchChanges('{}', '{}', args: {args})\n",
        object.schema_key, method.name
    ));
    output.push_str(&format!(
        "        .map((event) => {});\n",
        dart_decode_expr("event.value", &value_type)
    ));
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
    output.push_str(&format!("class {class_name} {{\n"));
    output.push_str(&format!("  const {class_name}({{\n"));
    for field in fields {
        output.push_str(&format!(
            "    required this.{},\n",
            dart_identifier(&field.name)
        ));
    }
    output.push_str("  });\n\n");
    output.push_str(&format!(
        "  factory {class_name}.fromJson(Map<String, Object?> json) {{\n"
    ));
    output.push_str(&format!("    return {class_name}(\n"));
    for field in fields {
        let field_type = dart_type(&field.ty, serializable_types);
        output.push_str(&format!(
            "      {}: {},\n",
            dart_identifier(&field.name),
            dart_decode_expr(&format!("json['{}']", field.json_name), &field_type)
        ));
    }
    output.push_str("    );\n");
    output.push_str("  }\n\n");
    output.push_str("  Map<String, Object?> toJson() {\n");
    output.push_str("    return <String, Object?>{\n");
    for field in fields {
        let field_type = dart_type(&field.ty, serializable_types);
        output.push_str(&format!(
            "      '{}': {},\n",
            field.json_name,
            dart_encode_expr(&dart_identifier(&field.name), &field_type)
        ));
    }
    output.push_str("    };\n");
    output.push_str("  }\n\n");
    for field in fields {
        output.push_str(&format!(
            "  final {} {};\n",
            dart_type(&field.ty, serializable_types),
            dart_identifier(&field.name)
        ));
    }
    output.push_str("}\n\n");
    output
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
                dart_encode_expr(&dart_parameter_name(&arg.name), &arg_type)
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
                MethodProtocol::Call(CallProtocol::Value(ty))
                | MethodProtocol::Call(CallProtocol::ResultValue(ty)) => {
                    collect_reachable_type(ty, serializable_types, &mut out);
                }
                MethodProtocol::Watch(watch) => {
                    if let Some(snapshot_type) = &watch.snapshot_type {
                        collect_reachable_type(snapshot_type, serializable_types, &mut out);
                    }
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
        if let Some(SerializableType {
            kind: SerializableTypeKind::Struct { fields },
            ..
        }) = serializable_types.get(ty)
        {
            for field in fields {
                collect_reachable_type(&field.ty, serializable_types, out);
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
    if let Some(inner) = single_generic_arg(ty, "Option") {
        let inner_type = dart_type(inner, serializable_types);
        if inner_type.ends_with('?') {
            return inner_type;
        }
        return format!("{inner_type}?");
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
                kind: SerializableTypeKind::Enum { .. },
                ..
            }) => "Object?".to_string(),
            None => "Object?".to_string(),
        },
    }
}

fn dart_decode_expr(value: &str, dart_type: &str) -> String {
    if dart_type == "Object?" {
        return value.to_string();
    }
    if let Some(inner) = dart_type.strip_suffix('?') {
        return format!(
            "{value} == null ? null : {}",
            dart_decode_expr(value, inner)
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
    if let Some(inner) = list_inner(dart_type) {
        return format!(
            "({value} as List<Object?>).map((item) => {}).toList(growable: false)",
            dart_decode_expr("item", inner)
        );
    }
    if let Some((key, value_type)) = map_inner(dart_type) {
        return format!(
            "({value} as Map<Object?, Object?>).map((key, value) => MapEntry({}, {}))",
            dart_decode_expr("key", key),
            dart_decode_expr("value", value_type)
        );
    }
    format!("{dart_type}.fromJson({value} as Map<String, Object?>)")
}

fn dart_encode_expr(value: &str, dart_type: &str) -> String {
    if dart_type == "Object?" {
        return value.to_string();
    }
    if let Some(inner) = dart_type.strip_suffix('?') {
        if inner == "Object?" || matches!(inner, "bool" | "int" | "double" | "String" | "void") {
            return value.to_string();
        }
        if let Some(list_inner) = list_inner(inner) {
            return format!(
                "{value}?.map((item) => {}).toList(growable: false)",
                dart_encode_expr("item", list_inner)
            );
        }
        if let Some((key, value_type)) = map_inner(inner) {
            return format!(
                "{value}?.map((key, value) => MapEntry({}, {}))",
                dart_encode_expr("key", key),
                dart_encode_expr("value", value_type)
            );
        }
        return format!("{value}?.toJson()");
    }
    if matches!(dart_type, "bool" | "int" | "double" | "String" | "void") {
        return value.to_string();
    }
    if let Some(inner) = list_inner(dart_type) {
        return format!(
            "{value}.map((item) => {}).toList(growable: false)",
            dart_encode_expr("item", inner)
        );
    }
    if let Some((key, value_type)) = map_inner(dart_type) {
        return format!(
            "{value}.map((key, value) => MapEntry({}, {}))",
            dart_encode_expr("key", key),
            dart_encode_expr("value", value_type)
        );
    }
    format!("{value}.toJson()")
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
