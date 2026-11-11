//! Enforces the native filesystem module boundary before the tools crate builds.
//! The provider trait stays public for platform implementations; application code
//! must use VFS unless its source module is explicitly admitted below.
use std::path::Path;
use syn::visit::{self, Visit};

// Only foundational assembly and platform adapters may access native filesystem contracts.
const NATIVE_MODULES: &[(&str, &str)] = &[
    (
        "core/crates/foundation/host-contract/src/lib.rs",
        "platform provider contract",
    ),
    (
        "core/crates/foundation/host-api/src/HostManager.rs",
        "host capability assembly",
    ),
    (
        "core/crates/foundation/vfs/src/VisualFileSystem.rs",
        "VFS backend dispatch",
    ),
    (
        "apps/flutter/native/operit-flutter-bridge/src/platform_runtime/android.rs",
        "Android host assembly",
    ),
    (
        "apps/flutter/native/operit-flutter-bridge/src/platform_runtime/ios.rs",
        "iOS host assembly",
    ),
];

const NATIVE_LOCATOR_MODULES: &[&str] = &[
    "core/crates/foundation/host-contract/src/lib.rs",
    "core/crates/foundation/vfs/src/PathMapper.rs",
    "core/crates/foundation/vfs/src/VisualFileSystem.rs",
    "core/crates/foundation/vfs/src/FileReference.rs",
    "core/crates/foundation/vfs/src/MountRegistry.rs",
    "core/crates/foundation/vfs/src/StorageManagement.rs",
    "apps/cli/src/cli/host_ops.rs",
    "apps/flutter/native/operit-flutter-bridge/src/FlutterOwnerCapabilities.rs",
];
#[derive(Default)]
struct NativeUse {
    found: bool,
    locator: bool,
    raw_io: bool,
}

/// Identifies test-only items before visiting their implementation details.
fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        let syn::Meta::List(list) = &attr.meta else {
            return false;
        };
        let text = list.tokens.to_string();
        (text == "test" || text.starts_with("all (test ,")) && !text.contains("not (test)")
    })
}

impl<'ast> Visit<'ast> for NativeUse {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let attrs = match item {
            syn::Item::Mod(item) => &item.attrs,
            syn::Item::Fn(item) => &item.attrs,
            syn::Item::Use(item) => &item.attrs,
            syn::Item::Struct(item) => &item.attrs,
            syn::Item::Impl(item) => &item.attrs,
            syn::Item::Enum(item) => &item.attrs,
            syn::Item::Type(item) => &item.attrs,
            _ => {
                visit::visit_item(self, item);
                return;
            }
        };
        if !test_only(attrs) {
            visit::visit_item(self, item);
        }
    }

    fn visit_ident(&mut self, ident: &'ast syn::Ident) {
        if ident == "FileSystemHost" {
            self.found = true;
        }
    }

    /// Detects native file extraction even when callers only import a file capability.
    fn visit_expr_method_call(&mut self, expression: &'ast syn::ExprMethodCall) {
        if [
            "nativePath",
            "resolvePath",
            "runtimeRootDir",
            "workspaceRootDir",
        ]
        .iter()
        .any(|name| expression.method == *name)
        {
            self.locator = true;
        }
        if expression.method == "fileSystemHost" {
            self.found = true;
        }
        visit::visit_expr_method_call(self, expression);
    }

    /// Rejects fully qualified operating-system file and process calls in business code.
    fn visit_path(&mut self, path: &'ast syn::Path) {
        let segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        if segments.first().is_some_and(|name| name == "std")
            && segments.get(1).is_some_and(|name| {
                name == "fs"
                    || name == "process" && segments.get(2).is_some_and(|name| name == "Command")
            })
        {
            self.raw_io = true;
        }
        visit::visit_path(self, path);
    }

    /// Covers aliases imported from operating-system modules as well as direct paths.
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        /// Walks grouped imports while retaining their complete module prefix.
        fn inspect(tree: &syn::UseTree, prefix: &str) -> bool {
            let next = |name: &syn::Ident| {
                if prefix.is_empty() {
                    name.to_string()
                } else {
                    format!("{prefix}::{name}")
                }
            };
            match tree {
                syn::UseTree::Path(path) => inspect(&path.tree, &next(&path.ident)),
                syn::UseTree::Group(group) => group.items.iter().any(|tree| inspect(tree, prefix)),
                syn::UseTree::Name(name) => inspect_name(&next(&name.ident)),
                syn::UseTree::Rename(name) => inspect_name(&next(&name.ident)),
                syn::UseTree::Glob(_) => inspect_name(prefix),
            }
        }
        /// Matches native modules rather than guessing from imported alias names.
        fn inspect_name(name: &str) -> bool {
            name == "std::fs"
                || name.starts_with("std::fs::")
                || name == "std::process::Command"
                || name == "std::process"
        }
        if !test_only(&item.attrs) && inspect(&item.tree, "") {
            self.raw_io = true;
        }
        visit::visit_item_use(self, item);
    }

    /// Covers fully qualified trait calls as well as ordinary method syntax.
    fn visit_expr_call(&mut self, expression: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = expression.func.as_ref() {
            if function
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "nativePath")
            {
                self.locator = true;
            }
        }
        visit::visit_expr_call(self, expression);
    }

    /// Prevents bypassing the accessor by reading the legacy resolved-locator field.
    fn visit_expr_field(&mut self, expression: &'ast syn::ExprField) {
        if matches!(&expression.member, syn::Member::Named(name) if name == "fileSystemHost") {
            self.found = true;
        }
        if matches!(&expression.member, syn::Member::Named(name) if name == "physicalPath") {
            self.locator = true;
        }
        visit::visit_expr_field(self, expression);
    }
}

