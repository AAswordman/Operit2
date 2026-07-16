use quote::ToTokens;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::{
    Attribute, ExprPath, ImplItemFn, ItemConst, ItemExternCrate, ItemFn, ItemMod, ItemStatic,
    ItemUse, Lit, Meta, Path as SynPath, Token, TraitItemFn, TypePath, UseTree,
};

use super::SourceRoot;

/// Rejects direct platform APIs from Wasm builds and audits them for native builds.
pub(crate) fn enforce_host_platform_boundaries(source_roots: &[SourceRoot], target_arch: &str) {
    let mut violations = BTreeSet::new();
    for source_root in source_roots {
        collect_source_violations(source_root, source_root.as_path(), &mut violations);
    }
    if violations.is_empty() {
        return;
    }
    let messages = violations
        .iter()
        .map(PlatformApiViolation::message)
        .collect::<Vec<_>>()
        .join("\n");
    if target_arch == "wasm32" {
        panic!(
            "Host platform boundary violations detected:\n{}\nUse the corresponding operit_host_api Host abstraction instead.",
            messages
        );
    }
    println!(
        "cargo:warning=Host platform boundary audit found {} native-source violation(s); Wasm builds reject these APIs.",
        violations.len()
    );
}

/// Recursively parses one crate source directory and records restricted platform API usage.
fn collect_source_violations(
    source_root: &SourceRoot,
    directory: &Path,
    violations: &mut BTreeSet<PlatformApiViolation>,
) {
    let mut source_files = Vec::new();
    collect_source_files(directory, &mut source_files);
    let test_sources = collect_test_source_files(&source_files);
    for path in source_files {
        if test_sources.contains(&path) {
            continue;
        }
        collect_file_violations(source_root, &path, violations);
    }
}

/// Recursively collects Rust source files for one crate without inspecting their contents.
fn collect_source_files(directory: &Path, source_files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read source directory {}: {error}", directory.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| panic!("read source entry: {error}"));
        let path = entry.path();
        if path.is_dir() {
            collect_source_files(&path, source_files);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        source_files.push(path);
    }
}

/// Finds external module sources that are compiled only by test builds.
fn collect_test_source_files(source_files: &[PathBuf]) -> BTreeSet<PathBuf> {
    let mut test_sources = BTreeSet::new();
    for source_path in source_files {
        let content = fs::read_to_string(source_path)
            .unwrap_or_else(|error| panic!("read source file {}: {error}", source_path.display()));
        let file = syn::parse_file(&content)
            .unwrap_or_else(|error| panic!("parse source file {}: {error}", source_path.display()));
        for item in file.items {
            let syn::Item::Mod(module) = item else {
                continue;
            };
            if module.content.is_some() || !is_excluded_from_wasm(&module.attrs) {
                continue;
            }
            if let Some(path) = external_module_source_path(source_path, &module) {
                test_sources.insert(path);
            }
        }
    }
    test_sources
}

/// Resolves the source file for one external test-only module declaration.
fn external_module_source_path(source_path: &Path, module: &ItemMod) -> Option<PathBuf> {
    let parent = source_path.parent()?;
    for attribute in &module.attrs {
        if !attribute.path().is_ident("path") {
            continue;
        }
        let Meta::NameValue(value) = &attribute.meta else {
            continue;
        };
        let syn::Expr::Lit(expression) = &value.value else {
            continue;
        };
        let Lit::Str(path) = &expression.lit else {
            continue;
        };
        return Some(parent.join(path.value()));
    }
    let module_name = module.ident.to_string();
    let direct_file = parent.join(format!("{module_name}.rs"));
    if direct_file.is_file() {
        return Some(direct_file);
    }
    Some(parent.join(module_name).join("mod.rs"))
}

/// Parses one Rust source file and records its restricted platform API usage.
fn collect_file_violations(
    source_root: &SourceRoot,
    path: &Path,
    violations: &mut BTreeSet<PlatformApiViolation>,
) {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read source file {}: {error}", path.display()));
    let file = syn::parse_file(&content)
        .unwrap_or_else(|error| panic!("parse source file {}: {error}", path.display()));
    let mut visitor = PlatformApiVisitor::new(source_root, path, violations);
    visitor.visit_file(&file);
}

