use super::*;

pub(crate) fn collect_type_registry(runtime_src: &Path) -> TypeRegistry {
    let mut registry = TypeRegistry::default();
    collect_type_registry_from_dir(runtime_src, runtime_src, &mut registry);
    registry
}

fn collect_type_registry_from_dir(root: &Path, dir: &Path, registry: &mut TypeRegistry) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_type_registry_from_dir(root, &path, registry);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read runtime source");
        let file = syn::parse_file(&content).expect("parse runtime source");
        let module_path = module_path_for_source(root, &path);
        let resolver =
            TypeResolver::from_file(&file, &module_path, HashSet::new(), TypeRegistry::default());
        for item in &file.items {
            match item {
                Item::Type(item_type) => {
                    let alias = full_type_for_source(root, &path, &item_type.ident.to_string());
                    registry
                        .aliases
                        .insert(alias, normalize_type(&item_type.ty, &resolver));
                }
                Item::Impl(item_impl) => {
                    let self_type = normalize_type(&item_impl.self_ty, &resolver);
                    if let Some((_, trait_path, _)) = &item_impl.trait_ {
                        if let Some(trait_name) = trait_path
                            .segments
                            .last()
                            .map(|segment| segment.ident.to_string())
                        {
                            registry
                                .trait_impls
                                .entry(self_type.clone())
                                .or_default()
                                .insert(trait_name);
                        }
                    }
                    for item in &item_impl.items {
                        let ImplItem::Type(item_type) = item else {
                            continue;
                        };
                        if item_type.ident == "Item" {
                            registry.stream_items.insert(
                                self_type.clone(),
                                normalize_type(&item_type.ty, &resolver),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

pub(crate) fn collect_serializable_type_definitions(
    runtime_src: &Path,
) -> HashMap<String, SerializableType> {
    let mut out = HashMap::new();
    collect_serializable_type_definitions_from_dir(runtime_src, runtime_src, &mut out);
    out
}

fn collect_serializable_type_definitions_from_dir(
    root: &Path,
    dir: &Path,
    out: &mut HashMap<String, SerializableType>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_serializable_type_definitions_from_dir(root, &path, out);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read runtime source");
        let file = syn::parse_file(&content).expect("parse runtime source");
        let module_path = module_path_for_source(root, &path);
        let resolver =
            TypeResolver::from_file(&file, &module_path, HashSet::new(), TypeRegistry::default());
        for item in &file.items {
            match item {
                Item::Struct(item_struct)
                    if matches!(item_struct.vis, Visibility::Public(_))
                        && derives_serde_pair(&item_struct.attrs) =>
                {
                    let full_type =
                        full_type_for_source(root, &path, &item_struct.ident.to_string());
                    out.insert(
                        full_type.clone(),
                        serializable_struct_type(full_type, item_struct, &resolver),
                    );
                }
                Item::Enum(item_enum)
                    if matches!(item_enum.vis, Visibility::Public(_))
                        && derives_serde_pair(&item_enum.attrs) =>
                {
                    let full_type = full_type_for_source(root, &path, &item_enum.ident.to_string());
                    out.insert(
                        full_type.clone(),
                        serializable_enum_type(full_type, item_enum),
                    );
                }
                _ => {}
            }
        }
    }
}

fn serializable_struct_type(
    full_type: String,
    item_struct: &ItemStruct,
    resolver: &TypeResolver,
) -> SerializableType {
    let fields = match &item_struct.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter(|field| matches!(field.vis, Visibility::Public(_)))
            .filter_map(|field| {
                let field_name = field.ident.as_ref()?.to_string();
                Some(SerializableField {
                    name: field_name.clone(),
                    json_name: serde_rename(&field.attrs)
                        .unwrap_or_else(|| field_name.trim_start_matches("r#").to_string()),
                    ty: normalize_type(&field.ty, resolver),
                })
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    SerializableType {
        full_type,
        kind: SerializableTypeKind::Struct { fields },
    }
}

fn serializable_enum_type(full_type: String, item_enum: &ItemEnum) -> SerializableType {
    let unit_only = item_enum
        .variants
        .iter()
        .all(|variant| matches!(variant.fields, Fields::Unit));
    SerializableType {
        full_type,
        kind: SerializableTypeKind::Enum {
            variants: item_enum
                .variants
                .iter()
                .map(|variant| {
                    let name = variant.ident.to_string();
                    SerializableEnumVariant {
                        json_name: serde_rename(&variant.attrs).unwrap_or_else(|| name.clone()),
                        name,
                    }
                })
                .collect(),
            unit_only,
        },
    }
}

fn derives_serde_pair(attrs: &[syn::Attribute]) -> bool {
    let mut has_serialize = false;
    let mut has_deserialize = false;
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let tokens = attr.meta.to_token_stream().to_string();
        has_serialize |= tokens.contains("Serialize");
        has_deserialize |= tokens.contains("Deserialize");
    }
    has_serialize && has_deserialize
}

fn serde_rename(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let mut rename = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let literal: syn::LitStr = value.parse()?;
                rename = Some(literal.value());
            }
            Ok(())
        });
        if rename.is_some() {
            return rename;
        }
    }
    None
}

pub(crate) struct TypeResolver {
    pub(crate) names: HashMap<String, String>,
    pub(crate) serializable_types: HashSet<String>,
    pub(crate) type_registry: TypeRegistry,
}

impl TypeResolver {
    pub(crate) fn from_file(
        file: &syn::File,
        module_path: &str,
        serializable_types: HashSet<String>,
        type_registry: TypeRegistry,
    ) -> Self {
        let mut names = HashMap::new();
        for item in &file.items {
            match item {
                Item::Use(item_use) => collect_use_tree(&item_use.tree, Vec::new(), &mut names),
                Item::Struct(item_struct) => {
                    let name = item_struct.ident.to_string();
                    names.insert(name.clone(), format!("{module_path}::{name}"));
                }
                Item::Enum(item_enum) => {
                    let name = item_enum.ident.to_string();
                    names.insert(name.clone(), format!("{module_path}::{name}"));
                }
                Item::Type(item_type) => {
                    let name = item_type.ident.to_string();
                    names.insert(name.clone(), format!("{module_path}::{name}"));
                }
                _ => {}
            }
        }
        Self {
            names,
            serializable_types,
            type_registry,
        }
    }
}

fn collect_use_tree(tree: &UseTree, mut prefix: Vec<String>, names: &mut HashMap<String, String>) {
    match tree {
        UseTree::Path(path) => {
            let segment = normalize_import_segment(&path.ident.to_string());
            prefix.push(segment);
            collect_use_tree(&path.tree, prefix, names);
        }
        UseTree::Name(name) => {
            let local = name.ident.to_string();
            let mut full = prefix;
            full.push(local.clone());
            names.insert(local, full.join("::"));
        }
        UseTree::Rename(rename) => {
            let local = rename.rename.to_string();
            let mut full = prefix;
            full.push(rename.ident.to_string());
            names.insert(local, full.join("::"));
        }
        UseTree::Group(group) => {
            for item in group.items.iter() {
                collect_use_tree(item, prefix.clone(), names);
            }
        }
        UseTree::Glob(_) => {}
    }
}

fn normalize_import_segment(segment: &str) -> String {
    match segment {
        "crate" => "operit_runtime".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn normalize_type(ty: &Type, resolver: &TypeResolver) -> String {
    let normalized = ty
        .to_token_stream()
        .to_string()
        .replace(' ', "")
        .replace("crate::", "operit_runtime::");
    resolve_bare_type_names(&normalized, resolver)
}

fn resolve_bare_type_names(ty: &str, resolver: &TypeResolver) -> String {
    let mut out = String::with_capacity(ty.len());
    let mut cursor = 0usize;
    while cursor < ty.len() {
        let ch = ty[cursor..]
            .chars()
            .next()
            .expect("cursor must be on a char boundary");
        if is_ident_start(ch) {
            let start = cursor;
            cursor += ch.len_utf8();
            while cursor < ty.len() {
                let next = ty[cursor..]
                    .chars()
                    .next()
                    .expect("cursor must be on a char boundary");
                if is_ident_continue(next) {
                    cursor += next.len_utf8();
                } else {
                    break;
                }
            }
            let ident = &ty[start..cursor];
            if is_path_segment(ty, start, cursor) {
                out.push_str(ident);
            } else if let Some(full) = resolver.names.get(ident) {
                out.push_str(full);
            } else {
                out.push_str(ident);
            }
        } else {
            out.push(ch);
            cursor += ch.len_utf8();
        }
    }
    out
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn is_path_segment(value: &str, start: usize, end: usize) -> bool {
    value[..start].ends_with("::") || value[end..].starts_with("::")
}

pub(crate) fn is_supported_arg_type(ty: &str, resolver: &TypeResolver) -> bool {
    if ty == "&str" || ty == "Option<&str>" || ty == "&std::path::Path" {
        return true;
    }
    if let Some(inner) = single_generic_arg(ty, "Option").and_then(|inner| inner.strip_prefix('&'))
    {
        return is_supported_return_type(inner, resolver);
    }
    if let Some(inner) = borrowed_slice_inner(ty) {
        if inner == "std::path::PathBuf" {
            return true;
        }
        return is_supported_return_type(inner, resolver);
    }
    if let Some(inner) = ty.strip_prefix('&') {
        return !inner.starts_with("mut") && is_supported_return_type(inner, resolver);
    }
    is_supported_return_type(ty, resolver)
}

pub(crate) fn is_supported_return_type(ty: &str, resolver: &TypeResolver) -> bool {
    if is_never_link_value_type(ty) {
        return false;
    }
    if is_primitive_link_value_type(ty)
        || ty == "serde_json::Value"
        || is_serializable_named_value_type(ty, resolver)
        || is_tuple_value_type(ty, resolver)
    {
        return true;
    }
    if let Some(inner) = single_generic_arg(ty, "Option") {
        return is_supported_return_type(inner, resolver);
    }
    if let Some(inner) = single_generic_arg(ty, "Vec") {
        return is_supported_return_type(inner, resolver);
    }
    if let Some(inner) = single_generic_arg(ty, "HashSet")
        .or_else(|| single_generic_arg(ty, "std::collections::HashSet"))
    {
        return is_supported_return_type(inner, resolver);
    }
    if let Some(args) = generic_args(ty, "HashMap")
        .or_else(|| generic_args(ty, "std::collections::HashMap"))
        .or_else(|| generic_args(ty, "BTreeMap"))
        .or_else(|| generic_args(ty, "std::collections::BTreeMap"))
    {
        return args.len() == 2
            && is_supported_map_key_type(args[0], resolver)
            && is_supported_return_type(args[1], resolver);
    }
    if let Some((base, args)) = generic_named_type(ty) {
        return is_serializable_named_value_type(base, resolver)
            && args
                .iter()
                .copied()
                .all(|arg| is_supported_return_type(arg, resolver));
    }
    false
}

fn is_tuple_value_type(ty: &str, resolver: &TypeResolver) -> bool {
    let Some(inner) = ty
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
    else {
        return false;
    };
    if inner.is_empty() {
        return true;
    }
    split_top_level_args(inner)
        .iter()
        .copied()
        .all(|item| is_supported_return_type(item, resolver))
}

fn is_never_link_value_type(ty: &str) -> bool {
    ty.is_empty()
        || ty == "Self"
        || ty.starts_with('&')
        || ty.starts_with("fn(")
        || generic_args(ty, "Flow").is_some()
        || generic_args(ty, "StateFlow").is_some()
        || ty.contains("&mut")
        || ty.contains("dyn")
}

fn is_primitive_link_value_type(ty: &str) -> bool {
    matches!(
        ty,
        "()" | "bool"
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
            | "String"
    )
}

fn is_supported_map_key_type(ty: &str, resolver: &TypeResolver) -> bool {
    is_primitive_link_value_type(ty) || is_serializable_named_value_type(ty, resolver)
}

fn is_serializable_named_value_type(ty: &str, resolver: &TypeResolver) -> bool {
    resolver.serializable_types.contains(ty)
}

pub(crate) fn single_generic_arg<'a>(ty: &'a str, name: &str) -> Option<&'a str> {
    let args = generic_args(ty, name)?;
    if args.len() == 1 {
        Some(args[0])
    } else {
        None
    }
}

pub(crate) fn borrowed_slice_inner(ty: &str) -> Option<&str> {
    ty.strip_prefix("&[")?.strip_suffix(']')
}

pub(crate) fn generic_args<'a>(ty: &'a str, name: &str) -> Option<Vec<&'a str>> {
    let generic_start = ty.find('<')?;
    if !ty.ends_with('>') {
        return None;
    }
    let base = &ty[..generic_start];
    if base.rsplit("::").next()? != name {
        return None;
    }
    let inner = &ty[generic_start + 1..ty.len() - 1];
    Some(split_top_level_args(inner))
}

fn generic_named_type<'a>(ty: &'a str) -> Option<(&'a str, Vec<&'a str>)> {
    let generic_start = ty.find('<')?;
    if !ty.ends_with('>') {
        return None;
    }
    let base = &ty[..generic_start];
    if base.is_empty() {
        return None;
    }
    let inner = &ty[generic_start + 1..ty.len() - 1];
    Some((base, split_top_level_args(inner)))
}

pub(crate) fn split_top_level_args(value: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (index, ch) in value.char_indices() {
        match ch {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                args.push(value[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    args.push(value[start..].trim());
    args
}

pub(crate) fn state_flow_inner(ty: &str) -> Option<&str> {
    single_generic_arg(ty, "StateFlow")
}

pub(crate) fn flow_inner(ty: &str) -> Option<&str> {
    single_generic_arg(ty, "Flow")
}

pub(crate) fn result_value_inner(ty: &str) -> Option<&str> {
    let args = generic_args(ty, "Result")?;
    let value = args.first().copied()?;
    if value == "()" {
        None
    } else {
        Some(value)
    }
}

pub(crate) fn result_unit(ty: &str) -> bool {
    matches!(generic_args(ty, "Result").as_deref(), Some(["()", _]))
}
