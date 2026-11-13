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
    );
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
