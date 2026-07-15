use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::Path;

use syn::{
    Attribute, Fields, FnArg, GenericArgument, GenericParam, ImplItem, Item, ItemEnum, ItemImpl,
    ItemStruct, ItemTrait, ItemType, LitStr, PathArguments, ReturnType, Signature, TraitItem, Type,
    TypeParamBound,
};

/// Describes one generated TypeScript declaration file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclarationFile {
    /// Contains the output file name.
    pub file_name: String,
    /// Contains the generated TypeScript source.
    pub source: String,
}

struct ModuleSpec {
    rust_file: &'static str,
    additional_rust_file: Option<&'static str>,
    ts_file: &'static str,
}

const MODULES: &[ModuleSpec] = &[
    ModuleSpec {
        rust_file: "js_sdk/results.rs",
        additional_rust_file: None,
        ts_file: "results.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/core.rs",
        additional_rust_file: None,
        ts_file: "core.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/tool_types.rs",
        additional_rust_file: None,
        ts_file: "tool-types.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/files.rs",
        additional_rust_file: None,
        ts_file: "files.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/network.rs",
        additional_rust_file: None,
        ts_file: "network.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/system.rs",
        additional_rust_file: None,
        ts_file: "system.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/software_settings.rs",
        additional_rust_file: None,
        ts_file: "software_settings.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/ui.rs",
        additional_rust_file: None,
        ts_file: "ui.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/chat.rs",
        additional_rust_file: None,
        ts_file: "chat.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/memory.rs",
        additional_rust_file: None,
        ts_file: "memory.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/compose_dsl.rs",
        additional_rust_file: None,
        ts_file: "compose-dsl.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/compose_dsl_material3_generated.rs",
        additional_rust_file: None,
        ts_file: "compose-dsl.material3.generated.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/material_icons.rs",
        additional_rust_file: None,
        ts_file: "material-icons.d.ts",
    },
    ModuleSpec {
        rust_file: "js_sdk/toolpkg.rs",
        additional_rust_file: None,
        ts_file: "toolpkg.d.ts",
    },
];

/// Generates every plugin SDK declaration file from the Rust source tree.
pub fn generate_declaration_tree(
    source_root: &Path,
    output_root: &Path,
) -> Result<Vec<DeclarationFile>, Box<dyn Error>> {
    fs::create_dir_all(output_root)?;
    let generated = render_declaration_tree(source_root)?;
    for declaration in &generated {
        fs::write(
            output_root.join(&declaration.file_name),
            &declaration.source,
        )?;
    }
    Ok(generated)
}

/// Checks that committed TypeScript declarations exactly match the Rust SDK source.
pub fn check_declaration_tree(
    source_root: &Path,
    declaration_root: &Path,
) -> Result<Vec<DeclarationFile>, Box<dyn Error>> {
    let generated = render_declaration_tree(source_root)?;
    let mut mismatched_files = Vec::new();
    for declaration in &generated {
        let committed = fs::read_to_string(declaration_root.join(&declaration.file_name))?;
        if committed != declaration.source {
            mismatched_files.push(declaration.file_name.as_str());
        }
    }
    if !mismatched_files.is_empty() {
        return Err(format!(
            "TypeScript declarations differ from Rust SDK source: {}",
            mismatched_files.join(", ")
        )
        .into());
    }
    Ok(generated)
}

/// Renders every declaration file without writing to the filesystem.
fn render_declaration_tree(source_root: &Path) -> Result<Vec<DeclarationFile>, Box<dyn Error>> {
    let mut modules = Vec::with_capacity(MODULES.len());
    for spec in MODULES {
        let mut sources = vec![fs::read_to_string(source_root.join(spec.rust_file))?];
        if let Some(additional_rust_file) = spec.additional_rust_file {
            sources.push(fs::read_to_string(source_root.join(additional_rust_file))?);
        }
        let files = sources
            .iter()
            .map(|source| syn::parse_file(source))
            .collect::<Result<Vec<_>, _>>()?;
        modules.push((spec, files));
    }
    let catalog = build_type_catalog(&modules);
    Ok(modules
        .iter()
        .map(|(spec, files)| generate_declaration_file(files, spec, &catalog))
        .collect())
}

/// Generates one TypeScript declaration file from its canonical Rust modules.
fn generate_declaration_file(
    files: &[syn::File],
    spec: &ModuleSpec,
    catalog: &TypeCatalog,
) -> DeclarationFile {
    let context = EmitContext::new(files, spec, catalog.clone());
    DeclarationFile {
        file_name: spec.ts_file.to_string(),
        source: context.emit(),
    }
}

#[derive(Clone)]
struct CatalogEntry {
    ts_file: &'static str,
    path: Vec<String>,
}

type TypeCatalog = BTreeMap<String, CatalogEntry>;

