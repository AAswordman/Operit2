#![cfg(not(target_arch = "wasm32"))]

use operit_host_api::HostManager::HostManager;
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_tools::tools::skill::SkillManager::SkillManager;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

static STORAGE_LOCK: Mutex<()> = Mutex::new(());

struct Fixture {
    manager: SkillManager,
    root: PathBuf,
    _guard: MutexGuard<'static, ()>,
}

impl Fixture {
    fn new() -> Self {
        let guard = STORAGE_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let root =
            std::env::temp_dir().join(format!("operit-skill-import-{}", uuid::Uuid::new_v4()));
        let storage = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        setDefaultRuntimeStorageHost(storage.clone());
        setDefaultRuntimeSqliteHost(storage.clone());
        let context = HostManager {
            fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
            runtimeStorageHost: Some(storage),
            ..Default::default()
        };
        let manager = SkillManager::fromDefaultPaths(Arc::new(context.vfs().unwrap()));
        Self {
            manager,
            root,
            _guard: guard,
        }
    }

    fn import(&self, entries: &[(&str, &str)], subdir: Option<&str>) -> String {
        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (path, content) in entries {
            archive
                .start_file(*path, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(content.as_bytes()).unwrap();
        }
        self.manager.importSkillArchiveBytes(
            &archive.finish().unwrap().into_inner(),
            Path::new("bundle.zip"),
            subdir,
        )
    }

    fn installed_file(&self, path: &str) -> PathBuf {
        self.root
            .join("runtime/extensions/device/skills")
            .join(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Skips even an unreadable local duplicate and never substitutes it for missing shared content.
#[test]
fn shared_skill_shadows_local_content_before_parsing() {
    use operit_store::ExtensionStore::{recordPath, ExtensionStore};
    let fixture = Fixture::new();
    assert_eq!(fixture.import(&[("demo/SKILL.md", "---\nname: demo\n---\n# Local")], None),
        "Imported skill: demo");
    assert!(fixture.manager.getAvailableSkills().contains_key("demo"));
    let store = ExtensionStore::default();
    let mut record = store.record("skill", "demo").unwrap();
    record.scope = "space".into();
    let shared = fixture.root.join("runtime/extensions/space/skills/demo");
    std::fs::create_dir_all(&shared).unwrap();
    std::fs::write(shared.join("SKILL.md"), "---\nname: demo\n---\n# Shared").unwrap();
    let descriptor = fixture.root.join(recordPath("skill", "demo", "space").unwrap());
    std::fs::create_dir_all(descriptor.parent().unwrap()).unwrap();
    std::fs::write(descriptor, serde_json::to_vec(&record).unwrap()).unwrap();
    std::fs::write(fixture.installed_file("demo/SKILL.md"), [0xff]).unwrap();
    let (skills, errors) = fixture.manager.refreshAvailableSkills();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(skills.len(), 1);
    assert_eq!(fixture.manager.readSkillContent("demo").unwrap(), "---\nname: demo\n---\n# Shared");
    std::fs::remove_file(shared.join("SKILL.md")).unwrap();
    let (skills, errors) = fixture.manager.refreshAvailableSkills();
    assert!(!skills.contains_key("demo"));
    assert!(!errors.is_empty());
    assert_eq!(std::fs::read(fixture.installed_file("demo/SKILL.md")).unwrap(), [0xff]);
}

#[test]
fn imports_all_packages_in_a_wrapped_archive_without_a_ten_skill_limit() {
    let fixture = Fixture::new();
    let mut files = Vec::new();
    for index in 0..12 {
        files.push((
            format!("repo-main/skills/folder-{index}/SKILL.md"),
            format!("---\nname: skill-{index}\ndescription: Skill {index}\n---\n# Instructions"),
        ));
        files.push((
            format!("repo-main/skills/folder-{index}/scripts/run.sh"),
            "echo hello".into(),
        ));
    }
    let entries = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect::<Vec<_>>();
    let result = fixture.import(&entries, None);
    assert!(
        result.starts_with("Imported 12 skills; failed: 0"),
        "{result}"
    );
    let skills = fixture.manager.getAvailableSkills();
    assert_eq!(skills.len(), 12);
    for index in 0..12 {
        assert!(skills.contains_key(&format!("skill-{index}")));
        assert_eq!(
            std::fs::read_to_string(
                fixture.installed_file(&format!("skill-{index}/scripts/run.sh"))
            )
            .unwrap(),
            "echo hello"
        );
    }
    assert!(!std::fs::read_dir(fixture.installed_file(""))
        .unwrap()
        .any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".import_tmp_")));
}

#[test]
fn imports_sibling_folders_and_lowercase_skill_files() {
    let fixture = Fixture::new();
    let result = fixture.import(
        &[
            ("alpha/SKILL.md", "# Alpha"),
            ("category/beta/skill.md", "---\nname: beta\n---\n# Beta"),
            ("__MACOSX/ghost/SKILL.md", "# Finder metadata"),
        ],
        None,
    );
    assert!(
        result.starts_with("Imported 2 skills; failed: 0"),
        "{result}"
    );
    let skills = fixture.manager.getAvailableSkills();
    assert_eq!(
        skills.keys().map(String::as_str).collect::<Vec<_>>(),
        ["alpha", "beta"]
    );
}

#[test]
fn keeps_single_skill_imports_and_nested_example_files_together() {
    let fixture = Fixture::new();
    let result = fixture.import(
        &[
            ("SKILL.md", "# Instructions"),
            (
                "references/example/SKILL.md",
                "---\nname: example\n---\n# Example",
            ),
        ],
        None,
    );
    assert_eq!(result, "Imported skill: bundle");
    assert_eq!(fixture.manager.getAvailableSkills().len(), 1);
    assert!(fixture
        .installed_file("bundle/references/example/SKILL.md")
        .is_file());
}

#[test]
fn imports_all_skills_only_within_the_selected_subdirectory() {
    let fixture = Fixture::new();
    let result = fixture.import(
        &[
            ("repo-main/selected/alpha/SKILL.md", "# Alpha"),
            ("repo-main/selected/beta/SKILL.md", "# Beta"),
            ("repo-main/outside/SKILL.md", "# Outside"),
        ],
        Some("selected"),
    );
    assert!(
        result.starts_with("Imported 2 skills; failed: 0"),
        "{result}"
    );
    let skills = fixture.manager.getAvailableSkills();
    assert_eq!(
        skills.keys().map(String::as_str).collect::<Vec<_>>(),
        ["alpha", "beta"]
    );
}

#[test]
fn reports_conflicts_and_invalid_names_while_importing_other_skills() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.import(&[("existing/SKILL.md", "# Original")], None),
        "Imported skill: existing"
    );
    let result = fixture.import(
        &[
            (
                "a-existing/SKILL.md",
                "---\nname: existing\n---\n# Replacement",
            ),
            ("b-invalid/SKILL.md", "---\nname: ../escape\n---\n# Invalid"),
            ("c-valid/SKILL.md", "---\nname: valid\n---\n# Valid"),
            ("d-duplicate/SKILL.md", "---\nname: valid\n---\n# Duplicate"),
        ],
        None,
    ).replace('\\', "/");
    assert!(
        result.starts_with("Imported 1 skills; failed: 3"),
        "{result}"
    );
    assert!(
        result.contains("a-existing/SKILL.md: Skill already exists"),
        "{result}"
    );
    assert!(result.contains("Invalid skill name: ../escape"), "{result}");
    assert!(
        result.contains("d-duplicate/SKILL.md: Skill 'valid' already exists"),
        "{result}"
    );
    assert_eq!(fixture.manager.getAvailableSkills().len(), 2);
    assert_eq!(
        std::fs::read_to_string(fixture.installed_file("existing/SKILL.md")).unwrap(),
        "# Original"
    );
    assert!(!fixture.installed_file("../escape").exists());
}