/// Traverses one source file and reports forbidden standard-library platform APIs.
struct PlatformApiVisitor<'a> {
    source_root: &'a SourceRoot,
    path: &'a Path,
    violations: &'a mut BTreeSet<PlatformApiViolation>,
}

impl<'a> PlatformApiVisitor<'a> {
    /// Creates a visitor bound to one source file.
    fn new(
        source_root: &'a SourceRoot,
        path: &'a Path,
        violations: &'a mut BTreeSet<PlatformApiViolation>,
    ) -> Self {
        Self {
            source_root,
            path,
            violations,
        }
    }

    /// Records one platform-boundary violation with crate and source location context.
    fn report(&mut self, api: &str) {
        self.violations.insert(PlatformApiViolation {
            crate_name: self.source_root.crate_name.clone(),
            path: self.path.to_path_buf(),
            api: api.to_string(),
        });
    }
}

/// Identifies one unique restricted platform API use in a source file.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PlatformApiViolation {
    crate_name: String,
    path: PathBuf,
    api: String,
}

impl PlatformApiViolation {
    /// Formats the violation for the Cargo build failure message.
    fn message(&self) -> String {
        format!(
            "{}: {} uses forbidden {}",
            self.crate_name,
            self.path.display(),
            self.api
        )
    }
}

impl<'ast> Visit<'ast> for PlatformApiVisitor<'_> {
    /// Rejects imports that introduce direct file, clock, or thread APIs.
    fn visit_item_use(&mut self, item_use: &ItemUse) {
        if is_excluded_from_wasm(&item_use.attrs) {
            return;
        }
        let mut segments = Vec::new();
        collect_use_tree_violations(&item_use.tree, &mut segments, self);
        syn::visit::visit_item_use(self, item_use);
    }

    /// Rejects direct fully qualified platform API paths.
    fn visit_expr_path(&mut self, expression: &ExprPath) {
        if let Some(api) = prohibited_path_api(&expression.path) {
            self.report(api);
        }
        syn::visit::visit_expr_path(self, expression);
    }

    /// Rejects restricted platform types used through fully qualified paths.
    fn visit_type_path(&mut self, type_path: &TypePath) {
        if let Some(api) = prohibited_path_api(&type_path.path) {
            self.report(api);
        }
        syn::visit::visit_type_path(self, type_path);
    }

    /// Rejects an aliased `std` extern crate because it can bypass import checks.
    fn visit_item_extern_crate(&mut self, item: &ItemExternCrate) {
        if item.ident == "std" {
            self.report("std extern crate alias");
        }
        syn::visit::visit_item_extern_crate(self, item);
    }

    /// Skips test-only module bodies because the restriction applies to production runtime code.
    fn visit_item_mod(&mut self, item: &ItemMod) {
        if is_excluded_from_wasm(&item.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, item);
    }

    /// Skips function bodies that cannot be selected for the Wasm target.
    fn visit_item_fn(&mut self, item: &ItemFn) {
        if is_excluded_from_wasm(&item.attrs) {
            return;
        }
        syn::visit::visit_item_fn(self, item);
    }

    /// Skips associated function bodies that cannot be selected for the Wasm target.
    fn visit_impl_item_fn(&mut self, item: &ImplItemFn) {
        if is_excluded_from_wasm(&item.attrs) {
            return;
        }
        syn::visit::visit_impl_item_fn(self, item);
    }

    /// Skips trait default bodies that cannot be selected for the Wasm target.
    fn visit_trait_item_fn(&mut self, item: &TraitItemFn) {
        if is_excluded_from_wasm(&item.attrs) {
            return;
        }
        syn::visit::visit_trait_item_fn(self, item);
    }

    /// Skips constants that cannot be selected for the Wasm target.
    fn visit_item_const(&mut self, item: &ItemConst) {
        if is_excluded_from_wasm(&item.attrs) {
            return;
        }
        syn::visit::visit_item_const(self, item);
    }

    /// Skips statics that cannot be selected for the Wasm target.
    fn visit_item_static(&mut self, item: &ItemStatic) {
        if is_excluded_from_wasm(&item.attrs) {
            return;
        }
        syn::visit::visit_item_static(self, item);
    }
}

