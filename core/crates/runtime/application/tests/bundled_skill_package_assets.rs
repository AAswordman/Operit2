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

    use std::sync::{Arc, OnceLock};
    static PATHS: OnceLock<RuntimeStorePaths> = OnceLock::new();
    PATHS.get_or_init(|| {
        let root =
            std::env::temp_dir().join(format!("operit-bundled-skill-{}", uuid::Uuid::new_v4()));
        let runtime = root.join("runtime");
        let workspaces = root.join("workspaces");
        std::fs::create_dir_all(&runtime).unwrap();

        let storage = Arc::new(NativeRuntimeStorageHost::new(
            runtime.clone(),
            workspaces.clone(),
        ));
        setDefaultRuntimeStorageHost(storage.clone());
        setDefaultRuntimeSqliteHost(storage);
        RuntimeStorePaths::default()
    })
}

/// Creates the shared runtime asset service without initializing or installing any plugins.
fn asset_runtime(
) -> std::sync::Arc<operit_runtime::services::ToolRuntimeSupportService::RuntimeToolSupport> {
    use std::sync::Arc;
    runtime_paths();
    let host = operit_host_api::HostManager::HostManager::withFileSystemHost(Arc::new(
        operit_host_native_filesystem::PosixFileSystemHost::new(),
    ))
    .withRuntimeStorageHost(operit_store::RuntimeStorageHost::defaultRuntimeStorageHost());
    let filesystem = Arc::new(host.vfs().unwrap());
    operit_runtime::services::ToolRuntimeSupportService::ToolRuntimeSupportService::create(
        host,
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
            BUNDLED_EXTERNAL_SKILL_ASSETS.iter().any(|asset| {
                asset.skill_name == "PackageBuilder" && asset.path == required
            }),
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
        archive: BUNDLED_EXTERNAL_PLUGIN_ASSETS
            .iter()
            .find(|package| package.name == "deepseek_dafeiyu.toolpkg")
            .unwrap()
            .bytes,
        member: "this-member-does-not-exist.bin",
        ..corrupt
    };
    assert!(missing.readBytes().is_err());
}

