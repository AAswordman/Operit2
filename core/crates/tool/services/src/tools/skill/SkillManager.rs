use operit_vfs::VisualFileSystem::VisualFileSystem;
use operit_store::ExtensionStore::ExtensionStore;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use operit_host_api::{TimeUtils::currentLocalDateTime};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use serde::{Deserialize, Serialize};

use crate::runtime_support::ToolRuntimeSupport;
use crate::tools::skill::SkillPackage::SkillPackage;

const QUICK_PLUGIN_CREATOR_SKILL_NAME: &str = "PackageBuilder";

#[derive(Clone)]
pub struct SkillManager {
    paths: RuntimeStorePaths,
    filesystem: Arc<VisualFileSystem>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundledExternalSkillCandidate {
    pub name: String,
    pub description: String,
}

impl SkillManager {
    /// Creates a skill manager using the default runtime store paths.
    #[allow(non_snake_case)]
    pub fn fromDefaultPaths(filesystem: Arc<VisualFileSystem>) -> Self {
        Self::new(RuntimeStorePaths::default(), filesystem)
    }

    /// Creates a skill manager rooted at explicit runtime store paths.
    pub fn new(paths: RuntimeStorePaths, filesystem: Arc<VisualFileSystem>) -> Self {
        Self {
            paths,
            filesystem,
        }
    }

    /// Returns the directory where user-installed skills are stored.
    #[allow(non_snake_case)]
    pub fn getSkillsDirectoryPath(&self) -> String {
        let skillsDir = self.getSkillsRootDir();
        skillsDir.to_string_lossy().replace('\\', "/")
    }

    /// Scans both locations using the same skill parser and rejects conflicting names explicitly.
    #[allow(non_snake_case)]
    pub fn refreshAvailableSkills(
        &self,
    ) -> (BTreeMap<String, SkillPackage>, BTreeMap<String, String>) {
        let mut skills = BTreeMap::new();
        let mut errors = BTreeMap::new();
        let mut conflicts = BTreeSet::new();
        for scope in ["device", "space"] {
            let root =
                ExtensionStore::root("skill", scope).expect("Skill scope constants are valid");
            let directory = self.paths.runtime_storage_path(&root);
            let (scanned, scanErrors) = self.scanSkillsDirectory(directory);
            errors.extend(scanErrors);
            for (name, skill) in scanned {
                if skills.contains_key(&name) || conflicts.contains(&name) {
                    skills.remove(&name);
                    conflicts.insert(name.clone());
                    errors.insert(
                        name.clone(),
                        format!("Skill ownership conflict: {name} exists in both locations"),
                    );
                    continue;
                }
                let registration = (|| -> Result<(), String> {
                    let source = skill
                        .directory
                        .file_name()
                        .ok_or("Skill directory has no name")?
                        .to_string_lossy();
                    if scope == "device" {
                        ExtensionStore::default().registerDevice(
                            "skill",
                            &name,
                            &source,
                            serde_json::json!({"visible": true}),
                        )?;
                    }
                    let record = ExtensionStore::default().record("skill", &name)?;
                    if record.scope != scope || record.sourceName != source {
                        return Err(
                            "Skill content does not match its registered location".to_string()
                        );
                    }
                    Ok(())
                })();
                match registration {
                    Ok(()) => {
                        skills.insert(name, skill);
                    }
                    Err(error) => {
                        errors.insert(name, error);
                    }
                }
            }
        }
        (skills, errors)
    }

