use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::{
    FnArg, ImplItem, ImplItemFn, Item, ItemImpl, Pat, ReturnType, Type, TypePath, UseTree, Visibility,
};

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let runtime_src = manifest_dir.join("../operit-runtime/src");
    let serializable_types = collect_serializable_types(&runtime_src);
    let object_specs = object_specs(&runtime_src);
    for spec in &object_specs {
        println!("cargo:rerun-if-changed={}", spec.source_path.display());
    }

    let objects = object_specs
        .iter()
        .map(|spec| ScannedObject {
            schema_key: spec.schema_key.clone(),
            dispatch_name: spec.dispatch_name.clone(),
            full_type: spec.full_type.clone(),
            access: spec.access.clone(),
            methods: collect_methods(
                &spec.source_path,
                &spec.type_name,
                parent_module_path(&spec.full_type),
                &serializable_types,
            ),
        })
        .collect::<Vec<_>>();
    let generated = render_generated(&objects);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out_dir.join("generated_core_dispatch.rs"), generated)
        .expect("write generated_core_dispatch.rs");
}

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
    GitHubTokenNewConstruct,
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
                | ObjectAccess::GitHubTokenNewConstruct
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
    discover_constructible_objects_recursive_inner(runtime_src, &dir, &dir, schema_prefix, &mut specs);
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
            discover_constructible_objects_recursive_inner(runtime_src, root_dir, &path, schema_prefix, specs);
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
        if !matches!(item_struct.vis, Visibility::Public(_)) || !item_struct.generics.params.is_empty() {
            continue;
        }
        public_types.push(item_struct.ident.to_string());
    }
    for type_name in public_types {
        let mut has_default = false;
        let mut has_get_instance = false;
        let mut has_result_get_instance = false;
        let mut has_new = false;
        let mut has_github_token_new = false;
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
                    has_github_token_new |= name == "newWithGitHubToken"
                        && arg_type.contains("Option < String >");
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
        if has_github_token_new {
            return Some((type_name, ObjectAccess::GitHubTokenNewConstruct));
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
    let relative = source_path
        .strip_prefix(runtime_src)
        .expect("source path must be inside runtime src");
    let mut module_path = Vec::from(["operit_runtime".to_string()]);
    for component in relative.with_extension("").components() {
        module_path.push(component.as_os_str().to_string_lossy().to_string());
    }
    module_path.push(type_name.to_string());
    module_path.join("::")
}

fn dispatch_name_from_schema_key(schema_key: &str) -> String {
    let mut out = String::new();
    let mut previous_was_word = false;
    let mut previous_was_lower_or_digit = false;
    for ch in schema_key.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() && previous_was_lower_or_digit {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            previous_was_word = true;
            previous_was_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else {
            if previous_was_word && !out.ends_with('_') {
                out.push('_');
            }
            previous_was_word = false;
            previous_was_lower_or_digit = false;
        }
    }
    while out.ends_with('_') {
        out.pop();
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

fn collect_serializable_types(runtime_src: &Path) -> HashSet<String> {
    let mut out = HashSet::new();
    collect_serializable_types_from_dir(runtime_src, runtime_src, &mut out);
    out
}

fn collect_serializable_types_from_dir(root: &Path, dir: &Path, out: &mut HashSet<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_serializable_types_from_dir(root, &path, out);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read runtime source");
        let file = syn::parse_file(&content).expect("parse runtime source");
        for item in &file.items {
            match item {
                Item::Struct(item_struct)
                    if matches!(item_struct.vis, Visibility::Public(_))
                        && derives_serde_pair(&item_struct.attrs) =>
                {
                    out.insert(full_type_for_source(root, &path, &item_struct.ident.to_string()));
                }
                Item::Enum(item_enum)
                    if matches!(item_enum.vis, Visibility::Public(_))
                        && derives_serde_pair(&item_enum.attrs) =>
                {
                    out.insert(full_type_for_source(root, &path, &item_enum.ident.to_string()));
                }
                _ => {}
            }
        }
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

#[derive(Clone, Debug)]
struct ScannedObject {
    schema_key: String,
    dispatch_name: String,
    full_type: String,
    access: ObjectAccess,
    methods: Vec<ScannedMethod>,
}

#[derive(Clone, Debug)]
struct ScannedMethod {
    name: String,
    args: Vec<ScannedArg>,
    return_type: ReturnKind,
    is_async: bool,
    callable: bool,
    watchable: bool,
    unsupported_reason: Option<String>,
}

#[derive(Clone, Debug)]
struct ScannedArg {
    name: String,
    ty: String,
}

#[derive(Clone, Debug)]
enum ReturnKind {
    Unit,
    ResultUnit,
    ResultValue(String),
    ResultFlow(String),
    Value(String),
    Flow(String),
    StateFlow(String),
    SharedTextStream,
    Unsupported(String),
}

fn collect_methods(
    path: &Path,
    type_name: &str,
    module_path: &str,
    serializable_types: &HashSet<String>,
) -> Vec<ScannedMethod> {
    let content = fs::read_to_string(path).expect("read runtime source");
    let file = syn::parse_file(&content).expect("parse runtime source");
    let resolver = TypeResolver::from_file(&file, module_path, serializable_types.clone());
    let mut methods = Vec::new();
    for item in file.items.iter() {
        let Item::Impl(item_impl) = item else {
            continue;
        };
        if impl_type_name(&item_impl) != Some(type_name.to_string()) {
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
    path.segments.last().map(|segment| segment.ident.to_string())
}

fn scan_method(function: &ImplItemFn, resolver: &TypeResolver) -> ScannedMethod {
    let name = function.sig.ident.to_string();
    let mut args = Vec::new();
    let mut unsupported_reason = None::<String>;
    let is_async = function.sig.asyncness.is_some();
    let mut has_receiver = false;
    for input in function.sig.inputs.iter() {
        match input {
            FnArg::Receiver(_) => {
                has_receiver = true;
            }
            FnArg::Typed(pat_type) => {
                let Pat::Ident(pat_ident) = pat_type.pat.as_ref() else {
                    unsupported_reason = Some("non-ident argument pattern".to_string());
                    continue;
                };
                let ty = normalize_type(&pat_type.ty, resolver);
                if !is_supported_arg_type(&ty, resolver) {
                    unsupported_reason = Some(format!("unsupported argument type: {ty}"));
                }
                args.push(ScannedArg {
                    name: pat_ident.ident.to_string(),
                    ty,
                });
            }
        }
    }
    if !has_receiver {
        unsupported_reason = Some("associated function is not an instance method".to_string());
    }
    let return_type = scan_return_type(&function.sig.output, resolver);
    if let ReturnKind::Unsupported(reason) = &return_type {
        unsupported_reason = Some(reason.clone());
    }
    let callable = unsupported_reason.is_none()
        && !matches!(
            return_type,
            ReturnKind::ResultFlow(_)
                | ReturnKind::Flow(_)
                | ReturnKind::StateFlow(_)
                | ReturnKind::SharedTextStream
        );
    let watchable = unsupported_reason.is_none()
        && matches!(
            return_type,
            ReturnKind::ResultFlow(_)
                | ReturnKind::Flow(_)
                | ReturnKind::StateFlow(_)
                | ReturnKind::SharedTextStream
        );
    ScannedMethod {
        name,
        args,
        return_type,
        is_async,
        callable,
        watchable,
        unsupported_reason,
    }
}

fn scan_return_type(return_type: &ReturnType, resolver: &TypeResolver) -> ReturnKind {
    match return_type {
        ReturnType::Default => ReturnKind::Unit,
        ReturnType::Type(_, ty) => {
            let normalized = normalize_type(ty, resolver);
            if normalized == "()" {
                ReturnKind::Unit
            } else if result_unit(&normalized) {
                ReturnKind::ResultUnit
            } else if let Some(inner) = result_value_inner(&normalized) {
                if let Some(flow_inner) = flow_inner(inner) {
                    if is_supported_return_type(flow_inner, resolver) {
                        ReturnKind::ResultFlow(flow_inner.to_string())
                    } else {
                        ReturnKind::Unsupported(format!(
                            "unsupported Result Flow value type: {flow_inner}"
                        ))
                    }
                } else if is_supported_return_type(inner, resolver) {
                    ReturnKind::ResultValue(inner.to_string())
                } else {
                    ReturnKind::Unsupported(format!("unsupported Result value type: {inner}"))
                }
            } else if let Some(inner) = state_flow_inner(&normalized) {
                if is_supported_return_type(inner, resolver) {
                    ReturnKind::StateFlow(inner.to_string())
                } else {
                    ReturnKind::Unsupported(format!("unsupported StateFlow value type: {inner}"))
                }
            } else if let Some(inner) = flow_inner(&normalized) {
                if is_supported_return_type(inner, resolver) {
                    ReturnKind::Flow(inner.to_string())
                } else {
                    ReturnKind::Unsupported(format!("unsupported Flow value type: {inner}"))
                }
            } else if is_shared_text_stream_return_type(&normalized) {
                ReturnKind::SharedTextStream
            } else if normalized.starts_with('&') {
                ReturnKind::Unsupported(format!("borrowed return type cannot cross link: {normalized}"))
            } else if is_supported_return_type(&normalized, resolver) {
                ReturnKind::Value(normalized)
            } else {
                ReturnKind::Unsupported(format!("unsupported return type: {normalized}"))
            }
        }
    }
}

struct TypeResolver {
    names: HashMap<String, String>,
    serializable_types: HashSet<String>,
}

impl TypeResolver {
    fn from_file(file: &syn::File, module_path: &str, serializable_types: HashSet<String>) -> Self {
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
        Self { names, serializable_types }
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

fn is_supported_arg_type(ty: &str, resolver: &TypeResolver) -> bool {
    if ty == "&str" || ty == "Option<&str>" || ty == "&std::path::Path" {
        return true;
    }
    if let Some(inner) = single_generic_arg(ty, "Option").and_then(|inner| inner.strip_prefix('&')) {
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

fn is_never_link_value_type(ty: &str) -> bool {
    ty.is_empty()
        || ty == "Self"
        || ty.starts_with('&')
        || ty.starts_with("fn(")
        || ty.starts_with("Flow<")
        || ty.starts_with("StateFlow<")
        || ty.contains("&mut")
        || ty.contains("dyn")
}

fn is_primitive_link_value_type(ty: &str) -> bool {
    matches!(
        ty,
        "()"
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
            '<' => depth += 1,
            '>' => depth -= 1,
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

fn is_shared_text_stream_return_type(ty: &str) -> bool {
    if is_shared_text_stream_type(ty) {
        return true;
    }
    single_generic_arg(ty, "Option")
        .map(is_shared_text_stream_type)
        .unwrap_or(false)
}

fn is_shared_text_stream_type(ty: &str) -> bool {
    ty == "SharedAiResponseStream"
        || ty == "operit_runtime::api::chat::llmprovider::AIService::SharedAiResponseStream"
        || ty == "DelegatingRevisableSharedTextStream"
        || ty == "operit_runtime::util::stream::RevisableTextStream::DelegatingRevisableSharedTextStream"
}

fn render_generated(objects: &[ScannedObject]) -> String {
    let schema_json = render_schema_objects(objects);
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_core_proxy_schema() -> serde_json::Value {\n");
    output.push_str("    serde_json::from_str(r#\"");
    output.push_str(&schema_json);
    output.push_str("\"#).expect(\"generated core proxy schema must be valid JSON\")\n");
    output.push_str("}\n\n");
    for object in objects {
        output.push_str(&render_object_call_dispatch(object));
        output.push('\n');
        if object.access == ObjectAccess::GitHubTokenNewConstruct {
            output.push_str(&render_object_call_dispatch_sync(object));
            output.push('\n');
        }
        output.push_str(&render_object_watch_dispatch(object));
        output.push('\n');
        output.push_str(&render_object_watch_stream_dispatch(object));
        output.push('\n');
    }
    output.push_str(&render_core_proxy_dispatch(objects));
    output.push('\n');
    output.push_str(&render_generated_proxy(objects));
    output
}

fn render_schema_objects(objects: &[ScannedObject]) -> String {
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

fn render_object_call_dispatch(object: &ScannedObject) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "async fn generated_dispatch_{}_call(object: &mut {}, request: operit_link::CoreCallRequest) -> Result<serde_json::Value, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = object_args(request.args)?;\n");
    output.push_str("    match request.methodName.as_str() {\n");
    for method in object.methods.iter().filter(|method| method.callable) {
        output.push_str(&render_call_arm(method));
    }
    if object.schema_key == "application" {
        output.push_str("        \"coreProxySchema\" => Ok(generated_core_proxy_schema()),\n");
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::methodNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

fn render_object_call_dispatch_sync(object: &ScannedObject) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_call_sync(object: &mut {}, request: operit_link::CoreCallRequest) -> Result<serde_json::Value, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = object_args(request.args)?;\n");
    output.push_str("    match request.methodName.as_str() {\n");
    for method in object.methods.iter().filter(|method| method.callable) {
        output.push_str(&render_call_arm(method));
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::methodNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

fn render_object_watch_dispatch(object: &ScannedObject) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_watch_snapshot(object: &mut {}, request: &operit_link::CoreWatchRequest) -> Result<serde_json::Value, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = object_args(request.args.clone())?;\n");
    output.push_str("    match request.propertyName.as_str() {\n");
    for method in object
        .methods
        .iter()
        .filter(|method| matches!(method.return_type, ReturnKind::Flow(_) | ReturnKind::StateFlow(_)))
    {
        output.push_str(&render_watch_arm(method));
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::watchNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

fn render_object_watch_stream_dispatch(object: &ScannedObject) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str(&format!(
        "fn generated_dispatch_{}_watch(object: &mut {}, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {{\n",
        object.dispatch_name, object.full_type
    ));
    output.push_str("    let registryKey = request.registryKey();\n");
    output.push_str("    let mut __core_args = object_args(request.args.clone())?;\n");
    output.push_str("    match request.propertyName.as_str() {\n");
    for method in object.methods.iter().filter(|method| method.watchable) {
        output.push_str(&render_watch_stream_arm(method));
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::watchNotFound(&registryKey)),\n");
    output.push_str("    }\n}\n");
    output
}

fn render_core_proxy_dispatch(objects: &[ScannedObject]) -> String {
    let mut output = String::new();
    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("async fn generated_dispatch_core_proxy_call(proxy: &mut LocalCoreProxy, request: operit_link::CoreCallRequest) -> Result<serde_json::Value, operit_link::CoreLinkError> {\n");
    if let Some(application) = objects.iter().find(|object| object.access == ObjectAccess::Application) {
        output.push_str(&format!(
            "    if request.targetPath.key() == {:?} {{\n        return generated_dispatch_{}_call(&mut proxy.application, request).await;\n    }}\n",
            application.schema_key, application.dispatch_name
        ));
    }
    if let Some(chat_runtime) = objects.iter().find(|object| object.access == ObjectAccess::ChatRuntimeMain) {
        output.push_str(&format!(
            "    if let Some(slot) = chat_runtime_slot(&request.targetPath) {{\n        let core = proxy.application.chatRuntimeHolder.getCore(slot);\n        return generated_dispatch_{}_call(core, request).await;\n    }}\n",
            chat_runtime.dispatch_name
        ));
    }
    output.push_str("    match request.targetPath.key().as_str() {\n");
    for object in objects.iter().filter(|object| object.access == ObjectAccess::StringNewConstruct) {
        output.push_str(&render_string_constructible_dispatch(object, "call"));
    }
    for object in objects
        .iter()
        .filter(|object| object.access.is_constructible() && object.access != ObjectAccess::StringNewConstruct)
    {
        let dispatch = if object.access == ObjectAccess::GitHubTokenNewConstruct {
            format!(
                "            std::thread::spawn(move || {{\n                let __core_github_token = operit_runtime::data::preferences::GitHubAuthPreferences::GitHubAuthPreferences::getInstance().getCurrentAccessToken();\n                let mut object = {}::newWithGitHubToken(__core_github_token);\n                generated_dispatch_{}_call_sync(&mut object, request)\n            }}).join().map_err(|error| operit_link::CoreLinkError::internal(format!(\"market service call panicked: {{:?}}\", error)))?\n",
                object.full_type, object.dispatch_name
            )
        } else {
            format!(
                "            generated_dispatch_{}_call(&mut object, request).await\n",
                object.dispatch_name
            )
        };
        output.push_str(&format!(
            "        {:?} => {{\n{}{}        }}\n",
            object.schema_key,
            if object.access == ObjectAccess::GitHubTokenNewConstruct {
                String::new()
            } else {
                render_object_constructor(object)
            },
            dispatch
        ));
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::methodNotFound(&request.registryKey())),\n");
    output.push_str("    }\n}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_dispatch_core_proxy_watch_snapshot(proxy: &mut LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEvent, operit_link::CoreLinkError> {\n");
    if let Some(chat_runtime) = objects.iter().find(|object| object.access == ObjectAccess::ChatRuntimeMain) {
        output.push_str(&format!(
            "    if let Some(slot) = chat_runtime_slot(&request.targetPath) {{\n        let propertyName = request.propertyName.clone();\n        let core = proxy.application.chatRuntimeHolder.getCore(slot);\n        let value = generated_dispatch_{}_watch_snapshot(core, &request)?;\n        return Ok(operit_link::CoreEvent {{ requestId: Some(request.requestId), targetPath: request.targetPath, propertyName, kind: operit_link::CoreEventKind::Snapshot, value }});\n    }}\n",
            chat_runtime.dispatch_name
        ));
    }
    output.push_str("    let propertyName = request.propertyName.clone();\n");
    output.push_str("    let value = match request.targetPath.key().as_str() {\n");
    if let Some(application) = objects.iter().find(|object| object.access == ObjectAccess::Application) {
        output.push_str(&format!(
            "        {:?} => generated_dispatch_{}_watch_snapshot(&mut proxy.application, &request)?,\n",
            application.schema_key, application.dispatch_name
        ));
    }
    for object in objects.iter().filter(|object| object.access == ObjectAccess::StringNewConstruct) {
        output.push_str(&render_string_constructible_dispatch(object, "watch_snapshot"));
    }
    for object in objects
        .iter()
        .filter(|object| object.access.is_constructible() && object.access != ObjectAccess::StringNewConstruct)
    {
        let dispatch = if object.access == ObjectAccess::GitHubTokenNewConstruct {
            format!(
                "            let __core_result = generated_dispatch_{}_watch_snapshot(&mut object, &request)?;\n            std::mem::forget(object);\n            __core_result\n",
                object.dispatch_name
            )
        } else {
            format!(
                "            generated_dispatch_{}_watch_snapshot(&mut object, &request)?\n",
                object.dispatch_name
            )
        };
        output.push_str(&format!(
            "        {:?} => {{\n{}{}        }}\n",
            object.schema_key,
            render_object_constructor(object),
            dispatch
        ));
    }
    output.push_str("        _ => return Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),\n");
    output.push_str("    };\n");
    output.push_str("    Ok(operit_link::CoreEvent { requestId: Some(request.requestId), targetPath: request.targetPath, propertyName, kind: operit_link::CoreEventKind::Snapshot, value })\n");
    output.push_str("}\n\n");

    output.push_str("#[allow(unused_mut, unused_variables)]\n");
    output.push_str("fn generated_dispatch_core_proxy_watch(proxy: &mut LocalCoreProxy, request: operit_link::CoreWatchRequest) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {\n");
    if let Some(chat_runtime) = objects.iter().find(|object| object.access == ObjectAccess::ChatRuntimeMain) {
        output.push_str(&format!(
            "    if let Some(slot) = chat_runtime_slot(&request.targetPath) {{\n        let core = proxy.application.chatRuntimeHolder.getCore(slot);\n        return generated_dispatch_{}_watch(core, request);\n    }}\n",
            chat_runtime.dispatch_name
        ));
    }
    output.push_str("    match request.targetPath.key().as_str() {\n");
    if let Some(application) = objects.iter().find(|object| object.access == ObjectAccess::Application) {
        output.push_str(&format!(
            "        {:?} => generated_dispatch_{}_watch(&mut proxy.application, request),\n",
            application.schema_key, application.dispatch_name
        ));
    }
    for object in objects.iter().filter(|object| object.access == ObjectAccess::StringNewConstruct) {
        output.push_str(&render_string_constructible_dispatch(object, "watch"));
    }
    for object in objects
        .iter()
        .filter(|object| object.access.is_constructible() && object.access != ObjectAccess::StringNewConstruct)
    {
        let dispatch = if object.access == ObjectAccess::GitHubTokenNewConstruct {
            format!(
                "            let __core_result = generated_dispatch_{}_watch(&mut object, request);\n            std::mem::forget(object);\n            __core_result\n",
                object.dispatch_name
            )
        } else {
            format!(
                "            generated_dispatch_{}_watch(&mut object, request)\n",
                object.dispatch_name
            )
        };
        output.push_str(&format!(
            "        {:?} => {{\n{}{}        }}\n",
            object.schema_key,
            render_object_constructor(object),
            dispatch
        ));
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),\n");
    output.push_str("    }\n}\n");
    output
}

fn render_string_constructible_dispatch(object: &ScannedObject, mode: &str) -> String {
    let base_segments = object.schema_key.split('.').collect::<Vec<_>>();
    let len = base_segments.len();
    let segment_checks = base_segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            format!("request.targetPath.segments.get({index}).map(String::as_str) == Some({segment:?})")
        })
        .collect::<Vec<_>>()
        .join(" && ");
    let constructor = render_object_constructor(object);
    let dispatch = match mode {
        "call" => format!(
            "            return generated_dispatch_{}_call(&mut object, request).await;\n",
            object.dispatch_name
        ),
        "watch_snapshot" => format!(
            "            generated_dispatch_{}_watch_snapshot(&mut object, &request)?\n",
            object.dispatch_name
        ),
        "watch" => format!(
            "            return generated_dispatch_{}_watch(&mut object, request);\n",
            object.dispatch_name
        ),
        _ => String::new(),
    };
    format!(
        "        _ if request.targetPath.segments.len() == {} && {} => {{\n{}{}        }}\n",
        len + 1,
        segment_checks,
        constructor,
        dispatch
    )
}

fn render_object_constructor(object: &ScannedObject) -> String {
    match object.access {
        ObjectAccess::DefaultConstruct => {
            format!("            let mut object = {}::default();\n", object.full_type)
        }
        ObjectAccess::GetInstanceConstruct => {
            format!("            let mut object = {}::getInstance();\n", object.full_type)
        }
        ObjectAccess::ResultGetInstanceConstruct => {
            format!(
                "            let mut object = {}::getInstance().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n",
                object.full_type
            )
        }
        ObjectAccess::NewConstruct => {
            format!("            let mut object = {}::new();\n", object.full_type)
        }
        ObjectAccess::GitHubTokenNewConstruct => {
            format!(
                "            let __core_github_token = operit_runtime::data::preferences::GitHubAuthPreferences::GitHubAuthPreferences::getInstance().getCurrentAccessToken();\n            let mut object = {}::newWithGitHubToken(__core_github_token);\n",
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
        ObjectAccess::Application | ObjectAccess::ChatRuntimeMain => String::new(),
    }
}

fn render_schema_methods(methods: &[ScannedMethod]) -> String {
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
                "{{\"name\":{},\"args\":[{}],\"async\":{},\"callable\":{},\"watchable\":{},\"returnType\":{},\"unsupportedReason\":{}}}",
                json_string(&method.name),
                args,
                method.is_async,
                method.callable,
                method.watchable,
                json_string(&return_type_label(&method.return_type)),
                option_json_string(method.unsupported_reason.as_deref())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn render_generated_proxy(objects: &[ScannedObject]) -> String {
    let mut output = String::new();
    output.push_str("pub struct GeneratedCoreProxy<C> {\n");
    output.push_str("    client: C,\n");
    output.push_str("}\n\n");
    output.push_str("impl<C: operit_link::CoreLinkClient> GeneratedCoreProxy<C> {\n");
    output.push_str("    pub fn new(client: C) -> Self {\n");
    output.push_str("        Self { client }\n");
    output.push_str("    }\n\n");
    output.push_str("    pub fn intoInner(self) -> C {\n");
    output.push_str("        self.client\n");
    output.push_str("    }\n\n");
    output.push_str("    pub fn clientMut(&mut self) -> &mut C {\n");
    output.push_str("        &mut self.client\n");
    output.push_str("    }\n\n");
    for object in objects {
        let proxy_type = proxy_object_type_name(object);
        if object.access == ObjectAccess::StringNewConstruct {
            output.push_str(&format!(
                "    pub fn {}(&mut self, instanceId: &str) -> {}<'_, C> {{\n",
                object.dispatch_name, proxy_type
            ));
            let segments = object
                .schema_key
                .split('.')
                .map(|segment| format!("{segment:?}.to_string()"))
                .collect::<Vec<_>>()
                .join(", ");
            output.push_str("        let mut segments = vec![");
            output.push_str(&segments);
            output.push_str("];\n");
            output.push_str("        segments.push(instanceId.to_string());\n");
            output.push_str(&format!(
                "        {}::new(&mut self.client, operit_link::CoreObjectPath {{ segments }})\n",
                proxy_type
            ));
        } else {
            output.push_str(&format!(
                "    pub fn {}(&mut self) -> {}<'_, C> {{\n",
                object.dispatch_name, proxy_type
            ));
            output.push_str(&format!(
                "        {}::new(&mut self.client, operit_link::CoreObjectPath::parse({:?}))\n",
                proxy_type, object.schema_key
            ));
        }
        output.push_str("    }\n\n");
    }
    output.push_str("}\n\n");
    for object in objects {
        let proxy_type = proxy_object_type_name(object);
        output.push_str(&format!("pub struct {}<'a, C> {{\n", proxy_type));
        output.push_str("    client: &'a mut C,\n");
        output.push_str("    target_path: operit_link::CoreObjectPath,\n");
        output.push_str("}\n\n");
        output.push_str(&format!("impl<'a, C: operit_link::CoreLinkClient> {}<'a, C> {{\n", proxy_type));
        output.push_str("    fn new(client: &'a mut C, target_path: operit_link::CoreObjectPath) -> Self {\n");
        output.push_str("        Self { client, target_path }\n");
        output.push_str("    }\n\n");
        output.push_str("    async fn callGenerated<T: serde::de::DeserializeOwned>(&mut self, methodName: &str, args: serde_json::Value) -> Result<T, operit_link::CoreLinkError> {\n");
        output.push_str(&format!(
            "        let response = self.client.call(operit_link::CoreCallRequest::new(generated_proxy_request_id(), self.target_path.clone(), methodName, args)).await;\n"
        ));
        output.push_str("        let value = response.result?;\n");
        output.push_str("        serde_json::from_value(value).map_err(|error| operit_link::CoreLinkError::new(\"INVALID_RESPONSE\", error.to_string()))\n");
        output.push_str("    }\n\n");
        output.push_str("    async fn callGeneratedUnit(&mut self, methodName: &str, args: serde_json::Value) -> Result<(), operit_link::CoreLinkError> {\n");
        output.push_str(&format!(
            "        let response = self.client.call(operit_link::CoreCallRequest::new(generated_proxy_request_id(), self.target_path.clone(), methodName, args)).await;\n"
        ));
        output.push_str("        response.result.map(|_| ())\n");
        output.push_str("    }\n\n");
        output.push_str("    async fn watchGenerated<T: serde::de::DeserializeOwned>(&mut self, propertyName: &str, args: serde_json::Value) -> Result<T, operit_link::CoreLinkError> {\n");
        output.push_str(&format!(
            "        let event = self.client.watchSnapshot(operit_link::CoreWatchRequest::new(generated_proxy_request_id(), self.target_path.clone(), propertyName, args)).await?;\n"
        ));
        output.push_str("        serde_json::from_value(event.value).map_err(|error| operit_link::CoreLinkError::new(\"INVALID_RESPONSE\", error.to_string()))\n");
        output.push_str("    }\n\n");
        for method in object.methods.iter().filter(|method| method.callable) {
            output.push_str(&render_proxy_call_method(method));
        }
        for method in object
            .methods
            .iter()
            .filter(|method| {
                matches!(
                    method.return_type,
                    ReturnKind::ResultFlow(_) | ReturnKind::Flow(_) | ReturnKind::StateFlow(_)
                )
            })
        {
            output.push_str(&render_proxy_watch_method(object, method));
        }
        for method in object
            .methods
            .iter()
            .filter(|method| matches!(method.return_type, ReturnKind::SharedTextStream))
        {
            output.push_str(&render_proxy_stream_watch_method(object, method));
        }
        output.push_str(&render_proxy_watch_all_method(object));
        output.push_str("}\n\n");
    }
    output
}

fn render_proxy_watch_all_method(object: &ScannedObject) -> String {
    let watchable_methods = object
        .methods
        .iter()
        .filter(|method| is_argless_state_flow_or_flow(method))
        .collect::<Vec<_>>();
    if watchable_methods.is_empty() {
        return "    pub async fn watchAllGeneratedStateFlows(&mut self, _sender: tokio::sync::mpsc::UnboundedSender<operit_link::CoreEvent>) -> Result<(), operit_link::CoreLinkError> {\n        Ok(())\n    }\n\n".to_string();
    }
    let watchable = watchable_methods
        .iter()
        .map(|method| json_string(&method.name))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "    pub async fn watchAllGeneratedStateFlows(&mut self, sender: tokio::sync::mpsc::UnboundedSender<operit_link::CoreEvent>) -> Result<(), operit_link::CoreLinkError> {{\n        for propertyName in [{}] {{\n            let request = operit_link::CoreWatchRequest::new(generated_proxy_request_id(), {:?}, propertyName, serde_json::json!({{}}));\n            let mut stream = self.client.watch(request).await?;\n            let sender = sender.clone();\n            tokio::spawn(async move {{\n                while let Some(event) = stream.recv().await {{\n                    let _ = sender.send(event);\n                }}\n            }});\n        }}\n        Ok(())\n    }}\n\n",
        watchable,
        object.schema_key
    )
}

fn proxy_object_type_name(object: &ScannedObject) -> String {
    let mut out = String::from("GeneratedCoreProxy");
    for part in object.dispatch_name.split('_') {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars);
        }
    }
    out
}

fn render_proxy_call_method(method: &ScannedMethod) -> String {
    let params = render_proxy_params(method);
    let args_json = render_proxy_args_json(method);
    match &method.return_type {
        ReturnKind::Unit | ReturnKind::ResultUnit => format!(
            "    pub async fn {}(&mut self{}) -> Result<(), operit_link::CoreLinkError> {{\n        self.callGeneratedUnit({:?}, {}).await\n    }}\n\n",
            method.name, params, method.name, args_json
        ),
        ReturnKind::ResultValue(value) | ReturnKind::Value(value) => format!(
            "    pub async fn {}(&mut self{}) -> Result<{}, operit_link::CoreLinkError> {{\n        self.callGenerated({:?}, {}).await\n    }}\n\n",
            method.name, params, value, method.name, args_json
        ),
        ReturnKind::ResultFlow(_) => String::new(),
        ReturnKind::Flow(_)
        | ReturnKind::StateFlow(_)
        | ReturnKind::SharedTextStream
        | ReturnKind::Unsupported(_) => String::new(),
    }
}

fn render_proxy_watch_method(object: &ScannedObject, method: &ScannedMethod) -> String {
    let value = match &method.return_type {
        ReturnKind::ResultFlow(value) | ReturnKind::Flow(value) | ReturnKind::StateFlow(value) => value,
        _ => return String::new(),
    };
    let params = render_proxy_params(method);
    let args_json = render_proxy_args_json(method);
    let mut output = format!(
        "    pub async fn {}Snapshot(&mut self{}) -> Result<{}, operit_link::CoreLinkError> {{\n        self.watchGenerated({:?}, {}).await\n    }}\n\n",
        method.name, params, value, method.name, args_json
    );
    let Some(alias) = method.name.strip_suffix("Flow") else {
        return output;
    };
    if alias.is_empty()
        || object
            .methods
            .iter()
            .any(|existing| existing.name == alias)
    {
        return output;
    }
    output.push_str(&format!(
        "    pub async fn {}(&mut self{}) -> Result<{}, operit_link::CoreLinkError> {{\n        self.watchGenerated({:?}, {}).await\n    }}\n\n",
        alias, params, value, method.name, args_json
    ));
    output
}

fn state_flow_or_flow_value(method: &ScannedMethod) -> Option<&str> {
    match &method.return_type {
        ReturnKind::ResultFlow(value) | ReturnKind::Flow(value) | ReturnKind::StateFlow(value) => Some(value),
        _ => None,
    }
}

fn is_state_flow_or_flow(method: &ScannedMethod) -> bool {
    state_flow_or_flow_value(method).is_some()
}

fn is_argless_state_flow_or_flow(method: &ScannedMethod) -> bool {
    method.args.is_empty() && is_state_flow_or_flow(method)
}

fn render_proxy_stream_watch_method(object: &ScannedObject, method: &ScannedMethod) -> String {
    let params = render_proxy_params(method);
    let args_json = render_proxy_args_json(method);
    format!(
        "    pub async fn {}(&mut self{}) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {{\n        self.client.watch(operit_link::CoreWatchRequest::new(generated_proxy_request_id(), self.target_path.clone(), {:?}, {})).await\n    }}\n\n",
        method.name, params, method.name, args_json
    )
}

fn render_proxy_params(method: &ScannedMethod) -> String {
    if method.args.is_empty() {
        return String::new();
    }
    let params = method
        .args
        .iter()
        .map(|arg| format!("{}: {}", arg.name, render_proxy_arg_type(arg)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(", {params}")
}

fn render_proxy_arg_type(arg: &ScannedArg) -> &str {
    &arg.ty
}

fn render_proxy_args_json(method: &ScannedMethod) -> String {
    if method.args.is_empty() {
        return "serde_json::json!({})".to_string();
    }
    let entries = method
        .args
        .iter()
        .map(|arg| format!("{:?}: {}", arg.name, render_proxy_arg_json_expr(arg)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("serde_json::json!({{{entries}}})")
}

fn render_proxy_arg_json_expr(arg: &ScannedArg) -> String {
    if arg.ty == "&std::path::Path" {
        format!("{}.to_string_lossy().to_string()", arg.name)
    } else {
        arg.name.clone()
    }
}

fn render_call_arm(method: &ScannedMethod) -> String {
    let args = render_arg_decoders(method);
    let call_args = method
        .args
        .iter()
        .map(render_arg_call_expr)
        .collect::<Vec<_>>()
        .join(", ");
    match method.return_type {
        ReturnKind::Unit => format!(
            "        {:?} => {{\n{}            object.{}({}){};\n            Ok(serde_json::Value::Null)\n        }}\n",
            method.name, args, method.name, call_args, await_suffix(method)
        ),
        ReturnKind::ResultUnit => format!(
            "        {:?} => {{\n{}            object.{}({}){}.map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?;\n            Ok(serde_json::Value::Null)\n        }}\n",
            method.name, args, method.name, call_args, await_suffix(method)
        ),
        ReturnKind::ResultValue(_) => format!(
            "        {:?} => {{\n{}            to_core_value(object.{}({}){}.map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?)\n        }}\n",
            method.name, args, method.name, call_args, await_suffix(method)
        ),
        ReturnKind::ResultFlow(_) => format!(
            "        {:?} => {{\n{}            to_core_value(object.{}({}){}.map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?.first().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?)\n        }}\n",
            method.name, args, method.name, call_args, await_suffix(method)
        ),
        ReturnKind::Value(_) => format!(
            "        {:?} => {{\n{}            to_core_value(object.{}({}){})\n        }}\n",
            method.name, args, method.name, call_args, await_suffix(method)
        ),
        ReturnKind::Flow(_)
        | ReturnKind::StateFlow(_)
        | ReturnKind::SharedTextStream
        | ReturnKind::Unsupported(_) => String::new(),
    }
}

fn await_suffix(method: &ScannedMethod) -> &'static str {
    if method.is_async {
        ".await"
    } else {
        ""
    }
}

fn render_watch_arm(method: &ScannedMethod) -> String {
    let args = render_arg_decoders(method);
    let call_args = method
        .args
        .iter()
        .map(render_arg_call_expr)
        .collect::<Vec<_>>()
        .join(", ");
    match method.return_type {
        ReturnKind::ResultFlow(_) => format!(
            "        {:?} => {{\n{}            to_core_value(object.{}({}).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?.first().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?)\n        }}\n",
            method.name, args, method.name, call_args
        ),
        ReturnKind::Flow(_) => format!(
            "        {:?} => {{\n{}            to_core_value(object.{}({}).first().map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?)\n        }}\n",
            method.name, args, method.name, call_args
        ),
        ReturnKind::StateFlow(_) => format!(
            "        {:?} => {{\n{}            to_core_value(object.{}({}).value())\n        }}\n",
            method.name, args, method.name, call_args
        ),
        _ => String::new(),
    }
}

fn render_watch_stream_arm(method: &ScannedMethod) -> String {
    match method.return_type {
        ReturnKind::ResultFlow(_) | ReturnKind::Flow(_) | ReturnKind::StateFlow(_) => {
            let args = render_arg_decoders(method);
            let call_args = method
                .args
                .iter()
                .map(render_arg_call_expr)
                .collect::<Vec<_>>()
                .join(", ");
            let flow_expr = if matches!(method.return_type, ReturnKind::ResultFlow(_)) {
                format!(
                    "object.{}({}).map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))?",
                    method.name, call_args
                )
            } else {
                format!("object.{}({})", method.name, call_args)
            };
            format!(
                "        {:?} => {{\n{}            let flow = {};\n            let (sender, receiver) = core_event_stream_channel();\n            let requestId = request.requestId;\n            let targetPath = request.targetPath;\n            let propertyName = request.propertyName;\n            std::thread::spawn(move || {{\n                let isFirstEvent = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));\n                let _ = flow.collect(|value| {{\n                    let kind = if isFirstEvent.swap(false, std::sync::atomic::Ordering::SeqCst) {{\n                        operit_link::CoreEventKind::Snapshot\n                    }} else {{\n                        operit_link::CoreEventKind::Changed\n                    }};\n                    if let Ok(value) = serde_json::to_value(value) {{\n                        let _ = sender.send(operit_link::CoreEvent {{\n                            requestId: Some(requestId.clone()),\n                            targetPath: targetPath.clone(),\n                            propertyName: propertyName.clone(),\n                            kind,\n                            value,\n                        }});\n                    }}\n                }});\n            }});\n            Ok(receiver)\n        }}\n",
                method.name, args, flow_expr
            )
        }
        ReturnKind::SharedTextStream => render_shared_text_stream_watch_arm(method),
        _ => String::new(),
    }
}

fn render_shared_text_stream_watch_arm(method: &ScannedMethod) -> String {
    let args = render_arg_decoders(method);
    let call_args = method
        .args
        .iter()
        .map(render_arg_call_expr)
        .collect::<Vec<_>>()
        .join(", ");
    let chat_id_expr = method
        .args
        .iter()
        .find(|arg| arg.name == "chatId" || arg.name == "chat_id")
        .map(|arg| arg.name.clone())
        .unwrap_or_else(|| "\"\".to_string()".to_string());
    format!(
        "        {:?} => {{\n{}            let streamChatId = {}.clone();\n            let stream = object.{}({}).ok_or_else(|| operit_link::CoreLinkError::watchNotFound(&registryKey))?;\n            let mut textStream = stream.clone();\n            let mut eventStream = operit_runtime::util::stream::RevisableTextStream::TextStreamEventCarrier::event_channel(&stream).clone();\n            let (sender, receiver) = core_event_stream_channel();\n            let requestId = request.requestId;\n            let targetPath = request.targetPath;\n            let propertyName = request.propertyName;\n            let eventSender = sender.clone();\n            let eventRequestId = requestId.clone();\n            let eventTargetPath = targetPath.clone();\n            let eventPropertyName = propertyName.clone();\n            let eventChatId = streamChatId.clone();\n            std::thread::spawn(move || {{\n                operit_runtime::util::stream::Stream::Stream::collect(&mut eventStream, &mut |event| {{\n                    let eventType = match event.event_type {{\n                        operit_runtime::util::stream::RevisableTextStream::TextStreamEventType::Savepoint => \"savepoint\",\n                        operit_runtime::util::stream::RevisableTextStream::TextStreamEventType::Rollback => \"rollback\",\n                    }};\n                    let value = serde_json::json!({{\"chatId\": eventChatId, \"type\": eventType, \"id\": event.id}});\n                    let _ = eventSender.send(operit_link::CoreEvent {{\n                        requestId: Some(eventRequestId.clone()),\n                        targetPath: eventTargetPath.clone(),\n                        propertyName: eventPropertyName.clone(),\n                        kind: operit_link::CoreEventKind::Changed,\n                        value,\n                    }});\n                }});\n            }});\n            std::thread::spawn(move || {{\n                operit_runtime::util::stream::Stream::Stream::collect(&mut textStream, &mut |chunk| {{\n                    let value = serde_json::json!({{\"chatId\": streamChatId, \"type\": \"chunk\", \"value\": chunk}});\n                    let _ = sender.send(operit_link::CoreEvent {{\n                        requestId: Some(requestId.clone()),\n                        targetPath: targetPath.clone(),\n                        propertyName: propertyName.clone(),\n                        kind: operit_link::CoreEventKind::Changed,\n                        value,\n                    }});\n                }});\n                let value = serde_json::json!({{\"chatId\": streamChatId, \"type\": \"completed\"}});\n                let _ = sender.send(operit_link::CoreEvent {{\n                    requestId: Some(requestId),\n                    targetPath,\n                    propertyName,\n                    kind: operit_link::CoreEventKind::Completed,\n                    value,\n                }});\n            }});\n            Ok(receiver)\n        }}\n",
        method.name, args, chat_id_expr, method.name, call_args
    )
}

fn render_arg_decoders(method: &ScannedMethod) -> String {
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

fn render_arg_decode_type(arg: &ScannedArg) -> String {
    if arg.ty == "&str" {
        "String".to_string()
    } else if arg.ty == "Option<&str>" {
        "Option<String>".to_string()
    } else if let Some(inner) = single_generic_arg(&arg.ty, "Option").and_then(|inner| inner.strip_prefix('&')) {
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

fn render_arg_call_expr(arg: &ScannedArg) -> String {
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

fn return_type_label(kind: &ReturnKind) -> String {
    match kind {
        ReturnKind::Unit => "()".to_string(),
        ReturnKind::ResultUnit => "Result<(), String>".to_string(),
        ReturnKind::ResultValue(value) => format!("Result<{value}, _>"),
        ReturnKind::ResultFlow(value) => format!("Result<Flow<{value}>, _>"),
        ReturnKind::Value(value) => value.clone(),
        ReturnKind::Flow(value) => format!("Flow<{value}>"),
        ReturnKind::StateFlow(value) => format!("StateFlow<{value}>"),
        ReturnKind::SharedTextStream => "SharedAiResponseStream".to_string(),
        ReturnKind::Unsupported(value) => value.clone(),
    }
}

fn json_string(value: &str) -> String {
    serde_json_escape(value)
}

fn option_json_string(value: Option<&str>) -> String {
    match value {
        Some(value) => serde_json_escape(value),
        None => "null".to_string(),
    }
}

fn serde_json_escape(value: &str) -> String {
    let mut result = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            other => result.push(other),
        }
    }
    result.push('"');
    result
}
