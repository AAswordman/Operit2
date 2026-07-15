use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::Path;

use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprForLoop, ExprMethodCall, Item, Pat, Type};

const REGISTRATION_HELPERS: &[&str] = &[
    "registerChatTool",
    "registerSystemOperationTool",
    "registerTerminalTool",
    "registerMusicTool",
    "registerBluetoothTool",
    "registerMemoryTool",
    "registerHttpTool",
    "registerFileSystemTool",
];

/// Generates the typed built-in tool-name catalog from the canonical Rust result map.
pub fn generate_builtin_tool_names(
    tool_types_path: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let tool_types = syn::parse_file(&fs::read_to_string(tool_types_path)?)?;
    let entries = required_tool_entries_in_order(&tool_types)?;
    let variants = entries
        .iter()
        .map(|(name, result)| (name.as_str(), rust_variant_name(name), result.as_ref()))
        .collect::<Vec<_>>();
    let mut output = String::from(
        "// Generated from ToolResultMap. Do not edit.\n\n\
         /// Identifies one statically declared built-in tool.\n\
         #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\n\
         pub enum BuiltinToolName {\n",
    );
    for (name, variant, _) in &variants {
        output.push_str(&format!(
            "    /// Selects the `{name}` built-in tool.\n    {variant},\n"
        ));
    }
    output.push_str("}\n\nimpl BuiltinToolName {\n");
    output.push_str("    /// Contains every statically declared built-in tool.\n");
    output.push_str("    pub const ALL: &'static [Self] = &[\n");
    for (_, variant, _) in &variants {
        output.push_str(&format!("        Self::{variant},\n"));
    }
    output.push_str("    ];\n\n");
    output.push_str(
        "    /// Returns the stable tool name used by JavaScript and the runtime registry.\n",
    );
    output.push_str("    pub const fn as_str(self) -> &'static str {\n        match self {\n");
    for (name, variant, _) in &variants {
        output.push_str(&format!("            Self::{variant} => \"{name}\",\n"));
    }
    output.push_str("        }\n    }\n\n");
    output.push_str(
        "    /// Resolves an exact runtime name to its statically declared built-in tool.\n",
    );
    output.push_str("    pub fn from_name(name: &str) -> Option<Self> {\n        match name {\n");
    for (name, variant, _) in &variants {
        output.push_str(&format!(
            "            \"{name}\" => Some(Self::{variant}),\n"
        ));
    }
    output.push_str("            _ => None,\n        }\n    }\n\n");
    output.push_str(
        "    /// Reports whether a successful runtime payload matches this tool's public contract.\n",
    );
    output.push_str(
        "    pub fn accepts_runtime_result(self, result: &ToolResultData) -> bool {\n        match self {\n",
    );
    for (_, variant, result_variant) in &variants {
        let pattern = match (variant.as_str(), result_variant) {
            ("ExecuteInTerminalSessionStreaming", _) => {
                "ToolResultData::TerminalStreamEventData(_) | ToolResultData::TerminalCommandResultData(_)".to_string()
            }
            (_, Some(result_variant)) => format!("ToolResultData::{result_variant}(_)") ,
            (_, None) => "_".to_string(),
        };
        output.push_str(&format!(
            "            Self::{variant} => matches!(result, {pattern}),\n"
        ));
    }
    output.push_str("        }\n    }\n}\n\n");
    output.push_str("impl std::fmt::Display for BuiltinToolName {\n");
    output.push_str("    /// Formats the stable runtime tool name.\n");
    output.push_str(
        "    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n\
                 formatter.write_str(self.as_str())\n\
             }\n\
         }\n",
    );
    fs::write(output_path, output)?;
    Ok(())
}

/// Verifies that every canonical built-in tool has a structured registration expression.
pub fn check_tool_registration_contract(
    tool_types_path: &Path,
    registration_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let tool_types = syn::parse_file(&fs::read_to_string(tool_types_path)?)?;
    let registration = syn::parse_file(&fs::read_to_string(registration_path)?)?;
    let required = required_tool_names(&tool_types)?;
    let mut visitor = RegistrationVisitor::default();
    visitor.visit_file(&registration);
    let registered = visitor.registered.keys().cloned().collect::<BTreeSet<_>>();
    let missing = required
        .difference(&registered)
        .cloned()
        .collect::<Vec<_>>();
    let unexpected = registered
        .difference(&required)
        .cloned()
        .collect::<Vec<_>>();
    let duplicates = visitor
        .registered
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(name, count)| format!("{name} ({count})"))
        .collect::<Vec<_>>();
    if !missing.is_empty()
        || !unexpected.is_empty()
        || !duplicates.is_empty()
        || !visitor.pairing_errors.is_empty()
    {
        return Err(format!(
            "ToolRegistration differs from the Rust SDK tool map; missing: [{}]; untyped: [{}]; duplicate: [{}]; operation mismatch: [{}]",
            missing.join(", "),
            unexpected.join(", "),
            duplicates.join(", "),
            visitor.pairing_errors.join(", ")
        )
        .into());
    }
    Ok(())
}

