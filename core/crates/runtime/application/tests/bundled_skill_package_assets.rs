use std::path::Path;

use operit_runtime::plugins::BuiltinPluginAssets::{
    BUILTIN_PLUGIN_ASSETS, BUNDLED_EXTERNAL_PLUGIN_ASSETS,
};
use operit_runtime::plugins::BundledExternalSkillAssets::{
    BUNDLED_EXTERNAL_SKILL_ASSETS, BUNDLED_EXTERNAL_SKILL_PACKAGE_ASSETS,
};
use operit_tools::runtime_support::RuntimeBundledExternalSkillPackageAsset;
use operit_tools::runtime_support::ToolRuntimeSupport;

/// Initializes one isolated storage host before any test constructs the shared runtime service.
fn runtime_paths() -> &'static operit_store::RuntimeStorePaths::RuntimeStorePaths {
    use operit_host_native_storage::NativeRuntimeStorageHost;
    use operit_store::RuntimeStorageHost::{
        setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost,
    };
    use operit_store::RuntimeStorePaths::RuntimeStorePaths;
    use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
    use std::sync::{Arc, OnceLock};
    static PATHS: OnceLock<RuntimeStorePaths> = OnceLock::new();
    PATHS.get_or_init(|| {
        let root =
            std::env::temp_dir().join(format!("operit-bundled-skill-{}", uuid::Uuid::new_v4()));
        let runtime = root.join("runtime");
        let workspaces = root.join("workspaces");
        std::fs::create_dir_all(&runtime).unwrap();
        setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
            runtime.clone(),
            workspaces.clone(),
        ));
        let storage = Arc::new(NativeRuntimeStorageHost::new(
            runtime.clone(),
            workspaces.clone(),
        ));
        setDefaultRuntimeStorageHost(storage.clone());
        setDefaultRuntimeSqliteHost(storage);
        RuntimeStorePaths::new(runtime, workspaces)
    })
}

/// Creates the shared runtime asset service without initializing or installing any plugins.
fn asset_runtime(
) -> std::sync::Arc<operit_runtime::services::ToolRuntimeSupportService::RuntimeToolSupport> {
    use std::sync::Arc;
    runtime_paths();
    let filesystem = Arc::new(operit_host_native_filesystem::PosixFileSystemHost::new());
    operit_runtime::services::ToolRuntimeSupportService::ToolRuntimeSupportService::create(
        operit_host_api::HostManager::HostManager::new(),
        Arc::new(tokio::sync::Mutex::new(
            operit_runtime::core::chat::ChatRuntimeHolder::ChatRuntimeHolder::new(filesystem),
        )),
    )
}

/// Verifies real skill examples share runtime archives and restore every original source byte.
#[test]
fn package_builder_examples_reuse_archives_without_losing_files() {
    let plugins = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../plugins");
    let mut reused = 0;
    let mut reused_bytes = 0;
    let support = asset_runtime();
    for asset in support.bundledExternalSkillPackageAssets() {
        assert_eq!(asset.skillName, "PackageBuilder");
        assert!(
            BUILTIN_PLUGIN_ASSETS
                .iter()
                .chain(BUNDLED_EXTERNAL_PLUGIN_ASSETS)
                .any(|package| { std::ptr::eq(package.bytes, asset.archive) }),
            "example must reference an existing package allocation: {}",
            asset.path
        );
        assert!(
            !BUNDLED_EXTERNAL_SKILL_ASSETS.iter().any(|inline| {
                inline.skill_name == asset.skillName && inline.path == asset.path
            }),
            "example must not also be embedded inline: {}",
            asset.path
        );
        let actual = asset.readBytes().unwrap();
        let source = plugins.join(asset.path.strip_prefix("examples/").unwrap());
        assert!(
            actual == std::fs::read(&source).unwrap(),
            "example differs: {}",
            source.display()
        );
        reused += 1;
        reused_bytes += actual.len();
    }
    assert!(
        reused > 0,
        "package examples must reuse the shared archives"
    );
    for required in [
        "examples/packages/external/deepseek_dafeiyu/resources/idle.webp",
        "examples/packages/external/deepseek_dafeiyu/resources/talk.webp",
        "examples/packages/buildin/workflow/resources/workflow.html",
    ] {
        assert!(
            BUNDLED_EXTERNAL_SKILL_PACKAGE_ASSETS
                .iter()
                .any(|asset| asset.path == required),
            "runtime resources must be shared with their example: {required}"
        );
    }
    println!(
        "Verified {reused} shared example files, {reused_bytes} source bytes restored exactly"
    );
    for required in [
        "SKILL.md",
        "types/toolpkg.d.ts",
        "references/TOOLPKG_FORMAT_GUIDE.md",
        "examples/packages/buildin/character_cards/package.json",
    ] {
        assert!(
            BUNDLED_EXTERNAL_SKILL_ASSETS
                .iter()
                .any(|asset| { asset.skill_name == "PackageBuilder" && asset.path == required }),
            "source-only skill file must remain available: {required}"
        );
    }
}

/// Rejects corrupt archives and missing members instead of hiding an invalid package reference.
#[test]
fn invalid_shared_package_assets_report_errors() {
    let corrupt = RuntimeBundledExternalSkillPackageAsset {
        skillName: "fixture",
        path: "examples/resource.bin",
        archive: b"invalid archive",
        member: "resource.bin",
    };
    assert!(corrupt.readBytes().is_err());
    let missing = RuntimeBundledExternalSkillPackageAsset {
        archive: BUNDLED_EXTERNAL_PLUGIN_ASSETS.iter()
            .find(|package| package.name == "deepseek_dafeiyu.toolpkg")
            .unwrap().bytes,
        member: "this-member-does-not-exist.bin",
        ..corrupt
    };
    assert!(missing.readBytes().is_err());
}

/// Installs the real skill through the shared CLI/App service without installing any plugin.
#[test]
fn shared_skill_install_restores_complete_examples_without_plugin_installation() {
    use operit_host_native_filesystem::PosixFileSystemHost;
    use operit_tools::tools::skill::SkillManager::SkillManager;
    use std::sync::Arc;

    let paths = runtime_paths().clone();
    let filesystem = Arc::new(PosixFileSystemHost::new());
    let support = asset_runtime();
    let manager = SkillManager::new(paths, filesystem);
    assert!(manager
        .getBundledExternalSkillCandidates(support.as_ref())
        .iter()
        .any(|skill| skill.name == "PackageBuilder"));
    let skill = manager
        .importBundledExternalSkill("PackageBuilder", support.as_ref())
        .unwrap();
    for asset in BUNDLED_EXTERNAL_SKILL_ASSETS
        .iter()
        .filter(|asset| asset.skill_name == "PackageBuilder")
    {
        assert!(std::fs::read(skill.directory.join(asset.path)).unwrap() == asset.bytes);
    }
    for asset in BUNDLED_EXTERNAL_SKILL_PACKAGE_ASSETS
        .iter()
        .filter(|asset| asset.skill_name == "PackageBuilder")
    {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../plugins")
            .join(asset.path.strip_prefix("examples/").unwrap());
        assert!(
            std::fs::read(skill.directory.join(asset.path)).unwrap()
                == std::fs::read(source).unwrap()
        );
    }
    assert!(!manager
        .getBundledExternalSkillCandidates(support.as_ref())
        .iter()
        .any(|skill| skill.name == "PackageBuilder"));
}
