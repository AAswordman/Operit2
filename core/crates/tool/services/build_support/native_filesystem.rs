//! Enforces the native filesystem module boundary before the tools crate builds.
//! The provider trait stays public for platform implementations; application code
//! must use VFS unless its source module is explicitly admitted below.
use std::path::Path;
use syn::visit::{self, Visit};

// Each entry is a module-level friend grant, not a directory-wide exemption.
// Native paths are valid here only for the purpose stated in the second column.
const NATIVE_MODULES: &[(&str, &str)] = &[
    ("core/crates/foundation/host-api/src/lib.rs", "platform provider contract"),
    ("core/crates/foundation/host-api/src/HostManager.rs", "host capability assembly"),
    ("core/crates/foundation/util/src/AppLogger.rs", "native append handles for logs"),
    ("core/crates/tool/services/src/files/VisualFileSystem.rs", "VFS dispatch after path mapping"),
    ("core/crates/tool/services/src/tools/AIToolHandler.rs", "runtime-owned plugin config directories and host assembly"),
    ("core/crates/tool/services/src/tools/defaultTool/ToolGetter.rs", "construct VFS and file capabilities"),
    ("core/crates/tool/services/src/tools/defaultTool/PathValidator.rs", "native host validation adapter"),
    ("core/crates/tool/services/src/tools/defaultTool/standard/StandardFileSystemTools.rs", "VFS assembly and resolved native operations"),
    ("core/crates/tool/services/src/tools/defaultTool/standard/StandardMediaTools.rs", "VFS assembly for media registration"),
    ("core/crates/tool/services/src/tools/ToolJsRuntime.rs", "export package resources into runtime-owned storage"),
    ("core/crates/tool/services/src/tools/transfer/FileTransferTools.rs", "resolved transfer targets and streaming staging files"),
    ("core/crates/tool/services/src/tools/packTool/RuntimePackageManager.rs", "installed package files and runtime resource cache"),
    ("core/crates/tool/services/src/tools/packTool/ToolPkgResourceReader.rs", "installed package archive resources"),
    ("core/crates/tool/services/src/tools/packTool/ToolPkgTextResourceCache.rs", "installed package content signatures"),
    ("core/crates/tool/services/src/tools/skill/SkillManager.rs", "installed skill storage and archive extraction"),
    ("core/crates/tool/services/src/tools/skill_runtime/SkillRepository.rs", "write installed skills after VFS source reads"),
    ("core/crates/tool/services/src/tools/mcp_runtime/MCPLocalServer.rs", "native MCP process configuration"),
    ("core/crates/tool/services/src/tools/mcp_runtime/MCPRepository.rs", "installed MCP storage and archive extraction"),
    ("core/crates/tool/services/src/tools/mcp_runtime/plugins/MCPProjectAnalyzer.rs", "inspect installed native MCP projects"),
    ("core/crates/plugin/sdk/src/PackageManager.rs", "installed package source loading"),
    ("core/crates/plugin/sdk/src/JsPackageLoader.rs", "native package source loader"),
    ("core/crates/plugin/sdk/src/toolpkg/ToolPkgLoader.rs", "native archive loader"),
    ("core/crates/plugin/sdk/src/toolpkg/ToolPkgParser.rs", "native archive parsing"),
    ("core/crates/plugin/sdk/src/toolpkg/ToolPkgManager.rs", "installed archive resource loading"),
    ("core/crates/plugin/sdk/src/toolpkg/ToolPkgPackageService.rs", "installed package resource service"),
    ("core/crates/provider/services/src/tts/SystemVoiceProvider.rs", "read native synthesis host output"),
    ("core/crates/provider/services/src/tts/VoiceServiceFactory.rs", "construct native synthesis adapter"),
    ("core/crates/runtime/application/src/services/LocalProviderService.rs", "native inference artifacts and generated audio"),
    ("core/crates/runtime/application/src/services/SttRecognitionService.rs", "recording staging for native inference"),
    ("core/crates/runtime/application/src/services/AttachmentTransferManager.rs", "native picker sources and resolved attachment transfers"),
    ("core/crates/runtime/application/src/services/WorkspaceService.rs", "VFS assembly"),
    ("core/crates/runtime/application/src/services/RuntimeHostInfoService.rs", "report capability presence without file access"),
    ("core/crates/runtime/application/src/services/RuntimeTerminalService.rs", "VFS resolution for native terminal directories"),
    ("core/crates/runtime/application/src/services/ToolRuntimeSupportService.rs", "installed skill manager assembly"),
    ("core/crates/runtime/application/src/services/core/MessageProcessingDelegate.rs", "construct attachment manager"),
    ("core/crates/runtime/application/src/core/chat/AIMessageManager.rs", "native attachment payloads and resolved file references"),
    ("core/crates/runtime/application/src/core/chat/ChatRuntimeHolder.rs", "construct per-chat native capabilities"),
    ("core/crates/runtime/application/src/services/ChatServiceCore.rs", "native screenshot and attachment payloads; VFS assembly"),
    ("core/crates/runtime/application/src/core/application/OperitApplication.rs", "runtime host assembly and native staging"),
    ("core/crates/runtime/application/src/core/application/MarketInstallSupport.rs", "installed marketplace metadata and native download staging"),
    ("core/crates/runtime/application/src/ui/features/chat/webview/workspace/WorkspaceBackupManager.rs", "VFS assembly for workspace archives"),
    ("core/crates/command/core/src/commands/workspace.rs", "VFS assembly"),
    ("core/crates/command/core/src/commands/market.rs", "market download staging and artifact inspection"),
    ("core/crates/proxy/local/src/lib.rs", "native transport staging"),
    ("apps/flutter/native/operit-flutter-bridge/src/platform_runtime/android.rs", "install Android native filesystem provider"),
    ("apps/flutter/native/operit-flutter-bridge/src/platform_runtime/ios.rs", "install iOS native filesystem provider"),
];

#[derive(Default)]
struct NativeUse {
    found: bool,
}

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
        if ident == "FileSystemHost" || ident == "fileSystemHost" {
            self.found = true;
        }
    }
}

pub fn check_source(path: &str, source: &str) -> Result<(), String> {
    if !source.contains("FileSystemHost") && !source.contains("fileSystemHost") {
        return Ok(());
    }
    let syntax = syn::parse_file(source).map_err(|error| format!("{path}: {error}"))?;
    let mut usage = NativeUse::default();
    usage.visit_file(&syntax);
    if usage.found && !NATIVE_MODULES.iter().any(|(allowed, _)| *allowed == path) {
        return Err(format!("{path}: direct FileSystemHost access is restricted to native filesystem friend modules. Use VisualFileSystem::fromContext and PathMapper for business file access; a native exception requires a reviewed module entry with its purpose in build_support/native_filesystem.rs."));
    }
    Ok(())
}

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
            if name.ends_with("_tests.rs") || name.ends_with("Tests.rs") {
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
            "fn read(context: HostManager) { VisualFileSystem::fromContext(&context); }",
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
            "core/crates/tool/services/src/files/VisualFileSystem.rs",
            "use operit_host_api::FileSystemHost;",
        )
        .unwrap();
    }
}