/// Reads required tool keys from the canonical Rust ToolResultMap fields.
fn required_tool_names(file: &syn::File) -> Result<BTreeSet<String>, Box<dyn Error>> {
    Ok(required_tool_names_in_order(file)?.into_iter().collect())
}

/// Reads required tool keys in declaration order from the canonical Rust result map.
fn required_tool_names_in_order(file: &syn::File) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(required_tool_entries_in_order(file)?
        .into_iter()
        .map(|(name, _)| name)
        .collect())
}

/// Reads built-in tool names and their successful runtime result variants.
fn required_tool_entries_in_order(
    file: &syn::File,
) -> Result<Vec<(String, Option<String>)>, Box<dyn Error>> {
    let item = file.items.iter().find_map(|item| match item {
        Item::Struct(item) if item.ident == "ToolResultMap" => Some(item),
        _ => None,
    });
    let item = item.ok_or("ToolResultMap is missing from the Rust SDK")?;
    let syn::Fields::Named(fields) = &item.fields else {
        return Err("ToolResultMap must use named fields".into());
    };
    fields
        .named
        .iter()
        .map(|field| {
            let name = field
                .ident
                .as_ref()
                .expect("named ToolResultMap fields have identifiers")
                .to_string();
            Ok((name, runtime_result_variant(&field.ty)?))
        })
        .collect()
}

/// Maps one public JS result type to the internal runtime enum variant carrying it.
fn runtime_result_variant(ty: &Type) -> Result<Option<String>, Box<dyn Error>> {
    let Type::Path(path) = ty else {
        return Err("ToolResultMap fields must use path result types".into());
    };
    let result_type = path
        .path
        .segments
        .last()
        .ok_or("ToolResultMap result type path is empty")?
        .ident
        .to_string();
    Ok(match result_type.as_str() {
        "String" => Some("StringResultData".to_string()),
        "Value" | "ToolResultData" => None,
        _ => Some(result_type),
    })
}

/// Converts one snake-case tool name into a Rust enum variant identifier.
fn rust_variant_name(name: &str) -> String {
    name.split('_')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut characters = segment.chars();
            let first = characters
                .next()
                .expect("filtered tool-name segments are non-empty");
            first.to_ascii_uppercase().to_string() + characters.as_str()
        })
        .collect()
}

#[derive(Default)]
struct RegistrationVisitor {
    registered: BTreeMap<String, usize>,
    pairing_errors: Vec<String>,
}

impl RegistrationVisitor {
    /// Records a typed built-in tool used by one structured registration expression.
    fn record_builtin(&mut self, expression: &Expr) {
        if let Some(value) = builtin_tool_name(expression) {
            *self.registered.entry(value).or_default() += 1;
        }
    }

    /// Verifies that one grouped executor operation is the unique operation for its built-in name.
    fn record_operation_pair<'a>(&mut self, arguments: impl Iterator<Item = &'a Expr>) {
        let arguments = arguments.collect::<Vec<_>>();
        let builtin = arguments
            .iter()
            .find_map(|argument| builtin_tool_variant(argument));
        let operation = arguments
            .iter()
            .find_map(|argument| operation_variant(argument));
        let (Some(builtin), Some((family, actual))) = (builtin, operation) else {
            return;
        };
        let Some(expected) = expected_operation_variant(&family, &builtin) else {
            self.pairing_errors
                .push(format!("{builtin} has no {family} contract"));
            return;
        };
        if actual != expected {
            self.pairing_errors.push(format!(
                "{builtin} requires {family}::{expected}, found {family}::{actual}"
            ));
        }
    }
}

impl<'ast> Visit<'ast> for RegistrationVisitor {
    /// Visits helper calls whose string argument is the registered tool name.
    fn visit_expr_call(&mut self, expression: &'ast ExprCall) {
        if let Expr::Path(function) = expression.func.as_ref() {
            let function_name = function
                .path
                .segments
                .last()
                .expect("registration helper paths have a segment")
                .ident
                .to_string();
            if REGISTRATION_HELPERS.contains(&function_name.as_str()) {
                for argument in &expression.args {
                    self.record_builtin(argument);
                }
                self.record_operation_pair(expression.args.iter());
            }
        }
        visit::visit_expr_call(self, expression);
    }

    /// Visits direct handler registration methods with a literal first argument.
    fn visit_expr_method_call(&mut self, expression: &'ast ExprMethodCall) {
        if expression.method == "registerBuiltinTool" {
            if let Some(argument) = expression.args.first() {
                self.record_builtin(argument);
            }
            self.record_operation_pair(expression.args.iter());
        }
        visit::visit_expr_method_call(self, expression);
    }