    /// Scans the skills directory and returns loaded packages plus load errors.
    #[allow(non_snake_case)]
    fn scanSkillsDirectory(
        &self,
        skillsDir: PathBuf,
    ) -> (BTreeMap<String, SkillPackage>, BTreeMap<String, String>) {
        let mut availableSkills = BTreeMap::new();
        let mut skillLoadErrors = BTreeMap::new();
        // Files and the installation descriptor can arrive in different sync batches.
        // Only published shared installations are visible; leftover empty directories are not skills.
        let sharedSources = if skillsDir == self.paths.runtime_storage_path(
            &ExtensionStore::root("skill", "space").expect("valid skill scope"),
        ) {
            match ExtensionStore::default().records("skill") {
                Ok(records) => Some(records.into_iter().filter(|record| record.scope == "space")
                    .map(|record| record.sourceName).collect::<BTreeSet<_>>()),
                Err(error) => { skillLoadErrors.insert("skills".to_string(), error); return (availableSkills, skillLoadErrors); }
            }
        } else { None };
        let skillsPath = hostPath(&skillsDir);

        if let Err(error) = self.filesystem.makeDirectory(&skillsPath, true) {
            skillLoadErrors.insert(
                "skills".to_string(),
                format!("Cannot access skills directory: {}", error),
            );
            return (availableSkills, skillLoadErrors);
        }

        let children = match self.filesystem.listFiles(&skillsPath) {
            Ok(children) => children,
            Err(error) => {
                skillLoadErrors.insert(
                    "skills".to_string(),
                    format!("Cannot read skills directory: {}", error),
                );
                return (availableSkills, skillLoadErrors);
            }
        };

        for child in children {
            if !child.isDirectory {
                continue;
            }
            let childName = child.name;
            if sharedSources.as_ref().is_some_and(|sources| !sources.contains(&childName)) { continue; }
            let childPath = skillsDir.join(&childName);
            let primarySkillFile = childPath.join("SKILL.md");
            let lowerSkillFile = childPath.join("skill.md");
            let primaryInfo = match self.filesystem.fileExists(&hostPath(&primarySkillFile)) {
                Ok(info) => info,
                Err(error) => {
                    skillLoadErrors.insert(childName.clone(), error.to_string());
                    continue;
                }
            };
            let skillFile = if primaryInfo.exists && !primaryInfo.isDirectory {
                primarySkillFile
            } else {
                lowerSkillFile
            };

            let skillInfo = match self.filesystem.fileExists(&hostPath(&skillFile)) {
                Ok(info) => info,
                Err(error) => {
                    skillLoadErrors.insert(childName.clone(), error.to_string());
                    continue;
                }
            };
            if !skillInfo.exists || skillInfo.isDirectory {
                skillLoadErrors.insert(
                    childName.clone(),
                    format!("Missing SKILL.md in {}", childPath.to_string_lossy()),
                );
                continue;
            }

            match parseSkillMetadata(self.filesystem.as_ref(), &skillFile) {
                Ok((name, description)) => {
                    let skillName = if name.trim().is_empty() {
                        childName.clone()
                    } else {
                        name
                    };
                    if availableSkills.contains_key(&skillName) {
                        let existingDirName = match availableSkills.get(&skillName) {
                            Some(skill) => match skill.directory.file_name() {
                                Some(name) => name.to_string_lossy().replace('\\', "/"),
                                None => skillName.clone(),
                            },
                            None => skillName.clone(),
                        };
                        skillLoadErrors.insert(
                            childName.clone(),
                            format!(
                                "Duplicate scanned skill name: {} already loaded from {}",
                                skillName, existingDirName
                            ),
                        );
                        continue;
                    }

                    availableSkills.insert(
                        skillName.clone(),
                        SkillPackage {
                            name: skillName,
                            description,
                            directory: childPath,
                            skillFile,
                        },
                    );
                }
                Err(error) => {
                    skillLoadErrors.insert(childName, format!("Failed to scan skill: {}", error));
                }
            }
        }

        (availableSkills, skillLoadErrors)
    }

    /// Reads the optional display label while preserving the skill's stable identity.
    #[allow(non_snake_case)]
    pub fn readSkillDisplayName(&self, skill: &SkillPackage) -> Result<String, String> {
        let content = self.filesystem.readFile(&hostPath(&skill.skillFile))?;
        let mut lines = content.lines();
        if lines.next().map(str::trim) == Some("---") {
            for line in lines.take_while(|line| line.trim() != "---") {
                if let Some(("display_name", value)) = line.trim().split_once(':') {
                    let label = unquote(value);
                    if !label.is_empty() {
                        return Ok(label);
                    }
                }
            }
        }
        Ok(skill.name.clone())
    }

