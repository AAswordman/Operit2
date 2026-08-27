use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Expr, Fields, FnArg, ImplItem, ImplItemFn, Item, ItemEnum, ItemImpl, ItemStruct,
    Lit, Meta, MetaNameValue, Pat, ReturnType, Token, Type, TypePath, UseTree, Visibility,
};

mod build_platform_api_guard;
mod build_scanner;

use build_platform_api_guard::*;
use build_scanner::*;
pub use operit_rslink_codegen::*;
use operit_rslink_codegen::*;

/// Describes the explicit roots used by root-registered core-app proxy generation.
pub struct CoreProxyRegisteredRoots {
    proxy_manifest_dir: PathBuf,
    runtime_src: PathBuf,
    proxy_src: PathBuf,
    model_src: PathBuf,
    local_models_src: PathBuf,
    plugin_sdk_src: PathBuf,
    store_src: PathBuf,
    link_src: PathBuf,
    link_access_src: PathBuf,
    server_src: PathBuf,
    util_src: PathBuf,
    tools_src: PathBuf,
    providers_src: PathBuf,
    host_api_src: PathBuf,
    javascript_bridge_src: PathBuf,
}

impl CoreProxyRegisteredRoots {
    /// Creates the standard Operit core-app proxy root registry from the proxy crate path.
    pub fn from_proxy_manifest_dir(proxy_manifest_dir: PathBuf) -> Self {
        Self {
            runtime_src: proxy_manifest_dir.join("../../runtime/application/src"),
            proxy_src: proxy_manifest_dir.join("src"),
            model_src: proxy_manifest_dir.join("../../foundation/model/src"),
            local_models_src: proxy_manifest_dir.join("../../provider/local-model/src"),
            plugin_sdk_src: proxy_manifest_dir.join("../../plugin/sdk/src"),
            store_src: proxy_manifest_dir.join("../../persistence/store/src"),
            link_src: proxy_manifest_dir.join("../../foundation/link/src"),
            link_access_src: proxy_manifest_dir.join("../../access/runtime/src"),
            server_src: proxy_manifest_dir.join("../../node/runtime/src"),
            util_src: proxy_manifest_dir.join("../../foundation/util/src"),
            tools_src: proxy_manifest_dir.join("../../tool/services/src"),
            providers_src: proxy_manifest_dir.join("../../provider/services/src"),
            host_api_src: proxy_manifest_dir.join("../../foundation/host-api/src"),
            javascript_bridge_src: proxy_manifest_dir.join("../../plugin/javascript-bridge/src"),
            proxy_manifest_dir,
        }
    }

    /// Returns the proxy crate manifest directory used by emitter crates.
    pub fn proxy_manifest_dir(&self) -> &Path {
        &self.proxy_manifest_dir
    }
}

/// Configures one root-registered core-app proxy scan.
pub struct CoreProxyScanConfig {
    roots: CoreProxyRegisteredRoots,
}

impl CoreProxyScanConfig {
    /// Creates a config for scanning artifacts from an explicit root registry.
    pub fn new(roots: CoreProxyRegisteredRoots) -> Self {
        Self { roots }
    }

    /// Creates the standard Operit proxy config from the proxy crate path.
    pub fn from_proxy_manifest_dir(proxy_manifest_dir: PathBuf) -> Self {
        Self::new(CoreProxyRegisteredRoots::from_proxy_manifest_dir(
            proxy_manifest_dir,
        ))
    }
}

/// Carries reusable outputs produced by one core-app proxy scan.
pub struct CoreProxyScanOutput {
    pub proxy_manifest_dir: PathBuf,
    pub objects: Vec<SourceObject>,
    pub serializable_type_definitions: HashMap<String, SerializableType>,
    pub error_type_definitions: HashMap<String, ErrorTypeDefinition>,
}

