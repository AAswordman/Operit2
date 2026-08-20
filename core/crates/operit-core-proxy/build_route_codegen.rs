use super::*;

/// Renders method-level Binding route metadata for generated Link requests.
pub(crate) fn render_core_route_classifier(objects: &[SourceObject]) -> String {
    let scopes = binding_scope_objects(objects);
    let mut output = String::new();
    output.push_str("/// Identifies the CoreNode selected by one generated request.\n");
    output.push_str("#[derive(Clone, Debug, PartialEq, Eq)]\n");
    output.push_str("pub(crate) enum GeneratedCoreRoute {\n");
    output.push_str("    Local,\n");
    output.push_str("    Binding { scope: usize, key: String },\n");
    output.push_str("    CurrentBinding { scope: usize, resolver: &'static str },\n");
    output.push_str("}\n\n");
    output.push_str(&render_binding_scope_matcher(&scopes));
    output.push_str("/// Resolves an explicit Binding key from one generated argument map.\n");
    output.push_str("fn generated_binding_route_from_args(path: &operit_link::CoreObjectPath, args: &operit_link::CoreValue, bindingArgument: &str, scope: usize) -> Result<Option<GeneratedCoreRoute>, operit_link::CoreLinkError> {\n");
    output.push_str("    if !generated_binding_scope_matches(scope, path) { return Ok(None); }\n");
    output.push_str("    let operit_link::CoreValue::Map(arguments) = args else { return Err(operit_link::CoreLinkError::new(\"INVALID_ARGS\", \"Binding request arguments must be a map\")); };\n");
    output.push_str(
        "    let Some(value) = arguments.get(bindingArgument) else { return Ok(None); };\n",
    );
    output.push_str("    if matches!(value, operit_link::CoreValue::Null) { return Ok(None); }\n");
    output.push_str("    let operit_link::CoreValue::String(key) = value else { return Err(operit_link::CoreLinkError::new(\"CORE_BINDING_KEY_INVALID\", \"Binding key must be a string\")); };\n");
    output.push_str("    if key.trim().is_empty() { return Err(operit_link::CoreLinkError::new(\"CORE_BINDING_KEY_REQUIRED\", \"Binding requires a non-empty key\")); }\n");
    output.push_str("    Ok(Some(GeneratedCoreRoute::Binding { scope, key: key.clone() }))\n");
    output.push_str("}\n\n");
    output.push_str(&render_protocol_route_classifier(
        &scopes,
        BindingRequestProtocol::Call,
    ));
    output.push_str(&render_protocol_route_classifier(
        &scopes,
        BindingRequestProtocol::Watch,
    ));
    output.push_str(&render_protocol_route_classifier(
        &scopes,
        BindingRequestProtocol::Push,
    ));
    output.push_str(&render_binding_argument_injectors(&scopes));
    output
}
/// Returns objects that declare at least one method-level Binding route.
fn binding_scope_objects<'a>(objects: &'a [SourceObject]) -> Vec<&'a SourceObject> {
    objects
        .iter()
        .filter(|object| {
            object
                .methods
                .iter()
                .any(|method| !matches!(method.route, MethodRoute::Local))
        })
        .collect()
}

/// Renders exact path matching for generated Binding method scopes.
fn render_binding_scope_matcher(scopes: &[&SourceObject]) -> String {
    let mut output = String::new();
    output.push_str(
        "/// Returns whether a concrete object path belongs to one generated Binding scope.\n",
    );
    output.push_str("fn generated_binding_scope_matches(scope: usize, path: &operit_link::CoreObjectPath) -> bool {\n");
    output.push_str("    match scope {\n");
    for (scope, object) in scopes.iter().enumerate() {
        output.push_str(&format!(
            "        {scope} => generated_object_path_matches_{}(path),\n",
            object.dispatch_name
        ));
    }
    output.push_str("        _ => false,\n");
    output.push_str("    }\n");
    output.push_str("}\n\n");
    output
}

/// Identifies the request protocol whose Binding key may be injected locally.
#[derive(Clone, Copy)]
enum BindingRequestProtocol {
    Call,
    Watch,
    Push,
}