    /// Returns all valid skill packages currently installed.
    #[allow(non_snake_case)]
    pub fn getAvailableSkills(&self) -> BTreeMap<String, SkillPackage> {
        self.refreshAvailableSkills().0
    }

    /// Returns a scan snapshot containing valid skill packages and load errors.
    #[allow(non_snake_case)]
    pub fn getAvailableSkillsSnapshot(
        &self,
    ) -> (BTreeMap<String, SkillPackage>, BTreeMap<String, String>) {
        self.refreshAvailableSkills()
    }

    /// Returns load errors from scanning installed skill directories.
    #[allow(non_snake_case)]
    pub fn getSkillLoadErrors(&self) -> BTreeMap<String, String> {
        self.refreshAvailableSkills().1
    }

    /// Installs missing built-in skills as AI-hidden and preserves existing installations and switches.
    #[allow(non_snake_case)]
    pub fn ensureBuiltinSkills(
        &self,
        runtimeSupport: &dyn ToolRuntimeSupport,
    ) -> BTreeMap<String, String> {
        let (installed, mut errors) = self.refreshAvailableSkills();
        for name in runtimeSupport.builtinSkillNames() {
            if installed.contains_key(*name) || errors.contains_key(*name) {
                continue;
            }
            if let Err(error) = self.importBundledExternalSkill(name, runtimeSupport) {
                errors.insert((*name).to_string(), error);
            }
        }
        errors
    }

    /// Lists bundled external skills that have not been installed yet.
    #[allow(non_snake_case)]
    pub fn getBundledExternalSkillCandidates(
        &self,
        runtimeSupport: &dyn ToolRuntimeSupport,
    ) -> Vec<BundledExternalSkillCandidate> {
        let loadedSkillNames = self
            .getAvailableSkills()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut grouped = BTreeMap::<String, Vec<_>>::new();
        for asset in runtimeSupport.bundledExternalSkillAssets() {
            grouped
                .entry(asset.skillName.to_string())
                .or_default()
                .push(asset);
        }

        grouped
            .into_iter()
            .filter_map(|(assetName, assets)| {
                if runtimeSupport.builtinSkillNames().contains(&assetName.as_str()) {
                    return None;
                }
                let mut name = assetName.clone();
                let mut description = String::new();
                for asset in assets {
                    if asset.path.eq_ignore_ascii_case("SKILL.md")
                        || asset.path.eq_ignore_ascii_case("skill.md")
                    {
                        let content = String::from_utf8_lossy(asset.bytes);
                        let (metaName, metaDescription) = parseSkillMetadataContent(&content);
                        if !metaName.trim().is_empty() {
                            name = metaName;
                        }
                        description = metaDescription;
                        break;
                    }
                }
                if loadedSkillNames.contains(&name) {
                    None
                } else {
                    Some(BundledExternalSkillCandidate { name, description })
                }
            })
            .collect()
    }

    /// Compares every shipped file, including shared example resources, with an installed skill.
    /// None means this application has no bundled update source for the skill.
    #[allow(non_snake_case)]
    pub fn bundledSkillNeedsUpdate(
        &self,
        skill: &SkillPackage,
        runtimeSupport: &dyn ToolRuntimeSupport,
    ) -> Result<Option<bool>, String> {
        let assets = runtimeSupport.bundledExternalSkillAssets();
        if !assets.iter().any(|asset| asset.skillName == skill.name) {
            return Ok(None);
        }
        let differs = |path: &str, bytes: &[u8]| -> Result<bool, String> {
            let path = hostPath(&skill.directory.join(normalizeAssetRelativePath(path)?));
            if !self.filesystem.fileExists(&path)?.exists {
                return Ok(true);
            }
            Ok(self.filesystem.readFileBytes(&path)? != bytes)
        };
        for asset in assets.iter().filter(|asset| asset.skillName == skill.name) {
            if differs(asset.path, asset.bytes)? {
                return Ok(Some(true));
            }
        }
        Ok(Some(false))
    }

