use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

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
    ("js_sdk/models.rs", "ModelsHost", "Models"),
    ("js_sdk/tts.rs", "TtsHost", "Tts"),
    ("js_sdk/stt.rs", "SttHost", "Stt"),
    ("js_sdk/theme.rs", "ThemeHost", "Theme"),
    ("js_sdk/catalog.rs", "CatalogHost", "Catalog"),
    ("js_sdk/chat.rs", "ChatHost", "Chat"),
    ("js_sdk/storage.rs", "StorageHost", "Storage"),
];

/// Describes one generated JavaScript Tools method binding.
#[derive(Clone, Debug)]
struct JsToolBindingSpec {
    tool: String,
    strict_arguments: bool,
    facade: String,
    fixed_parameters: Vec<(String, String)>,
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
        let mut strict_arguments = false;
        let mut facade = None;
        let mut fixed_parameters = None;
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
                ("target", Expr::Path(value)) => {
                    let segments = &value.path.segments;
                    if segments.len() != 2 || segments[0].ident != "BuiltinToolName" {
                        return Err("Tools must target a canonical BuiltinToolName".into());
                    }
                    tool = Some(pascal_case_to_snake_case(&segments[1].ident.to_string()));
                }
                ("strict_arguments", Expr::Lit(value)) => {
                    let Lit::Bool(value) = &value.lit else { return Err("Argument validation must be a boolean".into()); };
                    strict_arguments = value.value;
                }
                ("facade", Expr::Path(value)) => facade = Some(value.path.segments.last().ok_or("Result facade is missing")?.ident.to_string()),
                ("fixed_parameters", expression) => fixed_parameters = Some(parse_fixed_parameters(expression)?),
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
                    tool: tool.ok_or("JsToolBinding target is missing")?,
                    strict_arguments,
                    facade: facade.ok_or("JsToolBinding result facade is missing")?,
                    fixed_parameters: fixed_parameters.ok_or("Builtin binding fixed parameters must be declared")?,
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

