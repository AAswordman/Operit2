use super::*;

pub(crate) fn collect_type_registry(source_roots: &[SourceRoot]) -> TypeRegistry {
    let mut registry = TypeRegistry::default();
    for source_root in source_roots {
        collect_type_registry_from_dir(source_root, source_root.as_path(), &mut registry);
    }
    registry
}

fn collect_type_registry_from_dir(
    source_root: &SourceRoot,
    dir: &Path,
    registry: &mut TypeRegistry,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_type_registry_from_dir(source_root, &path, registry);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read runtime source");
        let file = syn::parse_file(&content).expect("parse runtime source");
        let module_path = module_path_for_source_with_crate(
            source_root.as_path(),
            &path,
            &source_root.crate_name,
        );
        let resolver = TypeResolver::from_file(
            &file,
            &module_path,
            &source_root.crate_name,
            HashSet::new(),
            HashSet::new(),
            TypeRegistry::default(),
        );
        for item in &file.items {
            match item {
                Item::Use(item_use) if matches!(item_use.vis, Visibility::Public(_)) => {
                    collect_public_use_aliases(
                        &item_use.tree,
                        Vec::new(),
                        &source_root.crate_name,
                        &module_path,
                        registry,
                    );
                }
                Item::Type(item_type) => {
                    let alias = full_type_for_source_with_crate(
                        source_root.as_path(),
                        &path,
                        &item_type.ident.to_string(),
                        &source_root.crate_name,
                    );
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

/// Records aliases introduced by public use declarations.
fn collect_public_use_aliases(
    tree: &UseTree,
    mut prefix: Vec<String>,
    crate_name: &str,
    module_path: &str,
    registry: &mut TypeRegistry,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(normalize_import_segment(
                &path.ident.to_string(),
                crate_name,
            ));
            collect_public_use_aliases(&path.tree, prefix, crate_name, module_path, registry);
        }
        UseTree::Name(name) => {
            let local = name.ident.to_string();
            let mut target = prefix;
            target.push(local.clone());
            let alias = format!("{module_path}::{local}");
            registry.aliases.insert(alias, target.join("::"));
        }
        UseTree::Rename(rename) => {
            let local = rename.rename.to_string();
            let mut target = prefix;
            target.push(rename.ident.to_string());
            let alias = format!("{module_path}::{local}");
            registry.aliases.insert(alias, target.join("::"));
        }
        UseTree::Group(group) => {
            for item in group.items.iter() {
                collect_public_use_aliases(item, prefix.clone(), crate_name, module_path, registry);
            }
        }
        UseTree::Glob(_) => {}
    }
}

pub(crate) fn collect_serializable_type_definitions(
    source_roots: &[SourceRoot],
) -> HashMap<String, SerializableType> {
    let mut out = HashMap::new();
    for source_root in source_roots {
        collect_serializable_type_definitions_from_dir(
            source_root,
            source_root.as_path(),
            &mut out,
        );
    }
    out
}

pub(crate) fn collect_error_type_definitions(
    runtime_src: &Path,
    crate_name: &str,
) -> HashMap<String, ErrorTypeDefinition> {
    let mut out = HashMap::new();
    collect_error_type_definitions_from_dir(runtime_src, runtime_src, crate_name, &mut out);
    out
}

fn collect_error_type_definitions_from_dir(
    root: &Path,
    dir: &Path,
    crate_name: &str,
    out: &mut HashMap<String, ErrorTypeDefinition>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_error_type_definitions_from_dir(root, &path, crate_name, out);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read runtime source");
        let file = syn::parse_file(&content).expect("parse runtime source");
        let module_path = module_path_for_source_with_crate(root, &path, crate_name);
        let resolver = TypeResolver::from_file(
            &file,
            &module_path,
            crate_name,
            HashSet::new(),
            HashSet::new(),
            TypeRegistry::default(),
        );
        for item in &file.items {
            let Item::Enum(item_enum) = item else {
                continue;
            };
            if !matches!(item_enum.vis, Visibility::Public(_)) || !derives_error(&item_enum.attrs) {
                continue;
            }
            let full_type = full_type_for_source_with_crate(
                root,
                &path,
                &item_enum.ident.to_string(),
                crate_name,
            );
            out.insert(
                full_type.clone(),
                error_enum_type(full_type, item_enum, &resolver),
            );
        }
    }
}

fn error_enum_type(
    full_type: String,
    item_enum: &ItemEnum,
    resolver: &TypeResolver,
) -> ErrorTypeDefinition {
    let variants = item_enum
        .variants
        .iter()
        .map(|variant| {
            let (fields_kind, fields) = match &variant.fields {
                Fields::Named(fields) => (
                    ErrorFieldsKind::Named,
                    fields
                        .named
                        .iter()
                        .filter_map(|field| {
                            let name = field.ident.as_ref()?.to_string();
                            Some(ErrorField {
                                name,
                                ty: normalize_type(&field.ty, resolver),
                            })
                        })
                        .collect::<Vec<_>>(),
                ),
                Fields::Unnamed(fields) => (
                    ErrorFieldsKind::Unnamed,
                    fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(index, field)| ErrorField {
                            name: if fields.unnamed.len() == 1 {
                                "value".to_string()
                            } else {
                                format!("value{index}")
                            },
                            ty: normalize_type(&field.ty, resolver),
                        })
                        .collect::<Vec<_>>(),
                ),
                Fields::Unit => (ErrorFieldsKind::Unit, Vec::new()),
            };
            ErrorEnumVariant {
                name: variant.ident.to_string(),
                fields_kind,
                fields,
            }
        })
        .collect::<Vec<_>>();
    ErrorTypeDefinition {
        full_type,
        variants,
    }
}

