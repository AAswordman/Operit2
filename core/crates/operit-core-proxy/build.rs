use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::{
    Fields, FnArg, ImplItem, ImplItemFn, Item, ItemEnum, ItemImpl, ItemStruct, Pat, ReturnType,
    Type, TypePath, UseTree, Visibility,
};

mod build_dart_codegen;
mod build_rust_codegen;

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let runtime_src = manifest_dir.join("../operit-runtime/src");
    let serializable_type_definitions = collect_serializable_type_definitions(&runtime_src);
    let serializable_types = serializable_type_definitions
        .keys()
        .cloned()
        .collect::<HashSet<_>>();
    let type_registry = collect_type_registry(&runtime_src);
    let object_specs = object_specs(&runtime_src);
    for spec in &object_specs {
        println!("cargo:rerun-if-changed={}", spec.source_path.display());
    }
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("build_dart_codegen.rs").display()
    );

    let objects = object_specs
        .iter()
        .map(|spec| scan_object(spec, &serializable_types, &type_registry))
        .collect::<Vec<_>>();
    let schema_json = build_rust_codegen::render_schema(&objects, &serializable_type_definitions);
    let generated = build_rust_codegen::render_generated(&objects, &schema_json);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out_dir.join("generated_core_dispatch.rs"), generated)
        .expect("write generated_core_dispatch.rs");
    build_dart_codegen::write_dart_proxy_artifacts(
        &manifest_dir,
        &schema_json,
        &objects,
        &serializable_type_definitions,
    );
}

#[derive(Clone, Debug)]
struct ObjectSpec {
    schema_key: String,
    dispatch_name: String,
    type_name: String,
    full_type: String,
    source_path: PathBuf,
    access: ObjectAccess,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ObjectAccess {
    Application,
    ChatRuntimeMain,
    DefaultConstruct,
    GetInstanceConstruct,
    ResultGetInstanceConstruct,
    NewConstruct,
    StringNewConstruct,
    ContextGetInstanceConstruct,
    ContextRefGetInstanceConstruct,
    StorePathsConstruct,
    ResultStorePathsConstruct,
}

impl ObjectAccess {
    fn is_constructible(&self) -> bool {
        matches!(
            self,
            ObjectAccess::DefaultConstruct
                | ObjectAccess::GetInstanceConstruct
                | ObjectAccess::ResultGetInstanceConstruct
                | ObjectAccess::NewConstruct
                | ObjectAccess::StringNewConstruct
                | ObjectAccess::ContextGetInstanceConstruct
                | ObjectAccess::ContextRefGetInstanceConstruct
                | ObjectAccess::StorePathsConstruct
                | ObjectAccess::ResultStorePathsConstruct
        )
    }
}

fn object_specs(runtime_src: &Path) -> Vec<ObjectSpec> {
    let mut specs = Vec::new();
    specs.push(required_object_spec(
        runtime_src,
        "application",
        "core/application/OperitApplication.rs",
        "OperitApplication",
        ObjectAccess::Application,
    ));
    specs.push(required_object_spec(
        runtime_src,
        "chatRuntimeHolder.main",
        "services/ChatServiceCore.rs",
        "ChatServiceCore",
        ObjectAccess::ChatRuntimeMain,
    ));
    specs.extend(discover_constructible_objects(
        runtime_src,
        "data/preferences",
        "preferences",
    ));
    specs.extend(discover_constructible_objects(
        runtime_src,
        "data/api",
        "api",
    ));
    specs.extend(discover_constructible_objects(
        runtime_src,
        "data/skill",
        "skill",
    ));
    specs.extend(discover_constructible_objects(
        runtime_src,
        "data/mcp",
        "mcp",
    ));
    specs.extend(discover_constructible_objects(
        runtime_src,
        "data/repository",
        "repository",
    ));
    specs.extend(discover_constructible_objects_recursive(
        runtime_src,
        "core/tools",
        "permissions",
    ));
    specs.extend(discover_constructible_objects_recursive(
        runtime_src,
        "plugins",
        "plugins",
    ));
    specs.sort_by(|left, right| left.schema_key.cmp(&right.schema_key));
    specs
}

fn required_object_spec(
    runtime_src: &Path,
    schema_key: &str,
    relative_path: &str,
    type_name: &str,
    access: ObjectAccess,
) -> ObjectSpec {
    let source_path = runtime_src.join(relative_path);
    ObjectSpec {
        schema_key: schema_key.to_string(),
        dispatch_name: dispatch_name_from_schema_key(schema_key),
        type_name: type_name.to_string(),
        full_type: full_type_for_source(runtime_src, &source_path, type_name),
        source_path,
        access,
    }
}

fn discover_constructible_objects(
    runtime_src: &Path,
    relative_dir: &str,
    schema_prefix: &str,
) -> Vec<ObjectSpec> {
    let dir = runtime_src.join(relative_dir);
    let mut specs = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return specs;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read runtime source");
        let file = syn::parse_file(&content).expect("parse runtime source");
        let Some((type_name, access)) = discover_constructible_type(&file) else {
            continue;
        };
        let schema_key = format!("{schema_prefix}.{}", lower_first(&type_name));
        specs.push(ObjectSpec {
            schema_key: schema_key.clone(),
            dispatch_name: dispatch_name_from_schema_key(&schema_key),
            full_type: full_type_for_source(runtime_src, &path, &type_name),
            type_name,
            source_path: path,
            access,
        });
    }
    specs
}

