use std::{collections::BTreeMap, path::Path};

use oxc_allocator::{Allocator, ArenaVec, TakeIn};
use oxc_ast::{
    ast::{
        Argument, ClassElement, Directive, Expression, Function, IdentifierName,
        IdentifierReference, Statement, StringLiteral,
    },
    builder::AstBuilder,
};
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_codegen::Codegen;
use oxc_parser::{ParseOptions, Parser};
use oxc_semantic::{ScopeFlags, SemanticBuilder};
use oxc_span::{GetSpan, SourceType, Span, SPAN};
use oxc_transformer::{HelperLoaderMode, TransformOptions, Transformer};

/// Keeps generated async helpers separate from plugin-authored Babel bindings.
struct AsyncHelpers<'a> {
    allocator: &'a Allocator,
    kinds: BTreeMap<(u32, u32), &'static str>,
}

/// Retains portable callback source before async lowering introduces private wrapper closures.
struct AsyncSources<'a> {
    source: &'a str,
    allocator: &'a Allocator,
    kinds: BTreeMap<(u32, u32), &'static str>,
}

impl<'a> AsyncSources<'a> {
    /// Attaches the original async function text without changing its callable identity.
    fn remember(&self, function: Expression<'a>, span: Span) -> Expression<'a> {
        let builder = AstBuilder::new(self.allocator);
        let helper = Expression::new_static_member_expression(
            SPAN,
            Expression::new_identifier(SPAN, "__operitAsyncHelpers", &builder),
            IdentifierName::new(SPAN, "remember", &builder),
            false,
            &builder,
        );
        let source = self
            .allocator
            .alloc_str(&self.source[span.start as usize..span.end as usize]);
        Expression::new_call_expression(
            SPAN,
            helper,
            None,
            ArenaVec::from_array_in(
                [
                    Argument::from(function),
                    Argument::from(Expression::new_string_literal(SPAN, source, None, &builder)),
                ],
                &builder,
            ),
            false,
            &builder,
        )
    }
}

impl<'a> VisitMut<'a> for AsyncSources<'a> {
    /// Records explicit function kinds before lowering removes the native async prototypes.
    fn visit_function(&mut self, function: &mut Function<'a>, flags: ScopeFlags) {
        if function.r#async {
            self.kinds.insert(
                (function.span.start, function.span.end),
                if function.generator {
                    "operit.async.generator"
                } else {
                    "operit.async.function"
                },
            );
        }
        walk_mut::walk_function(self, function, flags);
    }

    /// Preserves inline async callbacks, including arrows and async generators.
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        let is_async = match expression {
            Expression::FunctionExpression(function) => function.r#async,
            Expression::ArrowFunctionExpression(function) => function.r#async,
            _ => false,
        };
        let span = expression.span();
        walk_mut::walk_expression(self, expression);
        if is_async {
            *expression = self.remember(expression.take_in(&self.allocator), span);
        }
    }

    /// Records hoisted declarations before any statement can serialize their function references.
    fn visit_statements(&mut self, statements: &mut ArenaVec<'a, Statement<'a>>) {
        let builder = AstBuilder::new(self.allocator);
        let registrations = statements
            .iter()
            .filter_map(|statement| {
                let Statement::FunctionDeclaration(function) = statement else {
                    return None;
                };
                if !function.r#async {
                    return None;
                }
                let name = function.id.as_ref()?.name;
                Some(Statement::new_expression_statement(
                    SPAN,
                    self.remember(
                        Expression::new_identifier(SPAN, name, &builder),
                        function.span,
                    ),
                    &builder,
                ))
            })
            .collect::<Vec<_>>();
        walk_mut::walk_statements(self, statements);
        statements.splice(0..0, registrations);
    }
}

impl<'a> VisitMut<'a> for AsyncHelpers<'a> {
    /// Retains a parsed ABI directive for decorators that distinguish async methods and generators.
    fn visit_function(&mut self, function: &mut Function<'a>, flags: ScopeFlags) {
        if let Some(kind) = self.kinds.get(&(function.span.start, function.span.end)) {
            if let Some(body) = &mut function.body {
                let builder = AstBuilder::new(self.allocator);
                body.directives.push(Directive::new(
                    SPAN,
                    StringLiteral::new(SPAN, *kind, None, &builder),
                    *kind,
                    &builder,
                ));
            }
        }
        walk_mut::walk_function(self, function, flags);
    }

    /// Renames only synthesized helper references, preserving all original identifiers.
    fn visit_identifier_reference(&mut self, identifier: &mut IdentifierReference<'a>) {
        if identifier.span.is_empty() && identifier.name == "babelHelpers" {
            identifier.name = "__operitAsyncHelpers".into();
        }
    }
}

/// Routes native async functions and iterators through the runtime's owned Promise continuations.
pub(crate) fn compile(source: &str) -> Result<String, String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::cjs())
        .with_options(ParseOptions {
            allow_return_outside_function: true,
            ..ParseOptions::default()
        })
        .parse();
    if !parsed.diagnostics.is_empty() {
        return Err(format!(
            "JavaScript parsing failed: {:?}",
            parsed.diagnostics
        ));
    }
    let mut program = parsed.program;
    let mut sources = AsyncSources {
        source,
        allocator: &allocator,
        kinds: BTreeMap::new(),
    };
    sources.visit_program(&mut program);
    let semantic = SemanticBuilder::new().build(&program);
    if !semantic.diagnostics.is_empty() {
        return Err(format!(
            "JavaScript scope analysis failed: {:?}",
            semantic.diagnostics
        ));
    }
    let mut options = TransformOptions::default();
    options.env.es2017.async_to_generator = true;
    options.env.es2018.async_generator_functions = true;
    options.helper_loader.mode = HelperLoaderMode::External;
    let transformed = Transformer::new(&allocator, Path::new("operit.js"), &options)
        .build_with_scoping(semantic.semantic.into_scoping(), &mut program);
    if !transformed.diagnostics.is_empty() {
        return Err(format!(
            "JavaScript async compilation failed: {:?}",
            transformed.diagnostics
        ));
    }
    AsyncHelpers {
        allocator: &allocator,
        kinds: sources.kinds,
    }
    .visit_program(&mut program);
    Ok(Codegen::new().build(&program).code)
}

/// Reads the compiler's explicit method kind without relying on lowered function prototypes.
pub(crate) fn method_kind(source: &str) -> Result<&'static str, String> {
    let allocator = Allocator::default();
    let source = format!("class OperitMethod {{ {source} }}");
    let parsed = Parser::new(&allocator, &source, SourceType::cjs()).parse();
    if !parsed.diagnostics.is_empty() {
        return Err(format!(
            "Invalid route method source: {:?}",
            parsed.diagnostics
        ));
    }
    let Some(Statement::ClassDeclaration(class)) = parsed.program.body.first() else {
        return Err("Route source must contain a method".into());
    };
    let Some(ClassElement::MethodDefinition(method)) = class.body.body.first() else {
        return Err("Route source must contain a method".into());
    };
    for directive in method.value.body.iter().flat_map(|body| &body.directives) {
        match directive.expression.value.as_str() {
            "operit.async.function" => return Ok("function"),
            "operit.async.generator" => return Ok("generator"),
            _ => {}
        }
    }
    Err("Routes require an async method or async generator".into())
}