/// Indexes every public Rust SDK type by its generated TypeScript module and path.
fn build_type_catalog(modules: &[(&'static ModuleSpec, Vec<syn::File>)]) -> TypeCatalog {
    let mut catalog = TypeCatalog::new();
    for (spec, files) in modules {
        for file in files {
            for item in &file.items {
                let Some(item_ref) = public_item_ref(item) else {
                    continue;
                };
                let rust_name = item_ref.rust_name();
                let Some((path, _)) = infer_declaration(item_ref, spec.ts_file) else {
                    continue;
                };
                let entry = CatalogEntry {
                    ts_file: spec.ts_file,
                    path,
                };
                if let Some(existing) = catalog.insert(rust_name.clone(), entry) {
                    assert_eq!(
                        existing.ts_file, spec.ts_file,
                        "Rust SDK type `{rust_name}` is declared by multiple TypeScript modules"
                    );
                }
            }
        }
    }
    catalog
}

#[derive(Clone, Copy)]
enum ItemRef<'a> {
    Struct(&'a ItemStruct),
    Enum(&'a ItemEnum),
    Type(&'a ItemType),
    Trait(&'a ItemTrait),
}

impl<'a> ItemRef<'a> {
    /// Returns the attributes attached to this declaration item.
    fn attrs(self) -> &'a [Attribute] {
        match self {
            Self::Struct(item) => &item.attrs,
            Self::Enum(item) => &item.attrs,
            Self::Type(item) => &item.attrs,
            Self::Trait(item) => &item.attrs,
        }
    }

    /// Returns the Rust identifier of this declaration item.
    fn rust_name(self) -> String {
        match self {
            Self::Struct(item) => item.ident.to_string(),
            Self::Enum(item) => item.ident.to_string(),
            Self::Type(item) => item.ident.to_string(),
            Self::Trait(item) => item.ident.to_string(),
        }
    }
}

struct DeclarationGroup<'a> {
    path: Vec<String>,
    kind: String,
    order: usize,
    items: Vec<ItemRef<'a>>,
}

struct EmitContext<'a> {
    inline_items: BTreeMap<String, ItemRef<'a>>,
    named_paths: BTreeMap<String, Vec<String>>,
    impls: BTreeMap<String, Vec<&'a ItemImpl>>,
    groups: Vec<DeclarationGroup<'a>>,
    root_traits: Vec<&'a ItemTrait>,
    type_catalog: TypeCatalog,
    current_file: &'static str,
    imports: RefCell<BTreeMap<&'static str, BTreeSet<String>>>,
}

impl<'a> EmitContext<'a> {
    /// Indexes Rust declarations by TypeScript path and inline helper identity.
    fn new(files: &'a [syn::File], spec: &ModuleSpec, type_catalog: TypeCatalog) -> Self {
        let mut context = Self {
            inline_items: BTreeMap::new(),
            named_paths: BTreeMap::new(),
            impls: BTreeMap::new(),
            groups: Vec::new(),
            root_traits: Vec::new(),
            type_catalog,
            current_file: spec.ts_file,
            imports: RefCell::new(BTreeMap::new()),
        };
        let mut grouped = BTreeMap::<String, DeclarationGroup<'a>>::new();
        let mut source_order = 0usize;
        for file in files {
            for item in &file.items {
                if let Item::Impl(item_impl) = item {
                    if let Some(type_name) = impl_self_type_name(item_impl) {
                        context.impls.entry(type_name).or_default().push(item_impl);
                    }
                    continue;
                }
                let Some(item_ref) = public_item_ref(item) else {
                    continue;
                };
                let rust_name = item_ref.rust_name();
                if rust_name == "CoreHost" {
                    if let ItemRef::Trait(item_trait) = item_ref {
                        context.root_traits.push(item_trait);
                    }
                    continue;
                }
                let Some((path, kind)) = infer_declaration(item_ref, spec.ts_file) else {
                    continue;
                };
                context.named_paths.insert(rust_name, path.clone());
                let order = source_order;
                source_order += 1;
                let key = path.join("\u{1f}");
                let group = grouped.entry(key).or_insert_with(|| DeclarationGroup {
                    path,
                    kind,
                    order,
                    items: Vec::new(),
                });
                group.order = group.order.min(order);
                group.items.push(item_ref);
            }
        }
        context.groups = grouped.into_values().collect();
        context
    }

    /// Emits the complete TypeScript module represented by this context.
    fn emit(&self) -> String {
        let mut body = String::new();
        self.emit_scope(&mut body, &[], "");

        let mut output = String::from("// Generated from operit-plugin-sdk Rust declarations.\n\n");
        for (file, names) in self.imports.borrow().iter() {
            output.push_str("import type { ");
            output.push_str(&names.iter().cloned().collect::<Vec<_>>().join(", "));
            output.push_str(" } from \"");
            output.push_str(&typescript_module_path(file));
            output.push_str("\";\n");
        }
        if !self.imports.borrow().is_empty() {
            output.push('\n');
        }
        output.push_str(body.trim_end());
        output.push('\n');
        output
    }

    /// Emits declarations directly contained by one TypeScript namespace path.
    fn emit_scope(&self, output: &mut String, scope: &[String], indent: &str) {
        let mut entries = self.scope_entries(scope);
        entries.sort_by(|left, right| {
            left.order()
                .cmp(&right.order())
                .then_with(|| left.name().cmp(&right.name()))
        });
        for entry in entries {
            match entry {
                ScopeEntry::Namespace { name, group, .. } => {
                    if let Some(group) = group {
                        emit_jsdoc(output, group_docs(group), indent);
                    }
                    if name == "global" {
                        output.push_str(indent);
                        output.push_str("declare global {\n");
                    } else {
                        output.push_str(indent);
                        output.push_str("export namespace ");
                        output.push_str(&name);
                        output.push_str(" {\n");
                    }
                    let mut child_scope = scope.to_vec();
                    child_scope.push(name);
                    self.emit_scope(output, &child_scope, &format!("{indent}  "));
                    output.push_str(indent);
                    output.push_str("}\n\n");
                }
                ScopeEntry::Group(group) => self.emit_group(output, group, scope, indent),
                ScopeEntry::Method {
                    method,
                    root_function,
                    ..
                } => {
                    self.emit_method(
                        output,
                        &method.attrs,
                        &method.sig,
                        scope,
                        indent,
                        true,
                        root_function,
                    );
                }
            }
        }
    }

    /// Collects namespace, declaration, and function entries for one scope.
    fn scope_entries<'context>(&'context self, scope: &[String]) -> Vec<ScopeEntry<'context, 'a>> {
        let mut namespace_orders = BTreeMap::<String, usize>::new();
        let mut namespace_groups = BTreeMap::<String, &DeclarationGroup<'a>>::new();
        let mut entries = Vec::new();
        for group in &self.groups {
            if !path_starts_with(&group.path, scope) || group.path.len() <= scope.len() {
                continue;
            }
            let child = group.path[scope.len()].clone();
            if group.path.len() > scope.len() + 1 || group.kind == "namespace" {
                namespace_orders
                    .entry(child.clone())
                    .and_modify(|order| *order = (*order).min(group.order))
                    .or_insert(group.order);
                if group.path.len() == scope.len() + 1 && group.kind == "namespace" {
                    namespace_groups.insert(child, group);
                }
            } else {
                entries.push(ScopeEntry::Group(group));
            }
        }
        for (name, order) in namespace_orders {
            entries.push(ScopeEntry::Namespace {
                group: namespace_groups.get(&name).copied(),
                name,
                order,
            });
        }
        if let Some(namespace_group) = self
            .groups
            .iter()
            .find(|group| group.kind == "namespace" && group.path == scope)
        {
            for item in &namespace_group.items {
                if let ItemRef::Trait(item_trait) = item {
                    for trait_item in &item_trait.items {
                        if let TraitItem::Fn(method) = trait_item {
                            entries.push(ScopeEntry::Method {
                                method,
                                order: namespace_group.order,
                                root_function: false,
                            });
                        }
                    }
                }
            }
        }
        if scope.is_empty() {
            for item_trait in &self.root_traits {
                for trait_item in &item_trait.items {
                    if let TraitItem::Fn(method) = trait_item {
                        entries.push(ScopeEntry::Method {
                            method,
                            order: usize::MAX,
                            root_function: true,
                        });
                    }
                }
            }
        }
        entries
    }

    /// Emits one grouped TypeScript interface, class, or type alias.
    fn emit_group(
        &self,
        output: &mut String,
        group: &DeclarationGroup<'a>,
        scope: &[String],
        indent: &str,
    ) {
        match group.kind.as_str() {
            "interface" | "class" => self.emit_interface_group(output, group, scope, indent),
            "type" => self.emit_type_group(output, group, scope, indent),
            "type_map" => self.emit_type_map_group(output, group, scope, indent),
            "variable" => self.emit_variable_group(output, group, scope, indent),
            kind => panic!(
                "unsupported TypeScript declaration kind `{kind}` for {:?}",
                group.path
            ),
        }
    }

    /// Emits a string-keyed TypeScript type map represented by a Rust enum.
    fn emit_type_map_group(
        &self,
        output: &mut String,
        group: &DeclarationGroup<'a>,
        scope: &[String],
        indent: &str,
    ) {
        let ItemRef::Enum(item) = group.items[0] else {
            panic!("Rust TypeMap declarations must be enums");
        };
        emit_jsdoc(output, &item.attrs, indent);
        output.push_str(indent);
        output.push_str("export interface ");
        output.push_str(group.path.last().expect("type map paths are non-empty"));
        output.push_str(" {\n");
        let child_indent = format!("{indent}  ");
        for variant in &item.variants {
            let Fields::Unnamed(fields) = &variant.fields else {
                panic!("Rust TypeMap variants must carry one unnamed value type");
            };
            let field = fields
                .unnamed
                .first()
                .filter(|_| fields.unnamed.len() == 1)
                .expect("Rust TypeMap variants must carry exactly one value type");
            emit_jsdoc(output, &variant.attrs, &child_indent);
            output.push_str(&child_indent);
            output.push_str(&format!(
                "{:?}: {};\n",
                renamed_identifier(&variant.attrs, &variant.ident.to_string()),
                self.emit_type(&field.ty, scope)
            ));
        }
        output.push_str(indent);
        output.push_str("}\n\n");
    }

    /// Emits one interface or class assembled from data, behavior, and inherent Rust items.
    fn emit_interface_group(
        &self,
        output: &mut String,
        group: &DeclarationGroup<'a>,
        scope: &[String],
        indent: &str,
    ) {
        emit_jsdoc(output, group_docs(group), indent);
        let name = group.path.last().expect("declaration paths are non-empty");
        output.push_str(indent);
        output.push_str("export ");
        output.push_str(if group.kind == "class" {
            "class "
        } else {
            "interface "
        });
        output.push_str(name);
        output.push_str(&self.emit_item_generics(group, scope));
        let bases = group
            .items
            .iter()
            .filter_map(|item| match item {
                ItemRef::Struct(item_struct) => Some(flattened_bases(item_struct, self, scope)),
                _ => None,
            })
            .flatten()
            .collect::<Vec<_>>();
        if !bases.is_empty() {
            output.push_str(" extends ");
            output.push_str(&bases.join(", "));
        }
        output.push_str(" {\n");

        let child_indent = format!("{indent}  ");
        for item in &group.items {
            if let ItemRef::Struct(item_struct) = item {
                self.emit_struct_fields(
                    output,
                    item_struct,
                    scope,
                    &child_indent,
                    group.kind == "class",
                );
            }
        }
        for method in self.group_methods(group) {
            self.emit_method(
                output,
                method.attrs(),
                method.signature(),
                scope,
                &child_indent,
                false,
                false,
            );
        }
        output.push_str(indent);
        output.push_str("}\n\n");
    }

    /// Emits all visible fields from one Rust struct into an interface body.
    fn emit_struct_fields(
        &self,
        output: &mut String,
        item: &ItemStruct,
        scope: &[String],
        indent: &str,
        readonly: bool,
    ) {
        let Fields::Named(fields) = &item.fields else {
            return;
        };
        for field in &fields.named {
            if is_additional_properties_field(field) {
                self.emit_index_signature(output, &field.ty, scope, indent);
                continue;
            }
            if is_inherited_base_field(field) {
                continue;
            }
            emit_jsdoc(output, &field.attrs, indent);
            output.push_str(indent);
            if readonly {
                output.push_str("readonly ");
            }
            output.push_str(&field_name(field));
            if is_optional_field(&field.ty) {
                output.push('?');
            }
            output.push_str(": ");
            output.push_str(&self.emit_field_type(&field.ty, scope));
            output.push_str(";\n");
        }
    }

    /// Emits an index signature stored in a flattened Rust map field.
    fn emit_index_signature(
        &self,
        output: &mut String,
        field_type: &Type,
        scope: &[String],
        indent: &str,
    ) {
        let Type::Path(type_path) = field_type else {
            panic!("flattened additional properties must use a Rust map type");
        };
        let segment = type_path
            .path
            .segments
            .last()
            .expect("Rust map paths have a segment");
        let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
            panic!("flattened additional properties must declare map arguments");
        };
        let Some(GenericArgument::Type(value_type)) = arguments.args.iter().nth(1) else {
            panic!("flattened additional properties must declare a value type");
        };
        output.push_str(indent);
        output.push_str("[key: string]: ");
        output.push_str(&self.emit_type(value_type, scope));
        output.push_str(";\n");
    }

    /// Collects behavior trait methods and public inherent methods for one declaration group.
    fn group_methods(&self, group: &DeclarationGroup<'a>) -> Vec<MethodRef<'a>> {
        let mut methods = Vec::new();
        let mut struct_names = BTreeSet::new();
        for item in &group.items {
            match item {
                ItemRef::Trait(item_trait) => {
                    for trait_item in &item_trait.items {
                        if let TraitItem::Fn(method) = trait_item {
                            methods.push(MethodRef::Trait(method));
                        }
                    }
                }
                ItemRef::Struct(item_struct) => {
                    struct_names.insert(item_struct.ident.to_string());
                }
                _ => {}
            }
        }
        for struct_name in struct_names {
            if let Some(impls) = self.impls.get(&struct_name) {
                for item_impl in impls {
                    if item_impl.trait_.is_some() {
                        continue;
                    }
                    for item in &item_impl.items {
                        if let ImplItem::Fn(method) = item {
                            if is_public(&method.vis) {
                                methods.push(MethodRef::Impl(method));
                            }
                        }
                    }
                }
            }
        }
        methods
    }

    /// Emits one type alias from a Rust enum, alias, or exact-syntax wrapper struct.
    fn emit_type_group(
        &self,
        output: &mut String,
        group: &DeclarationGroup<'a>,
        scope: &[String],
        indent: &str,
    ) {
        let item = group.items[0];
        emit_jsdoc(output, item.attrs(), indent);
        output.push_str(indent);
        output.push_str("export type ");
        output.push_str(group.path.last().expect("type paths are non-empty"));
        output.push_str(&self.emit_item_generics(group, scope));
        output.push_str(" = ");
        if item.rust_name() == "ToolReturnType" {
            output.push_str(
                "T extends keyof import(\"./tool-types\").ToolResultMap \
                 ? import(\"./tool-types\").ToolResultMap[T] : any",
            );
            output.push_str(";\n\n");
            return;
        }
        match item {
            ItemRef::Enum(item_enum) => output.push_str(&self.emit_enum_union(item_enum, scope)),
            ItemRef::Type(item_type) => output.push_str(&self.emit_type(&item_type.ty, scope)),
            ItemRef::Struct(item_struct) => match &item_struct.fields {
                Fields::Unnamed(fields) => {
                    let field = fields
                        .unnamed
                        .first()
                        .expect("Rust type wrapper structs require one tuple field");
                    output.push_str(&self.emit_type(&field.ty, scope));
                }
                Fields::Named(_) if is_flattened_intersection(item_struct) => {
                    output.push_str(&flattened_bases(item_struct, self, scope).join(" & "));
                }
                _ => panic!("Rust type wrapper structs require tuple or intersection fields"),
            },
            ItemRef::Trait(_) => {
                panic!("traits cannot directly represent TypeScript type aliases")
            }
        }
        output.push_str(";\n\n");
    }

    /// Emits a TypeScript variable declaration represented by an exact Rust wrapper.
    fn emit_variable_group(
        &self,
        output: &mut String,
        group: &DeclarationGroup<'a>,
        scope: &[String],
        indent: &str,
    ) {
        let item = group.items[0];
        emit_jsdoc(output, item.attrs(), indent);
        output.push_str(indent);
        if scope.is_empty() {
            output.push_str("export declare ");
        }
        let variable_keyword = match item {
            ItemRef::Struct(item_struct) if item_struct.ident == "CommonJsExports" => "var ",
            _ => "const ",
        };
        output.push_str(variable_keyword);
        output.push_str(group.path.last().expect("variable paths are non-empty"));
        output.push_str(": ");
        output.push_str(&self.emit_variable_type(item, scope));
        output.push_str(";\n\n");
    }

    /// Resolves the public Rust type carried by a JavaScript global binding marker.
    fn emit_variable_type(&self, item: ItemRef<'a>, scope: &[String]) -> String {
        let ItemRef::Struct(item_struct) = item else {
            panic!("variable declarations require a Rust marker struct");
        };
        match &item_struct.fields {
            Fields::Unnamed(fields) => {
                let field = fields
                    .unnamed
                    .first()
                    .expect("variable marker structs require one tuple field");
                if let Type::Path(type_path) = &field.ty {
                    let field_name = type_path
                        .path
                        .segments
                        .last()
                        .expect("binding field type paths have a segment")
                        .ident
                        .to_string();
                    if let Some(parameter) = item_struct
                        .generics
                        .type_params()
                        .find(|parameter| parameter.ident == field_name)
                    {
                        let bound = parameter
                            .bounds
                            .iter()
                            .find_map(|bound| match bound {
                                TypeParamBound::Trait(bound) => Some(bound),
                                _ => None,
                            })
                            .expect("generic variable binding fields require a trait bound");
                        let bound_name = bound
                            .path
                            .segments
                            .last()
                            .expect("binding trait paths have a segment")
                            .ident
                            .to_string();
                        return self.resolve_named_type(&bound_name, scope);
                    }
                }
                self.emit_type(&field.ty, scope)
            }
            Fields::Unit => match item_struct.ident.to_string().as_str() {
                "ToolPkgGlobalBinding" => "ToolPkg.Registry".to_string(),
                name => panic!("unit variable marker `{name}` does not carry a binding type"),
            },
            Fields::Named(_) => {
                panic!("variable marker structs must use a tuple field")
            }
        }
    }

    /// Emits one Rust method signature as a TypeScript method or function declaration.
    #[allow(clippy::too_many_arguments)]
    fn emit_method(
        &self,
        output: &mut String,
        attrs: &[Attribute],
        signature: &Signature,
        scope: &[String],
        indent: &str,
        function: bool,
        root_function: bool,
    ) {
        emit_jsdoc(output, attrs, indent);
        output.push_str(indent);
        let constructor = !function
            && signature.ident == "new"
            && !signature
                .inputs
                .iter()
                .any(|argument| matches!(argument, FnArg::Receiver(_)));
        let static_method = !function
            && !constructor
            && !signature
                .inputs
                .iter()
                .any(|argument| matches!(argument, FnArg::Receiver(_)));
        if root_function {
            output.push_str("export declare function ");
        } else if function {
            output.push_str("function ");
        } else if static_method {
            output.push_str("static ");
        }
        let name = if constructor {
            "constructor".to_string()
        } else {
            method_name(&signature.ident.to_string())
        };
        output.push_str(&name);
        output.push_str(&self.emit_rust_generics(&signature.generics.params, scope));
        output.push('(');
        let mut parameters = Vec::new();
        for argument in &signature.inputs {
            let FnArg::Typed(argument) = argument else {
                continue;
            };
            let name = pattern_name(&argument.pat);
            let (optional, parameter_type) = self.emit_parameter_type(&argument.ty, scope);
            parameters.push(format!(
                "{}{}: {}",
                name,
                if optional { "?" } else { "" },
                parameter_type
            ));
        }
        output.push_str(&parameters.join(", "));
        output.push(')');
        if !constructor {
            output.push_str(": ");
            match &signature.output {
                ReturnType::Default => output.push_str("void"),
                ReturnType::Type(_, return_type) => {
                    output.push_str(&self.emit_type(return_type, scope))
                }
            }
        }
        output.push_str(";\n");
    }

    /// Emits a field type while moving optional and undefined state into property syntax.
    fn emit_field_type(&self, field_type: &Type, scope: &[String]) -> String {
        if let Some((name, inner)) = single_type_argument(field_type) {
            match name.as_str() {
                "Option" => return self.emit_type(inner, scope),
                "JsOptional" => return format!("{} | null", self.emit_type(inner, scope)),
                _ => {}
            }
        }
        self.emit_type(field_type, scope)
    }

    /// Emits a positional parameter type and reports whether the parameter is optional.
    fn emit_parameter_type(&self, parameter_type: &Type, scope: &[String]) -> (bool, String) {
        if let Some((name, inner)) = single_type_argument(parameter_type) {
            match name.as_str() {
                "Option" => return (true, self.emit_type(inner, scope)),
                "JsOptional" => return (true, format!("{} | null", self.emit_type(inner, scope))),
                _ => {}
            }
        }
        (false, self.emit_type(parameter_type, scope))
    }

    /// Emits the generic parameters declared by one Rust declaration group.
    fn emit_item_generics(&self, group: &DeclarationGroup<'_>, scope: &[String]) -> String {
        match group.items[0] {
            ItemRef::Struct(item) => self.emit_rust_generics(&item.generics.params, scope),
            ItemRef::Enum(item) => self.emit_rust_generics(&item.generics.params, scope),
            ItemRef::Type(item) => self.emit_rust_generics(&item.generics.params, scope),
            ItemRef::Trait(item) => self.emit_rust_generics(&item.generics.params, scope),
        }
    }

    /// Emits Rust generic type parameters with TypeScript defaults resolved in module context.
    fn emit_rust_generics(
        &self,
        parameters: &syn::punctuated::Punctuated<GenericParam, syn::Token![,]>,
        scope: &[String],
    ) -> String {
        let parameters = parameters
            .iter()
            .filter_map(|parameter| match parameter {
                GenericParam::Type(parameter) => {
                    let mut value = parameter.ident.to_string();
                    let constraints = parameter
                        .bounds
                        .iter()
                        .filter_map(typescript_generic_bound)
                        .collect::<Vec<_>>();
                    if !constraints.is_empty() {
                        value.push_str(" extends ");
                        value.push_str(&constraints.join(" & "));
                    }
                    if let Some(default) = &parameter.default {
                        value.push_str(" = ");
                        value.push_str(&self.emit_type(default, scope));
                    }
                    Some(value)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if parameters.is_empty() {
            String::new()
        } else {
            format!("<{}>", parameters.join(", "))
        }
    }

    /// Emits one Rust type as its TypeScript boundary equivalent.
    fn emit_type(&self, rust_type: &Type, scope: &[String]) -> String {
        match rust_type {
            Type::Path(type_path) => self.emit_path_type(type_path, scope),
            Type::Tuple(tuple) if tuple.elems.is_empty() => "void".to_string(),
            Type::Tuple(tuple) => format!(
                "[{}]",
                tuple
                    .elems
                    .iter()
                    .map(|item| self.emit_type(item, scope))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Type::TraitObject(trait_object) => self.emit_trait_object(trait_object, scope),
            Type::Reference(reference) => self.emit_type(&reference.elem, scope),
            Type::Never(_) => "never".to_string(),
            Type::Paren(parenthesized) => self.emit_type(&parenthesized.elem, scope),
            Type::Group(group) => self.emit_type(&group.elem, scope),
            Type::Slice(slice) => format!("{}[]", self.emit_type(&slice.elem, scope)),
            _ => panic!("unsupported Rust SDK type node"),
        }
    }

    /// Emits a Rust path type and its generic arguments.
    fn emit_path_type(&self, type_path: &syn::TypePath, scope: &[String]) -> String {
        let segment = type_path
            .path
            .segments
            .last()
            .expect("Rust type paths have a segment");
        let rust_name = segment.ident.to_string();
        let arguments = angle_arguments(&segment.arguments);
        match rust_name.as_str() {
            "String" | "str" | "JsDate" => "string".to_string(),
            "bool" => "boolean".to_string(),
            "f32" | "f64" | "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16"
            | "u32" | "u64" | "u128" | "usize" => "number".to_string(),
            "Value" => "unknown".to_string(),
            "JsAny" => "any".to_string(),
            "JsObject" => "object".to_string(),
            "JsNever" => "never".to_string(),
            "Vec" => format!("{}[]", self.emit_first_argument(arguments, scope)),
            "Option" | "JsUndefined" => {
                format!("{} | undefined", self.emit_first_argument(arguments, scope))
            }
            "JsOptional" => format!(
                "{} | null | undefined",
                self.emit_first_argument(arguments, scope)
            ),
            "JsNullable" => format!("{} | null", self.emit_first_argument(arguments, scope)),
            "JsFuture" => format!("Promise<{}>", self.emit_first_argument(arguments, scope)),
            "Arc" | "Box" | "Pin" => self.emit_first_argument(arguments, scope),
            "BTreeMap" | "HashMap" => format!(
                "Record<{}, {}>",
                self.emit_argument(arguments, 0, scope),
                self.emit_argument(arguments, 1, scope)
            ),
            "JsTypeIndex" => {
                let map = self.emit_argument(arguments, 0, scope);
                let key = self.emit_argument(arguments, 1, scope);
                format!("{map}[{key} & keyof {map}]")
            }
            "PhantomData" => self.emit_first_argument(arguments, scope),
            _ => {
                if let Some(item) = self.inline_items.get(&rust_name) {
                    return self.emit_inline_item(*item, scope);
                }
                let name = self.resolve_named_type(&rust_name, scope);
                let type_arguments = arguments
                    .into_iter()
                    .flat_map(|arguments| arguments.args.iter())
                    .filter_map(|argument| match argument {
                        GenericArgument::Type(argument) => Some(self.emit_type(argument, scope)),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if type_arguments.is_empty() {
                    name
                } else {
                    format!("{}<{}>", name, type_arguments.join(", "))
                }
            }
        }
    }

    /// Emits the first generic type argument from a Rust path.
    fn emit_first_argument(
        &self,
        arguments: Option<&syn::AngleBracketedGenericArguments>,
        scope: &[String],
    ) -> String {
        self.emit_argument(arguments, 0, scope)
    }

    /// Emits a generic type argument at a fixed position.
    fn emit_argument(
        &self,
        arguments: Option<&syn::AngleBracketedGenericArguments>,
        index: usize,
        scope: &[String],
    ) -> String {
        match arguments.and_then(|arguments| arguments.args.iter().nth(index)) {
            Some(GenericArgument::Type(argument)) => self.emit_type(argument, scope),
            _ => panic!("missing Rust generic type argument at index {index}"),
        }
    }

    /// Expands one migration helper directly at its TypeScript use site.
    fn emit_inline_item(&self, item: ItemRef<'a>, scope: &[String]) -> String {
        match item {
            ItemRef::Struct(item_struct) => self.emit_inline_struct(item_struct, scope),
            ItemRef::Enum(item_enum) => self.emit_enum_union(item_enum, scope),
            ItemRef::Type(item_type) => self.emit_type(&item_type.ty, scope),
            ItemRef::Trait(item_trait) => self.emit_inline_trait(item_trait, scope),
        }
    }

    /// Emits a Rust behavior trait as an inline TypeScript object API.
    fn emit_inline_trait(&self, item: &ItemTrait, scope: &[String]) -> String {
        let mut output = String::from("{\n");
        for trait_item in &item.items {
            if let TraitItem::Fn(method) = trait_item {
                self.emit_method(
                    &mut output,
                    &method.attrs,
                    &method.sig,
                    scope,
                    "  ",
                    false,
                    false,
                );
            }
        }
        output.push('}');
        output
    }

    /// Emits a named-field Rust struct as an inline TypeScript object type.
    fn emit_inline_struct(&self, item: &ItemStruct, scope: &[String]) -> String {
        let Fields::Named(fields) = &item.fields else {
            let Fields::Unnamed(fields) = &item.fields else {
                panic!("inline Rust unit structs do not carry a TypeScript value type")
            };
            let field = fields
                .unnamed
                .first()
                .expect("inline Rust tuple structs require one field");
            return self.emit_type(&field.ty, scope);
        };
        let mut output = String::from("{ ");
        for field in &fields.named {
            if has_serde_flatten(&field.attrs) {
                if field_name(field) == "additionalProperties" {
                    let Type::Path(type_path) = &field.ty else {
                        panic!("inline index signatures require a Rust map");
                    };
                    let arguments = angle_arguments(
                        &type_path
                            .path
                            .segments
                            .last()
                            .expect("Rust map paths have a segment")
                            .arguments,
                    );
                    output.push_str("[key: string]: ");
                    output.push_str(&self.emit_argument(arguments, 1, scope));
                    output.push_str("; ");
                }
                continue;
            }
            output.push_str(&field_name(field));
            if is_optional_field(&field.ty) {
                output.push('?');
            }
            output.push_str(": ");
            output.push_str(&self.emit_field_type(&field.ty, scope));
            output.push_str("; ");
        }
        output.push('}');
        output
    }

    /// Emits a Rust enum as a TypeScript literal or value union.
    fn emit_enum_union(&self, item: &ItemEnum, scope: &[String]) -> String {
        item.variants
            .iter()
            .map(|variant| {
                if matches!(variant.fields, Fields::Unit) {
                    match variant.ident.to_string().as_str() {
                        "Null" => return "null".to_string(),
                        "Undefined" => return "undefined".to_string(),
                        "Void" => return "void".to_string(),
                        _ => {}
                    }
                }
                match &variant.fields {
                    Fields::Unit => format!(
                        "\"{}\"",
                        renamed_identifier(&variant.attrs, &variant.ident.to_string())
                    ),
                    Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                        self.emit_union_member(&fields.unnamed[0].ty, scope)
                    }
                    Fields::Unnamed(fields) => format!(
                        "[{}]",
                        fields
                            .unnamed
                            .iter()
                            .map(|field| self.emit_type(&field.ty, scope))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    Fields::Named(fields) => {
                        let mut value = String::from("{ ");
                        for field in &fields.named {
                            value.push_str(&field_name(field));
                            if is_optional_field(&field.ty) {
                                value.push('?');
                            }
                            value.push_str(": ");
                            value.push_str(&self.emit_field_type(&field.ty, scope));
                            value.push_str("; ");
                        }
                        value.push('}');
                        value
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }

    /// Emits a TypeScript union member with the grouping required for function types.
    fn emit_union_member(&self, rust_type: &Type, scope: &[String]) -> String {
        let value = self.emit_type(rust_type, scope);
        if is_function_type(rust_type) {
            format!("({value})")
        } else {
            value
        }
    }

    /// Emits a Rust dynamic function trait as a TypeScript function type.
    fn emit_trait_object(&self, trait_object: &syn::TypeTraitObject, scope: &[String]) -> String {
        for bound in &trait_object.bounds {
            let TypeParamBound::Trait(bound) = bound else {
                continue;
            };
            let segment = bound
                .path
                .segments
                .last()
                .expect("Rust trait paths have a segment");
            if segment.ident != "Fn" {
                continue;
            }
            let PathArguments::Parenthesized(arguments) = &segment.arguments else {
                panic!("Rust Fn boundary types require parenthesized arguments");
            };
            let parameters = arguments
                .inputs
                .iter()
                .enumerate()
                .map(|(index, argument)| {
                    let (optional, parameter_type) = self.emit_parameter_type(argument, scope);
                    format!(
                        "arg{index}{}: {parameter_type}",
                        if optional { "?" } else { "" }
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let output = match &arguments.output {
                ReturnType::Default => "void".to_string(),
                ReturnType::Type(_, output) => self.emit_type(output, scope),
            };
            return format!("({parameters}) => {output}");
        }
        let named_bound = trait_object.bounds.iter().find_map(|bound| {
            let TypeParamBound::Trait(bound) = bound else {
                return None;
            };
            bound
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
        });
        let rust_name =
            named_bound.unwrap_or_else(|| panic!("unsupported Rust trait object boundary type"));
        self.resolve_named_type(&rust_name, scope)
    }

    /// Resolves a Rust type name and records a type-only import for external SDK modules.
    fn resolve_named_type(&self, rust_name: &str, scope: &[String]) -> String {
        if let Some(path) = self.named_paths.get(rust_name) {
            return relative_ts_path(path, scope);
        }
        let Some(entry) = self.type_catalog.get(rust_name) else {
            return rust_name.to_string();
        };
        if entry.ts_file == self.current_file {
            return relative_ts_path(&entry.path, scope);
        }
        let import_name = entry
            .path
            .first()
            .expect("cataloged TypeScript paths are non-empty")
            .clone();
        self.imports
            .borrow_mut()
            .entry(entry.ts_file)
            .or_default()
            .insert(import_name);
        entry.path.join(".")
    }
}

enum ScopeEntry<'context, 'items> {
    Namespace {
        name: String,
        order: usize,
        group: Option<&'context DeclarationGroup<'items>>,
    },
    Group(&'context DeclarationGroup<'items>),
    Method {
        method: &'items syn::TraitItemFn,
        order: usize,
        root_function: bool,
    },
}

impl ScopeEntry<'_, '_> {
    /// Returns the source declaration order for this scope entry.
    fn order(&self) -> usize {
        match self {
            Self::Namespace { order, .. } | Self::Method { order, .. } => *order,
            Self::Group(group) => group.order,
        }
    }

    /// Returns a stable name used to order otherwise equal scope entries.
    fn name(&self) -> String {
        match self {
            Self::Namespace { name, .. } => name.clone(),
            Self::Group(group) => group.path.last().cloned().unwrap_or_default(),
            Self::Method { method, .. } => method.sig.ident.to_string(),
        }
    }
}

enum MethodRef<'a> {
    Trait(&'a syn::TraitItemFn),
    Impl(&'a syn::ImplItemFn),
}

impl MethodRef<'_> {
    /// Returns the method attributes.
    fn attrs(&self) -> &[Attribute] {
        match self {
            Self::Trait(method) => &method.attrs,
            Self::Impl(method) => &method.attrs,
        }
    }

    /// Returns the method signature.
    fn signature(&self) -> &Signature {
        match self {
            Self::Trait(method) => &method.sig,
            Self::Impl(method) => &method.sig,
        }
    }
}

/// Returns a public declaration item reference.
fn public_item_ref(item: &Item) -> Option<ItemRef<'_>> {
    match item {
        Item::Struct(item) if is_public(&item.vis) => Some(ItemRef::Struct(item)),
        Item::Enum(item) if is_public(&item.vis) => Some(ItemRef::Enum(item)),
        Item::Type(item) if is_public(&item.vis) => Some(ItemRef::Type(item)),
        Item::Trait(item) if is_public(&item.vis) => Some(ItemRef::Trait(item)),
        _ => None,
    }
}

/// Reports whether a Rust visibility is public.
fn is_public(visibility: &syn::Visibility) -> bool {
    matches!(visibility, syn::Visibility::Public(_))
}

/// Returns the self type name for one inherent or trait implementation.
fn impl_self_type_name(item_impl: &ItemImpl) -> Option<String> {
    let Type::Path(type_path) = item_impl.self_ty.as_ref() else {
        return None;
    };
    type_path
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

/// Returns the default TypeScript category for a Rust declaration item.
fn default_item_kind(item: ItemRef<'_>) -> &'static str {
    match item {
        ItemRef::Struct(item) if matches!(item.fields, Fields::Unnamed(_)) => "type",
        ItemRef::Struct(item) if is_flattened_intersection(item) => "type",
        ItemRef::Struct(_) | ItemRef::Trait(_) => "interface",
        ItemRef::Enum(_) | ItemRef::Type(_) => "type",
    }
}

/// Reports whether a Rust struct represents a pure TypeScript intersection.
fn is_flattened_intersection(item: &ItemStruct) -> bool {
    let Fields::Named(fields) = &item.fields else {
        return false;
    };
    if fields.named.len() <= 1 {
        return false;
    }
    let explicitly_flattened = fields
        .named
        .iter()
        .all(|field| has_serde_flatten(&field.attrs));
    let numbered_members = fields.named.iter().enumerate().all(|(index, field)| {
        field
            .ident
            .as_ref()
            .is_some_and(|ident| ident == &format!("member_{}", index + 1))
    });
    explicitly_flattened || numbered_members
}

/// Infers one TypeScript declaration path directly from the Rust module and identifier.
fn infer_declaration(item: ItemRef<'_>, ts_file: &str) -> Option<(Vec<String>, String)> {
    let rust_name = item.rust_name();
    if rust_name == "ToolPkgBroadcastTopicKey" {
        return None;
    }
    let variable = match rust_name.as_str() {
        "LodashGlobal" => Some(vec!["_".to_string()]),
        "DataUtilsGlobal" => Some(vec!["dataUtils".to_string()]),
        "CommonJsExports" => Some(vec!["exports".to_string()]),
        "NetCookiesBinding" => Some(vec!["Net".to_string(), "cookies".to_string()]),
        "ToolPkgGlobalBinding" => Some(vec!["global".to_string(), "ToolPkg".to_string()]),
        _ => None,
    };
    if let Some(path) = variable {
        return Some((path, "variable".to_string()));
    }
    if matches!(item, ItemRef::Enum(_)) && rust_name.ends_with("TypeMap") {
        let path = match ts_file {
            "toolpkg.d.ts" => prefixed_path(&rust_name, "ToolPkg"),
            _ => vec![rust_name],
        };
        return Some((path, "type_map".to_string()));
    }
    if rust_name == "ComposeNumberExtensions" {
        return Some((
            vec!["global".to_string(), "Number".to_string()],
            "interface".to_string(),
        ));
    }
    if let ItemRef::Trait(_) = item {
        let namespace = match rust_name.as_str() {
            "FilesHost" => Some(vec!["Files".to_string()]),
            "NetHost" => Some(vec!["Net".to_string()]),
            "NetFutureHost" => Some(vec!["Net".to_string()]),
            "SystemHost" => Some(vec!["System".to_string()]),
            "SystemBluetoothHost" => Some(vec!["System".to_string(), "bluetooth".to_string()]),
            "SystemBluetoothBleHost" => Some(vec![
                "System".to_string(),
                "bluetooth".to_string(),
                "ble".to_string(),
            ]),
            "SystemTerminalHost" => Some(vec!["System".to_string(), "terminal".to_string()]),
            "SystemMusicHost" => Some(vec!["System".to_string(), "music".to_string()]),
            "SoftwareSettingsHost" => Some(vec!["SoftwareSettings".to_string()]),
            "UIHost" => Some(vec!["UI".to_string()]),
            "ChatHost" => Some(vec!["Chat".to_string()]),
            "MemoryHost" => Some(vec!["Memory".to_string()]),
            "NativeInterfaceHost" => Some(vec!["NativeInterface".to_string()]),
            "GlobalHost" => Some(vec!["global".to_string()]),
            _ => None,
        };
        if let Some(path) = namespace {
            return Some((path, "namespace".to_string()));
        }
    }
    let declaration_name = rust_name.strip_suffix("Methods").unwrap_or(&rust_name);
    let path = match ts_file {
        "files.d.ts" => prefixed_path(declaration_name, "Files"),
        "network.d.ts" => prefixed_path(declaration_name, "Net"),
        "system.d.ts" => prefixed_path(declaration_name, "System"),
        "software_settings.d.ts" => prefixed_path(declaration_name, "SoftwareSettings"),
        "ui.d.ts" if declaration_name.starts_with("UINode") => {
            vec![declaration_name.to_string()]
        }
        "ui.d.ts" => prefixed_path(declaration_name, "UI"),
        "chat.d.ts" => prefixed_path(declaration_name, "Chat"),
        "memory.d.ts" => prefixed_path(declaration_name, "Memory"),
        "toolpkg.d.ts" => prefixed_path(declaration_name, "ToolPkg"),
        _ => vec![declaration_name.to_string()],
    };
    let kind = if rust_name == "UINode" {
        "class".to_string()
    } else {
        default_item_kind(item).to_string()
    };
    Some((path, kind))
}

/// Places a Rust item under a namespace when its identifier starts with that namespace prefix.
fn prefixed_path(rust_name: &str, prefix: &str) -> Vec<String> {
    match rust_name.strip_prefix(prefix) {
        Some(local_name) if !local_name.is_empty() => {
            vec![prefix.to_string(), local_name.to_string()]
        }
        _ => vec![rust_name.to_string()],
    }
}

/// Returns declaration-level attributes suitable for JSDoc emission.
fn group_docs<'a>(group: &DeclarationGroup<'a>) -> &'a [Attribute] {
    group_primary_attrs(group)
}

/// Returns the preferred declaration item attributes for one merged group.
fn group_primary_attrs<'a>(group: &DeclarationGroup<'a>) -> &'a [Attribute] {
    group
        .items
        .iter()
        .find_map(|item| match item {
            ItemRef::Struct(item) => Some(item.attrs.as_slice()),
            _ => None,
        })
        .unwrap_or_else(|| group.items[0].attrs())
}

/// Collects flattened base interfaces or every member of an intersection struct.
fn flattened_bases(item: &ItemStruct, context: &EmitContext<'_>, scope: &[String]) -> Vec<String> {
    let Fields::Named(fields) = &item.fields else {
        return Vec::new();
    };
    let intersection = is_flattened_intersection(item);
    fields
        .named
        .iter()
        .filter(|field| intersection || is_inherited_base_field(field))
        .map(|field| context.emit_type(&field.ty, scope))
        .collect()
}

/// Reports whether a Rust composition field represents an inherited TypeScript interface.
fn is_inherited_base_field(field: &syn::Field) -> bool {
    if has_serde_flatten(&field.attrs) && !is_additional_properties_field(field) {
        return true;
    }
    field
        .ident
        .as_ref()
        .is_some_and(|ident| ident.to_string().starts_with("base_"))
        && !is_optional_field(&field.ty)
}

/// Reports whether a Rust map field represents an open TypeScript object index signature.
fn is_additional_properties_field(field: &syn::Field) -> bool {
    field
        .ident
        .as_ref()
        .is_some_and(|ident| ident == "additional_properties")
        || field_name(field) == "additionalProperties"
}

/// Returns angle-bracketed generic arguments when a Rust path has them.
fn angle_arguments(arguments: &PathArguments) -> Option<&syn::AngleBracketedGenericArguments> {
    match arguments {
        PathArguments::AngleBracketed(arguments) => Some(arguments),
        _ => None,
    }
}

/// Returns a path type's sole generic type argument and path name.
fn single_type_argument(rust_type: &Type) -> Option<(String, &Type)> {
    let Type::Path(type_path) = rust_type else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let GenericArgument::Type(inner) = arguments.args.first()? else {
        return None;
    };
    Some((segment.ident.to_string(), inner))
}

/// Maps a Rust generic bound that carries TypeScript boundary meaning.
fn typescript_generic_bound(bound: &TypeParamBound) -> Option<String> {
    let TypeParamBound::Trait(bound) = bound else {
        return None;
    };
    let segment = bound
        .path
        .segments
        .last()
        .expect("generic trait bounds have a path segment");
    match segment.ident.to_string().as_str() {
        "AsRef" => {
            let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                panic!("AsRef SDK bounds require an explicit target type")
            };
            let Some(GenericArgument::Type(Type::Path(target))) = arguments.args.first() else {
                panic!("AsRef SDK bounds require a path target type")
            };
            let target = target
                .path
                .segments
                .last()
                .expect("AsRef target paths have a segment")
                .ident
                .to_string();
            match target.as_str() {
                "str" | "String" => Some("string".to_string()),
                _ => panic!("unsupported AsRef SDK target `{target}`"),
            }
        }
        "ToolPkgBroadcastTopicKey" => Some("ToolPkg.BroadcastTopic".to_string()),
        "Send" | "Sync" | "Sized" => None,
        name => panic!("unsupported Rust SDK generic bound `{name}`"),
    }
}

/// Reports whether a Rust boundary type resolves to a TypeScript function type.
fn is_function_type(rust_type: &Type) -> bool {
    match rust_type {
        Type::TraitObject(trait_object) => trait_object.bounds.iter().any(|bound| {
            matches!(bound, TypeParamBound::Trait(bound) if bound.path.segments.last().is_some_and(|segment| segment.ident == "Fn"))
        }),
        Type::Path(_) => single_type_argument(rust_type).is_some_and(|(name, inner)| {
            matches!(name.as_str(), "Arc" | "Box" | "Pin") && is_function_type(inner)
        }),
        Type::Paren(parenthesized) => is_function_type(&parenthesized.elem),
        Type::Group(group) => is_function_type(&group.elem),
        Type::Reference(reference) => is_function_type(&reference.elem),
        _ => false,
    }
}

/// Reports whether a field is optional in TypeScript.
fn is_optional_field(field_type: &Type) -> bool {
    single_type_argument(field_type)
        .is_some_and(|(name, _)| matches!(name.as_str(), "Option" | "JsOptional"))
}

/// Resolves a Rust field identifier to its TypeScript property name.
fn field_name(field: &syn::Field) -> String {
    let rust_name = field
        .ident
        .as_ref()
        .map(|identifier| identifier.to_string())
        .unwrap_or_else(|| "value".to_string());
    renamed_identifier(&field.attrs, rust_name.trim_start_matches("r#"))
}

/// Resolves serde rename metadata for an identifier.
fn renamed_identifier(attrs: &[Attribute], rust_name: &str) -> String {
    serde_rename(attrs).unwrap_or_else(|| rust_name.to_string())
}

/// Reads a serde rename attribute through syn's structured metadata parser.
fn serde_rename(attrs: &[Attribute]) -> Option<String> {
    for attribute in attrs {
        if !attribute.path().is_ident("serde") {
            continue;
        }
        let mut rename = None;
        let _ = attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value: LitStr = meta.value()?.parse()?;
                rename = Some(value.value());
            }
            Ok(())
        });
        if rename.is_some() {
            return rename;
        }
    }
    None
}

/// Reports whether serde flatten metadata exists on a Rust field.
fn has_serde_flatten(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attribute| {
        if !attribute.path().is_ident("serde") {
            return false;
        }
        let mut flatten = false;
        let _ = attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("flatten") {
                flatten = true;
            }
            Ok(())
        });
        flatten
    })
}

/// Returns the string stored in a Rust doc attribute.
fn doc_value(attribute: &Attribute) -> Option<String> {
    if !attribute.path().is_ident("doc") {
        return None;
    }
    let syn::Meta::NameValue(name_value) = &attribute.meta else {
        return None;
    };
    let syn::Expr::Lit(expression) = &name_value.value else {
        return None;
    };
    let syn::Lit::Str(value) = &expression.lit else {
        return None;
    };
    Some(value.value().trim().to_string())
}

/// Emits ordinary Rust documentation as a TypeScript JSDoc block.
fn emit_jsdoc(output: &mut String, attrs: &[Attribute], indent: &str) {
    let docs = attrs.iter().filter_map(doc_value).collect::<Vec<_>>();
    let Some(first_content) = docs.iter().position(|doc| !doc.is_empty()) else {
        return;
    };
    let last_content = docs
        .iter()
        .rposition(|doc| !doc.is_empty())
        .expect("JSDoc content bounds must contain a non-empty line");
    output.push_str(indent);
    output.push_str("/**\n");
    for doc in &docs[first_content..=last_content] {
        output.push_str(indent);
        output.push_str(" *");
        if !doc.is_empty() {
            output.push(' ');
            output.push_str(doc);
        }
        output.push('\n');
    }
    output.push_str(indent);
    output.push_str(" */\n");
}

/// Removes the Rust-only overload suffix from a JavaScript method identifier.
fn method_name(rust_name: &str) -> String {
    rust_name
        .rsplit_once("_overload_")
        .map(|(base, _)| base)
        .unwrap_or(rust_name)
        .trim_start_matches("r#")
        .to_string()
}

/// Returns a TypeScript parameter name from a Rust pattern.
fn pattern_name(pattern: &syn::Pat) -> String {
    match pattern {
        syn::Pat::Ident(identifier) => identifier
            .ident
            .to_string()
            .trim_start_matches("r#")
            .to_string(),
        _ => panic!("SDK trait parameters must use identifier patterns"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that generated JSDoc excludes empty boundary lines and preserves inner paragraphs.
    #[test]
    fn trims_jsdoc_boundary_lines() {
        let attrs = vec![
            syn::parse_quote!(#[doc = ""]),
            syn::parse_quote!(#[doc = " Summary."]),
            syn::parse_quote!(#[doc = ""]),
            syn::parse_quote!(#[doc = " Details."]),
            syn::parse_quote!(#[doc = ""]),
        ];
        let mut output = String::new();

        emit_jsdoc(&mut output, &attrs, "  ");

        assert_eq!(output, "  /**\n   * Summary.\n   *\n   * Details.\n   */\n");
    }

    /// Verifies that ordinary Rust composition fields retain interface and open-object semantics.
    #[test]
    fn recognizes_interface_composition_fields() {
        let item: ItemStruct = syn::parse_quote! {
            pub struct Example {
                pub base_json_object: ToolPkgJsonObject,
                pub base_url: Option<String>,
                pub additional_properties: BTreeMap<String, String>,
            }
        };
        let Fields::Named(fields) = item.fields else {
            panic!("test struct must use named fields");
        };
        let fields = fields.named.iter().collect::<Vec<_>>();

        assert!(is_inherited_base_field(fields[0]));
        assert!(!is_inherited_base_field(fields[1]));
        assert!(is_additional_properties_field(fields[2]));
    }
}

/// Reports whether a declaration path starts with one namespace path.
fn path_starts_with(path: &[String], prefix: &[String]) -> bool {
    path.len() >= prefix.len()
        && path
            .iter()
            .zip(prefix)
            .all(|(component, expected)| component == expected)
}

/// Resolves a named declaration path relative to the current namespace.
fn relative_ts_path(path: &[String], scope: &[String]) -> String {
    if path_starts_with(path, scope) && path.len() > scope.len() {
        path[scope.len()..].join(".")
    } else {
        path.join(".")
    }
}

/// Converts a generated declaration file name into a relative TypeScript module path.
fn typescript_module_path(file_name: &str) -> String {
    let module_name = file_name
        .strip_suffix(".d.ts")
        .expect("generated declaration files must end with .d.ts");
    format!("./{module_name}")
}