/// Renders the classifier for one concrete Link request protocol.
fn render_protocol_route_classifier(
    scopes: &[&SourceObject],
    protocol: BindingRequestProtocol,
) -> String {
    let (function_name, request_type, method_field, description) = match protocol {
        BindingRequestProtocol::Call => (
            "generated_core_call_route",
            "operit_link::CoreCallRequest",
            "methodName",
            "call",
        ),
        BindingRequestProtocol::Watch => (
            "generated_core_watch_route",
            "operit_link::CoreWatchRequest",
            "propertyName",
            "watch",
        ),
        BindingRequestProtocol::Push => (
            "generated_core_push_route",
            "operit_link::CorePushRequest",
            "methodName",
            "push",
        ),
    };
    let mut output = String::new();
    output.push_str(&format!(
        "/// Resolves method-level Binding metadata for one {description} request.\n"
    ));
    output.push_str(&format!(
        "pub(crate) fn {function_name}(request: &{request_type}) -> Result<GeneratedCoreRoute, operit_link::CoreLinkError> {{\n"
    ));
    for (scope, object) in scopes.iter().enumerate() {
        for method in &object.methods {
            let MethodRoute::Binding {
                binding_argument,
                current_resolver,
                ..
            } = &method.route
            else {
                continue;
            };
            if !method_uses_protocol(method, protocol) {
                continue;
            }
            output.push_str(&format!(
                "    if generated_binding_scope_matches({scope}, &request.targetPath) && request.{method_field} == {:?} {{\n",
                method.name
            ));
            output.push_str(&format!(
                "        if let Some(route) = generated_binding_route_from_args(&request.targetPath, &request.args, {:?}, {scope})? {{ return Ok(route); }}\n",
                binding_argument
            ));
            if let Some(current_resolver) = current_resolver {
                output.push_str(&format!(
                    "        return Ok(GeneratedCoreRoute::CurrentBinding {{ scope: {scope}, resolver: {:?} }});\n",
                    current_resolver
                ));
            } else {
                output.push_str("        return Err(operit_link::CoreLinkError::new(\"CORE_BINDING_KEY_REQUIRED\", \"Binding request does not include its required key\"));\n");
            }
            output.push_str("    }\n");
        }
    }
    output.push_str("    Ok(GeneratedCoreRoute::Local)\n");
    output.push_str("}\n\n");
    output
}

/// Reports whether one method is carried by a particular Link request protocol.
fn method_uses_protocol(method: &SourceMethod, protocol: BindingRequestProtocol) -> bool {
    match protocol {
        BindingRequestProtocol::Call => method.call_protocol().is_some(),
        BindingRequestProtocol::Watch => method.watch_protocol().is_some(),
        BindingRequestProtocol::Push => method.reverse_stream_protocol().is_some(),
    }
}

/// Renders Binding-key injection for methods whose explicit key resolved locally.
fn render_binding_argument_injectors(scopes: &[&SourceObject]) -> String {
    [
        BindingRequestProtocol::Call,
        BindingRequestProtocol::Watch,
        BindingRequestProtocol::Push,
    ]
    .into_iter()
    .map(|protocol| render_binding_argument_injector(scopes, protocol))
    .collect()
}

/// Renders method-to-key argument mapping for one Link request protocol.
fn render_binding_argument_injector(
    scopes: &[&SourceObject],
    protocol: BindingRequestProtocol,
) -> String {
    let (function_name, request_type, method_field, description) = match protocol {
        BindingRequestProtocol::Call => (
            "generated_inject_current_binding_call",
            "operit_link::CoreCallRequest",
            "methodName",
            "call",
        ),
        BindingRequestProtocol::Watch => (
            "generated_inject_current_binding_watch",
            "operit_link::CoreWatchRequest",
            "propertyName",
            "watch",
        ),
        BindingRequestProtocol::Push => (
            "generated_inject_current_binding_push",
            "operit_link::CorePushRequest",
            "methodName",
            "push",
        ),
    };
    let mut mappings = String::new();
    for (scope, object) in scopes.iter().enumerate() {
        for method in &object.methods {
            let MethodRoute::Binding {
                binding_argument, ..
            } = &method.route
            else {
                continue;
            };
            if method_uses_protocol(method, protocol) {
                mappings.push_str(&format!(
                    "        ({scope}, {:?}) => {:?},\n",
                    method.name, binding_argument
                ));
            }
        }
    }
    let mut output = String::new();
    output.push_str(&format!(
        "/// Injects a locally resolved Binding key into one generated {description} request.\n"
    ));
    output.push_str(&format!(
        "pub(crate) fn {function_name}(scope: usize, request: &mut {request_type}, key: String) -> Result<(), operit_link::CoreLinkError> {{\n"
    ));
    output.push_str(&format!(
        "    let argumentName: &str = match (scope, request.{method_field}.as_str()) {{\n"
    ));
    output.push_str(&mappings);
    output.push_str("        _ => return Err(operit_link::CoreLinkError::new(\"CORE_BINDING_ARGUMENT_UNDECLARED\", \"Binding request does not declare a key argument\")),\n");
    output.push_str("    };\n");
    output.push_str("    let operit_link::CoreValue::Map(arguments) = &mut request.args else { return Err(operit_link::CoreLinkError::new(\"INVALID_ARGS\", \"Binding request arguments must be a map\")); };\n");
    output.push_str(
        "    arguments.insert(argumentName.to_string(), operit_link::CoreValue::String(key));\n",
    );
    output.push_str("    Ok(())\n");
    output.push_str("}\n\n");
    output
}