    let parameter_rules = generate_parameter_rules(bindings_path, &methods)?;
    let mut output = format!("var __operitToolParameterRules = {parameter_rules};\n");
    output.push_str(
        r#"// Generated from canonical Rust Tools traits and bindings. Do not edit.
var Tools = {};
var tools = Tools;

/** Converts public argument spelling to the executor naming convention. */
function __operitToolsSnakeCase(name) {
    return String(name).replace(/[A-Z]/g, function(character) {
        return "_" + character.toLowerCase();
    });
}

/** Selects object expansion using the parameter conventions declared in the SDK contract. */
function __operitToolsFlattens(name) {
    return __operitToolParameterRules.flattenArguments.indexOf(name) !== -1 ||
        __operitToolParameterRules.scalarUnionSuffixes.some(function(suffix) { return name.endsWith(suffix); });
}

/** Resolves the scalar branch name of a declared union argument. */
function __operitToolsScalarField(name) {
    for (var suffix of __operitToolParameterRules.scalarUnionSuffixes) {
        if (name.endsWith(suffix)) return __operitToolsSnakeCase(name.slice(0, -suffix.length));
    }
    return __operitToolsSnakeCase(name);
}

/** Resolves an exact positional alias or applies the declared snake-case naming convention. */
function __operitToolsWireName(namespace, method, name) {
    var key = namespace + "." + method + "." + name;
    if (Object.prototype.hasOwnProperty.call(__operitToolParameterRules.argumentNames, key)) {
        return __operitToolParameterRules.argumentNames[key];
    }
    return __operitToolsSnakeCase(name);
}

/** Selects the declared object or positional overload from the actual arguments. */
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

/** Maps declared SDK arguments and fixed fields to one executor request. */
function __operitToolsBuildParameters(namespace, method, overloads, args, fixedParameters) {
    var names = __operitToolsSelectOverload(overloads, args);
    var parameters = {};
    for (var index = 0; index < names.length; index += 1) {
        var name = names[index];
        var value = args[index];
        if (__operitToolsFlattens(name)) {
            if (value === undefined || value === null) continue;
            if (typeof value === "object" && !Array.isArray(value)) {
                Object.keys(value).forEach(function(field) {
                    var wireField = __operitToolParameterRules.preserveObjectNamespaces.indexOf(namespace) !== -1 ? field : __operitToolsSnakeCase(field);
                    parameters[wireField] = value[field];
                });
            } else {
                parameters[__operitToolsScalarField(name)] = value;
            }
        } else if (value !== undefined) {
            parameters[__operitToolsWireName(namespace, method, name)] = value;
        }
    }
    for (var fixed of fixedParameters) parameters[fixed[0]] = fixed[1];
    return parameters;
}

/** Sends one mapped request through the existing structured tool transport. */
function __operitInvokeToolsBinding(namespace, method, toolName, overloads, args, fixedParameters, receiver) {
    var parameters = __operitToolsBuildParameters(namespace, method, overloads, args, fixedParameters);
    if (receiver && receiver.__operitDeviceContext) {
        var device = receiver.__operitDeviceContext;
        return __operitDeviceToolCall(device.nodeId, "default", toolName, parameters, device.options);
    }
    return toolCall(toolName, parameters);
}


"#,
    );
    let mut initialized_namespaces = BTreeSet::new();
    output.push_str(&fs::read_to_string(sdk_src.join("chat_runtime.js"))?);
    output.push_str(&fs::read_to_string(sdk_src.join("js_sdk/storage_runtime.js"))?);
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
        // Argument checks and result facades never select another native execution path.
        let make_function = |signature: &str, overloads: &str| -> Result<String, Box<dyn Error>> {
            match binding.facade.as_str() {
                "AsyncIterator" => Ok(format!("function({signature}) {{ if (arguments.length !== 1) throw new Error(\"{namespace}.{method} requires exactly one request\"); return __operitCreateToolStream(\"{}\", {signature}); }}", binding.tool)),
                "Promise" | "Storage" => {
                    let validation = if binding.strict_arguments {
                        Some(argument_checks(sdk_src, &namespace, &method)?)
                    } else { None };
                    Ok(generated_js_function(&namespace, &method, &binding.tool, signature, overloads, &binding.fixed_parameters, validation.as_deref()))
                }
                _ => Err(format!("Unknown result facade: {}", binding.facade).into()),
            }
        };
        let function = if binding.api_variants.is_empty() {
            make_function(&canonical_signature, &canonical_overloads)?
        } else {
            let mut expression = "__operitToolPkgApi.method()".to_string();
            for variant in &binding.api_variants {
                let signature = variant.arguments.as_ref().map(|args| args.join(", ")).unwrap_or_else(|| canonical_signature.clone());
                let overloads = variant.arguments.as_ref().map(|args| format!("[{}]", args.iter().map(|arg| format!("{arg:?}")).collect::<Vec<_>>().join(", "))).unwrap_or_else(|| canonical_overloads.clone());
                let function = make_function(&signature, &overloads)?;
                if let Some(until) = &variant.until {
                    expression.push_str(&format!(".between(\"{}\", \"{}\", {})", variant.since, until, function));
                } else {
                    expression.push_str(&format!(".since(\"{}\", {})", variant.since, function));
                }
            }
            expression
        };
        if namespace == "Chat" {
            if !chat_namespace_open {
                output.push_str("Tools[\"Chat\"] = __operitToolPkgApi.namespace(\"Tools.Chat\", {\n");
                chat_namespace_open = true;
            }
            output.push_str(&format!("\"{method}\": {function},\n"));
        } else {
            let function = if binding.api_variants.is_empty() { function } else { format!("{function}.build(\"Tools.{namespace}.{method}\")") };
            output.push_str(&format!("{expression}[\"{method}\"] = {function};\n"));
        }
        if binding.facade == "Storage" { output.push_str("__operitInstallStorageFacade();\n"); }
    }
    if chat_namespace_open {
        output.push_str("});\n");
    }
    output.push_str(r#"
function __operitCopyToolsNamespaces(source) {
    var target = {};
    Object.keys(source).forEach(function(key) {
        var value = source[key];
        target[key] = value && typeof value === "object" ? __operitCopyToolsNamespaces(value) : value;
    });
    return target;
}
var __operitCanonicalTools = __operitCopyToolsNamespaces(Tools);
"#);
    // Embed the handwritten legacy adapters after the canonical Tools namespace exists.
    for relative_path in [
        "compat/v1/files.js",
        "compat/v1/chat.js",
        "compat/v1/workflow.js",
        "compat/v1/characters.js",
        "compat/v1/memory.js",
        "compat/v1/install.js",
        "DeviceTools.script.js",
    ] {
        output.push('\n');
        output.push_str(&fs::read_to_string(sdk_src.join(relative_path))?);
    }
    validate_javascript_syntax(&output)?;
    fs::write(output_path, output)?;
    Ok(())
}

