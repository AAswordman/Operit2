use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::Path;

use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprForLoop, ExprMethodCall, Item, Lit, Pat};

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
    let missing = required
        .difference(&visitor.registered)
        .cloned()
        .collect::<Vec<_>>();
    let unexpected = visitor
        .registered
        .difference(&required)
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() || !unexpected.is_empty() {
        return Err(format!(
            "ToolRegistration differs from the Rust SDK tool map; missing: [{}]; untyped: [{}]",
            missing.join(", "),
            unexpected.join(", ")
        )
        .into());
    }
    Ok(())
}

/// Reads required tool keys from the canonical Rust ToolResultMap fields.
fn required_tool_names(file: &syn::File) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let item = file.items.iter().find_map(|item| match item {
        Item::Struct(item) if item.ident == "ToolResultMap" => Some(item),
        _ => None,
    });
    let item = item.ok_or("ToolResultMap is missing from the Rust SDK")?;
    let syn::Fields::Named(fields) = &item.fields else {
        return Err("ToolResultMap must use named fields".into());
    };
    Ok(fields
        .named
        .iter()
        .map(|field| {
            field
                .ident
                .as_ref()
                .expect("named ToolResultMap fields have identifiers")
                .to_string()
        })
        .collect())
}

#[derive(Default)]
struct RegistrationVisitor {
    registered: BTreeSet<String>,
}

impl RegistrationVisitor {
    /// Records a string literal used as a structured tool registration argument.
    fn record_literal(&mut self, expression: &Expr) {
        if let Some(value) = string_literal(expression) {
            self.registered.insert(value);
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
                    self.record_literal(argument);
                }
            }
        }
        visit::visit_expr_call(self, expression);
    }

    /// Visits direct handler registration methods with a literal first argument.
    fn visit_expr_method_call(&mut self, expression: &'ast ExprMethodCall) {
        if matches!(expression.method.to_string().as_str(), "registerTool" | "registerInternalTool")
        {
            if let Some(argument) = expression.args.first() {
                self.record_literal(argument);
            }
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
                    self.record_literal(element);
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
        if matches!(expression.method.to_string().as_str(), "registerTool" | "registerInternalTool")
        {
            self.used = expression
                .args
                .first()
                .is_some_and(|argument| expression_uses_binding(argument, &self.binding));
        }
        visit::visit_expr_method_call(self, expression);
    }
}

/// Extracts a string literal through ordinary parenthesis and to_string wrappers.
fn string_literal(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Lit(expression) => match &expression.lit {
            Lit::Str(value) => Some(value.value()),
            _ => None,
        },
        Expr::MethodCall(expression) if expression.method == "to_string" => {
            string_literal(&expression.receiver)
        }
        Expr::Paren(expression) => string_literal(&expression.expr),
        Expr::Group(expression) => string_literal(&expression.expr),
        _ => None,
    }
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
        let tool_types = syn::parse_file(
            "pub struct ToolResultMap { pub alpha: String, pub beta: String }",
        )
        .expect("test tool map must parse");
        let registration = syn::parse_file(
            "fn register(handler: &mut Handler) { registerFileSystemTool(handler, \"alpha\"); }",
        )
        .expect("test registration must parse");
        let required = required_tool_names(&tool_types).expect("test tool map must be valid");
        let mut visitor = RegistrationVisitor::default();
        visitor.visit_file(&registration);

        assert_eq!(
            required
                .difference(&visitor.registered)
                .cloned()
                .collect::<Vec<_>>(),
            vec!["beta".to_string()]
        );
    }
}