#[test]
fn returns_an_error_when_no_skills_are_found() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.import(&[("README.md", "No skills here")], None),
        "No SKILL.md found in the imported zip"
    );
    assert!(fixture.manager.getAvailableSkills().is_empty());
}

#[test]
fn imports_named_flat_archive_without_registering_staging_content() {
    use operit_store::ExtensionStore::ExtensionStore;
    let fixture = Fixture::new();
    let content = "---\nname: neat-freak\ndescription: Knowledge closeout\n---\n# Instructions";
    let result = fixture.import(
        &[
            ("SKILL.md", content),
            ("references/guide.md", "# Guide"),
            ("scripts/audit.sh", "echo audit"),
            (
                "evals/example/SKILL.md",
                "---\nname: example\n---\n# Example",
            ),
        ],
        None,
    );
    assert_eq!(result, "Imported skill: neat-freak - Knowledge closeout");
    let (skills, errors) = fixture.manager.getAvailableSkillsSnapshot();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(skills.len(), 1);
    assert_eq!(
        fixture.manager.readSkillContent("neat-freak").unwrap(),
        content
    );
    assert!(fixture
        .installed_file("neat-freak/references/guide.md")
        .is_file());
    assert!(fixture
        .installed_file("neat-freak/scripts/audit.sh")
        .is_file());
    assert!(fixture
        .installed_file("neat-freak/evals/example/SKILL.md")
        .is_file());
    let records = ExtensionStore::default().records("skill").unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].sourceName, "neat-freak");
    assert_eq!(
        std::fs::read_dir(fixture.installed_file(""))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn ignores_legacy_import_directories_even_when_they_contain_named_skills() {
    use operit_store::ExtensionStore::ExtensionStore;
    let fixture = Fixture::new();
    let temporary = fixture.installed_file(".import_tmp_123");
    std::fs::create_dir_all(&temporary).unwrap();
    std::fs::write(
        temporary.join("SKILL.md"),
        "---\nname: ghost\n---\n# Staging",
    )
    .unwrap();
    let (skills, errors) = fixture.manager.getAvailableSkillsSnapshot();
    assert!(skills.is_empty());
    assert!(errors.is_empty(), "{errors:?}");
    assert!(ExtensionStore::default()
        .records("skill")
        .unwrap()
        .is_empty());
    assert!(temporary.join("SKILL.md").is_file());
}

#[test]
fn recovers_copied_skill_from_missing_temporary_owner_and_preserves_settings() {
    use operit_store::ExtensionStore::ExtensionStore;
    let fixture = Fixture::new();
    let store = ExtensionStore::default();
    let settings = serde_json::json!({"visible": false, "custom": "preserved"});
    store
        .registerDevice("skill", "demo", ".import_tmp_123", settings.clone())
        .unwrap();
    std::fs::create_dir_all(fixture.installed_file("demo")).unwrap();
    std::fs::write(
        fixture.installed_file("demo/SKILL.md"),
        "---\nname: demo\n---\n# Recovered",
    )
    .unwrap();
    for _ in 0..2 {
        let (skills, errors) = fixture.manager.getAvailableSkillsSnapshot();
        assert!(errors.is_empty(), "{errors:?}");
        assert!(skills.contains_key("demo"));
        let record = store.record("skill", "demo").unwrap();
        assert_eq!(record.sourceName, "demo");
        assert_eq!(record.settings, settings);
        assert_eq!(record.scope, "device");
    }
    assert_eq!(
        fixture.manager.readSkillContent("demo").unwrap(),
        "---\nname: demo\n---\n# Recovered"
    );
}

#[test]
fn retries_import_after_removing_only_the_missing_temporary_registration() {
    use operit_store::ExtensionStore::ExtensionStore;
    let fixture = Fixture::new();
    let store = ExtensionStore::default();
    store
        .registerDevice(
            "skill",
            "demo",
            ".import_tmp_123",
            serde_json::json!({"visible": true}),
        )
        .unwrap();
    assert!(fixture.manager.getAvailableSkills().is_empty());
    assert!(store.records("skill").unwrap().is_empty());
    assert_eq!(
        fixture.import(&[("SKILL.md", "---\nname: demo\n---\n# Retried")], None),
        "Imported skill: demo"
    );
    assert_eq!(store.record("skill", "demo").unwrap().sourceName, "demo");
    assert!(fixture.manager.getAvailableSkills().contains_key("demo"));
    assert!(fixture
        .import(&[("SKILL.md", "---\nname: demo\n---\n# Replacement")], None)
        .contains("already exists"));
    assert_eq!(
        fixture.manager.readSkillContent("demo").unwrap(),
        "---\nname: demo\n---\n# Retried"
    );
}

#[test]
fn preserves_unrelated_broken_registration_and_rejects_replacement() {
    use operit_store::ExtensionStore::ExtensionStore;
    let fixture = Fixture::new();
    let store = ExtensionStore::default();
    store
        .registerDevice(
            "skill",
            "demo",
            "custom-location",
            serde_json::json!({"visible": false}),
        )
        .unwrap();
    let result = fixture.import(
        &[("demo/SKILL.md", "---\nname: demo\n---\n# Replacement")],
        None,
    );
    assert!(result.contains("already exists"), "{result}");
    assert!(!fixture.installed_file("demo").exists());
    let record = store.record("skill", "demo").unwrap();
    assert_eq!(record.sourceName, "custom-location");
    assert_eq!(record.settings["visible"], false);
}

#[test]
fn does_not_recover_a_temporary_registration_while_its_source_still_exists() {
    use operit_store::ExtensionStore::ExtensionStore;
    let fixture = Fixture::new();
    let store = ExtensionStore::default();
    let temporary = fixture.installed_file(".import_tmp_123");
    std::fs::create_dir_all(&temporary).unwrap();
    std::fs::write(
        temporary.join("SKILL.md"),
        "---\nname: demo\n---\n# Staging",
    )
    .unwrap();
    store
        .registerDevice(
            "skill",
            "demo",
            ".import_tmp_123",
            serde_json::json!({"visible": false}),
        )
        .unwrap();
    let (skills, errors) = fixture.manager.getAvailableSkillsSnapshot();
    assert!(skills.is_empty());
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        store.record("skill", "demo").unwrap().sourceName,
        ".import_tmp_123"
    );
    assert!(temporary.join("SKILL.md").is_file());
}