/// Counts the exact positional arguments required by a validated SDK method.
fn overloads_count(signature: &str) -> usize {
    if signature.is_empty() { 0 } else { signature.split(',').count() }
}

/// Derives public argument checks from canonical Rust types without selecting a separate transport.
fn argument_checks(sdk_src: &Path, namespace: &str, method: &str) -> Result<String, Box<dyn Error>> {
    let mut checks = Vec::new();
    for (path, trait_name, declared_namespace) in HOST_TRAITS {
        if *declared_namespace != namespace { continue; }
        let file = syn::parse_file(&fs::read_to_string(sdk_src.join(path))?)?;
        let contract = file.items.iter().find_map(|item| match item {
            Item::Trait(item) if item.ident == *trait_name => Some(item), _ => None,
        }).ok_or("Canonical Tools trait is missing")?;
        let operation = contract.items.iter().find_map(|item| match item {
            TraitItem::Fn(item) if runtime_method_name(&item.sig.ident.to_string()) == method => Some(item), _ => None,
        }).ok_or("Canonical Tools method is missing")?;
        for argument in &operation.sig.inputs {
            let FnArg::Typed(argument) = argument else { continue; };
            let Pat::Ident(name) = &*argument.pat else { return Err("Tool arguments require named fields".into()); };
            let name = name.ident.to_string();
            match &*argument.ty {
                syn::Type::Path(path) if path.path.is_ident("String") => checks.push(format!("if (typeof {name} !== 'string' || {name}.trim() === '' || {name}.trim() !== {name}) throw new TypeError('Identifier must be exact nonblank text');")),
                syn::Type::Path(_) => checks.push(format!("{name} = __operitSnapshotJsonObject({name});")),
                _ => return Err("Unsupported validated tool argument type".into()),
            }
        }
    }
    Ok(checks.join(" "))
}