    /// Installs or explicitly updates a bundled skill, retaining its location and settings.
    #[allow(non_snake_case)]
    pub fn importBundledExternalSkill(
        &self,
        skillName: &str,
        runtimeSupport: &dyn ToolRuntimeSupport,
    ) -> Result<SkillPackage, String> {
        let skillAssets = runtimeSupport
            .bundledExternalSkillAssets()
            .iter()
            .filter(|asset| asset.skillName == skillName)
            .collect::<Vec<_>>();
        if skillAssets.is_empty() {
            return Err(format!("Bundled external skill not found: {}", skillName));
        }
        // Resolve existing ownership first so a shared installation is updated in place.
        let (installed, errors) = self.getAvailableSkillsSnapshot();
        if let Some(error) = errors.get(skillName) {
            return Err(error.clone());
        }
        let existing = installed.get(skillName);
        let store = ExtensionStore::default();
        let record = existing
            .map(|_| store.record("skill", skillName))
            .transpose()?;
        let skillRoot = existing.map(|skill| skill.directory.clone()).unwrap_or(
            self.getSkillsRootDir()
                .join(normalizeAssetRelativePath(skillName)?),
        );
        let parent = skillRoot
            .parent()
            .ok_or("Invalid bundled skill directory")?;
        // Stage beside the skills root so catalog scans never see an incomplete skill.
        let staging = parent.parent().ok_or("Invalid bundled skill root")?
            .join(format!(".bundled_skill_{}", uuid::Uuid::new_v4()));
        let prepared = staging.join("prepared");
        let previous = staging.join("previous");
        self.filesystem.makeDirectory(&hostPath(&prepared), true)?;
        let install = || {
            for asset in skillAssets {
                let outputFile = prepared.join(normalizeAssetRelativePath(asset.path)?);
                self.filesystem
                    .writeFileBytes(&hostPath(&outputFile), asset.bytes)?;
            }
            let skillFile = directSkillFile(self.filesystem.as_ref(), &prepared)
                .ok_or("Bundled skill has no SKILL.md")?;
            let (name, description) = parseSkillMetadata(self.filesystem.as_ref(), &skillFile)?;
            if name != skillName {
                return Err(format!(
                    "Bundled skill identity mismatch: expected {skillName}, got {name}"
                ));
            }
            let fileName = skillFile.file_name().ok_or("Invalid bundled skill file")?;
            let hadPrevious = self.filesystem.fileExists(&hostPath(&skillRoot))?.exists;
            if hadPrevious {
                self.filesystem
                    .moveFile(&hostPath(&skillRoot), &hostPath(&previous))?;
            }
            if let Err(error) = self
                .filesystem
                .moveFile(&hostPath(&prepared), &hostPath(&skillRoot))
            {
                if hadPrevious {
                    self.filesystem
                        .moveFile(&hostPath(&previous), &hostPath(&skillRoot))
                        .map_err(|rollback| {
                            format!("{error}; failed to restore previous skill: {rollback}")
                        })?;
                }
                return Err(error);
            }
            let builtin = runtimeSupport.builtinSkillNames().contains(&skillName);
            let published =
                if let Some(record) = &record {
                    // Preserve visibility, scope and any other user settings.
                    let mut settings = record.settings.clone();
                    settings["builtin"] = serde_json::json!(builtin);
                    settings["bundled"] = serde_json::json!(true);
                    store.setSettings("skill", skillName, settings)
                } else {
                    store.registerDevice("skill", skillName, skillName,
                    serde_json::json!({"visible": !builtin, "builtin": builtin, "bundled": true}))
                };
            if let Err(error) = published {
                self.filesystem.deleteFile(&hostPath(&skillRoot), true)?;
                if hadPrevious {
                    self.filesystem
                        .moveFile(&hostPath(&previous), &hostPath(&skillRoot))
                        .map_err(|rollback| {
                            format!("{error}; failed to restore previous skill: {rollback}")
                        })?;
                }
                return Err(error);
            }
            operit_store::ExtensionStore::notifyCatalogChanged();
            Ok(SkillPackage {
                name,
                description,
                skillFile: skillRoot.join(fileName),
                directory: skillRoot.clone(),
            })
        };
        let result = if record
            .as_ref()
            .is_some_and(|record| record.scope == "space")
        {
            operit_store::RuntimeFileSyncStore::RuntimeFileSyncStore::new(
                operit_store::RuntimeStorageHost::defaultRuntimeStorageHost(),
                operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
            )
            .trackChanges(&[self.paths.runtime_storage_key(&skillRoot)?], install)
        } else {
            install()
        };
        // Keep the backup available if restoring it failed.
        if !self.filesystem.fileExists(&hostPath(&previous))?.exists || result.is_ok() {
            let _ = self.filesystem.deleteFile(&hostPath(&staging), true);
        }
        result
    }

