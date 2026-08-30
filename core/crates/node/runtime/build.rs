use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{Expr, ImplItem, Item, Meta, MetaNameValue, ReturnType, Type};

/// Scans every runtime source file for route annotations and writes server-owned route lookup code.
fn main() {
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be available"),
    );
    let runtime_root = manifest_dir.join("../../runtime/application/src");
    println!("cargo:rerun-if-changed={}", runtime_root.display());
    let mut declarations = BTreeSet::new();
    scan_source_tree(&runtime_root, &runtime_root, &mut declarations);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR must be available"));
    fs::write(
        out_dir.join("generated_route_catalog.rs"),
        render_route_catalog(&declarations),
    )
    .expect("write generated route catalog");
}

/// Recursively scans runtime Rust files for route declarations.
fn scan_source_tree(
    runtime_root: &Path,
    path: &Path,
    declarations: &mut BTreeSet<(String, String, String, String, String)>,
) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            scan_source_tree(runtime_root, &entry_path, declarations);
            continue;
        }
        if entry_path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let content = fs::read_to_string(&entry_path)
            .unwrap_or_else(|error| panic!("read route source {}: {error}", entry_path.display()));
        let syntax = syn::parse_file(&content)
            .unwrap_or_else(|error| panic!("parse route source {}: {error}", entry_path.display()));
        for item in syntax.items {
            let Item::Impl(item_impl) = item else {
                continue;
            };
            for impl_item in item_impl.items {
                let ImplItem::Fn(function) = impl_item else {
                    continue;
                };
                let Some(binding) = route_binding(&function.attrs) else {
                    continue;
                };
                let targetType =
                    generated_target_type(runtime_root, &entry_path, &item_impl.self_ty);
                let routeKind = route_kind(&function.sig.output);
                let lifecycle = route_lifecycle(&function.attrs);
                declarations.insert((
                    function.sig.ident.to_string(),
                    binding,
                    targetType,
                    routeKind,
                    lifecycle,
                ));
            }
        }
    }
}

/// Builds the stable fully-qualified runtime type name for one annotated impl.
fn generated_target_type(runtime_root: &Path, source_path: &Path, self_ty: &syn::Type) -> String {
    let relative = source_path
        .strip_prefix(runtime_root)
        .expect("route source must be inside runtime root");
    let mut modules = vec!["operit_runtime".to_string()];
    if relative != Path::new("lib.rs") {
        modules.extend(
            relative
                .with_extension("")
                .components()
                .map(|component| component.as_os_str().to_string_lossy().to_string()),
        );
    }
    let self_name = match self_ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .expect("route impl type must have a final segment")
            .ident
            .to_string(),
        _ => panic!("route impl type must be a path"),
    };
    modules.push(self_name);
    modules.join("::")
}

/// Extracts the binding argument from one route annotation.
fn route_binding(attributes: &[syn::Attribute]) -> Option<String> {
    let attribute = attributes.iter().find(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "operit_core_route")
    })?;
    let Meta::List(meta_list) = &attribute.meta else {
        return None;
    };
    let arguments = meta_list
        .parse_args_with(
            syn::punctuated::Punctuated::<MetaNameValue, syn::Token![,]>::parse_terminated,
        )
        .ok()?;
    arguments.into_iter().find_map(|argument| {
        let argument_name = argument.path.segments.last()?.ident.to_string();
        if argument_name != "binding" {
            return None;
        }
        let Expr::Path(expression) = argument.value else {
            return None;
        };
        expression
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
    })
}

/// Identifies whether an annotated method is a value call or a StateFlow watch.
fn route_kind(output: &ReturnType) -> String {
    if return_type_is_state_flow(output) {
        "watch".to_string()
    } else {
        "call".to_string()
    }
}

/// Identifies whether a route declaration participates in route lifecycle dispatch.
fn route_lifecycle(attributes: &[syn::Attribute]) -> String {
    if attributes.iter().any(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "before_change_route")
    }) {
        return "BeforeChangeRoute".to_string();
    }
    if attributes.iter().any(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "after_change_route")
    }) {
        return "AfterChangeRoute".to_string();
    }
    "Normal".to_string()
}

/// Returns whether a route return type is a StateFlow.
fn return_type_is_state_flow(output: &ReturnType) -> bool {
    let ReturnType::Type(_, ty) = output else {
        return false;
    };
    let Type::Path(path) = ty.as_ref() else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "StateFlow")
}

