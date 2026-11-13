use operit_runtime::plugins::BundledExternalSkillAssets::BUNDLED_EXTERNAL_SKILL_ASSETS;
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

/// Keeps authoring resources small and points discovery at both live installation scopes.
#[test]
fn package_builder_bundles_docs_and_types_with_live_vfs_paths() {
    let assets = BUNDLED_EXTERNAL_SKILL_ASSETS
        .iter()
        .filter(|asset| asset.skill_name == "PackageBuilder")
        .collect::<Vec<_>>();
    for required in [
        "SKILL.md",
        "types/toolpkg.d.ts",
        "references/PLUGIN_CREATION_WORKFLOW.md",
        "references/TOOLPKG_FORMAT_GUIDE.md",
    ] {
        assert!(
            assets.iter().any(|asset| asset.path == required),
            "missing {required}"
        );
    }
    assert!(assets.iter().all(|asset| asset.path == "SKILL.md"
        || asset.path.starts_with("types/")
        || asset.path.starts_with("references/")));
    let skill = std::str::from_utf8(
        assets
            .iter()
            .find(|asset| asset.path == "SKILL.md")
            .unwrap()
            .bytes,
    )
    .unwrap();
    let paths = operit_store::RuntimeStorePaths::RuntimeStorePaths::default();
    for scope in ["device", "space"] {
        let key = operit_store::ExtensionStore::ExtensionStore::root("package", scope).unwrap();
        assert!(skill.contains(
            &paths
                .runtime_storage_path(&key)
                .to_string_lossy()
                .to_string()
        ));
    }
    assert!(skill.contains(&paths.toolpkg_cache_dir().to_string_lossy().to_string()));
}

/// Installs only authoring resources and removes legacy examples on explicit updates.
#[test]
fn lightweight_skill_install_and_update_preserve_settings_and_installed_plugins() {
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
    assert_eq!(
        repository.readSkillDisplayName(&skill).unwrap(),
        "Operit 插件开发套件"
    );
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
    assert!(
        !filesystem
            .fileExists(&skill.directory.join("examples").to_string_lossy())
            .unwrap()
            .exists
    );
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
    assert_eq!(
        repository.bundledSkillNeedsUpdate(&skill).unwrap(),
        Some(false)
    );

    // Legacy installations have the stable name but no builtin/source metadata or version.
    let legacy =
        "---\nname: PackageBuilder\ndescription: Legacy package builder\n---\n# PackageBuilder\n";
    filesystem
        .writeFile(&skill.skillFile.to_string_lossy(), legacy, false)
        .unwrap();
    let store = operit_store::ExtensionStore::ExtensionStore::default();
    store
        .setSettings("skill", &skill.name, serde_json::json!({"visible": true}))
        .unwrap();
    assert!(manager.ensureBuiltinSkills(support.as_ref()).is_empty());
    assert_eq!(
        repository.readSkillDisplayName(&skill).unwrap(),
        "PackageBuilder"
    );
    assert_eq!(
        repository.bundledSkillNeedsUpdate(&skill).unwrap(),
        Some(true)
    );
    let stale = skill
        .directory
        .join("examples/packages/external/old-plugin/resource.bin");
    filesystem
        .writeFile(&stale.to_string_lossy(), "old instructions", false)
        .unwrap();
    let installed = skill
        .directory
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("packages/user-plugin.toolpkg");
    filesystem
        .writeFileBytes(&installed.to_string_lossy(), b"installed plugin bytes")
        .unwrap();
    let updated = repository.importBundledExternalSkill(&skill.name).unwrap();
    assert_eq!(updated.directory, skill.directory);
    assert_eq!(
        repository.readSkillDisplayName(&updated).unwrap(),
        "Operit 插件开发套件"
    );
    assert!(repository.isSkillVisibleToAi(&skill.name));
    assert!(
        !filesystem
            .fileExists(&updated.directory.join("examples").to_string_lossy())
            .unwrap()
            .exists
    );
    assert_eq!(
        filesystem
            .readFileBytes(&installed.to_string_lossy())
            .unwrap(),
        b"installed plugin bytes"
    );
    assert_eq!(
        repository.bundledSkillNeedsUpdate(&updated).unwrap(),
        Some(false)
    );

    // Updates still detect changed type definitions and missing documentation.
    let types = updated.directory.join("types/toolpkg.d.ts");
    filesystem
        .writeFile(&types.to_string_lossy(), "old declarations", false)
        .unwrap();
    assert_eq!(
        repository.bundledSkillNeedsUpdate(&updated).unwrap(),
        Some(true)
    );
    repository.setSkillVisibleToAi(&skill.name, false).unwrap();
    repository.importBundledExternalSkill(&skill.name).unwrap();
    assert!(!repository.isSkillVisibleToAi(&skill.name));
    assert_eq!(
        repository.bundledSkillNeedsUpdate(&updated).unwrap(),
        Some(false)
    );
    filesystem
        .deleteFile(
            &updated
                .directory
                .join("references/PLUGIN_CREATION_WORKFLOW.md")
                .to_string_lossy(),
            false,
        )
        .unwrap();
    assert_eq!(
        repository.bundledSkillNeedsUpdate(&updated).unwrap(),
        Some(true)
    );
    repository.importBundledExternalSkill(&skill.name).unwrap();
    assert_eq!(
        repository.bundledSkillNeedsUpdate(&updated).unwrap(),
        Some(false)
    );

    // Explicit updates preserve space ownership as well as visibility.
    store.moveScope("skill", &skill.name, "space").unwrap();
    let shared = repository
        .getAvailableSkillPackages()
        .remove(&skill.name)
        .unwrap();
    filesystem
        .writeFile(&shared.skillFile.to_string_lossy(), legacy, false)
        .unwrap();
    let updated = repository.importBundledExternalSkill(&skill.name).unwrap();
    assert_eq!(updated.directory, shared.directory);
    assert_eq!(store.record("skill", &skill.name).unwrap().scope, "space");
    assert!(!repository.isSkillVisibleToAi(&skill.name));
    assert_eq!(
        repository.bundledSkillNeedsUpdate(&updated).unwrap(),
        Some(false)
    );
}
