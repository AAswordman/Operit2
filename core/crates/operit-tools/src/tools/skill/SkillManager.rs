use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use operit_host_api::FileSystemHost;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use serde::{Deserialize, Serialize};

use crate::runtime_support::ToolRuntimeSupport;
use crate::tools::skill::SkillPackage::SkillPackage;

const QUICK_PLUGIN_CREATOR_SKILL_NAME: &str = "PackageBuilder";

#[derive(Clone)]
pub struct SkillManager {
    paths: RuntimeStorePaths,
    fileSystemHost: Arc<dyn FileSystemHost>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundledExternalSkillCandidate {
    pub name: String,
    pub description: String,
}

impl SkillManager {
    /// Creates a skill manager using the default runtime store paths.
    #[allow(non_snake_case)]
    pub fn fromDefaultPaths(fileSystemHost: Arc<dyn FileSystemHost>) -> Self {
        Self::new(RuntimeStorePaths::default(), fileSystemHost)
    }

    /// Creates a skill manager rooted at explicit runtime store paths.
    pub fn new(paths: RuntimeStorePaths, fileSystemHost: Arc<dyn FileSystemHost>) -> Self {
        Self {
            paths,
            fileSystemHost,
        }
    }

    /// Returns the directory where user-installed skills are stored.
    #[allow(non_snake_case)]
    pub fn getSkillsDirectoryPath(&self) -> String {
        let skillsDir = self.getSkillsRootDir();
        skillsDir.to_string_lossy().to_string()
    }