/// Emits parameter metadata from the canonical contract and verifies every exact positional alias.
fn generate_parameter_rules(
    path: &Path,
    methods: &BTreeMap<(String, String), Vec<Vec<String>>>,
) -> Result<String, Box<dyn Error>> {
    let file = syn::parse_file(&fs::read_to_string(path)?)?;
    let item = file.items.iter().find_map(|item| match item {
        Item::Const(item) if item.ident == "JS_TOOL_PARAMETER_RULES" => Some(item),
        _ => None,
    }).ok_or("Missing canonical tool parameter rules")?;
    let Expr::Struct(rules) = &*item.expr else { return Err("Tool parameter rules must be declared as a struct".into()); };
    let mut fields = BTreeMap::new();
    for field in &rules.fields {
        let syn::Member::Named(name) = &field.member else { return Err("Parameter rule fields must be named".into()); };
        if fields.insert(name.to_string(), &field.expr).is_some() { return Err("Duplicate parameter rule field".into()); }
    }
    if fields.len() != 4 { return Err("Unexpected tool parameter rule fields".into()); }
    let text_array = |name: &str| -> Result<Vec<String>, Box<dyn Error>> {
        let Expr::Reference(reference) = fields.get(name).ok_or("Missing parameter rule array")? else { return Err("Parameter rules must use static arrays".into()); };
        parse_string_array(&reference.expr, name)
    };
    let flattened = text_array("flatten_arguments")?;
    let suffixes = text_array("scalar_union_suffixes")?;
    let preserved = text_array("preserve_object_namespaces")?;
    let Expr::Reference(reference) = fields.get("argument_names").ok_or("Missing positional argument names")? else { return Err("Argument names must use a static tuple array".into()); };
    let Expr::Array(entries) = &*reference.expr else { return Err("Argument names must be an array".into()); };
    let mut aliases = BTreeMap::new();
    for entry in &entries.elems {
        let Expr::Tuple(tuple) = entry else { return Err("Argument names must contain exact four-field tuples".into()); };
        let mut values = Vec::new();
        for value in &tuple.elems {
            let Expr::Lit(value) = value else { return Err("Argument names must be literals".into()); };
            let Lit::Str(value) = &value.lit else { return Err("Argument names must be text".into()); };
            values.push(value.value());
        }
        let [namespace, method, argument, wire]: [String; 4] = values.try_into().map_err(|_| "Argument aliases require exactly four fields")?;
        let overloads = methods.get(&(namespace.clone(), method.clone())).ok_or("Argument alias references an undeclared method")?;
        if !overloads.iter().any(|arguments| arguments.iter().any(|name| name == &argument)) { return Err("Argument alias references an undeclared positional argument".into()); }
        if aliases.insert(format!("{namespace}.{method}.{argument}"), wire).is_some() { return Err("Duplicate positional argument alias".into()); }
    }
    let array = |values: &[String]| format!("[{}]", values.iter().map(|value| format!("{value:?}")).collect::<Vec<_>>().join(", "));
    let aliases = aliases.iter().map(|(key, value)| format!("{key:?}: {value:?}")).collect::<Vec<_>>().join(", ");
    Ok(format!("{{flattenArguments: {}, scalarUnionSuffixes: {}, preserveObjectNamespaces: {}, argumentNames: {{{aliases}}}}}", array(&flattened), array(&suffixes), array(&preserved)))
}

/// Reads each explicitly declared fixed executor parameter from the canonical binding contract.
fn parse_fixed_parameters(expression: &Expr) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let Expr::Reference(reference) = expression else { return Err("Fixed parameters must be a static tuple slice".into()); };
    let Expr::Array(array) = &*reference.expr else { return Err("Fixed parameters must be an array".into()); };
    let mut parameters = Vec::new();
    let mut names = BTreeSet::new();
    for entry in &array.elems {
        let Expr::Tuple(tuple) = entry else { return Err("Fixed parameters must contain string pairs".into()); };
        let mut fields = Vec::new();
        for field in &tuple.elems {
            let Expr::Lit(value) = field else { return Err("Fixed parameter fields must be string literals".into()); };
            let Lit::Str(value) = &value.lit else { return Err("Fixed parameter fields must be text".into()); };
            fields.push(value.value());
        }
        let [name, value]: [String; 2] = fields.try_into().map_err(|_| "Fixed parameters require exactly two fields")?;
        if !names.insert(name.clone()) { return Err("Duplicate fixed executor parameter".into()); }
        parameters.push((name, value));
    }
    Ok(parameters)
}

/// Generates one JavaScript function expression for a Tools API variant.
fn generated_js_function(
    namespace: &str,
    method: &str,
    tool: &str,
    signature: &str,
    overloads: &str,
    fixed_parameters: &[(String, String)],
    validation: Option<&str>,
) -> String {
    let fixed_parameters = format!("[{}]", fixed_parameters.iter().map(|(key, value)| format!("[{key:?}, {value:?}]")).collect::<Vec<_>>().join(", "));
    let invoke = format!("return __operitInvokeToolsBinding(\"{namespace}\", \"{method}\", \"{tool}\", [{overloads}], Array.prototype.slice.call(arguments), {fixed_parameters}, this);");
    match validation {
        Some(validation) => {
            let count = overloads_count(signature);
            format!("function({signature}) {{ if (arguments.length !== {count}) return Promise.reject(new Error(\"{namespace}.{method} requires exactly {count} arguments\")); try {{ {validation} {invoke} }} catch (error) {{ return Promise.reject(error); }} }}")
        }
        None => format!("function({signature}) {{\n    {invoke}\n}}"),
    }
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

/// Converts one snake-case built-in name into its generated enum variant identifier.
#[cfg(test)]
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