#[test]
fn rolls_back_copied_content_when_registration_fails_and_allows_retry() {
    use operit_store::ExtensionStore::ExtensionStore;
    use operit_util::RuntimeStorageLayout::SKILL_VISIBILITY_PREFERENCES_PATH;
    let fixture = Fixture::new();
    // A directory at the legacy preferences path makes registration fail after publication.
    let blocked = fixture.root.join(SKILL_VISIBILITY_PREFERENCES_PATH);
    std::fs::create_dir_all(&blocked).unwrap();
    let entries = [("SKILL.md", "---\nname: demo\n---\n# Instructions")];
    let result = fixture.import(&entries, None);
    assert!(result.starts_with("Failed to import skill:"), "{result}");
    assert!(!fixture.installed_file("demo").exists());
    assert!(ExtensionStore::default()
        .records("skill")
        .unwrap()
        .is_empty());
    std::fs::remove_dir(&blocked).unwrap();
    assert_eq!(fixture.import(&entries, None), "Imported skill: demo");
    assert!(fixture.manager.getAvailableSkills().contains_key("demo"));
}

#[test]
fn rejects_reserved_temporary_skill_names_without_installing_or_registering_them() {
    use operit_store::ExtensionStore::ExtensionStore;
    let fixture = Fixture::new();
    let result = fixture.import(
        &[(
            "SKILL.md",
            "---\nname: .import_tmp_123\n---\n# Instructions",
        )],
        None,
    );
    assert_eq!(result, "Invalid skill name: .import_tmp_123");
    assert!(ExtensionStore::default()
        .records("skill")
        .unwrap()
        .is_empty());
    assert_eq!(
        std::fs::read_dir(fixture.installed_file(""))
            .unwrap()
            .count(),
        0
    );
}