/// Scans registered core-app roots into a language-neutral proxy model.
pub fn scan_core_proxy(config: CoreProxyScanConfig) -> CoreProxyScanOutput {
    let roots = config.roots;
    let proxy_manifest_dir = roots.proxy_manifest_dir.clone();
    let runtime_root = SourceRoot::new(roots.runtime_src, "operit_runtime");
    let core_proxy_root = SourceRoot::new(roots.proxy_src, "operit_proxy_local");
    let model_root = SourceRoot::new(roots.model_src, "operit_model");
    let local_models_root = SourceRoot::new(roots.local_models_src, "operit_local_models");
    let plugin_sdk_root = SourceRoot::new(roots.plugin_sdk_src, "operit_plugin_sdk");
    let store_root = SourceRoot::new(roots.store_src, "operit_store");
    let link_root = SourceRoot::new(roots.link_src, "operit_link");
    let link_access_root = SourceRoot::new(roots.link_access_src, "operit_access_runtime");
    let server_root = SourceRoot::new(roots.server_src, "operit_node_runtime");
    let util_root = SourceRoot::new(roots.util_src, "operit_util");
    let tools_root = SourceRoot::new(roots.tools_src, "operit_tools");
    let provider_root = SourceRoot::new(roots.providers_src, "operit_providers");
    let host_api_root = SourceRoot::new(roots.host_api_src, "operit_host_api");
    let javascript_bridge_root = SourceRoot::new(roots.javascript_bridge_src, "operit_js_bridge");
    let restricted_source_roots = vec![
        runtime_root.clone(),
        model_root.clone(),
        local_models_root.clone(),
        plugin_sdk_root.clone(),
        store_root.clone(),
        util_root.clone(),
        tools_root.clone(),
        provider_root.clone(),
        javascript_bridge_root.clone(),
    ];
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH")
        .expect("Cargo must provide CARGO_CFG_TARGET_ARCH to the proxy generator");
    enforce_host_platform_boundaries(&restricted_source_roots, &target_arch);
    let source_roots = vec![
        core_proxy_root.clone(),
        runtime_root.clone(),
        model_root,
        local_models_root,
        plugin_sdk_root,
        store_root.clone(),
        link_root,
        link_access_root.clone(),
        util_root,
        tools_root.clone(),
        provider_root.clone(),
        javascript_bridge_root,
        host_api_root,
        server_root.clone(),
    ];
    for source_root in &source_roots {
        emit_source_tree_rerun_if_changed(source_root.as_path());
    }
    let serializable_type_definitions = collect_serializable_type_definitions(&source_roots);
    let mut error_type_definitions = HashMap::new();
    for source_root in &source_roots {
        error_type_definitions.extend(collect_error_type_definitions(
            source_root.as_path(),
            &source_root.crate_name,
        ));
    }
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
    let type_registry = collect_type_registry(&source_roots);
    let object_specs = object_specs(
        &core_proxy_root,
        &runtime_root,
        &store_root,
        &tools_root,
        &provider_root,
        &link_access_root,
        &server_root,
    );
    let public_object_types = collect_public_object_types(&source_roots);
    for spec in &object_specs {
        println!("cargo:rerun-if-changed={}", spec.source_path.display());
    }

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
    objects.extend(factory_specs.iter().map(|spec| {
        scan_object(
            spec,
            &serializable_types,
            &deserializable_types,
            &type_registry,
        )
    }));
    objects.sort_by(|left, right| left.schema_key.cmp(&right.schema_key));
    for (object_id, object) in objects.iter_mut().enumerate() {
        object.object_id = object_id as u32;
    }
    CoreProxyScanOutput {
        proxy_manifest_dir,
        objects,
        serializable_type_definitions,
        error_type_definitions,
    }
}

/// Registers every source directory and file that contributes to the proxy scan.
fn emit_source_tree_rerun_if_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
    let entries = fs::read_dir(path)
        .unwrap_or_else(|error| panic!("read source tree {}: {error}", path.display()));
    for entry in entries {
        let entry = entry
            .unwrap_or_else(|error| panic!("read source tree entry {}: {error}", path.display()));
        let entry_path = entry.path();
        if entry_path.is_dir() {
            emit_source_tree_rerun_if_changed(&entry_path);
        } else {
            println!("cargo:rerun-if-changed={}", entry_path.display());
        }
    }
}