    /// Installs the bundled package-builder skill used by quick plugin creation.
    #[allow(non_snake_case)]
    pub fn ensureQuickPluginCreatorBundledSkill(
        &self,
        runtimeSupport: &dyn ToolRuntimeSupport,
    ) -> Result<SkillPackage, String> {
        let skills = self.getAvailableSkills();
        match skills.get(QUICK_PLUGIN_CREATOR_SKILL_NAME) {
            Some(skill) => Ok(skill.clone()),
            None => {
                self.importBundledExternalSkill(QUICK_PLUGIN_CREATOR_SKILL_NAME, runtimeSupport)
            }
        }
    }

    /// Reads the SKILL.md content for one installed skill.
    #[allow(non_snake_case)]
    pub fn readSkillContent(&self, skillName: &str) -> Option<String> {
        let skills = self.getAvailableSkills();
        let skill = skills.get(skillName)?;
        self.filesystem
            .readFile(&hostPath(&skill.skillFile))
            .ok()
    }

    /// Builds the system prompt fragment used when a skill is activated.
    #[allow(non_snake_case)]
    pub fn getSkillSystemPrompt(&self, skillName: &str) -> Option<String> {
        let skills = self.getAvailableSkills();
        let skill = skills.get(skillName)?;
        let content = match self.filesystem.readFile(&hostPath(&skill.skillFile)) {
            Ok(value) => value,
            Err(_) => String::new(),
        };
        let mut prompt = String::new();
        prompt.push_str(&format!("Using package (Skill): {}\n", skill.name));
        prompt.push_str(&format!("Use Time: {}\n", currentLocalDateTime()));
        prompt.push_str("Execution policy:\n");
        prompt.push_str("Prioritize using the skill-provided instructions and bundled scripts, and complete tasks with terminal-related tools.\n");
        if !skill.description.trim().is_empty() {
            prompt.push_str(&format!("Description: {}\n", skill.description));
        }
        prompt.push_str(&format!(
            "SKILL.md path: {}\n",
            skill.skillFile.to_string_lossy()
        ));
        prompt.push_str(&format!(
            "Skill directory: {}\n",
            skill.directory.to_string_lossy()
        ));
        prompt.push_str("Directory structure:\n");
        prompt.push_str(&buildDirectoryTreeText(
            self.filesystem.as_ref(),
            &skill.directory,
        ));
        prompt.push_str("\n\nSKILL.md:\n");
        prompt.push_str(&content);
        prompt.push('\n');
        Some(prompt)
    }

    /// Deletes one installed skill directory.
    #[allow(non_snake_case)]
    pub fn deleteSkill(&self, skillName: &str) -> bool {
        let skills = self.getAvailableSkills();
        let Some(skill) = skills.get(skillName) else {
            return false;
        };
        ExtensionStore::default()
            .delete("skill", &skill.name)
            .is_ok()
    }

    /// Imports all skill packages from archive bytes supplied by a host-backed transport.
    #[allow(non_snake_case)]
    pub fn importSkillArchiveBytes(
        &self,
        zipBytes: &[u8],
        archiveLabel: &Path,
        subDirPathInZip: Option<&str>,
    ) -> String {
        let skillsRoot = self.getSkillsRootDir();
        let skillsRootPath = hostPath(&skillsRoot);
        if let Err(error) = self.filesystem.makeDirectory(&skillsRootPath, true) {
            return format!("Cannot access skills directory: {}", error);
        }

        let tmpDir = skillsRoot.join(format!(".import_tmp_{}", currentTimeMillis()));
        if let Err(error) = self.filesystem.makeDirectory(&hostPath(&tmpDir), true) {
            return format!(
                "Failed to create temporary import directory {}: {}",
                tmpDir.to_string_lossy(),
                error
            );
        }

        let result = self.importSkillArchiveBytesInner(
            zipBytes,
            archiveLabel,
            subDirPathInZip,
            &skillsRoot,
            &tmpDir,
        );
        let _ = self.filesystem.deleteFile(&hostPath(&tmpDir), true);
        result
    }

