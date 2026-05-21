use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::skill::SkillManager::SkillManager;
use crate::core::tools::skill::SkillPackage::SkillPackage;
use crate::data::preferences::SkillVisibilityPreferences::SkillVisibilityPreferences;

pub struct SkillRepository {
    skillManager: SkillManager,
    skillVisibilityPreferences: SkillVisibilityPreferences,
}

impl SkillRepository {
    #[allow(non_snake_case)]
    pub fn getInstance(_context: &OperitApplicationContext) -> Self {
        Self {
            skillManager: SkillManager::getInstance(),
            skillVisibilityPreferences: SkillVisibilityPreferences::getInstance(),
        }
    }

    #[allow(non_snake_case)]
    pub fn getSkillsDirectoryPath(&self) -> String {
        self.skillManager.getSkillsDirectoryPath()
    }

    #[allow(non_snake_case)]
    pub fn getAvailableSkillPackages(&self) -> BTreeMap<String, SkillPackage> {
        self.skillManager.getAvailableSkills()
    }

    #[allow(non_snake_case)]
    pub fn getAvailableSkillPackagesSnapshot(
        &self,
    ) -> (BTreeMap<String, SkillPackage>, BTreeMap<String, String>) {
        self.skillManager.getAvailableSkillsSnapshot()
    }

    #[allow(non_snake_case)]
    pub fn getSkillLoadErrors(&self) -> BTreeMap<String, String> {
        self.skillManager.getSkillLoadErrors()
    }

    #[allow(non_snake_case)]
    pub fn getAiVisibleSkillPackages(&self) -> BTreeMap<String, SkillPackage> {
        self.skillManager
            .getAvailableSkills()
            .into_iter()
            .filter(|(skillName, _)| {
                self.skillVisibilityPreferences
                    .isSkillVisibleToAi(skillName)
            })
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn readSkillContent(&self, skillName: &str) -> Option<String> {
        self.skillManager.readSkillContent(skillName)
    }

    #[allow(non_snake_case)]
    pub fn deleteSkill(&self, skillName: &str) -> bool {
        self.skillManager.deleteSkill(skillName)
    }

    #[allow(non_snake_case)]
    pub fn isSkillVisibleToAi(&self, skillName: &str) -> bool {
        self.skillVisibilityPreferences.isSkillVisibleToAi(skillName)
    }

    #[allow(non_snake_case)]
    pub fn setSkillVisibleToAi(
        &self,
        skillName: &str,
        visible: bool,
    ) -> Result<(), operit_store::PreferencesDataStore::PreferencesDataStoreError> {
        self.skillVisibilityPreferences
            .setSkillVisibleToAi(skillName, visible)
    }

    #[allow(non_snake_case)]
    pub fn importSkillFromZip(&self, zipFile: &Path) -> String {
        self.skillManager.importSkillFromZip(zipFile)
    }

    #[allow(non_snake_case)]
    pub fn importSkillFromZipWithSubDir(
        &self,
        zipFile: &Path,
        subDirPathInZip: Option<&str>,
    ) -> String {
        self.skillManager
            .importSkillFromZipWithSubDir(zipFile, subDirPathInZip)
    }

    #[allow(non_snake_case)]
    pub fn importSkillFromDirectInput(
        &self,
        skillId: &str,
        description: &str,
        content: &str,
        attachmentPaths: &[PathBuf],
    ) -> String {
        let trimmedId = skillId.trim();
        let trimmedDescription = description.trim();
        let trimmedContent = content.trim();

        if trimmedId.is_empty() {
            return "Skill id is required".to_string();
        }
        if !isValidSkillId(trimmedId) {
            return "Skill id may only contain letters, numbers, dot, underscore, and hyphen".to_string();
        }
        if trimmedContent.is_empty() {
            return "Skill content is required".to_string();
        }

        let skillsRootDir = PathBuf::from(self.getSkillsDirectoryPath());
        if let Err(error) = fs::create_dir_all(&skillsRootDir) {
            return format!(
                "Failed to create skills directory {}: {}",
                skillsRootDir.to_string_lossy(),
                error
            );
        }

        let finalDir = skillsRootDir.join(trimmedId);
        if finalDir.exists() {
            return format!("Skill '{}' already exists", trimmedId);
        }
        if let Err(error) = fs::create_dir_all(&finalDir) {
            return format!(
                "Failed to create skills directory {}: {}",
                finalDir.to_string_lossy(),
                error
            );
        }

        let result = self.writeDirectSkill(
            &finalDir,
            trimmedId,
            trimmedDescription,
            trimmedContent,
            attachmentPaths,
        );
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&finalDir);
            return format!("Failed to import skill: {}", error);
        }