    /// Visits literal tool-name arrays consumed by registration loops.
    fn visit_expr_for_loop(&mut self, expression: &'ast ExprForLoop) {
        let Pat::Ident(binding) = &*expression.pat else {
            visit::visit_expr_for_loop(self, expression);
            return;
        };
        let mut use_visitor = RegistrationBindingVisitor {
            binding: binding.ident.to_string(),
            used: false,
        };
        use_visitor.visit_block(&expression.body);
        if use_visitor.used {
            if let Expr::Array(array) = expression.expr.as_ref() {
                for element in &array.elems {
                    self.record_builtin(element);
                }
            }
        }
        visit::visit_expr_for_loop(self, expression);
    }
}

struct RegistrationBindingVisitor {
    binding: String,
    used: bool,
}

impl<'ast> Visit<'ast> for RegistrationBindingVisitor {
    /// Detects a loop binding passed as the first direct registration argument.
    fn visit_expr_method_call(&mut self, expression: &'ast ExprMethodCall) {
        if expression.method == "registerBuiltinTool" {
            self.used = expression
                .args
                .first()
                .is_some_and(|argument| expression_uses_binding(argument, &self.binding));
        }
        visit::visit_expr_method_call(self, expression);
    }
}

/// Resolves a `BuiltinToolName` enum path into its stable snake-case tool name.
fn builtin_tool_name(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Path(expression)
            if expression
                .path
                .segments
                .iter()
                .any(|segment| segment.ident == "BuiltinToolName") =>
        {
            expression
                .path
                .segments
                .last()
                .map(|segment| pascal_case_to_snake_case(&segment.ident.to_string()))
        }
        Expr::Paren(expression) => builtin_tool_name(&expression.expr),
        Expr::Group(expression) => builtin_tool_name(&expression.expr),
        _ => None,
    }
}

/// Returns the Rust enum variant naming one built-in tool expression.
fn builtin_tool_variant(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Path(expression)
            if expression
                .path
                .segments
                .iter()
                .any(|segment| segment.ident == "BuiltinToolName") =>
        {
            expression
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
        }
        Expr::Paren(expression) => builtin_tool_variant(&expression.expr),
        Expr::Group(expression) => builtin_tool_variant(&expression.expr),
        _ => None,
    }
}

/// Finds an operation enum variant inside a registration argument expression.
fn operation_variant(expression: &Expr) -> Option<(String, String)> {
    struct OperationVisitor {
        operation: Option<(String, String)>,
    }

    impl<'ast> Visit<'ast> for OperationVisitor {
        /// Records the first path whose enum type ends in `ToolOperation`.
        fn visit_expr_path(&mut self, expression: &'ast syn::ExprPath) {
            if self.operation.is_none() && expression.path.segments.len() >= 2 {
                let segments = expression.path.segments.iter().collect::<Vec<_>>();
                let family = segments[segments.len() - 2].ident.to_string();
                if family.ends_with("ToolOperation") {
                    self.operation = Some((family, segments[segments.len() - 1].ident.to_string()));
                    return;
                }
            }
            visit::visit_expr_path(self, expression);
        }
    }

    let mut visitor = OperationVisitor { operation: None };
    visitor.visit_expr(expression);
    visitor.operation
}

/// Resolves the exact grouped executor operation required for one built-in variant.
fn expected_operation_variant(family: &str, builtin: &str) -> Option<String> {
    match family {
        "ChatManagerToolOperation"
        | "FileSystemToolOperation"
        | "HttpToolOperation"
        | "MemoryToolOperation" => Some(builtin.to_string()),
        "SystemOperationToolOperation" => Some(
            match builtin {
                "DeviceInfo" => "GetDeviceInfo",
                value => value,
            }
            .to_string(),
        ),
        "TerminalToolOperation" => Some(
            match builtin {
                "GetTerminalInfo" => "GetTerminalInfo",
                "CreateTerminalSession" => "CreateSession",
                "ExecuteInTerminalSession" => "ExecuteInSession",
                "ExecuteInTerminalSessionStreaming" => "ExecuteInSessionStreaming",
                "ExecuteHiddenTerminalCommand" => "ExecuteHiddenCommand",
                "CloseTerminalSession" => "CloseSession",
                "InputInTerminalSession" => "InputInSession",
                "GetTerminalSessionScreen" => "GetSessionScreen",
                _ => return None,
            }
            .to_string(),
        ),
        "MusicToolOperation" => Some(
            match builtin {
                "MusicPlay" => "Play",
                "MusicPause" => "Pause",
                "MusicResume" => "Resume",
                "MusicStop" => "Stop",
                "MusicSeek" => "Seek",
                "MusicSetVolume" => "SetVolume",
                "MusicStatus" => "Status",
                _ => return None,
            }
            .to_string(),
        ),
        "BluetoothToolOperation" => Some(
            match builtin {
                "RequestBluetoothPermission" => "RequestPermission",
                "GetBluetoothState" => "GetState",
                "RequestEnableBluetooth" => "RequestEnable",
                "ListBluetoothBondedDevices" => "ListBondedDevices",
                "ScanBluetoothDevices" => "ScanDevices",
                "BluetoothConnect" => "Connect",
                "BluetoothListen" => "Listen",
                "BluetoothAccept" => "Accept",
                "BluetoothSend" => "Send",
                "BluetoothRead" => "Read",
                "BluetoothSendAndRead" => "SendAndRead",
                "BluetoothClose" => "Close",
                "BluetoothBleConnect" => "BleConnect",
                "BluetoothBleDiscoverServices" => "BleDiscoverServices",
                "BluetoothBleReadCharacteristic" => "BleReadCharacteristic",
                "BluetoothBleWriteCharacteristic" => "BleWriteCharacteristic",
                "BluetoothBleWriteAndReadCharacteristic" => "BleWriteAndReadCharacteristic",
                "BluetoothBleSubscribeCharacteristic" => "BleSubscribeCharacteristic",
                "BluetoothBleReadNotifications" => "BleReadNotifications",
                _ => return None,
            }
            .to_string(),
        ),
        _ => None,
    }
}