fn discover_constructible_objects_recursive(
    runtime_src: &Path,
    relative_dir: &str,
    schema_prefix: &str,
) -> Vec<ObjectSpec> {
    let dir = runtime_src.join(relative_dir);
    let mut specs = Vec::new();
    discover_constructible_objects_recursive_inner(
        runtime_src,
        &dir,
        &dir,
        schema_prefix,
        &mut specs,
    );
    specs
}

fn discover_constructible_objects_recursive_inner(
    runtime_src: &Path,
    root_dir: &Path,
    dir: &Path,
    schema_prefix: &str,
    specs: &mut Vec<ObjectSpec>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            discover_constructible_objects_recursive_inner(
                runtime_src,
                root_dir,
                &path,
                schema_prefix,
                specs,
            );
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read runtime source");
        let file = syn::parse_file(&content).expect("parse runtime source");
        let Some((type_name, access)) = discover_constructible_type(&file) else {
            continue;
        };
        let relative = path
            .strip_prefix(root_dir)
            .expect("source path must be inside discovered dir")
            .with_extension("");
        let mut schema_parts = vec![schema_prefix.to_string()];
        for component in relative.components() {
            schema_parts.push(component.as_os_str().to_string_lossy().to_string());
        }
        let mut schema_key = schema_parts.join(".");
        if let Some((prefix, _)) = schema_key.rsplit_once('.') {
            schema_key = format!("{prefix}.{}", lower_first(&type_name));
        }
        specs.push(ObjectSpec {
            schema_key: schema_key.clone(),
            dispatch_name: dispatch_name_from_schema_key(&schema_key),
            full_type: full_type_for_source(runtime_src, &path, &type_name),
            type_name,
            source_path: path,
            access,
        });
    }
}