    #[allow(non_snake_case)]
    fn importSkillArchiveBytesInner(
        &self,
        zipBytes: &[u8],
        archiveLabel: &Path,
        subDirPathInZip: Option<&str>,
        skillsRoot: &Path,
        tmpDir: &Path,
    ) -> String {
        if let Err(error) = unzipBytesToDirectory(self.filesystem.as_ref(), zipBytes, tmpDir) {
            return format!("Failed to import skill: {}", error);
        }

        let normalizedSubDir = subDirPathInZip
            .map(str::trim)
            .map(|value| value.trim_matches('/').to_string())
            .filter(|value| !value.is_empty());

        let zipRootDir = match singleChildDirectory(self.filesystem.as_ref(), tmpDir) {
            Some(path) => path,
            None => tmpDir.to_path_buf(),
        };
        let searchRoot = if let Some(subDir) = normalizedSubDir.as_ref() {
            match safeChildPath(&zipRootDir, subDir) {
                Ok(path) => match self.filesystem.fileExists(&hostPath(&path)) {
                    Ok(info) if info.exists && info.isDirectory => path,
                    Ok(_) => return format!("Import path not found: {}", subDir),
                    Err(error) => return format!("Failed to import skill: {}", error),
                },
                Err(error) => return error,
            }
        } else {
            tmpDir.to_path_buf()
        };

        let skillMdCandidates = findSkillFiles(self.filesystem.as_ref(), &searchRoot);
        if skillMdCandidates.is_empty() {
            return if normalizedSubDir.is_some() {
                "No SKILL.md found in the selected import path".to_string()
            } else {
                "No SKILL.md found in the imported zip".to_string()
            };
        }

        let installedNames = self.getAvailableSkills().into_keys().collect::<BTreeSet<_>>();
        let mut imported = Vec::new();
        let mut failed = Vec::new();
        let singleSkill = skillMdCandidates.len() == 1;
        for skillFile in skillMdCandidates {
            match self.importSkillDirectory(
                &skillFile,
                archiveLabel,
                skillsRoot,
                tmpDir,
                &installedNames,
            ) {
                Ok(message) => imported.push(message),
                Err(error) if singleSkill => return error,
                Err(error) => {
                    let relativePath = skillFile.strip_prefix(tmpDir).unwrap_or(&skillFile);
                    failed.push(format!("{}: {}", relativePath.display(), error));
                }
            }
        }
        if singleSkill {
            return imported.remove(0);
        }
        let mut messages = vec![format!(
            "Imported {} skills; failed: {}",
            imported.len(),
            failed.len()
        )];
        messages.extend(imported);
        messages.extend(failed);
        messages.join("\n")
    }