/// Walks a use tree while preserving its fully qualified import path.
fn collect_use_tree_violations(
    tree: &UseTree,
    segments: &mut Vec<String>,
    visitor: &mut PlatformApiVisitor<'_>,
) {
    match tree {
        UseTree::Path(path) => {
            segments.push(path.ident.to_string());
            collect_use_tree_violations(&path.tree, segments, visitor);
            segments.pop();
        }
        UseTree::Name(name) => {
            segments.push(name.ident.to_string());
            report_prohibited_import(segments, visitor);
            segments.pop();
        }
        UseTree::Rename(rename) => {
            segments.push(rename.ident.to_string());
            report_prohibited_import(segments, visitor);
            segments.pop();
        }
        UseTree::Glob(_) => report_prohibited_import(segments, visitor),
        UseTree::Group(group) => {
            for child in &group.items {
                collect_use_tree_violations(child, segments, visitor);
            }
        }
    }
}

/// Reports a restricted API imported through a fully qualified use-tree path.
fn report_prohibited_import(segments: &[String], visitor: &mut PlatformApiVisitor<'_>) {
    if let Some(api) = prohibited_segments_api(segments) {
        visitor.report(api);
    }
}

/// Identifies whether a fully qualified `syn::Path` selects a restricted API.
fn prohibited_path_api(path: &SynPath) -> Option<&'static str> {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    prohibited_segments_api(&segments)
}

/// Identifies restricted standard-library file, clock, and thread API paths.
fn prohibited_segments_api(segments: &[String]) -> Option<&'static str> {
    match segments {
        [std, fs, ..] if std == "std" && fs == "fs" => Some("std::fs"),
        [std, thread, ..] if std == "std" && thread == "thread" => Some("std::thread"),
        [std, time, clock, ..]
            if std == "std"
                && time == "time"
                && matches!(clock.as_str(), "Instant" | "SystemTime") =>
        {
            Some("std::time clock")
        }
        [std, time] if std == "std" && time == "time" => Some("std::time module alias"),
        _ => None,
    }
}

/// Reports whether an item's cfg attributes exclude the wasm32-unknown-unknown target.
fn is_excluded_from_wasm(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && attribute
                .parse_args::<Meta>()
                .is_ok_and(|condition| cfg_condition_excludes_wasm(&condition))
    })
}

/// Evaluates whether a cfg condition is false for wasm32-unknown-unknown.
fn cfg_condition_excludes_wasm(condition: &Meta) -> bool {
    match condition {
        Meta::Path(path) => path.is_ident("test"),
        Meta::NameValue(value) if value.path.is_ident("target_arch") => {
            value.value.to_token_stream().to_string() != "\"wasm32\""
        }
        Meta::List(list) if list.path.is_ident("not") => {
            parse_cfg_conditions(list).is_some_and(|conditions| {
                conditions.len() == 1 && !cfg_condition_excludes_wasm(&conditions[0])
            })
        }
        Meta::List(list) if list.path.is_ident("all") => parse_cfg_conditions(list)
            .is_some_and(|conditions| conditions.iter().any(cfg_condition_excludes_wasm)),
        Meta::List(list) if list.path.is_ident("any") => parse_cfg_conditions(list)
            .is_some_and(|conditions| conditions.iter().all(cfg_condition_excludes_wasm)),
        _ => false,
    }
}

/// Parses nested cfg predicate arguments from one cfg combinator.
fn parse_cfg_conditions(list: &syn::MetaList) -> Option<Vec<Meta>> {
    Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()
        .map(|conditions| conditions.into_iter().collect())
}