fn collect_serializable_type_definitions_from_dir(
    source_root: &SourceRoot,
    dir: &Path,
    out: &mut HashMap<String, SerializableType>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_serializable_type_definitions_from_dir(source_root, &path, out);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read runtime source");
        let file = syn::parse_file(&content).expect("parse runtime source");
        let module_path = module_path_for_source_with_crate(
            source_root.as_path(),
            &path,
            &source_root.crate_name,
        );
        let resolver = TypeResolver::from_file(
            &file,
            &module_path,
            &source_root.crate_name,
            HashSet::new(),
            HashSet::new(),
            TypeRegistry::default(),
        );
        for item in &file.items {
            match item {
                Item::Struct(item_struct)
                    if matches!(item_struct.vis, Visibility::Public(_))
                        && derives_serde_value(&item_struct.attrs) =>
                {
                    let full_type = full_type_for_source_with_crate(
                        source_root.as_path(),
                        &path,
                        &item_struct.ident.to_string(),
                        &source_root.crate_name,
                    );
                    out.insert(
                        full_type.clone(),
                        serializable_struct_type(full_type, item_struct, &resolver),
                    );
                }
                Item::Enum(item_enum)
                    if matches!(item_enum.vis, Visibility::Public(_))
                        && derives_serde_value(&item_enum.attrs) =>
                {
                    let full_type = full_type_for_source_with_crate(
                        source_root.as_path(),
                        &path,
                        &item_enum.ident.to_string(),
                        &source_root.crate_name,
                    );
                    out.insert(
                        full_type.clone(),
                        serializable_enum_type(full_type, item_enum, &resolver),
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
        supports_serialize: derives_serialize(&item_struct.attrs),
        supports_deserialize: derives_deserialize(&item_struct.attrs),
        kind: SerializableTypeKind::Struct { fields },
    }
}

fn serializable_enum_type(
    full_type: String,
    item_enum: &ItemEnum,
    resolver: &TypeResolver,
) -> SerializableType {
    let mapped: Vec<SerializableEnumVariant> = item_enum
        .variants
        .iter()
        .map(|variant| {
            let name = variant.ident.to_string();
            let (fields_are_unnamed, fields) = match &variant.fields {
                Fields::Named(fields) => {
                    let fields = fields
                        .named
                        .iter()
                        .filter_map(|field| {
                            let field_name = field.ident.as_ref()?.to_string();
                            Some(SerializableField {
                                name: field_name.clone(),
                                json_name: serde_rename(&field.attrs).unwrap_or_else(|| {
                                    field_name.trim_start_matches("r#").to_string()
                                }),
                                ty: normalize_type(&field.ty, resolver),
                            })
                        })
                        .collect::<Vec<_>>();
                    (false, fields)
                }
                Fields::Unnamed(fields) => {
                    let fields = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(index, field)| {
                            let field_name = if fields.unnamed.len() == 1 {
                                "value".to_string()
                            } else {
                                format!("value{index}")
                            };
                            SerializableField {
                                name: field_name.clone(),
                                json_name: field_name,
                                ty: normalize_type(&field.ty, resolver),
                            }
                        })
                        .collect::<Vec<_>>();
                    (true, fields)
                }
                Fields::Unit => (false, Vec::new()),
            };
            SerializableEnumVariant {
                json_name: serde_rename(&variant.attrs).unwrap_or_else(|| name.clone()),
                fields_are_unnamed,
                fields,
                name,
            }
        })
        .collect();
    let unit_only = mapped.iter().all(|variant| variant.fields.is_empty());
    if unit_only {
        return SerializableType {
            full_type: full_type.clone(),
            supports_serialize: derives_serialize(&item_enum.attrs),
            supports_deserialize: derives_deserialize(&item_enum.attrs),
            kind: SerializableTypeKind::Enum {
                variants: mapped,
                unit_only,
            },
        };
    }
    let (tag_name, content_name) = serde_tag_content(&item_enum.attrs).unwrap_or_else(|| {
        (
            full_type.rsplit("::").next().unwrap_or("type").to_string(),
            None,
        )
    });
    SerializableType {
        full_type,
        supports_serialize: derives_serialize(&item_enum.attrs),
        supports_deserialize: derives_deserialize(&item_enum.attrs),
        kind: SerializableTypeKind::TaggedEnum {
            tag_name,
            content_name,
            variants: mapped,
        },
    }
}

fn derives_serde_pair(attrs: &[syn::Attribute]) -> bool {
    derives_serialize(attrs) && derives_deserialize(attrs)
}

fn derives_error(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let tokens = attr.meta.to_token_stream().to_string();
        if tokens.contains("Error") {
            return true;
        }
    }
    false
}

fn derives_serde_value(attrs: &[syn::Attribute]) -> bool {
    derives_serialize(attrs) || derives_deserialize(attrs)
}

fn derives_serialize(attrs: &[syn::Attribute]) -> bool {
    let mut has_serialize = false;
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let tokens = attr.meta.to_token_stream().to_string();
        has_serialize |= tokens.contains("Serialize");
    }
    has_serialize
}

