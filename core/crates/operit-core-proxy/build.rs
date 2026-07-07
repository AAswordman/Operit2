use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::{
    Fields, FnArg, ImplItem, ImplItemFn, Item, ItemEnum, ItemImpl, ItemStruct, Pat, ReturnType,
    Type, TypePath, UseTree, Visibility,
};

mod build_dart_codegen;
mod build_model;
mod build_rust_codegen;
mod build_rust_codegen_utils;
mod build_rust_dispatch_codegen;
mod build_rust_proxy_codegen;
mod build_rust_schema_codegen;
mod build_scanner;
mod build_type_resolver;
mod build_utils;

use build_model::*;
use build_scanner::*;
use build_type_resolver::*;
use build_utils::*;

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let runtime_src = manifest_dir.join("../operit-runtime/src");
    let store_src = manifest_dir.join("../operit-store/src");
    let serializable_type_definitions = collect_serializable_type_definitions(&runtime_src);
    let mut error_type_definitions =
        collect_error_type_definitions(&runtime_src, "operit_runtime");
    error_type_definitions.extend(collect_error_type_definitions(&store_src, "operit_store"));
    let serializable_types = serializable_type_definitions
        .iter()
        .filter(|(_, ty)| ty.supports_serialize)
        .map(|(name, _)| name.clone())
        .collect::<HashSet<_>>();
    let deserializable_types = serializable_type_definitions
        .iter()
        .filter(|(_, ty)| ty.supports_deserialize)
        .map(|(name, _)| name.clone())
        .collect::<HashSet<_>>();
    let type_registry = collect_type_registry(&runtime_src);
    let object_specs = object_specs(&runtime_src);
    let public_object_types = collect_public_object_types(&runtime_src);
    for spec in &object_specs {
        println!("cargo:rerun-if-changed={}", spec.source_path.display());
    }
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("build_dart_codegen.rs").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("build_rust_codegen.rs").display()
    );

    let mut objects = object_specs
        .iter()
        .map(|spec| {
            scan_object(
                spec,
                &serializable_types,
                &deserializable_types,
                &type_registry,
            )
        })
        .collect::<Vec<_>>();
    let factory_specs = discover_factory_object_specs(
        &objects,
        &object_specs,
        &public_object_types,
        &serializable_types,
        &deserializable_types,
        &type_registry,
    );
    mark_factory_methods(&mut objects, &factory_specs);
    for spec in &factory_specs {
        println!("cargo:rerun-if-changed={}", spec.source_path.display());
    }
    objects.extend(
        factory_specs
            .iter()
            .map(|spec| {
                scan_object(
                    spec,
                    &serializable_types,
                    &deserializable_types,
                    &type_registry,
                )
            }),
    );
    let schema_json = build_rust_codegen::render_schema(&objects, &serializable_type_definitions);
    let generated =
        build_rust_codegen::render_generated(&objects, &schema_json, &error_type_definitions);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out_dir.join("generated_core_dispatch.rs"), generated)
        .expect("write generated_core_dispatch.rs");
    build_dart_codegen::write_dart_proxy_artifacts(
        &manifest_dir,
        &schema_json,
        &objects,
        &serializable_type_definitions,
    );
}