        if trimmedDescription.is_empty() {
            format!("Imported skill: {}", trimmedId)
        } else {
            format!("Imported skill: {} - {}", trimmedId, trimmedDescription)
        }
    }

    #[allow(non_snake_case)]
    fn writeDirectSkill(
        &self,
        finalDir: &Path,
        skillId: &str,
        description: &str,
        content: &str,
        attachmentPaths: &[PathBuf],
    ) -> Result<(), String> {
        fs::write(
            finalDir.join("SKILL.md"),
            buildDirectSkillMarkdown(skillId, description, content),
        )
        .map_err(|error| error.to_string())?;

        if !attachmentPaths.is_empty() {
            let assetsDir = finalDir.join("assets");
            fs::create_dir_all(&assetsDir).map_err(|error| error.to_string())?;
            let mut usedFileNames = Vec::<String>::new();
            for (index, path) in attachmentPaths.iter().enumerate() {
                let displayName = match path.file_name() {
                    Some(value) => value.to_string_lossy().to_string(),
                    None => format!("attachment_{}", index + 1),
                };
                let safeName = ensureUniqueFileName(
                    &sanitizeAttachmentName(&displayName),
                    &mut usedFileNames,
                );
                fs::copy(path, assetsDir.join(safeName)).map_err(|error| error.to_string())?;
            }
        }

        Ok(())
    }
}

#[allow(non_snake_case)]
fn buildDirectSkillMarkdown(skillId: &str, description: &str, content: &str) -> String {
    let escapedName = escapeFrontMatterValue(skillId);
    let escapedDescription = escapeFrontMatterValue(description);
    format!(
        "---\nname: \"{}\"\ndescription: \"{}\"\n---\n\n{}\n",
        escapedName,
        escapedDescription,
        content.trim_end()
    )
}

#[allow(non_snake_case)]
fn escapeFrontMatterValue(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace("\r\n", "\n")
        .replace('\n', "\\n")
}

#[allow(non_snake_case)]
fn isValidSkillId(skillId: &str) -> bool {
    skillId != "."
        && skillId != ".."
        && skillId
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

#[allow(non_snake_case)]
fn sanitizeAttachmentName(rawName: &str) -> String {
    let sanitized = rawName
        .trim()
        .chars()
        .map(|ch| {
            if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>();
    if sanitized.trim().is_empty() {
        "attachment".to_string()
    } else {
        sanitized
    }
}

#[allow(non_snake_case)]
fn ensureUniqueFileName(baseName: &str, usedNames: &mut Vec<String>) -> String {
    if !usedNames.iter().any(|name| name == baseName) {
        usedNames.push(baseName.to_string());
        return baseName.to_string();
    }

    let dotIndex = baseName.rfind('.').filter(|index| *index > 0);
    let (prefix, extension) = match dotIndex {
        Some(index) => (&baseName[..index], &baseName[index..]),
        None => (baseName, ""),
    };
    let mut suffix = 1;
    loop {
        let candidate = format!("{}_{}{}", prefix, suffix, extension);
        if !usedNames.iter().any(|name| name == &candidate) {
            usedNames.push(candidate.clone());
            return candidate;
        }
        suffix += 1;
    }
}
