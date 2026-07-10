use super::*;

pub(crate) fn object_specs(
    runtime_root: &SourceRoot,
    store_root: &SourceRoot,
    tools_root: &SourceRoot,
    providers_root: &SourceRoot,
) -> Vec<ObjectSpec> {
    let mut specs = Vec::new();
    specs.push(required_object_spec(
        runtime_root,
        "application",
        "core/application/OperitApplication.rs",
        "OperitApplication",
        ObjectAccess::Application,
    ));
    specs.push(required_object_spec(
        runtime_root,
        "chatRuntimeHolder.main",
        "services/ChatServiceCore.rs",
        "ChatServiceCore",
        ObjectAccess::ChatRuntimeMain,
    ));
    specs.extend(discover_constructible_objects(
        runtime_root,
        "data/preferences",
        "preferences",
    ));
    specs.extend(discover_constructible_objects(
        runtime_root,
        "services",
        "services",
    ));
    specs.extend(discover_constructible_objects(
        store_root,
        "repository",
        "repository",
    ));
    specs.extend(discover_constructible_objects_recursive(
        tools_root,
        "tools",
        "permissions",
    ));
    specs.extend(discover_constructible_objects_recursive(
        runtime_root,
        "plugins",
        "plugins",
    ));
    specs.extend(discover_constructible_objects_recursive(
        providers_root,
        "chat",
        "providers.chat",
    ));
    specs.extend(discover_constructible_objects_recursive(
        providers_root,
        "market",
        "providers.market",
    ));
    specs.extend(discover_constructible_objects_recursive(
        providers_root,
        "voice",
        "providers.voice",
    ));
    specs.sort_by(|left, right| left.schema_key.cmp(&right.schema_key));
    specs
}

pub(crate) fn collect_public_object_types(
    source_roots: &[SourceRoot],
) -> HashMap<String, PublicObjectType> {
    let mut out = HashMap::new();
    for source_root in source_roots {
        collect_public_object_types_from_dir(source_root, source_root.as_path(), &mut out);
    }
    out
}

fn collect_public_object_types_from_dir(
    source_root: &SourceRoot,
    dir: &Path,
    out: &mut HashMap<String, PublicObjectType>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_public_object_types_from_dir(source_root, &path, out);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read runtime source");
        let file = syn::parse_file(&content).expect("parse runtime source");
        for item in &file.items {
            let Item::Struct(item_struct) = item else {
                continue;
            };
            if !matches!(item_struct.vis, Visibility::Public(_))
                || !item_struct.generics.params.is_empty()
            {
                continue;
            }
            let type_name = item_struct.ident.to_string();
            let full_type = full_type_for_source_with_crate(
                source_root.as_path(),
                &path,
                &type_name,
                &source_root.crate_name,
            );
            out.insert(
                full_type.clone(),
                PublicObjectType {
                    type_name,
                    full_type,
                    source_path: path.clone(),
                },
            );
        }
    }
}

pub(crate) fn discover_factory_object_specs(
    objects: &[SourceObject],
    object_specs: &[ObjectSpec],
    public_object_types: &HashMap<String, PublicObjectType>,
    serializable_types: &HashSet<String>,
    deserializable_types: &HashSet<String>,
    type_registry: &TypeRegistry,
) -> Vec<ObjectSpec> {
    let spec_by_schema = object_specs
        .iter()
        .map(|spec| (spec.schema_key.clone(), spec))
        .collect::<HashMap<_, _>>();
    let mut specs = Vec::new();
    let mut seen = HashSet::new();
    for object in objects {
        let Some(parent_spec) = spec_by_schema.get(&object.schema_key) else {
            continue;
        };
        if !parent_spec.access.supports_factory_methods() {
            continue;
        }
        for method in &object.methods {
            let Some((returned_type, returns_result, returns_arc_mutex)) =
                factory_returned_object_type(&method.rust_return_type)
            else {
                continue;
            };
            if serializable_types.contains(returned_type)
                || deserializable_types.contains(returned_type)
            {
                continue;
            }
            let Some(target_type) = public_object_types.get(returned_type) else {
                continue;
            };
            if !method
                .args
                .iter()
                .all(|arg| is_factory_path_arg_type(&arg.ty))
            {
                continue;
            }
            let target_methods = scan_methods(
                &target_type.source_path,
                &target_type.type_name,
                parent_module_path(&target_type.full_type),
                serializable_types,
                deserializable_types,
                type_registry,
            );
            if !has_proxyable_instance_methods(&target_methods) {
                continue;
            }
            let schema_key = format!("{}.{}", object.schema_key, method.name);
            if !seen.insert(schema_key.clone()) {
                continue;
            }
            specs.push(ObjectSpec {
                dispatch_name: dispatch_name_from_schema_key(&schema_key),
                schema_key,
                type_name: target_type.type_name.clone(),
                full_type: target_type.full_type.clone(),
                source_path: target_type.source_path.clone(),
                access: ObjectAccess::FactoryMethodConstruct {
                    parent_schema_key: parent_spec.schema_key.clone(),
                    parent_full_type: parent_spec.full_type.clone(),
                    parent_access: Box::new(parent_spec.access.clone()),
                    factory_method: method.name.clone(),
                    factory_arg_types: method.args.iter().map(|arg| arg.ty.clone()).collect(),
                    returns_result,
                    returns_arc_mutex,
                },
            });
        }
    }
    specs
}