fn discover_constructible_type(file: &syn::File) -> Option<(String, ObjectAccess)> {
    let mut public_types = Vec::new();
    for item in &file.items {
        let Item::Struct(item_struct) = item else {
            continue;
        };
        if !matches!(item_struct.vis, Visibility::Public(_))
            || !item_struct.generics.params.is_empty()
        {
            continue;
        }
        public_types.push(item_struct.ident.to_string());
    }

    for type_name in public_types {
        let mut has_default = false;
        let mut has_get_instance = false;
        let mut has_result_get_instance = false;
        let mut has_new = false;
        let mut has_string_new = false;
        let mut has_context_get_instance = false;
        let mut has_context_ref_get_instance = false;
        let mut has_store_paths_new = false;
        let mut has_result_store_paths_new = false;

        for item in &file.items {
            let Item::Impl(item_impl) = item else {
                continue;
            };
            if impl_type_name(item_impl) != Some(type_name.clone()) {
                continue;
            }
            for impl_item in &item_impl.items {
                let ImplItem::Fn(function) = impl_item else {
                    continue;
                };
                if !matches!(function.vis, Visibility::Public(_)) {
                    continue;
                }
                let name = function.sig.ident.to_string();
                if function.sig.inputs.is_empty() {
                    has_default |= name == "default";
                    if name == "getInstance" {
                        let return_type = function_return_type(function);
                        if return_type.starts_with("Result < Self")
                            || return_type.starts_with("Result<Self")
                        {
                            has_result_get_instance = true;
                        } else {
                            has_get_instance = true;
                        }
                    }
                    has_new |= name == "new";
                    continue;
                }
                if function.sig.inputs.len() == 1 {
                    let Some(arg_type) = first_function_arg_type(function) else {
                        continue;
                    };
                    if name == "getInstance" && arg_type.contains("OperitApplicationContext") {
                        if arg_type.trim_start().starts_with('&') {
                            has_context_ref_get_instance = true;
                        } else {
                            has_context_get_instance = true;
                        }
                    }
                    if name == "new" && arg_type.contains("RuntimeStorePaths") {
                        let return_type = function_return_type(function);
                        if return_type.contains("Result") {
                            has_result_store_paths_new = true;
                        } else {
                            has_store_paths_new = true;
                        }
                    }
                    has_string_new |= name == "new"
                        && (arg_type.contains("impl Into < String >")
                            || arg_type.contains("impl Into<String>")
                            || arg_type.trim() == "String");
                }
            }
        }

        if has_context_get_instance {
            return Some((type_name, ObjectAccess::ContextGetInstanceConstruct));
        }
        if has_context_ref_get_instance {
            return Some((type_name, ObjectAccess::ContextRefGetInstanceConstruct));
        }
        if has_get_instance {
            return Some((type_name, ObjectAccess::GetInstanceConstruct));
        }
        if has_result_get_instance {
            return Some((type_name, ObjectAccess::ResultGetInstanceConstruct));
        }
        if has_store_paths_new {
            return Some((type_name, ObjectAccess::StorePathsConstruct));
        }
        if has_result_store_paths_new {
            return Some((type_name, ObjectAccess::ResultStorePathsConstruct));
        }
        if has_string_new {
            return Some((type_name, ObjectAccess::StringNewConstruct));
        }
        if has_new {
            return Some((type_name, ObjectAccess::NewConstruct));
        }
        if has_default {
            return Some((type_name, ObjectAccess::DefaultConstruct));
        }
    }
    None
}

fn first_function_arg_type(function: &ImplItemFn) -> Option<String> {
    function.sig.inputs.iter().next().and_then(|arg| match arg {
        FnArg::Typed(pat_type) => Some(pat_type.ty.to_token_stream().to_string()),
        FnArg::Receiver(_) => None,
    })
}

fn function_return_type(function: &ImplItemFn) -> String {
    match &function.sig.output {
        ReturnType::Default => String::new(),
        ReturnType::Type(_, ty) => ty.to_token_stream().to_string(),
    }
}

fn full_type_for_source(runtime_src: &Path, source_path: &Path, type_name: &str) -> String {
    format!(
        "{}::{type_name}",
        module_path_for_source(runtime_src, source_path)
    )
}

fn module_path_for_source(runtime_src: &Path, source_path: &Path) -> String {
    let relative = source_path
        .strip_prefix(runtime_src)
        .expect("source path must be inside runtime src");
    let mut module_path = Vec::from(["operit_runtime".to_string()]);
    for component in relative.with_extension("").components() {
        module_path.push(component.as_os_str().to_string_lossy().to_string());
    }
    module_path.join("::")
}

fn dispatch_name_from_schema_key(schema_key: &str) -> String {
    identifier_words(schema_key)
        .into_iter()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("_")
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

fn lower_first(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_lowercase());
    out.extend(chars);
    out
}