    /// Scans the skills directory and returns loaded packages plus load errors.
    #[allow(non_snake_case)]
    pub fn refreshAvailableSkills(
        &self,
    ) -> (BTreeMap<String, SkillPackage>, BTreeMap<String, String>) {
        let mut availableSkills = BTreeMap::new();
        let mut skillLoadErrors = BTreeMap::new();
        let skillsDir = self.getSkillsRootDir();
        let skillsPath = hostPath(&skillsDir);

        if let Err(error) = self.fileSystemHost.makeDirectory(&skillsPath, true) {
            skillLoadErrors.insert(
                "skills".to_string(),
                format!("Cannot access skills directory: {}", error),
            );
            return (availableSkills, skillLoadErrors);
        }

        let children = match self.fileSystemHost.listFiles(&skillsPath) {
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
            let childPath = skillsDir.join(&childName);
            let primarySkillFile = childPath.join("SKILL.md");
            let lowerSkillFile = childPath.join("skill.md");
            let primaryInfo = match self.fileSystemHost.fileExists(&hostPath(&primarySkillFile)) {
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

            let skillInfo = match self.fileSystemHost.fileExists(&hostPath(&skillFile)) {
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

            match parseSkillMetadata(self.fileSystemHost.as_ref(), &skillFile) {
                Ok((name, description)) => {
                    let skillName = if name.trim().is_empty() {
                        childName.clone()
                    } else {
                        name
                    };
                    if availableSkills.contains_key(&skillName) {
                        let existingDirName = match availableSkills.get(&skillName) {
                            Some(skill) => match skill.directory.file_name() {
                                Some(name) => name.to_string_lossy().to_string(),
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

    /// Installs one bundled external skill into the skills directory.
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

        let skillsRoot = self.getSkillsRootDir();
        let skillsRootPath = hostPath(&skillsRoot);
        self.fileSystemHost
            .makeDirectory(&skillsRootPath, true)
            .map_err(|error| format!("Cannot access skills directory: {}", error))?;

        let skillRoot = skillsRoot.join(skillName);
        let skillRootPath = hostPath(&skillRoot);
        let skillRootInfo = self
            .fileSystemHost
            .fileExists(&skillRootPath)
            .map_err(|error| error.to_string())?;
        if skillRootInfo.exists && !skillRootInfo.isDirectory {
            return Err(format!(
                "Skill path is not a directory: {}",
                skillRoot.to_string_lossy()
            ));
        }
        self.fileSystemHost
            .makeDirectory(&skillRootPath, true)
            .map_err(|error| format!("Failed to create skill directory: {}", error))?;

        clearBundledSkillFiles(self.fileSystemHost.as_ref(), &skillRoot)?;
        for asset in skillAssets {
            let normalizedPath = normalizeAssetRelativePath(asset.path)?;
            let outputFile = skillRoot.join(normalizedPath);
            self.fileSystemHost
                .writeFileBytes(&hostPath(&outputFile), asset.bytes)
                .map_err(|error| format!("Failed to write bundled skill asset: {}", error))?;
        }

        let skills = self.getAvailableSkills();
        let Some(skill) = skills.get(skillName) else {
            return Err(format!(
                "Skill '{}' was not loaded after creation",
                skillName
            ));
        };
        Ok(skill.clone())
    }

    /// Installs the bundled package-builder skill used by quick plugin creation.
    #[allow(non_snake_case)]
    pub fn ensureQuickPluginCreatorBundledSkill(
        &self,
        runtimeSupport: &dyn ToolRuntimeSupport,
    ) -> Result<SkillPackage, String> {
        self.importBundledExternalSkill(QUICK_PLUGIN_CREATOR_SKILL_NAME, runtimeSupport)
    }

    /// Reads the SKILL.md content for one installed skill.
    #[allow(non_snake_case)]
    pub fn readSkillContent(&self, skillName: &str) -> Option<String> {
        let skills = self.getAvailableSkills();
        let skill = skills.get(skillName)?;
        self.fileSystemHost
            .readFile(&hostPath(&skill.skillFile))
            .ok()
    }

    /// Builds the system prompt fragment used when a skill is activated.
    #[allow(non_snake_case)]
    pub fn getSkillSystemPrompt(&self, skillName: &str) -> Option<String> {
        let skills = self.getAvailableSkills();
        let skill = skills.get(skillName)?;
        let content = match self.fileSystemHost.readFile(&hostPath(&skill.skillFile)) {
            Ok(value) => value,
            Err(_) => String::new(),
        };
        let mut prompt = String::new();
        prompt.push_str(&format!("Using package (Skill): {}\n", skill.name));
        prompt.push_str(&format!("Use Time: {}\n", currentUseTime()));
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
            self.fileSystemHost.as_ref(),
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
        self.fileSystemHost
            .deleteFile(&hostPath(&skill.directory), true)
            .is_ok()
    }

    /// Imports a skill from a zip archive by searching for SKILL.md.
    #[allow(non_snake_case)]
    pub fn importSkillFromZip(&self, zipFile: &Path) -> String {
        self.importSkillFromZipWithSubDir(zipFile, None)
    }

    /// Imports a skill from a zip archive using an optional subdirectory inside the zip.
    #[allow(non_snake_case)]
    pub fn importSkillFromZipWithSubDir(
        &self,
        zipFile: &Path,
        subDirPathInZip: Option<&str>,
    ) -> String {
        let zipPath = hostPath(zipFile);
        let zipInfo = match self.fileSystemHost.fileExists(&zipPath) {
            Ok(info) => info,
            Err(error) => return format!("Cannot read skill file: {}", error),
        };
        if !zipInfo.exists || zipInfo.isDirectory {
            return format!("Cannot read skill file: {}", zipFile.to_string_lossy());
        }
        let extension = zipFile
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase());
        if extension.as_deref() != Some("zip") {
            return "Only .zip files are supported".to_string();
        }

        let zipBytes = match self.fileSystemHost.readFileBytes(&zipPath) {
            Ok(bytes) => bytes,
            Err(error) => return format!("Cannot read skill file: {}", error),
        };
        self.importSkillArchiveBytes(&zipBytes, zipFile, subDirPathInZip)
    }

    /// Imports a skill from archive bytes supplied by a host-backed transport.
    #[allow(non_snake_case)]
    pub fn importSkillArchiveBytes(
        &self,
        zipBytes: &[u8],
        archiveLabel: &Path,
        subDirPathInZip: Option<&str>,
    ) -> String {
        let skillsRoot = self.getSkillsRootDir();
        let skillsRootPath = hostPath(&skillsRoot);
        if let Err(error) = self.fileSystemHost.makeDirectory(&skillsRootPath, true) {
            return format!("Cannot access skills directory: {}", error);
        }

        let tmpDir = skillsRoot.join(format!(".import_tmp_{}", currentTimeMillis()));
        if let Err(error) = self.fileSystemHost.makeDirectory(&hostPath(&tmpDir), true) {
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
        let _ = self.fileSystemHost.deleteFile(&hostPath(&tmpDir), true);
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
        if let Err(error) = unzipBytesToDirectory(self.fileSystemHost.as_ref(), zipBytes, tmpDir) {
            return format!("Failed to import skill: {}", error);
        }

        let normalizedSubDir = subDirPathInZip
            .map(str::trim)
            .map(|value| value.trim_matches('/').to_string())
            .filter(|value| !value.is_empty());

        let zipRootDir = match singleChildDirectory(self.fileSystemHost.as_ref(), tmpDir) {
            Some(path) => path,
            None => tmpDir.to_path_buf(),
        };
        let searchRoot = if let Some(subDir) = normalizedSubDir.as_ref() {
            match safeChildPath(&zipRootDir, subDir) {
                Ok(path) => match self.fileSystemHost.fileExists(&hostPath(&path)) {
                    Ok(info) if info.exists && info.isDirectory => path,
                    Ok(_) => return format!("Import path not found: {}", subDir),
                    Err(error) => return format!("Failed to import skill: {}", error),
                },
                Err(error) => return error,
            }
        } else {
            tmpDir.to_path_buf()
        };

        let skillMdCandidates = match directSkillFile(self.fileSystemHost.as_ref(), &searchRoot) {
            Some(skillFile) => vec![skillFile],
            None => findSkillFiles(self.fileSystemHost.as_ref(), &searchRoot, 10),
        };
        if skillMdCandidates.is_empty() {
            return if normalizedSubDir.is_some() {
                "No SKILL.md found in the selected import path".to_string()
            } else {
                "No SKILL.md found in the imported zip".to_string()
            };
        }

        let selectedSkillFile = skillMdCandidates[0].clone();
        let Some(selectedSkillDir) = selectedSkillFile.parent() else {
            return "Invalid SKILL.md path".to_string();
        };

        let (metaName, metaDesc) =
            match parseSkillMetadata(self.fileSystemHost.as_ref(), &selectedSkillFile) {
                Ok(value) => value,
                Err(error) => return format!("Failed to import skill: {}", error),
            };

        let baseName = if !metaName.trim().is_empty() {
            metaName.trim().to_string()
        } else if selectedSkillDir == tmpDir {
            match archiveLabel.file_stem() {
                Some(value) => value.to_string_lossy().to_string(),
                None => "skill".to_string(),
            }
        } else {
            let dirName = selectedSkillDir
                .file_name()
                .map(|value| value.to_string_lossy().to_string());
            match dirName {
                Some(value) if !value.trim().is_empty() => value,
                _ => match archiveLabel.file_stem() {
                    Some(value) => value.to_string_lossy().to_string(),
                    None => "skill".to_string(),
                },
            }
        };
        let finalDirName = if baseName.trim().is_empty() {
            "skill".to_string()
        } else {
            baseName.trim().to_string()
        };
        let finalDir = skillsRoot.join(&finalDirName);
        let finalDirPath = hostPath(&finalDir);
        let finalDirInfo = match self.fileSystemHost.fileExists(&finalDirPath) {
            Ok(info) => info,
            Err(error) => return format!("Failed to import skill: {}", error),
        };
        if finalDirInfo.exists {
            return format!("Skill '{}' already exists", finalDirName);
        }
        if let Err(error) = self.fileSystemHost.copyFile(
            &hostPath(selectedSkillDir),
            &finalDirPath,
            true,
        ) {
            return format!("Failed to import skill: {}", error);
        }

        if metaDesc.trim().is_empty() {
            format!("Imported skill: {}", finalDirName)
        } else {
            format!("Imported skill: {} - {}", finalDirName, metaDesc)
        }
    }

    #[allow(non_snake_case)]
    fn getSkillsRootDir(&self) -> PathBuf {
        self.paths.skills_dir()
    }
}

#[allow(non_snake_case)]
fn parseSkillMetadata(
    fileSystemHost: &dyn FileSystemHost,
    skillFile: &Path,
) -> Result<(String, String), String> {
    let content = fileSystemHost
        .readFile(&hostPath(skillFile))
        .map_err(|error| error.to_string())?;
    Ok(parseSkillMetadataContent(&content))
}

/// Converts a package metadata path into the corresponding host path string.
fn hostPath(path: &Path) -> String {
    path.to_string_lossy().to_string()
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
fn clearBundledSkillFiles(
    fileSystemHost: &dyn FileSystemHost,
    skillRoot: &Path,
) -> Result<(), String> {
    let children = fileSystemHost
        .listFiles(&hostPath(skillRoot))
        .map_err(|error| format!("Failed to read bundled skill directory: {}", error))?;
    for child in children {
        let path = skillRoot.join(child.name);
        fileSystemHost
            .deleteFile(&hostPath(&path), child.isDirectory)
            .map_err(|error| format!("Failed to clear bundled skill entry: {}", error))?;
    }
    Ok(())
}

#[allow(non_snake_case)]
fn buildDirectoryTreeText(fileSystemHost: &dyn FileSystemHost, rootDir: &Path) -> String {
    let mut output = String::new();
    walkDirectory(fileSystemHost, rootDir, "", &mut output);
    if output.trim().is_empty() {
        "(empty directory)".to_string()
    } else {
        output.trim_end().to_string()
    }
}

#[allow(non_snake_case)]
fn walkDirectory(
    fileSystemHost: &dyn FileSystemHost,
    dir: &Path,
    indent: &str,
    output: &mut String,
) {
    let Ok(mut children) = fileSystemHost.listFiles(&hostPath(dir)) else {
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
            walkDirectory(fileSystemHost, &childPath, &format!("{indent}  "), output);
        } else {
            output.push('\n');
        }
    }
}

#[allow(non_snake_case)]
fn currentUseTime() -> String {
    chrono::Local::now()
        .naive_local()
        .format("%Y-%m-%dT%H:%M:%S%.f")
        .to_string()
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> u128 {
    operit_host_api::TimeUtils::currentTimeMillisU128()
}

#[allow(non_snake_case)]
fn unzipBytesToDirectory(
    fileSystemHost: &dyn FileSystemHost,
    zipBytes: &[u8],
    destinationDir: &Path,
) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zipBytes))
        .map_err(|error| error.to_string())?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let Some(enclosedName) = entry.enclosed_name() else {
            return Err(format!("Zip entry is outside target dir: {}", entry.name()));
        };
        let outFile = destinationDir.join(enclosedName);
        let outPath = hostPath(&outFile);

        if entry.is_dir() {
            fileSystemHost
                .makeDirectory(&outPath, true)
                .map_err(|error| error.to_string())?;
        } else {
            let mut content = Vec::new();
            entry
                .read_to_end(&mut content)
                .map_err(|error| error.to_string())?;
            fileSystemHost
                .writeFileBytes(&outPath, &content)
                .map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

#[allow(non_snake_case)]
fn singleChildDirectory(fileSystemHost: &dyn FileSystemHost, root: &Path) -> Option<PathBuf> {
    let children = fileSystemHost.listFiles(&hostPath(root)).ok()?;
    if children.len() == 1 && children[0].isDirectory {
        Some(root.join(&children[0].name))
    } else {
        None
    }
}

#[allow(non_snake_case)]
fn directSkillFile(fileSystemHost: &dyn FileSystemHost, root: &Path) -> Option<PathBuf> {
    let primary = root.join("SKILL.md");
    if matches!(
        fileSystemHost.fileExists(&hostPath(&primary)),
        Ok(info) if info.exists && !info.isDirectory
    ) {
        return Some(primary);
    }
    let lower = root.join("skill.md");
    if matches!(
        fileSystemHost.fileExists(&hostPath(&lower)),
        Ok(info) if info.exists && !info.isDirectory
    ) {
        return Some(lower);
    }
    None
}

#[allow(non_snake_case)]
fn findSkillFiles(
    fileSystemHost: &dyn FileSystemHost,
    root: &Path,
    limit: usize,
) -> Vec<PathBuf> {
    let mut result = Vec::new();
    findSkillFilesInner(fileSystemHost, root, limit, &mut result);
    result
}

#[allow(non_snake_case)]
fn findSkillFilesInner(
    fileSystemHost: &dyn FileSystemHost,
    root: &Path,
    limit: usize,
    result: &mut Vec<PathBuf>,
) {
    if result.len() >= limit {
        return;
    }
    let Ok(children) = fileSystemHost.listFiles(&hostPath(root)) else {
        return;
    };
    for child in children {
        if result.len() >= limit {
            return;
        }
        let path = root.join(&child.name);
        if !child.isDirectory {
            if child.name.eq_ignore_ascii_case("SKILL.md") || child.name.eq_ignore_ascii_case("skill.md") {
                result.push(path);
            }
        } else {
            findSkillFilesInner(fileSystemHost, &path, limit, result);
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