/// Resolves the object type and synchronization wrapper returned by a factory method.
fn factory_returned_object_type(return_type: &str) -> Option<(&str, bool, bool)> {
    let (value_type, returns_result) = match generic_args(return_type, "Result") {
        Some(arguments) if arguments.len() == 2 => (arguments[0], true),
        Some(_) => return None,
        None => (return_type, false),
    };
    let Some(arc_inner) = single_generic_arg(value_type, "Arc") else {
        return Some((value_type, returns_result, false));
    };
    let mutex_inner = single_generic_arg(arc_inner, "Mutex")?;
    Some((mutex_inner, returns_result, true))
}

pub(crate) fn has_proxyable_instance_methods(methods: &[SourceMethod]) -> bool {
    methods
        .iter()
        .any(|method| method.call_protocol().is_some() || method.watch_protocol().is_some())
}

pub(crate) fn mark_factory_methods(objects: &mut [SourceObject], factory_specs: &[ObjectSpec]) {
    for object in objects {
        for method in &mut object.methods {
            let schema_key = format!("{}.{}", object.schema_key, method.name);
            if factory_specs
                .iter()
                .any(|spec| spec.schema_key == schema_key)
            {
                method.protocol = MethodProtocol::Factory(FactoryProtocol {
                    target_schema_key: schema_key,
                });
            }
        }
    }
}

fn is_factory_path_arg_type(ty: &str) -> bool {
    matches!(ty, "&str" | "String")
}

fn required_object_spec(
    source_root: &SourceRoot,
    schema_key: &str,
    relative_path: &str,
    type_name: &str,
    access: ObjectAccess,
) -> ObjectSpec {
    let source_path = source_root.as_path().join(relative_path);
    ObjectSpec {
        schema_key: schema_key.to_string(),
        dispatch_name: dispatch_name_from_schema_key(schema_key),
        type_name: type_name.to_string(),
        full_type: full_type_for_source_with_crate(
            source_root.as_path(),
            &source_path,
            type_name,
            &source_root.crate_name,
        ),
        source_path,
        access,
    }
}

fn discover_constructible_objects(
    source_root: &SourceRoot,
    relative_dir: &str,
    schema_prefix: &str,
) -> Vec<ObjectSpec> {
    let dir = source_root.as_path().join(relative_dir);
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
            full_type: full_type_for_source_with_crate(
                source_root.as_path(),
                &path,
                &type_name,
                &source_root.crate_name,
            ),
            type_name,
            source_path: path,
            access,
        });
    }
    specs
}

fn discover_constructible_objects_recursive(
    source_root: &SourceRoot,
    relative_dir: &str,
    schema_prefix: &str,
) -> Vec<ObjectSpec> {
    let dir = source_root.as_path().join(relative_dir);
    let mut specs = Vec::new();
    discover_constructible_objects_recursive_inner(
        source_root,
        &dir,
        &dir,
        schema_prefix,
        &mut specs,
    );
    specs
}