fn derives_deserialize(attrs: &[syn::Attribute]) -> bool {
    let mut has_deserialize = false;
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let tokens = attr.meta.to_token_stream().to_string();
        has_deserialize |= tokens.contains("Deserialize");
    }
    has_deserialize
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
    pub(crate) crate_name: String,
    pub(crate) serializable_types: HashSet<String>,
    pub(crate) deserializable_types: HashSet<String>,
    pub(crate) type_registry: TypeRegistry,
}

impl TypeResolver {
    pub(crate) fn from_file(
        file: &syn::File,
        module_path: &str,
        crate_name: &str,
        serializable_types: HashSet<String>,
        deserializable_types: HashSet<String>,
        type_registry: TypeRegistry,
    ) -> Self {
        let mut names = HashMap::new();
        for item in &file.items {
            match item {
                Item::Use(item_use) => {
                    collect_use_tree(&item_use.tree, Vec::new(), crate_name, &mut names)
                }
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
            crate_name: crate_name.to_string(),
            serializable_types,
            deserializable_types,
            type_registry,
        }
    }

    pub(crate) fn default_from(_variants: &[&syn::Variant], _full_type: &str) -> Self {
        Self {
            names: HashMap::new(),
            crate_name: String::new(),
            serializable_types: HashSet::new(),
            deserializable_types: HashSet::new(),
            type_registry: TypeRegistry::default(),
        }
    }
}

fn collect_use_tree(
    tree: &UseTree,
    mut prefix: Vec<String>,
    crate_name: &str,
    names: &mut HashMap<String, String>,
) {
    match tree {
        UseTree::Path(path) => {
            let segment = normalize_import_segment(&path.ident.to_string(), crate_name);
            prefix.push(segment);
            collect_use_tree(&path.tree, prefix, crate_name, names);
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
                collect_use_tree(item, prefix.clone(), crate_name, names);
            }
        }
        UseTree::Glob(_) => {}
    }
}