/// Installs the built-in skill hidden by default, restores its files, and preserves user visibility.
#[test]
fn shared_skill_install_restores_complete_examples_without_plugin_installation() {
    use operit_host_native_filesystem::PosixFileSystemHost;
    use operit_tools::tools::skill::SkillManager::SkillManager;
    use std::sync::Arc;

    let paths = runtime_paths().clone();
    let host = operit_host_api::HostManager::HostManager::withFileSystemHost(Arc::new(
        PosixFileSystemHost::new(),
    ))
    .withRuntimeStorageHost(operit_store::RuntimeStorageHost::defaultRuntimeStorageHost());
    let filesystem = Arc::new(host.vfs().unwrap());
    let support = asset_runtime();
    let manager = SkillManager::new(paths, filesystem.clone());
    assert_eq!(support.builtinSkillNames(), &["PackageBuilder"]);
    assert!(!manager
        .getBundledExternalSkillCandidates(support.as_ref())
        .iter()
        .any(|skill| skill.name == "PackageBuilder"));
    let repository =
        operit_tools::tools::skill_runtime::SkillRepository::SkillRepository::getInstance(
            &host,
            support.clone(),
        );
    let (mut skills, errors) = repository.getAvailableSkillPackagesSnapshot();
    assert!(errors.is_empty(), "{errors:?}");
    let skill = skills.remove("PackageBuilder").unwrap();
    assert_eq!(skill.name, "PackageBuilder");
    assert_eq!(skill.directory.file_name().unwrap(), "PackageBuilder");
    assert_eq!(repository.readSkillDisplayName(&skill).unwrap(), "Operit 插件开发套件");
    let record = operit_store::ExtensionStore::ExtensionStore::default()
        .record("skill", &skill.name)
        .unwrap();
    assert_eq!(record.settings["builtin"], true);
    assert!(!support.isSkillVisibleToAi(&skill.name));
    for asset in BUNDLED_EXTERNAL_SKILL_ASSETS
        .iter()
        .filter(|asset| asset.skill_name == "PackageBuilder")
    {
        assert!(
            filesystem
                .readFileBytes(&skill.directory.join(asset.path).to_string_lossy())
                .unwrap()
                == asset.bytes
        );
    }
    for asset in BUNDLED_EXTERNAL_SKILL_PACKAGE_ASSETS
        .iter()
        .filter(|asset| asset.skill_name == "PackageBuilder")
    {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../plugins")
            .join(asset.path.strip_prefix("examples/").unwrap());
        assert!(
            filesystem
                .readFileBytes(&skill.directory.join(asset.path).to_string_lossy())
                .unwrap()
                == std::fs::read(source).unwrap()
        );
    }
    assert!(!manager
        .getBundledExternalSkillCandidates(support.as_ref())
        .iter()
        .any(|skill| skill.name == "PackageBuilder"));
    support.setSkillVisibleToAi(&skill.name, true).unwrap();
    assert!(manager.ensureBuiltinSkills(support.as_ref()).is_empty());
    assert!(support.isSkillVisibleToAi(&skill.name));
    support.setSkillVisibleToAi(&skill.name, false).unwrap();
    assert!(manager.ensureBuiltinSkills(support.as_ref()).is_empty());
    assert!(!support.isSkillVisibleToAi(&skill.name));
    assert_eq!(repository.bundledSkillNeedsUpdate(&skill).unwrap(), Some(false));

    // Legacy installations have the stable name but no builtin/source metadata or version.
    let legacy = "---\nname: PackageBuilder\ndescription: Legacy package builder\n---\n# PackageBuilder\n";
    filesystem.writeFile(&skill.skillFile.to_string_lossy(), legacy, false).unwrap();
    let store = operit_store::ExtensionStore::ExtensionStore::default();
    store.setSettings("skill", &skill.name, serde_json::json!({"visible": true})).unwrap();
    assert!(manager.ensureBuiltinSkills(support.as_ref()).is_empty());
    assert_eq!(repository.readSkillDisplayName(&skill).unwrap(), "PackageBuilder");
    assert_eq!(repository.bundledSkillNeedsUpdate(&skill).unwrap(), Some(true));
    let stale = skill.directory.join("obsolete-reference.md");
    filesystem.writeFile(&stale.to_string_lossy(), "old instructions", false).unwrap();
    let updated = repository.importBundledExternalSkill(&skill.name).unwrap();
    assert_eq!(updated.directory, skill.directory);
    assert_eq!(repository.readSkillDisplayName(&updated).unwrap(), "Operit 插件开发套件");
    assert!(repository.isSkillVisibleToAi(&skill.name));
    assert!(!filesystem.fileExists(&stale.to_string_lossy()).unwrap().exists);
    assert_eq!(repository.bundledSkillNeedsUpdate(&updated).unwrap(), Some(false));

    // Type definitions and compressed example resources are independently updateable.
    let types = updated.directory.join("types/toolpkg.d.ts");
    filesystem.writeFile(&types.to_string_lossy(), "old declarations", false).unwrap();
    assert_eq!(repository.bundledSkillNeedsUpdate(&updated).unwrap(), Some(true));
    repository.setSkillVisibleToAi(&skill.name, false).unwrap();
    repository.importBundledExternalSkill(&skill.name).unwrap();
    assert!(!repository.isSkillVisibleToAi(&skill.name));
    assert_eq!(repository.bundledSkillNeedsUpdate(&updated).unwrap(), Some(false));
    let resource = BUNDLED_EXTERNAL_SKILL_PACKAGE_ASSETS.iter()
        .find(|asset| asset.skill_name == skill.name).unwrap();
    filesystem.deleteFile(&updated.directory.join(resource.path).to_string_lossy(), false).unwrap();
    assert_eq!(repository.bundledSkillNeedsUpdate(&updated).unwrap(), Some(true));
    repository.importBundledExternalSkill(&skill.name).unwrap();
    assert_eq!(repository.bundledSkillNeedsUpdate(&updated).unwrap(), Some(false));

    // Explicit updates preserve space ownership as well as visibility.
    store.moveScope("skill", &skill.name, "space").unwrap();
    let shared = repository.getAvailableSkillPackages().remove(&skill.name).unwrap();
    filesystem.writeFile(&shared.skillFile.to_string_lossy(), legacy, false).unwrap();
    let updated = repository.importBundledExternalSkill(&skill.name).unwrap();
    assert_eq!(updated.directory, shared.directory);
    assert_eq!(store.record("skill", &skill.name).unwrap().scope, "space");
    assert!(!repository.isSkillVisibleToAi(&skill.name));
    assert_eq!(repository.bundledSkillNeedsUpdate(&updated).unwrap(), Some(false));
}