fn parent_module_path(full_type: &str) -> &str {
    full_type
        .rsplit_once("::")
        .map(|(module, _)| module)
        .expect("object full_type must include module path")
}

#[derive(Clone, Debug, Default)]
struct TypeRegistry {
    aliases: HashMap<String, String>,
    trait_impls: HashMap<String, HashSet<String>>,
    stream_items: HashMap<String, String>,
}

impl TypeRegistry {
    fn resolve_alias(&self, ty: &str) -> String {
        let mut current = ty.to_string();
        let mut visited = HashSet::new();
        while visited.insert(current.clone()) {
            let Some(next) = self.aliases.get(&current) else {
                break;
            };
            current = next.clone();
        }
        current
    }

    fn implements(&self, ty: &str, trait_name: &str) -> bool {
        let resolved = self.resolve_alias(ty);
        self.trait_impls
            .get(&resolved)
            .map(|traits| traits.contains(trait_name))
            .unwrap_or(false)
    }

    fn stream_item(&self, ty: &str) -> Option<String> {
        let resolved = self.resolve_alias(ty);
        self.stream_items.get(&resolved).cloned()
    }
}

fn collect_type_registry(runtime_src: &Path) -> TypeRegistry {
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

fn collect_serializable_type_definitions(runtime_src: &Path) -> HashMap<String, SerializableType> {
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
    SerializableType {
        full_type,
        kind: SerializableTypeKind::Enum {
            variants: item_enum
                .variants
                .iter()
                .map(|variant| variant.ident.to_string())
                .collect(),
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

#[derive(Clone, Debug)]
struct SourceObject {
    schema_key: String,
    dispatch_name: String,
    full_type: String,
    access: ObjectAccess,
    methods: Vec<SourceMethod>,
}

#[derive(Clone, Debug)]
struct SourceMethod {
    name: String,
    args: Vec<SourceArg>,
    rust_return_type: String,
    is_async: bool,
    protocol: MethodProtocol,
}

#[derive(Clone, Debug)]
struct SourceArg {
    name: String,
    ty: String,
}

#[derive(Clone, Debug)]
struct SerializableType {
    full_type: String,
    kind: SerializableTypeKind,
}

#[derive(Clone, Debug)]
enum SerializableTypeKind {
    Struct { fields: Vec<SerializableField> },
    Enum { variants: Vec<String> },
}

#[derive(Clone, Debug)]
struct SerializableField {
    name: String,
    json_name: String,
    ty: String,
}

#[derive(Clone, Debug)]
enum MethodProtocol {
    Call(CallProtocol),
    Watch(WatchProtocol),
    Unsupported(String),
}

#[derive(Clone, Debug)]
enum CallProtocol {
    Unit,
    ResultUnit,
    Value(String),
    ResultValue(String),
}

#[derive(Clone, Debug)]
struct WatchProtocol {
    snapshot_type: Option<String>,
    stream: WatchStreamProtocol,
}

#[derive(Clone, Debug)]
enum WatchStreamProtocol {
    JsonFlow { fallible: bool },
    JsonState { fallible: bool },
    TextEvent { optional: bool },
}

impl SourceMethod {
    fn call_protocol(&self) -> Option<&CallProtocol> {
        match &self.protocol {
            MethodProtocol::Call(protocol) => Some(protocol),
            _ => None,
        }
    }

    fn watch_protocol(&self) -> Option<&WatchProtocol> {
        match &self.protocol {
            MethodProtocol::Watch(protocol) => Some(protocol),
            _ => None,
        }
    }

    fn unsupported_reason(&self) -> Option<&str> {
        match &self.protocol {
            MethodProtocol::Unsupported(reason) => Some(reason),
            _ => None,
        }
    }
}

fn scan_object(
    spec: &ObjectSpec,
    serializable_types: &HashSet<String>,
    type_registry: &TypeRegistry,
) -> SourceObject {
    SourceObject {
        schema_key: spec.schema_key.clone(),
        dispatch_name: spec.dispatch_name.clone(),
        full_type: spec.full_type.clone(),
        access: spec.access.clone(),
        methods: scan_methods(
            &spec.source_path,
            &spec.type_name,
            parent_module_path(&spec.full_type),
            serializable_types,
            type_registry,
        ),
    }
}

fn scan_methods(
    path: &Path,
    type_name: &str,
    module_path: &str,
    serializable_types: &HashSet<String>,
    type_registry: &TypeRegistry,
) -> Vec<SourceMethod> {
    let content = fs::read_to_string(path).expect("read runtime source");
    let file = syn::parse_file(&content).expect("parse runtime source");
    let resolver = TypeResolver::from_file(
        &file,
        module_path,
        serializable_types.clone(),
        type_registry.clone(),
    );
    let mut methods = Vec::new();
    for item in file.items.iter() {
        let Item::Impl(item_impl) = item else {
            continue;
        };
        if impl_type_name(item_impl) != Some(type_name.to_string()) {
            continue;
        }
        for impl_item in item_impl.items.iter() {
            let ImplItem::Fn(function) = impl_item else {
                continue;
            };
            if !matches!(function.vis, Visibility::Public(_)) {
                continue;
            }
            methods.push(scan_method(function, &resolver));
        }
    }
    methods
}

fn impl_type_name(item_impl: &ItemImpl) -> Option<String> {
    let Type::Path(TypePath { path, .. }) = item_impl.self_ty.as_ref() else {
        return None;
    };
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn scan_method(function: &ImplItemFn, resolver: &TypeResolver) -> SourceMethod {
    let name = function.sig.ident.to_string();
    let mut args = Vec::new();
    let mut method_error = None::<String>;
    let is_async = function.sig.asyncness.is_some();
    let mut has_receiver = false;

    for input in function.sig.inputs.iter() {
        match input {
            FnArg::Receiver(_) => {
                has_receiver = true;
            }
            FnArg::Typed(pat_type) => {
                let Pat::Ident(pat_ident) = pat_type.pat.as_ref() else {
                    method_error = Some("non-ident argument pattern".to_string());
                    continue;
                };
                let ty = normalize_type(&pat_type.ty, resolver);
                if !is_supported_arg_type(&ty, resolver) {
                    method_error = Some(format!("unsupported argument type: {ty}"));
                }
                args.push(SourceArg {
                    name: pat_ident.ident.to_string(),
                    ty,
                });
            }
        }
    }

    if !has_receiver {
        method_error = Some("associated function is not an instance method".to_string());
    }
    let (rust_return_type, mut protocol) = scan_return_protocol(&function.sig.output, resolver);
    if is_async && matches!(protocol, MethodProtocol::Watch(_)) {
        protocol = MethodProtocol::Unsupported("async watch method is not supported".to_string());
    }
    if let Some(reason) = method_error {
        protocol = MethodProtocol::Unsupported(reason);
    }

    SourceMethod {
        name,
        args,
        rust_return_type,
        is_async,
        protocol,
    }
}

struct TypeResolver {
    names: HashMap<String, String>,
    serializable_types: HashSet<String>,
    type_registry: TypeRegistry,
}

impl TypeResolver {
    fn from_file(
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

fn normalize_type(ty: &Type, resolver: &TypeResolver) -> String {
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

fn scan_return_protocol(
    return_type: &ReturnType,
    resolver: &TypeResolver,
) -> (String, MethodProtocol) {
    match return_type {
        ReturnType::Default => ("()".to_string(), MethodProtocol::Call(CallProtocol::Unit)),
        ReturnType::Type(_, ty) => {
            let normalized = normalize_type(ty, resolver);
            let protocol = classify_return_protocol(&normalized, resolver);
            (normalized, protocol)
        }
    }
}

fn classify_return_protocol(ty: &str, resolver: &TypeResolver) -> MethodProtocol {
    if ty == "()" {
        return MethodProtocol::Call(CallProtocol::Unit);
    }
    if result_unit(ty) {
        return MethodProtocol::Call(CallProtocol::ResultUnit);
    }
    if let Some(inner) = result_value_inner(ty) {
        if let Some(flow_inner) = flow_inner(inner) {
            return classify_json_watch(
                flow_inner,
                WatchStreamProtocol::JsonFlow { fallible: true },
                resolver,
            );
        }
        if let Some(state_inner) = state_flow_inner(inner) {
            return classify_json_watch(
                state_inner,
                WatchStreamProtocol::JsonState { fallible: true },
                resolver,
            );
        }
        return if is_supported_return_type(inner, resolver) {
            MethodProtocol::Call(CallProtocol::ResultValue(inner.to_string()))
        } else {
            MethodProtocol::Unsupported(format!("unsupported Result value type: {inner}"))
        };
    }
    if let Some(inner) = state_flow_inner(ty) {
        return classify_json_watch(
            inner,
            WatchStreamProtocol::JsonState { fallible: false },
            resolver,
        );
    }
    if let Some(inner) = flow_inner(ty) {
        return classify_json_watch(
            inner,
            WatchStreamProtocol::JsonFlow { fallible: false },
            resolver,
        );
    }
    if let Some(optional) = text_event_watch_optionality(ty, resolver) {
        return MethodProtocol::Watch(WatchProtocol {
            snapshot_type: None,
            stream: WatchStreamProtocol::TextEvent { optional },
        });
    }
    if ty.starts_with('&') {
        return MethodProtocol::Unsupported(format!(
            "borrowed return type cannot cross link: {ty}"
        ));
    }
    if is_supported_return_type(ty, resolver) {
        MethodProtocol::Call(CallProtocol::Value(ty.to_string()))
    } else {
        MethodProtocol::Unsupported(format!("unsupported return type: {ty}"))
    }
}

fn classify_json_watch(
    value_type: &str,
    stream: WatchStreamProtocol,
    resolver: &TypeResolver,
) -> MethodProtocol {
    if is_supported_return_type(value_type, resolver) {
        MethodProtocol::Watch(WatchProtocol {
            snapshot_type: Some(value_type.to_string()),
            stream,
        })
    } else {
        MethodProtocol::Unsupported(format!("unsupported watch value type: {value_type}"))
    }
}

fn text_event_watch_optionality(ty: &str, resolver: &TypeResolver) -> Option<bool> {
    if is_text_event_stream_type(ty, resolver) {
        return Some(false);
    }
    let inner = single_generic_arg(ty, "Option")?;
    is_text_event_stream_type(inner, resolver).then_some(true)
}

fn is_text_event_stream_type(ty: &str, resolver: &TypeResolver) -> bool {
    let resolved = resolver.type_registry.resolve_alias(ty);
    resolver
        .type_registry
        .stream_item(&resolved)
        .map(|item| item == "String")
        .unwrap_or(false)
        && resolver
            .type_registry
            .implements(&resolved, "TextStreamEventCarrier")
}

fn is_supported_arg_type(ty: &str, resolver: &TypeResolver) -> bool {
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

fn is_supported_return_type(ty: &str, resolver: &TypeResolver) -> bool {
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

fn single_generic_arg<'a>(ty: &'a str, name: &str) -> Option<&'a str> {
    let args = generic_args(ty, name)?;
    if args.len() == 1 {
        Some(args[0])
    } else {
        None
    }
}

fn borrowed_slice_inner(ty: &str) -> Option<&str> {
    ty.strip_prefix("&[")?.strip_suffix(']')
}

fn generic_args<'a>(ty: &'a str, name: &str) -> Option<Vec<&'a str>> {
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

fn split_top_level_args(value: &str) -> Vec<&str> {
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

fn state_flow_inner(ty: &str) -> Option<&str> {
    single_generic_arg(ty, "StateFlow")
}

fn flow_inner(ty: &str) -> Option<&str> {
    single_generic_arg(ty, "Flow")
}

fn result_value_inner(ty: &str) -> Option<&str> {
    let args = generic_args(ty, "Result")?;
    let value = args.first().copied()?;
    if value == "()" {
        None
    } else {
        Some(value)
    }
}

fn result_unit(ty: &str) -> bool {
    matches!(generic_args(ty, "Result").as_deref(), Some(["()", _]))
}