    /// Installs one discovered package without preventing other packages from importing.
    #[allow(non_snake_case)]
    fn importSkillDirectory(
        &self,
        selectedSkillFile: &Path,
        archiveLabel: &Path,
        skillsRoot: &Path,
        tmpDir: &Path,
        installedNames: &BTreeSet<String>,
    ) -> Result<String, String> {
        let selectedSkillDir = selectedSkillFile.parent().ok_or("Invalid SKILL.md path")?;

        let (metaName, metaDesc) = parseSkillMetadata(self.filesystem.as_ref(), selectedSkillFile)
            .map_err(|error| format!("Failed to import skill: {}", error))?;

        let baseName = if !metaName.trim().is_empty() {
            metaName.trim().to_string()
        } else if selectedSkillDir == tmpDir {
            match archiveLabel.file_stem() {
                Some(value) => value.to_string_lossy().replace('\\', "/"),
                None => "skill".to_string(),
            }
        } else {
            let dirName = selectedSkillDir
                .file_name()
                .map(|value| value.to_string_lossy().replace('\\', "/"));
            match dirName {
                Some(value) if !value.trim().is_empty() => value,
                _ => match archiveLabel.file_stem() {
                    Some(value) => value.to_string_lossy().replace('\\', "/"),
                    None => "skill".to_string(),
                },
            }
        };
        let finalDirName = if baseName.trim().is_empty() {
            "skill".to_string()
        } else {
            baseName.trim().to_string()
        };
        if installedNames.contains(&finalDirName) {
            return Err(format!(
                "Skill already exists in an installation location: {finalDirName}"
            ));
        }
        if finalDirName.contains(['/', '\\']) || Path::new(&finalDirName).components().count() != 1 {
            return Err(format!("Invalid skill name: {finalDirName}"));
        }
        let finalDir = safeChildPath(skillsRoot, &finalDirName)?;
        let finalDirPath = hostPath(&finalDir);
        let finalDirInfo = match self.filesystem.fileExists(&finalDirPath) {
            Ok(info) => info,
            Err(error) => return Err(format!("Failed to import skill: {}", error)),
        };
        if finalDirInfo.exists {
            return Err(format!("Skill '{}' already exists", finalDirName));
        }
        if let Err(error) =
            self.filesystem
                .copyFile(&hostPath(selectedSkillDir), &finalDirPath, true)
        {
            return Err(format!("Failed to import skill: {}", error));
        }

        Ok(if metaDesc.trim().is_empty() {
            format!("Imported skill: {}", finalDirName)
        } else {
            format!("Imported skill: {} - {}", finalDirName, metaDesc)
        })
    }

    #[allow(non_snake_case)]
    fn getSkillsRootDir(&self) -> PathBuf {
        self.paths.skills_dir()
    }
}

#[allow(non_snake_case)]
fn parseSkillMetadata(
    filesystem: &VisualFileSystem,
    skillFile: &Path,
) -> Result<(String, String), String> {
    let content = filesystem
        .readFile(&hostPath(skillFile))
        .map_err(|error| error.to_string())?;
    Ok(parseSkillMetadataContent(&content))
}

/// Converts a package metadata path into the corresponding host path string.
fn hostPath(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[allow(non_snake_case)]
fn parseSkillMetadataContent(content: &str) -> (String, String) {
    let lines = content.lines().collect::<Vec<_>>();
    let mut name = String::new();
    let mut description = String::new();

    if lines.first().map(|line| line.trim()) == Some("---") {
        if let Some(endIndex) = lines.iter().skip(1).position(|line| line.trim() == "---") {
            for lineRaw in &lines[1..endIndex + 1] {
                parseMetadataLine(lineRaw, &mut name, &mut description);
            }
        }
    }

    if name.trim().is_empty() || description.trim().is_empty() {
        for lineRaw in lines.iter().take(40) {
            parseMetadataLine(lineRaw, &mut name, &mut description);
        }
    }

    (name, description)
}

#[allow(non_snake_case)]
fn parseMetadataLine(lineRaw: &str, name: &mut String, description: &mut String) {
    let line = lineRaw.trim();
    let Some(index) = line.find(':') else {
        return;
    };
    if index == 0 {
        return;
    }
    let key = line[..index].trim().to_ascii_lowercase();
    let value = unquote(line[index + 1..].trim());
    match key.as_str() {
        "name" if name.trim().is_empty() => *name = value,
        "description" if description.trim().is_empty() => *description = value,
        _ => {}
    }
}

fn unquote(valueRaw: &str) -> String {
    let value = valueRaw.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        return value[1..value.len() - 1].to_string();
    }
    value.to_string()
}

#[allow(non_snake_case)]
fn normalizeAssetRelativePath(path: &str) -> Result<PathBuf, String> {
    let mut normalized = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            std::path::Component::Normal(part) => normalized.push(part),
            _ => return Err(format!("Invalid plugin type asset path: {}", path)),
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(format!("Invalid plugin type asset path: {}", path));
    }
    Ok(normalized)
}