fn normalize_import_segment(segment: &str, crate_name: &str) -> String {
    match segment {
        "crate" => crate_name.to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn normalize_type(ty: &Type, resolver: &TypeResolver) -> String {
    let crate_prefix = format!("{}::", resolver.crate_name);
    let normalized = ty
        .to_token_stream()
        .to_string()
        .replace(' ', "")
        .replace("crate::", &crate_prefix);
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
        return is_supported_arg_type(inner, resolver);
    }
    if let Some(inner) = borrowed_slice_inner(ty) {
        if inner == "std::path::PathBuf" {
            return true;
        }
        return is_supported_arg_type(inner, resolver);
    }
    if let Some(inner) = ty.strip_prefix('&') {
        return !inner.starts_with("mut") && is_supported_arg_type(inner, resolver);
    }
    if is_never_link_value_type(ty) {
        return false;
    }
    if is_primitive_link_value_type(ty)
        || ty == "serde_json::Value"
        || is_deserializable_named_value_type(ty, resolver)
        || is_tuple_arg_type(ty, resolver)
    {
        return true;
    }
    if let Some(inner) = single_generic_arg(ty, "Option") {
        return is_supported_arg_type(inner, resolver);
    }
    if let Some(inner) = single_generic_arg(ty, "Vec") {
        return is_supported_arg_type(inner, resolver);
    }
    if let Some(inner) = single_generic_arg(ty, "HashSet")
        .or_else(|| single_generic_arg(ty, "std::collections::HashSet"))
    {
        return is_supported_arg_type(inner, resolver);
    }
    if let Some(args) = generic_args(ty, "HashMap")
        .or_else(|| generic_args(ty, "std::collections::HashMap"))
        .or_else(|| generic_args(ty, "BTreeMap"))
        .or_else(|| generic_args(ty, "std::collections::BTreeMap"))
    {
        return args.len() == 2
            && is_supported_arg_map_key_type(args[0], resolver)
            && is_supported_arg_type(args[1], resolver);
    }
    if let Some((base, args)) = generic_named_type(ty) {
        return is_deserializable_named_value_type(base, resolver)
            && args
                .iter()
                .copied()
                .all(|arg| is_supported_arg_type(arg, resolver));
    }
    false
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

fn is_tuple_arg_type(ty: &str, resolver: &TypeResolver) -> bool {
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
        .all(|item| is_supported_arg_type(item, resolver))
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

fn is_supported_arg_map_key_type(ty: &str, resolver: &TypeResolver) -> bool {
    is_primitive_link_value_type(ty) || is_deserializable_named_value_type(ty, resolver)
}

fn is_serializable_named_value_type(ty: &str, resolver: &TypeResolver) -> bool {
    resolver.serializable_types.contains(ty)
}

fn is_deserializable_named_value_type(ty: &str, resolver: &TypeResolver) -> bool {
    resolver.deserializable_types.contains(ty)
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
    let (value, _) = result_args(ty)?;
    if value == "()" {
        None
    } else {
        Some(value)
    }
}

fn serde_tag_content(attrs: &[syn::Attribute]) -> Option<(String, Option<String>)> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let mut tag = None;
        let mut content = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("tag") {
                let value = meta.value()?;
                let literal: syn::LitStr = value.parse()?;
                tag = Some(literal.value());
            } else if meta.path.is_ident("content") {
                let value = meta.value()?;
                let literal: syn::LitStr = value.parse()?;
                content = Some(literal.value());
            }
            Ok(())
        });
        if let Some(tag) = tag {
            return Some((tag, content));
        }
    }
    None
}

pub(crate) fn result_value_parts(ty: &str) -> Option<(&str, &str)> {
    let (value, error) = result_args(ty)?;
    if value == "()" {
        None
    } else {
        Some((value, error))
    }
}

pub(crate) fn result_unit_error_type(ty: &str) -> Option<&str> {
    let (value, error) = result_args(ty)?;
    (value == "()").then_some(error)
}

pub(crate) fn result_unit(ty: &str) -> bool {
    result_unit_error_type(ty).is_some()
}

fn result_args(ty: &str) -> Option<(&str, &str)> {
    let args = generic_args(ty, "Result")?;
    if args.len() == 2 {
        Some((args[0], args[1]))
    } else {
        None
    }
}