/// Renders server-owned route lookup functions from annotation declarations.
fn render_route_catalog(
    declarations: &BTreeSet<(String, String, String, String, String)>,
) -> String {
    let mut output = String::new();
    output.push_str("/// Resolves one annotation-generated Space route by its wire route ID.\n");
    output.push_str("pub fn generated_space_route_for_id(routeId: u32, methodName: &str) -> Option<GeneratedSpaceRoute> {\n");
    output.push_str("    match (routeId, methodName) {\n");
    for (routeId, (method, binding, targetType, _routeKind, lifecycle)) in
        declarations.iter().enumerate()
    {
        output.push_str(&format!(
            "        ({routeId}, {method:?}) => Some(GeneratedSpaceRoute {{ routeId: {routeId}, methodName: {method:?}, bindingArgument: {binding:?}, targetType: {targetType:?}, lifecycle: GeneratedRouteLifecycle::{lifecycle} }}),\n"
        ));
    }
    output.push_str("        _ => None,\n    }\n}\n\n");
    output.push_str("/// Resolves one internal annotation route without a Proxy object address.\n");
    output.push_str("pub fn generated_space_route_for_method(methodName: &str) -> Option<GeneratedSpaceRoute> {\n");
    output.push_str("    match methodName {\n");
    for (routeId, (method, binding, targetType, _routeKind, lifecycle)) in
        declarations.iter().enumerate()
    {
        output.push_str(&format!(
            "        {method:?} => Some(GeneratedSpaceRoute {{ routeId: {routeId}, methodName: {method:?}, bindingArgument: {binding:?}, targetType: {targetType:?}, lifecycle: GeneratedRouteLifecycle::{lifecycle} }}),\n"
        ));
    }
    output.push_str("        _ => None,\n    }\n}\n\n");
    output.push_str("/// Resolves the generated Space route registered for one lifecycle hook.\n");
    output.push_str("pub fn generated_space_lifecycle_route(lifecycle: GeneratedRouteLifecycle) -> Option<GeneratedSpaceRoute> {\n");
    output.push_str("    match lifecycle {\n");
    for (routeId, (method, binding, targetType, _routeKind, lifecycle)) in
        declarations.iter().enumerate()
    {
        if lifecycle != "Normal" {
            output.push_str(&format!(
                "        GeneratedRouteLifecycle::{lifecycle} => Some(GeneratedSpaceRoute {{ routeId: {routeId}, methodName: {method:?}, bindingArgument: {binding:?}, targetType: {targetType:?}, lifecycle: GeneratedRouteLifecycle::{lifecycle} }}),\n"
            ));
        }
    }
    output.push_str("        GeneratedRouteLifecycle::Normal => None,\n    }\n}\n\n");
    output.push_str(
        "/// Resolves one annotation-generated Space route from a standard Link call request.\n",
    );
    output.push_str("pub fn generated_space_call_route(request: &operit_link::CoreCallRequest) -> Option<GeneratedSpaceRoute> { if request.targetObjectId == operit_link::CORE_INTERNAL_ROUTE_OBJECT_ID { generated_space_route_for_method(&request.methodName) } else { generated_space_route_for_id(request.targetObjectId, &request.methodName) } }\n\n");
    output.push_str(
        "/// Resolves one annotation-generated Space route from a standard Link watch request.\n",
    );
    output.push_str("pub fn generated_space_watch_route(request: &operit_link::CoreWatchRequest) -> Option<GeneratedSpaceRoute> { if request.targetObjectId == operit_link::CORE_INTERNAL_ROUTE_OBJECT_ID { generated_space_route_for_method(&request.propertyName) } else { generated_space_route_for_id(request.targetObjectId, &request.propertyName) } }\n\n");
    output.push_str(
        "/// Resolves one annotation-generated Space route from a standard Link push request.\n",
    );
    output.push_str("pub fn generated_space_push_route(request: &operit_link::CorePushRequest) -> Option<GeneratedSpaceRoute> { if request.targetObjectId == operit_link::CORE_INTERNAL_ROUTE_OBJECT_ID { generated_space_route_for_method(&request.methodName) } else { generated_space_route_for_id(request.targetObjectId, &request.methodName) } }\n\n");
    output.push_str(
        "/// Dispatches one generated Space call on the runtime's main ChatServiceCore.\n",
    );
    output.push_str("pub async fn generated_space_call_on_chat_core(core: &mut operit_runtime::services::ChatServiceCore::ChatServiceCore, request: operit_link::CoreCallRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {\n");
    output.push_str("    match request.methodName.as_str() {\n");
    for (method, _binding, _targetType, routeKind, _lifecycle) in declarations {
        if routeKind == "call" {
            output.push_str(&format!(
                "        {method:?} => core.__operit_core_route_call_{method}(request).await,\n"
            ));
        }
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::methodNotFound(&request.registryKey())),\n    }\n}\n\n");
    output.push_str(
        "/// Reads one generated Space watch snapshot on the runtime's main ChatServiceCore.\n",
    );
    output.push_str("pub async fn generated_space_watch_snapshot_on_chat_core(core: &mut operit_runtime::services::ChatServiceCore::ChatServiceCore, request: &operit_link::CoreWatchRequest) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {\n");
    output.push_str("    match request.propertyName.as_str() {\n");
    for (method, _binding, _targetType, routeKind, _lifecycle) in declarations {
        if routeKind == "watch" {
            output.push_str(&format!("        {method:?} => core.__operit_core_route_watch_snapshot_{method}(request).await,\n"));
        }
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),\n    }\n}\n\n");
    output.push_str("/// Opens one generated Space watch on the runtime's main ChatServiceCore.\n");
    output.push_str("pub async fn generated_space_watch_on_chat_core(core: &mut operit_runtime::services::ChatServiceCore::ChatServiceCore, request: operit_link::CoreWatchRequest, attachmentAdopter: std::sync::Arc<dyn Fn(Vec<operit_link::CoreStreamAttachment>) + Send + Sync>) -> Result<operit_link::CoreEventStream, operit_link::CoreLinkError> {\n");
    output.push_str("    match request.propertyName.as_str() {\n");
    for (method, _binding, _targetType, routeKind, _lifecycle) in declarations {
        if routeKind == "watch" {
            output.push_str(&format!(
                "        {method:?} => core.__operit_core_route_watch_{method}(request, attachmentAdopter).await,\n"
            ));
        }
    }
    output.push_str("        _ => Err(operit_link::CoreLinkError::watchNotFound(&request.registryKey())),\n    }\n}\n\n");
    output
        .push_str("/// Resolves one request using route declarations from runtime annotations.\n");
    output.push_str("fn generated_route_for_request(methodName: &str, args: &operit_link::CoreValue) -> Result<GeneratedCoreRoute, operit_link::CoreLinkError> {\n");
    output.push_str("    let bindingArgument = match methodName {\n");
    for (method, binding, _, _, _) in declarations {
        output.push_str(&format!("        {method:?} => Some({binding:?}),\n"));
    }
    output.push_str("        _ => None,\n    };\n");
    output.push_str("    let Some(bindingArgument) = bindingArgument else { return Ok(GeneratedCoreRoute::Local); };\n");
    output.push_str("    let operit_link::CoreValue::Map(arguments) = args else { return Err(operit_link::CoreLinkError::new(\"INVALID_ARGS\", \"Binding request arguments must be a map\")); };\n");
    output.push_str("    let Some(value) = arguments.get(bindingArgument) else { return Err(operit_link::CoreLinkError::new(\"CORE_BINDING_KEY_REQUIRED\", \"Binding request does not include its required key\")); };\n");
    output.push_str("    let key = match value {\n        operit_link::CoreValue::String(key) => key,\n        operit_link::CoreValue::Null => return Ok(GeneratedCoreRoute::Local),\n        _ => return Err(operit_link::CoreLinkError::new(\"CORE_BINDING_KEY_INVALID\", \"Binding key must be a string\")),\n    };\n");
    output.push_str("    if key.trim().is_empty() { return Err(operit_link::CoreLinkError::new(\"CORE_BINDING_KEY_REQUIRED\", \"Binding requires a non-empty key\")); }\n");
    output.push_str("    Ok(GeneratedCoreRoute::Binding { scope: 0, key: key.clone() })\n}\n\n");
    output.push_str(
        "/// Resolves one call request using route declarations from runtime annotations.\n",
    );
    output.push_str("pub fn generated_core_call_route(request: &operit_link::CoreCallRequest) -> Result<GeneratedCoreRoute, operit_link::CoreLinkError> { generated_route_for_request(&request.methodName, &request.args) }\n\n");
    output.push_str(
        "/// Resolves one watch request using route declarations from runtime annotations.\n",
    );
    output.push_str("pub fn generated_core_watch_route(request: &operit_link::CoreWatchRequest) -> Result<GeneratedCoreRoute, operit_link::CoreLinkError> { generated_route_for_request(&request.propertyName, &request.args) }\n\n");
    output.push_str(
        "/// Resolves one push request using route declarations from runtime annotations.\n",
    );
    output.push_str("pub fn generated_core_push_route(request: &operit_link::CorePushRequest) -> Result<GeneratedCoreRoute, operit_link::CoreLinkError> { generated_route_for_request(&request.methodName, &request.args) }\n");
    output
}