/// Converts a generated PascalCase enum variant back into its snake-case tool name.
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

/// Reports whether an expression resolves directly to one loop binding.
fn expression_uses_binding(expression: &Expr, binding: &str) -> bool {
    match expression {
        Expr::Path(expression) => expression.path.is_ident(binding),
        Expr::MethodCall(expression) if expression.method == "to_string" => {
            expression_uses_binding(&expression.receiver, binding)
        }
        Expr::Paren(expression) => expression_uses_binding(&expression.expr, binding),
        Expr::Group(expression) => expression_uses_binding(&expression.expr, binding),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that omitting one canonical tool is visible as a registration contract gap.
    #[test]
    fn detects_missing_tool_registration() {
        let tool_types =
            syn::parse_file("pub struct ToolResultMap { pub alpha: String, pub beta: String }")
                .expect("test tool map must parse");
        let registration = syn::parse_file(
            "fn register(handler: &mut Handler) { registerFileSystemTool(handler, BuiltinToolName::Alpha); }",
        )
        .expect("test registration must parse");
        let required = required_tool_names(&tool_types).expect("test tool map must be valid");
        let mut visitor = RegistrationVisitor::default();
        visitor.visit_file(&registration);
        let registered = visitor.registered.keys().cloned().collect::<BTreeSet<_>>();

        assert_eq!(
            required
                .difference(&registered)
                .cloned()
                .collect::<Vec<_>>(),
            vec!["beta".to_string()]
        );
    }

    /// Verifies that registering one canonical tool more than once is visible.
    #[test]
    fn detects_duplicate_tool_registration() {
        let registration = syn::parse_file(
            "fn register(handler: &mut Handler) {\n\
                 registerFileSystemTool(handler, BuiltinToolName::Alpha);\n\
                 handler.registerBuiltinTool(BuiltinToolName::Alpha, executor, visibility);\n\
             }",
        )
        .expect("test registration must parse");
        let mut visitor = RegistrationVisitor::default();
        visitor.visit_file(&registration);

        assert_eq!(visitor.registered.get("alpha"), Some(&2));
    }

    /// Verifies that a built-in name cannot be paired with another grouped executor operation.
    #[test]
    fn detects_mismatched_grouped_operation() {
        let registration = syn::parse_file(
            "fn register(handler: &mut Handler) {\n\
                 registerFileSystemTool(\n\
                     handler,\n\
                     tools,\n\
                     BuiltinToolName::ReadFile,\n\
                     FileSystemToolOperation::WriteFile,\n\
                 );\n\
             }",
        )
        .expect("test registration must parse");
        let mut visitor = RegistrationVisitor::default();
        visitor.visit_file(&registration);

        assert_eq!(
            visitor.pairing_errors,
            vec![
                "ReadFile requires FileSystemToolOperation::ReadFile, found FileSystemToolOperation::WriteFile"
                    .to_string()
            ]
        );
    }

    /// Verifies reversible conversion between generated enum variants and tool names.
    #[test]
    fn converts_generated_variant_names() {
        assert_eq!(
            rust_variant_name("execute_cli_command"),
            "ExecuteCliCommand"
        );
        assert_eq!(
            pascal_case_to_snake_case("ExecuteCliCommand"),
            "execute_cli_command"
        );
        assert_eq!(
            pascal_case_to_snake_case("BluetoothBleConnect"),
            "bluetooth_ble_connect"
        );
    }
}
