use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use serde::{Deserialize, Serialize};

use crate::core::tools::skill::SkillPackage::SkillPackage;
use crate::plugins::BundledExternalSkillAssets::BUNDLED_EXTERNAL_SKILL_ASSETS;

const QUICK_PLUGIN_CREATOR_SKILL_NAME: &str = "PackageBuilder";

#[derive(Clone, Debug)]
pub struct SkillManager {
    paths: RuntimeStorePaths,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundledExternalSkillCandidate {
    pub name: String,
    pub description: String,
}

impl SkillManager {
    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self { paths }
    }

    #[allow(non_snake_case)]
    pub fn getSkillsDirectoryPath(&self) -> String {
        let skillsDir = self.getSkillsRootDir();
        skillsDir.to_string_lossy().to_string()
    }

    #[allow(non_snake_case)]
    pub fn refreshAvailableSkills(
        &self,
    ) -> (BTreeMap<String, SkillPackage>, BTreeMap<String, String>) {
        let mut availableSkills = BTreeMap::new();
        let mut skillLoadErrors = BTreeMap::new();
        let skillsDir = self.getSkillsRootDir();

        if let Err(error) = fs::create_dir_all(&skillsDir) {
            skillLoadErrors.insert(
                "skills".to_string(),
                format!("Cannot access skills directory: {}", error),
            );
            return (availableSkills, skillLoadErrors);
        }

        let children = match fs::read_dir(&skillsDir) {
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
            let Ok(child) = child else {
                continue;
            };
            let childPath = child.path();
            if !childPath.is_dir() {
                continue;
            }
            let childName = child.file_name().to_string_lossy().to_string();
            let primarySkillFile = childPath.join("SKILL.md");
            let lowerSkillFile = childPath.join("skill.md");
            let skillFile = if primarySkillFile.is_file() {
                primarySkillFile
            } else {
                lowerSkillFile
            };

            if !skillFile.is_file() {
                skillLoadErrors.insert(
                    childName,
                    format!("Missing SKILL.md in {}", childPath.to_string_lossy()),
                );
                continue;
            }

            match parseSkillMetadata(&skillFile) {
                Ok((name, description)) => {
                    let skillName = if name.trim().is_empty() {
                        child.file_name().to_string_lossy().to_string()
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
                            child.file_name().to_string_lossy().to_string(),
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
                    skillLoadErrors.insert(
                        child.file_name().to_string_lossy().to_string(),
                        format!("Failed to scan skill: {}", error),
                    );
                }
            }
        }

        (availableSkills, skillLoadErrors)
    }

    #[allow(non_snake_case)]
    pub fn getAvailableSkills(&self) -> BTreeMap<String, SkillPackage> {
        self.refreshAvailableSkills().0
    }

    #[allow(non_snake_case)]
    pub fn getAvailableSkillsSnapshot(
        &self,
    ) -> (BTreeMap<String, SkillPackage>, BTreeMap<String, String>) {
        self.refreshAvailableSkills()
    }

    #[allow(non_snake_case)]
    pub fn getSkillLoadErrors(&self) -> BTreeMap<String, String> {
        self.refreshAvailableSkills().1
    }

    #[allow(non_snake_case)]
    pub fn getBundledExternalSkillCandidates(&self) -> Vec<BundledExternalSkillCandidate> {
        let loadedSkillNames = self
            .getAvailableSkills()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut grouped = BTreeMap::<String, Vec<_>>::new();
        for asset in BUNDLED_EXTERNAL_SKILL_ASSETS {
            grouped
                .entry(asset.skill_name.to_string())
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

    #[allow(non_snake_case)]
    pub fn importBundledExternalSkill(&self, skillName: &str) -> Result<SkillPackage, String> {
        let skillAssets = BUNDLED_EXTERNAL_SKILL_ASSETS
            .iter()
            .filter(|asset| asset.skill_name == skillName)
            .collect::<Vec<_>>();
        if skillAssets.is_empty() {
            return Err(format!("Bundled external skill not found: {}", skillName));
        }

        let skillsRoot = self.getSkillsRootDir();
        fs::create_dir_all(&skillsRoot)
            .map_err(|error| format!("Cannot access skills directory: {}", error))?;

        let skillRoot = skillsRoot.join(skillName);
        if skillRoot.exists() && !skillRoot.is_dir() {
            return Err(format!(
                "Skill path is not a directory: {}",
                skillRoot.to_string_lossy()
            ));
        }
        fs::create_dir_all(&skillRoot)
            .map_err(|error| format!("Failed to create skill directory: {}", error))?;

        clearBundledSkillFiles(&skillRoot)?;
        for asset in skillAssets {
            let normalizedPath = normalizeAssetRelativePath(asset.path)?;
            let outputFile = skillRoot.join(normalizedPath);
            if let Some(parent) = outputFile.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("Failed to create bundled skill parent: {}", error))?;
            }
            fs::write(&outputFile, asset.bytes)
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

    #[allow(non_snake_case)]
    pub fn ensureQuickPluginCreatorBundledSkill(&self) -> Result<SkillPackage, String> {
        self.importBundledExternalSkill(QUICK_PLUGIN_CREATOR_SKILL_NAME)
    }

    #[allow(non_snake_case)]
    pub fn readSkillContent(&self, skillName: &str) -> Option<String> {
        let skills = self.getAvailableSkills();
        let skill = skills.get(skillName)?;
        fs::read_to_string(&skill.skillFile).ok()
    }

    #[allow(non_snake_case)]
    pub fn getSkillSystemPrompt(&self, skillName: &str) -> Option<String> {
        let skills = self.getAvailableSkills();
        let skill = skills.get(skillName)?;
        let content = match fs::read_to_string(&skill.skillFile) {
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
        prompt.push_str(&buildDirectoryTreeText(&skill.directory));
        prompt.push_str("\n\nSKILL.md:\n");
        prompt.push_str(&content);
        prompt.push('\n');
        Some(prompt)
    }

    #[allow(non_snake_case)]
    pub fn deleteSkill(&self, skillName: &str) -> bool {
        let skills = self.getAvailableSkills();
        let Some(skill) = skills.get(skillName) else {
            return false;
        };
        fs::remove_dir_all(&skill.directory).is_ok()
    }

    #[allow(non_snake_case)]
    pub fn importSkillFromZip(&self, zipFile: &Path) -> String {
        self.importSkillFromZipWithSubDir(zipFile, None)
    }

    #[allow(non_snake_case)]
    pub fn importSkillFromZipWithSubDir(
        &self,
        zipFile: &Path,
        subDirPathInZip: Option<&str>,
    ) -> String {
        if !zipFile.exists() || !zipFile.is_file() {
            return format!("Cannot read skill file: {}", zipFile.to_string_lossy());
        }
        let extension = zipFile
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase());
        if extension.as_deref() != Some("zip") {
            return "Only .zip files are supported".to_string();
        }

        let skillsRoot = self.getSkillsRootDir();
        if let Err(error) = fs::create_dir_all(&skillsRoot) {
            return format!("Cannot access skills directory: {}", error);
        }

        let tmpDir = skillsRoot.join(format!(".import_tmp_{}", currentTimeMillis()));
        if let Err(error) = fs::create_dir_all(&tmpDir) {
            return format!(
                "Failed to create temporary import directory {}: {}",
                tmpDir.to_string_lossy(),
                error
            );
        }

        let result = self.importSkillFromZipInner(zipFile, subDirPathInZip, &skillsRoot, &tmpDir);
        let _ = fs::remove_dir_all(&tmpDir);
        result
    }

    #[allow(non_snake_case)]
    fn importSkillFromZipInner(
        &self,
        zipFile: &Path,
        subDirPathInZip: Option<&str>,
        skillsRoot: &Path,
        tmpDir: &Path,
    ) -> String {
        if let Err(error) = unzipToDirectory(zipFile, tmpDir) {
            return format!("Failed to import skill: {}", error);
        }

        let normalizedSubDir = subDirPathInZip
            .map(str::trim)
            .map(|value| value.trim_matches('/').to_string())
            .filter(|value| !value.is_empty());

        let zipRootDir = match singleChildDirectory(tmpDir) {
            Some(path) => path,
            None => tmpDir.to_path_buf(),
        };
        let searchRoot = if let Some(subDir) = normalizedSubDir.as_ref() {
            let baseCanonical = match zipRootDir.canonicalize() {
                Ok(path) => path,
                Err(error) => return format!("Failed to import skill: {}", error),
            };
            let resolved = zipRootDir.join(subDir);
            let resolvedCanonical = match resolved.canonicalize() {
                Ok(path) => path,
                Err(_) => return format!("Import path not found: {}", subDir),
            };
            if !isPathInside(&resolvedCanonical, &baseCanonical) {
                return "Invalid import path".to_string();
            }
            resolvedCanonical
        } else {
            tmpDir.to_path_buf()
        };

        let skillMdCandidates = match directSkillFile(&searchRoot) {
            Some(skillFile) => vec![skillFile],
            None => findSkillFiles(&searchRoot, 10),
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

        let (metaName, metaDesc) = match parseSkillMetadata(&selectedSkillFile) {
            Ok(value) => value,
            Err(error) => return format!("Failed to import skill: {}", error),
        };

        let baseName = if !metaName.trim().is_empty() {
            metaName.trim().to_string()
        } else if selectedSkillDir.canonicalize().ok() == tmpDir.canonicalize().ok() {
            match zipFile.file_stem() {
                Some(value) => value.to_string_lossy().to_string(),
                None => "skill".to_string(),
            }
        } else {
            let dirName = selectedSkillDir
                .file_name()
                .map(|value| value.to_string_lossy().to_string());
            match dirName {
                Some(value) if !value.trim().is_empty() => value,
                _ => match zipFile.file_stem() {
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
        if finalDir.exists() {
            return format!("Skill '{}' already exists", finalDirName);
        }
        if let Err(error) = copyDirectoryRecursively(selectedSkillDir, &finalDir) {
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
fn parseSkillMetadata(skillFile: &Path) -> Result<(String, String), std::io::Error> {
    let content = fs::read_to_string(skillFile)?;
    Ok(parseSkillMetadataContent(&content))
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
fn clearBundledSkillFiles(skillRoot: &Path) -> Result<(), String> {
    let children = fs::read_dir(skillRoot)
        .map_err(|error| format!("Failed to read bundled skill directory: {}", error))?;
    for child in children {
        let child =
            child.map_err(|error| format!("Failed to read bundled skill entry: {}", error))?;
        let path = child.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .map_err(|error| format!("Failed to clear bundled skill directory: {}", error))?;
        } else {
            fs::remove_file(&path)
                .map_err(|error| format!("Failed to clear bundled skill file: {}", error))?;
        }
    }
    Ok(())
}

#[allow(non_snake_case)]
fn buildDirectoryTreeText(rootDir: &Path) -> String {
    let mut output = String::new();
    walkDirectory(rootDir, "", &mut output);
    if output.trim().is_empty() {
        "(empty directory)".to_string()
    } else {
        output.trim_end().to_string()
    }
}

#[allow(non_snake_case)]
fn walkDirectory(dir: &Path, indent: &str, output: &mut String) {
    let Ok(children) = fs::read_dir(dir) else {
        return;
    };
    let mut children = children.filter_map(Result::ok).collect::<Vec<_>>();
    children.sort_by(|left, right| {
        let leftPath = left.path();
        let rightPath = right.path();
        leftPath.is_file().cmp(&rightPath.is_file()).then_with(|| {
            left.file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .cmp(&right.file_name().to_string_lossy().to_ascii_lowercase())
        })
    });
    for child in children {
        let childPath = child.path();
        output.push_str(indent);
        output.push_str("- ");
        output.push_str(&child.file_name().to_string_lossy());
        if childPath.is_dir() {
            output.push_str("/\n");
            walkDirectory(&childPath, &format!("{indent}  "), output);
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
fn unzipToDirectory(zipFile: &Path, destinationDir: &Path) -> Result<(), String> {
    let file = File::open(zipFile).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    let destCanonical = destinationDir
        .canonicalize()
        .map_err(|error| error.to_string())?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let Some(enclosedName) = entry.enclosed_name() else {
            return Err(format!("Zip entry is outside target dir: {}", entry.name()));
        };
        let outFile = destinationDir.join(enclosedName);
        let outCanonicalParent = outFile
            .parent()
            .ok_or_else(|| format!("Invalid zip entry path: {}", entry.name()))?
            .to_path_buf();
        fs::create_dir_all(&outCanonicalParent).map_err(|error| error.to_string())?;
        let parentCanonical = outCanonicalParent
            .canonicalize()
            .map_err(|error| error.to_string())?;
        if !isPathInside(&parentCanonical, &destCanonical) {
            return Err(format!("Zip entry is outside target dir: {}", entry.name()));
        }

        if entry.is_dir() {
            fs::create_dir_all(&outFile).map_err(|error| error.to_string())?;
        } else {
            let mut out = File::create(&outFile).map_err(|error| error.to_string())?;
            io::copy(&mut entry, &mut out).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

#[allow(non_snake_case)]
fn singleChildDirectory(root: &Path) -> Option<PathBuf> {
    let children = fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    if children.len() == 1 && children[0].path().is_dir() {
        Some(children[0].path())
    } else {
        None
    }
}

#[allow(non_snake_case)]
fn directSkillFile(root: &Path) -> Option<PathBuf> {
    if !root.is_dir() {
        return None;
    }
    let primary = root.join("SKILL.md");
    if primary.is_file() {
        return Some(primary);
    }
    let lower = root.join("skill.md");
    if lower.is_file() {
        return Some(lower);
    }
    None
}

#[allow(non_snake_case)]
fn findSkillFiles(root: &Path, limit: usize) -> Vec<PathBuf> {
    let mut result = Vec::new();
    findSkillFilesInner(root, limit, &mut result);
    result
}

#[allow(non_snake_case)]
fn findSkillFilesInner(root: &Path, limit: usize, result: &mut Vec<PathBuf>) {
    if result.len() >= limit {
        return;
    }
    let Ok(children) = fs::read_dir(root) else {
        return;
    };
    for child in children.filter_map(Result::ok) {
        if result.len() >= limit {
            return;
        }
        let path = child.path();
        if path.is_file() {
            let name = child.file_name().to_string_lossy().to_string();
            if name.eq_ignore_ascii_case("SKILL.md") || name.eq_ignore_ascii_case("skill.md") {
                result.push(path);
            }
        } else if path.is_dir() {
            findSkillFilesInner(&path, limit, result);
        }
    }
}

#[allow(non_snake_case)]
fn isPathInside(path: &Path, base: &Path) -> bool {
    path == base || path.starts_with(base)
}

#[allow(non_snake_case)]
fn copyDirectoryRecursively(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for child in fs::read_dir(source)? {
        let child = child?;
        let childPath = child.path();
        let targetPath = destination.join(child.file_name());
        if childPath.is_dir() {
            copyDirectoryRecursively(&childPath, &targetPath)?;
        } else {
            fs::copy(&childPath, &targetPath)?;
        }
    }
    Ok(())
}