#[allow(non_snake_case)]
fn buildDirectoryTreeText(filesystem: &VisualFileSystem, rootDir: &Path) -> String {
    let mut output = String::new();
    walkDirectory(filesystem, rootDir, "", &mut output);
    if output.trim().is_empty() {
        "(empty directory)".to_string()
    } else {
        output.trim_end().to_string()
    }
}

#[allow(non_snake_case)]
fn walkDirectory(
    filesystem: &VisualFileSystem,
    dir: &Path,
    indent: &str,
    output: &mut String,
) {
    let Ok(mut children) = filesystem.listFiles(&hostPath(dir)) else {
        return;
    };
    children.sort_by(|left, right| {
        left.isDirectory.cmp(&right.isDirectory).then_with(|| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        })
    });
    for child in children {
        let childPath = dir.join(&child.name);
        output.push_str(indent);
        output.push_str("- ");
        output.push_str(&child.name);
        if child.isDirectory {
            output.push_str("/\n");
            walkDirectory(filesystem, &childPath, &format!("{indent}  "), output);
        } else {
            output.push('\n');
        }
    }
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> u128 {
    operit_host_api::TimeUtils::currentTimeMillisU128()
}

#[allow(non_snake_case)]
fn unzipBytesToDirectory(
    filesystem: &VisualFileSystem,
    zipBytes: &[u8],
    destinationDir: &Path,
) -> Result<(), String> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(zipBytes)).map_err(|error| error.to_string())?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let Some(enclosedName) = entry.enclosed_name() else {
            return Err(format!("Zip entry is outside target dir: {}", entry.name()));
        };
        let outFile = destinationDir.join(enclosedName);
        let outPath = hostPath(&outFile);

        if entry.is_dir() {
            filesystem
                .makeDirectory(&outPath, true)
                .map_err(|error| error.to_string())?;
        } else {
            let mut content = Vec::new();
            entry
                .read_to_end(&mut content)
                .map_err(|error| error.to_string())?;
            filesystem
                .writeFileBytes(&outPath, &content)
                .map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

#[allow(non_snake_case)]
fn singleChildDirectory(filesystem: &VisualFileSystem, root: &Path) -> Option<PathBuf> {
    let children = filesystem.listFiles(&hostPath(root)).ok()?;
    if children.len() == 1 && children[0].isDirectory {
        Some(root.join(&children[0].name))
    } else {
        None
    }
}

#[allow(non_snake_case)]
fn directSkillFile(filesystem: &VisualFileSystem, root: &Path) -> Option<PathBuf> {
    let primary = root.join("SKILL.md");
    if matches!(
        filesystem.fileExists(&hostPath(&primary)),
        Ok(info) if info.exists && !info.isDirectory
    ) {
        return Some(primary);
    }
    let lower = root.join("skill.md");
    if matches!(
        filesystem.fileExists(&hostPath(&lower)),
        Ok(info) if info.exists && !info.isDirectory
    ) {
        return Some(lower);
    }
    None
}

#[allow(non_snake_case)]
fn findSkillFiles(filesystem: &VisualFileSystem, root: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    findSkillFilesInner(filesystem, root, &mut result);
    result.sort();
    result
}

#[allow(non_snake_case)]
fn findSkillFilesInner(filesystem: &VisualFileSystem, root: &Path, result: &mut Vec<PathBuf>) {
    // A package owns its entire subtree, including any example SKILL.md files.
    if let Some(skillFile) = directSkillFile(filesystem, root) {
        result.push(skillFile);
        return;
    }
    let Ok(children) = filesystem.listFiles(&hostPath(root)) else {
        return;
    };
    for child in children {
        if child.isDirectory && child.name != "__MACOSX" {
            findSkillFilesInner(filesystem, &root.join(&child.name), result);
        }
    }
}

#[allow(non_snake_case)]
fn safeChildPath(base: &Path, relativePath: &str) -> Result<PathBuf, String> {
    let mut path = base.to_path_buf();
    for component in Path::new(relativePath).components() {
        match component {
            std::path::Component::Normal(part) => path.push(part),
            _ => return Err("Invalid import path".to_string()),
        }
    }
    Ok(path)
}
