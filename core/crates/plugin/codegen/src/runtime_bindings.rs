use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use quote::quote;
use syn::{Expr, FnArg, Item, Lit, Pat, TraitItem};
use tree_sitter::Parser;

const HOST_TRAITS: &[(&str, &str, &str)] = &[
    ("js_sdk/files.rs", "FilesHost", "Files"),
    ("js_sdk/network.rs", "NetHost", "Net"),
    ("js_sdk/network.rs", "NetCookieManager", "Net.cookies"),
    ("js_sdk/system.rs", "SystemHost", "System"),
    (
        "js_sdk/system.rs",
        "SystemBluetoothHost",
        "System.bluetooth",
    ),
    (
        "js_sdk/system.rs",
        "SystemBluetoothBleHost",
        "System.bluetooth.ble",
    ),
    ("js_sdk/system.rs", "SystemTerminalHost", "System.terminal"),
    ("js_sdk/system.rs", "SystemMusicHost", "System.music"),
    (
        "js_sdk/software_settings.rs",
        "SoftwareSettingsHost",
        "SoftwareSettings",
    ),
    ("js_sdk/chat.rs", "ChatHost", "Chat"),
    ("js_sdk/memory.rs", "MemoryHost", "Memory"),
];

/// Generates concrete active Tools host trait implementations from canonical Rust signatures.
pub fn generate_js_tools_host_implementation(
    sdk_src: &Path,
    bindings_path: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let javascript_bindings = parse_rust_tool_bindings(bindings_path)?;
    let mut files = BTreeMap::<PathBuf, syn::File>::new();
    let mut output =
        String::from("// Generated from canonical Rust Tools traits. Do not edit.\n\n");

    for (relative_path, trait_name, namespace) in HOST_TRAITS {
        let path = sdk_src.join(relative_path);
        if !files.contains_key(&path) {
            files.insert(path.clone(), syn::parse_file(&fs::read_to_string(&path)?)?);
        }
        let file = files.get(&path).expect("inserted Rust SDK file is present");
        let item_trait = file.items.iter().find_map(|item| match item {
            Item::Trait(item_trait) if item_trait.ident == *trait_name => Some(item_trait),
            _ => None,
        });
        let item_trait = item_trait.ok_or_else(|| format!("missing Rust trait `{trait_name}`"))?;
        output.push_str(&format!("impl {trait_name} for AIToolHandler {{\n"));
        for item in &item_trait.items {
            let TraitItem::Fn(method) = item else {
                continue;
            };
            let runtime_name = runtime_method_name(&method.sig.ident.to_string());
            let binding = javascript_bindings
                .get(&(namespace.to_string(), runtime_name.clone()))
                .ok_or_else(|| {
                    format!("missing JavaScript binding for `{namespace}.{runtime_name}`")
                })?;
            let tool_variant = rust_variant_name(&binding.tool);
            let signature = &method.sig;
            let arguments = method_arguments(signature)?;
            output.push_str(&format!(
                "    /// Executes the canonical `{namespace}.{runtime_name}` Tools binding.\n"
            ));
            output.push_str("    ");
            output.push_str(
                &quote!(#signature)
                    .to_string()
                    .replace("super :: JsNever", "operit_plugin_sdk :: js_sdk :: JsNever"),
            );
            output.push_str(" {\n");
            output.push_str(&generated_method_body(
                namespace,
                &runtime_name,
                &tool_variant,
                &arguments,
            ));
            output.push_str("    }\n\n");
        }
        output.push_str("}\n\n");
    }
    fs::write(output_path, output)?;
    Ok(())
}

/// Describes one generated JavaScript Tools method binding.
#[derive(Clone, Debug)]
struct JsToolBindingSpec {
    tool: String,
    api_variants: Vec<JsToolApiVariantSpec>,
}

/// Describes one versioned generated JavaScript implementation.
#[derive(Clone, Debug)]
struct JsToolApiVariantSpec {
    since: String,
    until: Option<String>,
    arguments: Option<Vec<String>>,
}

/// Reads the canonical Rust method-to-tool bindings and API gates.
fn parse_rust_tool_bindings(
    path: &Path,
) -> Result<BTreeMap<(String, String), JsToolBindingSpec>, Box<dyn Error>> {
    let file = syn::parse_file(&fs::read_to_string(path)?)?;
    let binding_const = file.items.iter().find_map(|item| match item {
        Item::Const(item) if item.ident == "JS_TOOL_BINDINGS" => Some(item),
        _ => None,
    });
    let binding_const = binding_const.ok_or("JS_TOOL_BINDINGS is missing")?;
    let expression = match &*binding_const.expr {
        Expr::Reference(reference) => &*reference.expr,
        expression => expression,
    };
    let Expr::Array(array) = expression else {
        return Err("JS_TOOL_BINDINGS must reference an array literal".into());
    };
    let mut bindings = BTreeMap::<(String, String), JsToolBindingSpec>::new();
    for element in &array.elems {
        let Expr::Struct(binding) = element else {
            return Err("JS_TOOL_BINDINGS entries must be JsToolBinding structs".into());
        };
        let mut namespace = None;
        let mut method = None;
        let mut tool = None;
        for field in &binding.fields {
            let syn::Member::Named(name) = &field.member else {
                continue;
            };
            match (name.to_string().as_str(), &field.expr) {
                ("namespace", Expr::Lit(value)) => {
                    if let Lit::Str(value) = &value.lit {
                        namespace = Some(value.value());
                    }
                }
                ("method", Expr::Lit(value)) => {
                    if let Lit::Str(value) = &value.lit {
                        method = Some(value.value());
                    }
                }
                ("tool", Expr::Path(value)) => {
                    tool = value
                        .path
                        .segments
                        .last()
                        .map(|segment| pascal_case_to_snake_case(&segment.ident.to_string()));
                }
                _ => {}
            }
        }
        let key = (
            namespace.ok_or("JsToolBinding namespace is missing")?,
            method.ok_or("JsToolBinding method is missing")?,
        );
        if bindings
            .insert(
                key.clone(),
                JsToolBindingSpec {
                    tool: tool.ok_or("JsToolBinding tool is missing")?,
                    api_variants: Vec::new(),
                },
            )
            .is_some()
        {
            return Err(format!("duplicate JsToolBinding for `{}.{}`", key.0, key.1).into());
        }
    }
    let api_variant_const = file.items.iter().find_map(|item| match item {
        Item::Const(item) if item.ident == "JS_TOOL_API_VARIANTS" => Some(item),
        _ => None,
    });
    let api_variant_const = api_variant_const.ok_or("JS_TOOL_API_VARIANTS is missing")?;
    let expression = match &*api_variant_const.expr {
        Expr::Reference(reference) => &*reference.expr,
        expression => expression,
    };
    let Expr::Array(array) = expression else {
        return Err("JS_TOOL_API_VARIANTS must reference an array literal".into());
    };
    for element in &array.elems {
        let Expr::Struct(gate) = element else {
            return Err("JS_TOOL_API_VARIANTS entries must be JsToolApiVariant structs".into());
        };
        let mut namespace = None;
        let mut method = None;
        let mut since = None;
        let mut until = None;
        let mut arguments = None;
        for field in &gate.fields {
            let syn::Member::Named(name) = &field.member else {
                continue;
            };
            match (name.to_string().as_str(), &field.expr) {
                ("namespace", Expr::Lit(value)) => {
                    if let Lit::Str(value) = &value.lit {
                        namespace = Some(value.value());
                    }
                }
                ("method", Expr::Lit(value)) => {
                    if let Lit::Str(value) = &value.lit {
                        method = Some(value.value());
                    }
                }
                ("since", Expr::Lit(value)) => {
                    if let Lit::Str(value) = &value.lit {
                        since = Some(value.value());
                    }
                }
                ("until", expression) => {
                    until = Some(parse_optional_string(expression, "until")?);
                }
                ("arguments", expression) => {
                    arguments = parse_optional_string_array(expression)?;
                }
                _ => {}
            }
        }
        let key = (
            namespace.ok_or("JsToolApiVariant namespace is missing")?,
            method.ok_or("JsToolApiVariant method is missing")?,
        );
        let since = since.ok_or("JsToolApiVariant since is missing")?;
        let until = until.unwrap_or(None);
        let binding = bindings.get_mut(&key).ok_or_else(|| {
            format!(
                "JsToolApiVariant refers to missing binding `{}.{}`",
                key.0, key.1
            )
        })?;
        if binding
            .api_variants
            .iter()
            .any(|variant| variant.since == since)
        {
            return Err(format!(
                "duplicate JsToolApiVariant for `{}.{}` at {since}",
                key.0, key.1
            )
            .into());
        }
        binding.api_variants.push(JsToolApiVariantSpec {
            since,
            until,
            arguments,
        });
    }
    Ok(bindings)
}

/// Parses an optional static string used by one API variant declaration.
fn parse_optional_string(
    expression: &Expr,
    field_name: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    match expression {
        Expr::Path(path) if path.path.is_ident("None") => Ok(None),
        Expr::Call(call) if call.args.len() == 1 => {
            let Expr::Path(function) = &*call.func else {
                return Err(format!("JsToolApiVariant {field_name} must use Some(string)").into());
            };
            if !function.path.is_ident("Some") {
                return Err(format!("JsToolApiVariant {field_name} must use Some(string)").into());
            }
            let expression = call.args.first().expect("one Some argument is present");
            match expression {
                Expr::Lit(value) => match &value.lit {
                    Lit::Str(value) => Ok(Some(value.value())),
                    _ => Err(format!("JsToolApiVariant {field_name} must contain a string").into()),
                },
                _ => Err(format!("JsToolApiVariant {field_name} must contain a string").into()),
            }
        }
        _ => Err(format!("JsToolApiVariant {field_name} must be None or Some(string)").into()),
    }
}

/// Parses a static string slice used by one API variant declaration.
fn parse_string_array(expression: &Expr, field_name: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let Expr::Array(array) = expression else {
        return Err(format!("JsToolApiVariant {field_name} must be a string array").into());
    };
    array
        .elems
        .iter()
        .map(|element| match element {
            Expr::Lit(value) => match &value.lit {
                Lit::Str(value) => Ok(value.value()),
                _ => Err(format!("JsToolApiVariant {field_name} must contain strings").into()),
            },
            _ => Err(format!("JsToolApiVariant {field_name} must contain strings").into()),
        })
        .collect()
}

/// Parses an optional static string slice used by one API variant declaration.
fn parse_optional_string_array(expression: &Expr) -> Result<Option<Vec<String>>, Box<dyn Error>> {
    match expression {
        Expr::Path(path) if path.path.is_ident("None") => Ok(None),
        Expr::Call(call) if call.args.len() == 1 => {
            let Expr::Path(function) = &*call.func else {
                return Err("JsToolApiVariant arguments must use Some(string array)".into());
            };
            if !function.path.is_ident("Some") {
                return Err("JsToolApiVariant arguments must use Some(string array)".into());
            }
            let expression = call.args.first().expect("one Some argument is present");
            let expression = match expression {
                Expr::Reference(reference) => &*reference.expr,
                expression => expression,
            };
            Ok(Some(parse_string_array(expression, "arguments")?))
        }
        _ => Err("JsToolApiVariant arguments must be None or Some(string array)".into()),
    }
}

/// Generates the executable JavaScript Tools namespace from Rust traits and binding contracts.
pub fn generate_js_tools_runtime(
    sdk_src: &Path,
    bindings_path: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let bindings = parse_rust_tool_bindings(bindings_path)?;
    let methods = read_host_method_arguments(sdk_src)?;
    let declared = methods.keys().cloned().collect::<BTreeSet<_>>();
    let bound = bindings.keys().cloned().collect::<BTreeSet<_>>();
    if declared != bound {
        let missing = declared.difference(&bound).cloned().collect::<Vec<_>>();
        let unexpected = bound.difference(&declared).cloned().collect::<Vec<_>>();
        return Err(format!(
            "Rust Tools bindings differ from active host traits; missing: {missing:?}; unexpected: {unexpected:?}"
        )
        .into());
    }

    let mut output = String::from(
        r#"// Generated from canonical Rust Tools traits and bindings. Do not edit.
var Tools = {};

function __operitToolsSnakeCase(name) {
    return String(name).replace(/[A-Z]/g, function(character) {
        return "_" + character.toLowerCase();
    });
}

function __operitToolsFlattens(name) {
    return name === "options" || name === "params" || name === "updates" ||
        name.endsWith("OrOptions") || name.endsWith("OrParams");
}

function __operitToolsScalarField(name) {
    return __operitToolsSnakeCase(name.replace(/OrOptions$|OrParams$/, ""));
}

function __operitToolsWireName(namespace, method, name) {
    if (namespace === "Files" && method === "writeBinary" && name === "base64Content") return "base64Content";
    if (namespace === "Files" && (method === "apply" || method === "create" || method === "edit") && name === "newContent") return "new";
    if (namespace === "Files" && method === "edit" && name === "oldContent") return "old";
    if (namespace === "System" && method === "sleep" && name === "milliseconds") return "duration_ms";
    if (namespace === "System" && method === "listApps" && name === "includeSystem") return "include_system_apps";
    return __operitToolsSnakeCase(name);
}

function __operitToolsSelectOverload(overloads, args) {
    if (overloads.length === 1) return overloads[0];
    if (args.length > 0 && args[0] !== null && typeof args[0] === "object" && !Array.isArray(args[0])) {
        for (var index = 0; index < overloads.length; index += 1) {
            if (overloads[index].length === 1 && __operitToolsFlattens(overloads[index][0])) {
                return overloads[index];
            }
        }
    }
    var selected = overloads[0];
    for (var overloadIndex = 1; overloadIndex < overloads.length; overloadIndex += 1) {
        if (overloads[overloadIndex].length > selected.length) selected = overloads[overloadIndex];
    }
    return selected;
}

function __operitToolsBuildParameters(namespace, method, overloads, args) {
    var names = __operitToolsSelectOverload(overloads, args);
    var parameters = {};
    for (var index = 0; index < names.length; index += 1) {
        var name = names[index];
        var value = args[index];
        if (__operitToolsFlattens(name)) {
            if (value === undefined || value === null) continue;
            if (typeof value === "object" && !Array.isArray(value)) {
                Object.keys(value).forEach(function(field) {
                    var wireField = namespace === "Net" ? field : __operitToolsSnakeCase(field);
                    parameters[wireField] = value[field];
                });
            } else {
                parameters[__operitToolsScalarField(name)] = value;
            }
        } else if (value !== undefined) {
            parameters[__operitToolsWireName(namespace, method, name)] = value;
        }
    }
    if (namespace === "Net" && method === "httpGet") parameters.method = "GET";
    if (namespace === "Net" && method === "httpPost") parameters.method = "POST";
    if (namespace === "Net.cookies" && method === "get") parameters.action = "get";
    if (namespace === "Net.cookies" && method === "set") parameters.action = "set";
    if (namespace === "Net.cookies" && method === "clear") parameters.action = "clear";
    if (namespace === "Memory" && Array.isArray(parameters.titles)) parameters.titles = parameters.titles.join(",");
    return parameters;
}

function __operitInvokeToolsBinding(namespace, method, toolName, overloads, args) {
    return toolCall(toolName, __operitToolsBuildParameters(namespace, method, overloads, args));
}

"#,
    );
    let mut initialized_namespaces = BTreeSet::new();
    let mut chat_namespace_open = false;
    for ((namespace, method), overloads) in methods {
        let binding = bindings
            .get(&(namespace.clone(), method.clone()))
            .expect("validated binding is present");
        if chat_namespace_open && namespace != "Chat" {
            output.push_str("});\n");
            chat_namespace_open = false;
        }
        let mut expression = "Tools".to_string();
        if namespace != "Chat" {
            for segment in namespace.split('.') {
                expression.push_str(&format!("[\"{segment}\"]"));
                if initialized_namespaces.insert(expression.clone()) {
                    output.push_str(&format!("{expression} = {expression} || {{}};\n"));
                }
            }
        }
        let canonical_signature = overloads
            .iter()
            .max_by_key(|arguments| arguments.len())
            .expect("host method has one signature")
            .join(", ");
        let canonical_overloads = overloads
            .iter()
            .map(|arguments| {
                format!(
                    "[{}]",
                    arguments
                        .iter()
                        .map(|argument| format!("\"{argument}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        if namespace == "Chat" {
            if !chat_namespace_open {
                output
                    .push_str("Tools[\"Chat\"] = __operitToolPkgApi.namespace(\"Tools.Chat\", {\n");
                chat_namespace_open = true;
            }
            output.push_str(&format!("\"{method}\": "));
            if binding.api_variants.is_empty() {
                output.push_str(&generated_js_function(
                    &namespace,
                    &method,
                    &binding.tool,
                    &canonical_signature,
                    &canonical_overloads,
                ));
            } else {
                output.push_str("__operitToolPkgApi.method()");
                for variant in &binding.api_variants {
                    let signature = variant
                        .arguments
                        .as_ref()
                        .map(|arguments| arguments.join(", "))
                        .unwrap_or_else(|| canonical_signature.clone());
                    let variant_overloads = variant
                        .arguments
                        .as_ref()
                        .map(|arguments| {
                            format!(
                                "[{}]",
                                arguments
                                    .iter()
                                    .map(|argument| format!("\"{argument}\""))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })
                        .unwrap_or_else(|| canonical_overloads.clone());
                    let function = generated_js_function(
                        &namespace,
                        &method,
                        &binding.tool,
                        &signature,
                        &variant_overloads,
                    );
                    if let Some(until) = &variant.until {
                        output.push_str(&format!(
                            ".range(\"{}\", \"{}\", {})",
                            variant.since, until, function
                        ));
                    } else {
                        output.push_str(&format!(".since(\"{}\", {})", variant.since, function));
                    }
                }
            }
            output.push_str(",\n");
        } else {
            let function = generated_js_function(
                &namespace,
                &method,
                &binding.tool,
                &canonical_signature,
                &canonical_overloads,
            );
            output.push_str(&format!("{expression}[\"{method}\"] = {function};\n"));
        }
    }
    if chat_namespace_open {
        output.push_str("});\n");
    }
    validate_javascript_syntax(&output)?;
    fs::write(output_path, output)?;
    Ok(())
}

/// Generates one JavaScript function expression for a Tools API variant.
fn generated_js_function(
    namespace: &str,
    method: &str,
    tool: &str,
    signature: &str,
    overloads: &str,
) -> String {
    format!(
        "function({signature}) {{\n    return __operitInvokeToolsBinding(\"{namespace}\", \"{method}\", \"{tool}\", [{overloads}], Array.prototype.slice.call(arguments));\n}}"
    )
}

/// Reads every overload's ordered argument names from active Rust host traits.
fn read_host_method_arguments(
    sdk_src: &Path,
) -> Result<BTreeMap<(String, String), Vec<Vec<String>>>, Box<dyn Error>> {
    let mut files = BTreeMap::<PathBuf, syn::File>::new();
    let mut methods = BTreeMap::<(String, String), Vec<Vec<String>>>::new();
    for (relative_path, trait_name, namespace) in HOST_TRAITS {
        let path = sdk_src.join(relative_path);
        if !files.contains_key(&path) {
            files.insert(path.clone(), syn::parse_file(&fs::read_to_string(&path)?)?);
        }
        let file = files.get(&path).expect("inserted Rust SDK file is present");
        let item_trait = file.items.iter().find_map(|item| match item {
            Item::Trait(item_trait) if item_trait.ident == *trait_name => Some(item_trait),
            _ => None,
        });
        let item_trait = item_trait.ok_or_else(|| format!("missing Rust trait `{trait_name}`"))?;
        for item in &item_trait.items {
            let TraitItem::Fn(method) = item else {
                continue;
            };
            methods
                .entry((
                    (*namespace).to_string(),
                    runtime_method_name(&method.sig.ident.to_string()),
                ))
                .or_default()
                .push(
                    method_arguments(&method.sig)?
                        .into_iter()
                        .map(|argument| argument.trim_start_matches("r#").to_string())
                        .collect(),
                );
        }
    }
    Ok(methods)
}

/// Rejects generated JavaScript when its syntax tree contains any parser error node.
fn validate_javascript_syntax(source: &str) -> Result<(), Box<dyn Error>> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_javascript::LANGUAGE.into())?;
    let tree = parser
        .parse(source, None)
        .ok_or("JavaScript parser did not produce a syntax tree")?;
    if tree.root_node().has_error() {
        Err("generated JavaScript Tools runtime contains a syntax error".into())
    } else {
        Ok(())
    }
}

/// Reads owned argument identifiers from one host method signature.
fn method_arguments(signature: &syn::Signature) -> Result<Vec<String>, Box<dyn Error>> {
    signature
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Receiver(_) => None,
            FnArg::Typed(argument) => Some(&*argument.pat),
        })
        .map(|pattern| match pattern {
            Pat::Ident(pattern) => Ok(pattern.ident.to_string()),
            _ => Err(format!(
                "host method `{}` must use identifier arguments",
                signature.ident
            )
            .into()),
        })
        .collect()
}

/// Emits one generated host method body using the shared runtime parameter adapter.
fn generated_method_body(
    namespace: &str,
    method: &str,
    tool_variant: &str,
    arguments: &[String],
) -> String {
    if namespace == "System.terminal" && method == "execStreaming" {
        return format!(
            "        invoke_terminal_streaming(\n            self,\n            BuiltinToolName::{tool_variant},\n            sessionId,\n            command,\n            options,\n        )\n"
        );
    }
    if namespace == "Chat" && method == "sendMessageStreaming" {
        return format!(
            "        invoke_chat_streaming(\n            self,\n            BuiltinToolName::{tool_variant},\n            message,\n            chatId,\n            roleCardId,\n            senderName,\n            options,\n        )\n"
        );
    }
    let ignores_uninhabited_options = matches!(
        (namespace, method),
        ("Net", "browserNavigateBack") | ("Net", "browserClose") | ("Net", "browserCloseAll")
    );
    let arguments = arguments
        .iter()
        .map(|argument| {
            if ignores_uninhabited_options && argument == "options" {
                "generated_empty_argument(\"options\")".to_string()
            } else {
                format!("generated_argument(\"{argument}\", {argument})")
            }
        })
        .collect::<Vec<_>>();
    let arguments = if arguments.is_empty() {
        "generated_no_arguments()".to_string()
    } else {
        format!("vec![{}]", arguments.join(", "))
    };
    format!(
        "        invoke_generated(\n            self,\n            BuiltinToolName::{tool_variant},\n            \"{namespace}\",\n            \"{method}\",\n            {arguments},\n        )\n"
    )
}

/// Converts one snake-case built-in name into its generated enum variant identifier.
fn rust_variant_name(name: &str) -> String {
    name.split('_')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut characters = segment.chars();
            let first = characters
                .next()
                .expect("filtered built-in name segments are non-empty");
            first.to_ascii_uppercase().to_string() + characters.as_str()
        })
        .collect()
}