/// Enforces VFS ownership for one production source file, excluding its test-only items.
pub fn check_source(path: &str, source: &str) -> Result<(), String> {
    let syntax = syn::parse_file(source).map_err(|error| format!("{path}: {error}"))?;
    if test_only(&syntax.attrs) {
        return Ok(());
    }
    let mut usage = NativeUse::default();
    usage.visit_file(&syntax);
    if usage.raw_io
        && path.starts_with("core/crates/")
        && ![
            "/codegen/",
            "-codegen/",
            "/proxy/scan/",
            "/build.rs",
            "/foundation/host-contract/",
        ]
        .iter()
        .any(|segment| path.contains(segment))
    {
        return Err(format!("{path}: native filesystem and process I/O belongs in platform Hosts. Use VFS file capabilities in Core."));
    }
    if usage.found && !NATIVE_MODULES.iter().any(|(allowed, _)| *allowed == path) {
        return Err(format!("{path}: direct FileSystemHost access is restricted to foundational assembly and platform adapters. Use context.vfs() for business file access."));
    }
    if usage.locator && !NATIVE_LOCATOR_MODULES.contains(&path) {
        return Err(format!("{path}: native file locators are restricted to VFS internals and platform consumers. Pass a VFS file capability to the consuming Host instead."));
    }
    Ok(())
}

/// Scans application sources when building from a complete repository checkout.
pub fn check_repository(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Published crates do not contain the application repository.
    if !root.join("core/Cargo.toml").is_file() {
        return Ok(());
    }
    for directory in [
        "core/crates",
        "apps/flutter/native/operit-flutter-bridge/src",
        "apps/cli/src",
    ] {
        check_directory(root, &root.join(directory))?;
    }
    Ok(())
}

/// Recurses through production modules without treating fixture implementations as business code.
fn check_directory(root: &Path, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !directory.is_dir() {
        return Ok(());
    }
    println!("cargo:rerun-if-changed={}", directory.display());
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some("target" | "tests" | "build_support")
            ) {
                check_directory(root, &path)?;
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("build_")
                || name.ends_with("_tests.rs")
                || name.ends_with(".tests.rs")
                || name.ends_with("Tests.rs")
            {
                continue;
            }
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            check_source(&relative, &std::fs::read_to_string(&path)?)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unapproved_native_access_and_aliases() {
        for source in [
            "use operit_host_api::FileSystemHost;",
            "use operit_host_api::FileSystemHost as Files;",
            "fn bypass(host: HostManager) { host.fileSystemHost.unwrap(); }",
            "fn bypass(manager: Manager) { manager.fileSystemHost(); }",
        ] {
            assert!(check_source(
                "core/crates/runtime/application/src/services/NewService.rs",
                source
            )
            .is_err());
        }
    }
    #[test]
    fn allows_vfs_consumers_and_test_fixtures() {
        for source in [
            "fn read(context: HostManager) { (&context).vfs(); }",
            "#[cfg(test)] mod tests { use operit_host_api::FileSystemHost; }",
            "#[cfg(all(test, not(target_arch = \"wasm32\")))] mod tests { use operit_host_api::FileSystemHost; }",
            "const MESSAGE: &str = \"FileSystemHost\";",
        ] {
            check_source("core/crates/runtime/application/src/services/NewService.rs", source).unwrap();
        }
    }
    #[test]
    fn permits_reviewed_native_modules() {
        check_source(
            "core/crates/foundation/vfs/src/VisualFileSystem.rs",
            "use operit_host_api::FileSystemHost;",
        )
        .unwrap();
    }

    /// Blocks native extraction in ordinary services and already-migrated file tools.
    #[test]
    fn rejects_native_locator_escape_from_vfs_capabilities() {
        for path in [
            "core/crates/runtime/application/src/services/NewService.rs",
            "core/crates/tool/services/src/tools/defaultTool/standard/StandardFileSystemTools.rs",
        ] {
            for source in [
                "fn bypass(file: VfsFile) { file.nativePath(); }",
                "fn bypass(file: VfsFile) { FileSource::nativePath(&file); }",
                "fn bypass(path: ResolvedVfsPath) { consume(path.physicalPath); }",
            ] {
                assert!(check_source(path, source).is_err());
            }
            check_source(path, "fn read(file: VfsFile) { file.readBytes(); }").unwrap();
        }
    }
}