fn discover_constructible_objects_recursive_inner(
    source_root: &SourceRoot,
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
                source_root,
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
            full_type: full_type_for_source_with_crate(
                source_root.as_path(),
                &path,
                &type_name,
                &source_root.crate_name,
            ),
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
        let mut has_result_context_get_instance = false;
        let mut has_result_context_ref_get_instance = false;
        let mut has_context_get_instance_arc_mutex = false;
        let mut has_context_ref_get_instance_arc_mutex = false;
        let mut has_store_paths_new = false;
        let mut has_result_store_paths_new = false;
        let mut has_public_instance_method = false;

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
                has_public_instance_method |= function
                    .sig
                    .inputs
                    .iter()
                    .any(|input| matches!(input, FnArg::Receiver(_)));
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
                    if name == "getInstance" && arg_type.contains("HostManager") {
                        let return_type = function_return_type(function);
                        let returns_result_self = return_type.starts_with("Result < Self")
                            || return_type.starts_with("Result<Self");
                        if arg_type.trim_start().starts_with('&') {
                            if returns_arc_mutex_self(&return_type) {
                                has_context_ref_get_instance_arc_mutex = true;
                            } else if returns_result_self {
                                has_result_context_ref_get_instance = true;
                            } else {
                                has_context_ref_get_instance = true;
                            }
                        } else {
                            if returns_arc_mutex_self(&return_type) {
                                has_context_get_instance_arc_mutex = true;
                            } else if returns_result_self {
                                has_result_context_get_instance = true;
                            } else {
                                has_context_get_instance = true;
                            }
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

        if !has_public_instance_method {
            continue;
        }
        if has_context_get_instance_arc_mutex {
            return Some((type_name, ObjectAccess::ContextGetInstanceArcMutexConstruct));
        }
        if has_context_ref_get_instance_arc_mutex {
            return Some((
                type_name,
                ObjectAccess::ContextRefGetInstanceArcMutexConstruct,
            ));
        }
        if has_result_context_get_instance {
            return Some((type_name, ObjectAccess::ResultContextGetInstanceConstruct));
        }
        if has_result_context_ref_get_instance {
            return Some((
                type_name,
                ObjectAccess::ResultContextRefGetInstanceConstruct,
            ));
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

fn returns_arc_mutex_self(return_type: &str) -> bool {
    let compact = return_type
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    compact == "Arc<Mutex<Self>>"
        || compact == "std::sync::Arc<std::sync::Mutex<Self>>"
        || compact == "::std::sync::Arc<::std::sync::Mutex<Self>>"
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

pub(crate) fn scan_object(
    spec: &ObjectSpec,
    serializable_types: &HashSet<String>,
    deserializable_types: &HashSet<String>,
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
            deserializable_types,
            type_registry,
        ),
    }
}

fn scan_methods(
    path: &Path,
    type_name: &str,
    module_path: &str,
    serializable_types: &HashSet<String>,
    deserializable_types: &HashSet<String>,
    type_registry: &TypeRegistry,
) -> Vec<SourceMethod> {
    let content = fs::read_to_string(path).expect("read runtime source");
    let file = syn::parse_file(&content).expect("parse runtime source");
    let resolver = TypeResolver::from_file(
        &file,
        module_path,
        module_path
            .split("::")
            .next()
            .expect("module path must include crate name"),
        serializable_types.clone(),
        deserializable_types.clone(),
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
    let cfg_attrs = cfg_attrs(function);
    let doc_lines = doc_lines(function);
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
        cfg_attrs,
        doc_lines,
        protocol,
    }
}

fn doc_lines(function: &ImplItemFn) -> Vec<String> {
    function
        .attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            match &attr.meta {
                Meta::NameValue(name_value) => match &name_value.value {
                    Expr::Lit(expr_lit) => match &expr_lit.lit {
                        Lit::Str(value) => Some(value.value().trim().to_string()),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            }
        })
        .collect()
}

fn cfg_attrs(function: &ImplItemFn) -> Vec<String> {
    function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .map(|attr| attr.to_token_stream().to_string())
        .collect()
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
    if let Some(error_type) = result_unit_error_type(ty) {
        return MethodProtocol::Call(CallProtocol::ResultUnit {
            error_type: error_type.to_string(),
        });
    }
    if let Some((inner, error_type)) = result_value_parts(ty) {
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
            MethodProtocol::Call(CallProtocol::ResultValue {
                value_type: inner.to_string(),
                error_type: error_type.to_string(),
            })
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
    if let Some(stream_item) = plain_stream_item_type(ty, resolver) {
        if stream_item == "String" {
            return MethodProtocol::Watch(WatchProtocol {
                snapshot_type: None,
                stream: WatchStreamProtocol::StringStream,
            });
        }
        return classify_json_watch(&stream_item, WatchStreamProtocol::JsonStream, resolver);
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

fn plain_stream_item_type(ty: &str, resolver: &TypeResolver) -> Option<String> {
    let resolved = resolver.type_registry.resolve_alias(ty);
    resolver
        .type_registry
        .stream_item(&resolved)
        .map(|item| item.to_string())
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