/// Removes Rust-only raw-identifier and overload suffixes from a host method name.
fn runtime_method_name(name: &str) -> String {
    let name = name.strip_prefix("r#").unwrap_or(name);
    name.rsplit_once("_overload_")
        .map(|(base, _)| base)
        .unwrap_or(name)
        .to_string()
}

/// Converts one generated Pascal-case built-in variant into its stable snake-case name.
fn pascal_case_to_snake_case(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut output = String::new();
    for (index, character) in characters.iter().copied().enumerate() {
        let previous_is_lowercase = index
            .checked_sub(1)
            .and_then(|previous| characters.get(previous))
            .is_some_and(|previous| previous.is_ascii_lowercase() || previous.is_ascii_digit());
        let next_is_lowercase = characters
            .get(index + 1)
            .is_some_and(|next| next.is_ascii_lowercase());
        if character.is_ascii_uppercase()
            && index > 0
            && (previous_is_lowercase || next_is_lowercase)
        {
            output.push('_');
        }
        output.push(character.to_ascii_lowercase());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that generated JavaScript syntax is checked before build output is written.
    #[test]
    fn validates_generated_javascript_syntax() {
        validate_javascript_syntax("var Tools = { Files: {} };")
            .expect("valid JavaScript fixture must parse");
        assert!(validate_javascript_syntax("var Tools = {").is_err());
    }

    /// Verifies conversion from stable built-in names to generated Rust variants.
    #[test]
    fn converts_binding_tool_names() {
        assert_eq!(
            rust_variant_name("browser_take_screenshot"),
            "BrowserTakeScreenshot"
        );
    }

    /// Verifies versioned API variants can declare a distinct parameter shape.
    #[test]
    fn parses_variant_arguments() {
        let expression: Expr =
            syn::parse_str("Some(&[\"options\"])").expect("variant argument fixture must parse");
        assert_eq!(
            parse_optional_string_array(&expression).expect("variant arguments must parse"),
            Some(vec!["options".to_string()])
        );
    }

    /// Verifies versioned API variants can declare an exclusive upper bound.
    #[test]
    fn parses_variant_until() {
        let expression: Expr =
            syn::parse_str("Some(\"2.1.0\")").expect("variant upper-bound fixture must parse");
        assert_eq!(
            parse_optional_string(&expression, "until").expect("variant upper bound must parse"),
            Some("2.1.0".to_string())
        );
    }
}
