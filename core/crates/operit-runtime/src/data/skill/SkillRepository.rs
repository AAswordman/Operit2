use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::application::OperitApplicationContext::defaultHttpHost;
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::skill::SkillManager::{BundledExternalSkillCandidate, SkillManager};
use crate::core::tools::skill::SkillPackage::SkillPackage;
use crate::data::preferences::SkillVisibilityPreferences::SkillVisibilityPreferences;
use operit_host_api::HttpRequestData;
use url::Url;

pub struct SkillRepository {
    skillManager: SkillManager,
    skillVisibilityPreferences: SkillVisibilityPreferences,
}

#[derive(Clone, Debug)]
struct GitHubSkillTarget {
    owner: String,
    repo: String,
    refName: Option<String>,
    subDir: Option<String>,
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
    pub fn getBundledExternalSkillCandidates(&self) -> Vec<BundledExternalSkillCandidate> {
        self.skillManager.getBundledExternalSkillCandidates()
    }

    #[allow(non_snake_case)]
    pub fn importBundledExternalSkill(&self, skillName: &str) -> Result<SkillPackage, String> {
        self.skillManager.importBundledExternalSkill(skillName)
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
        self.skillVisibilityPreferences
            .isSkillVisibleToAi(skillName)
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
    pub fn ensureQuickPluginCreatorSkillVisible(&self) -> Result<SkillPackage, String> {
        let skill = self.skillManager.ensureQuickPluginCreatorBundledSkill()?;
        self.skillVisibilityPreferences
            .setSkillVisibleToAi(&skill.name, true)
            .map_err(|error| error.to_string())?;
        Ok(skill)
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
    pub fn importSkillFromGitHubRepo(&self, repoUrl: &str) -> String {
        let Some(target) = parseGitHubSkillTarget(repoUrl) else {
            return "Invalid GitHub repository URL".to_string();
        };

        let refName = match target.refName.clone() {
            Some(value) => value,
            None => match getGithubDefaultBranch(&target.owner, &target.repo) {
                Some(value) => value,
                None => {
                    return format!(
                        "Cannot determine default branch for {}/{}",
                        target.owner, target.repo
                    )
                }
            },
        };

        let zipUrl = format!(
            "https://codeload.github.com/{}/{}/zip/{}",
            target.owner,
            target.repo,
            encodePathSegment(&refName)
        );
        let tempFile = std::env::temp_dir().join(format!(
            "operit_skill_{}_{}_{}.zip",
            sanitizeTempPart(&target.owner),
            sanitizeTempPart(&target.repo),
            currentTimeMillis()
        ));

        if let Err(error) = downloadToFile(&zipUrl, &tempFile) {
            let _ = fs::remove_file(&tempFile);
            return format!("Failed to download skill zip: {error}");
        }

        let skillsRootDir = PathBuf::from(self.getSkillsDirectoryPath());
        let beforeDirs = directoryNameSet(&skillsRootDir);
        let result = self
            .skillManager
            .importSkillFromZipWithSubDir(&tempFile, target.subDir.as_deref());
        let _ = fs::remove_file(&tempFile);

        if result.starts_with("Imported skill:") {
            let afterDirs = directoryNameSet(&skillsRootDir);
            let newDirs = afterDirs
                .into_iter()
                .filter(|name| !beforeDirs.iter().any(|before| before == name))
                .collect::<Vec<_>>();
            if newDirs.len() == 1 {
                let markerPath = skillsRootDir.join(&newDirs[0]).join(".operit_repo_url");
                let _ = fs::write(markerPath, repoUrl.trim());
            }
        }

        result
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
            return "Skill id may only contain letters, numbers, dot, underscore, and hyphen"
                .to_string();
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
                let safeName =
                    ensureUniqueFileName(&sanitizeAttachmentName(&displayName), &mut usedFileNames);
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

#[allow(non_snake_case)]
fn parseGitHubSkillTarget(inputUrlRaw: &str) -> Option<GitHubSkillTarget> {
    let inputUrl = inputUrlRaw.trim();
    if inputUrl.is_empty() {
        return None;
    }
    let urlWithScheme = if inputUrl.starts_with("http://") || inputUrl.starts_with("https://") {
        inputUrl.to_string()
    } else {
        format!("https://{inputUrl}")
    };
    let urlNoFragment = urlWithScheme.split('#').next().unwrap_or_default();
    let url = Url::parse(urlNoFragment).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    let segments = url
        .path_segments()
        .map(|segments| segments.filter(|item| !item.is_empty()).collect::<Vec<_>>())?;

    if host == "github.com" || host.ends_with(".github.com") {
        if segments.len() < 2 {
            return None;
        }
        let owner = segments[0].to_string();
        let repo = cleanRepoName(segments[1]);
        if owner.is_empty() || repo.is_empty() {
            return None;
        }

        let mut refName = None;
        let mut subDir = None;
        if segments.len() >= 4 && (segments[2] == "tree" || segments[2] == "blob") {
            refName = Some(segments[3].to_string());
            let remainder = if segments.len() > 4 {
                segments[4..].join("/")
            } else {
                String::new()
            };
            if !remainder.is_empty() {
                subDir = if segments[2] == "blob" {
                    if remainder.ends_with("SKILL.md") || remainder.ends_with("skill.md") {
                        remainder.rsplit_once('/').map(|(dir, _)| dir.to_string())
                    } else {
                        remainder
                            .rsplit_once('/')
                            .map(|(dir, _)| dir.to_string())
                            .filter(|dir| !dir.is_empty())
                    }
                } else {
                    Some(remainder)
                };
            }
        }
        return Some(GitHubSkillTarget {
            owner,
            repo,
            refName,
            subDir,
        });
    }

    if host == "raw.githubusercontent.com" {
        if segments.len() < 4 {
            return None;
        }
        let owner = segments[0].to_string();
        let repo = cleanRepoName(segments[1]);
        let refName = Some(segments[2].to_string());
        let remainder = segments[3..].join("/");
        let subDir = if remainder.ends_with("SKILL.md") || remainder.ends_with("skill.md") {
            remainder.rsplit_once('/').map(|(dir, _)| dir.to_string())
        } else {
            remainder
                .rsplit_once('/')
                .map(|(dir, _)| dir.to_string())
                .filter(|dir| !dir.is_empty())
        };
        return Some(GitHubSkillTarget {
            owner,
            repo,
            refName,
            subDir,
        });
    }

    None
}

#[allow(non_snake_case)]
fn cleanRepoName(repoRaw: &str) -> String {
    repoRaw.trim_end_matches(".git").to_string()
}

#[allow(non_snake_case)]
fn getGithubDefaultBranch(owner: &str, repoName: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{owner}/{repoName}");
    let response = defaultHttpHost()
        .executeHttpRequest(HttpRequestData {
            url,
            method: "GET".to_string(),
            headers: vec![
                (
                    "Accept".to_string(),
                    "application/vnd.github.v3+json".to_string(),
                ),
                ("User-Agent".to_string(), "Operit-Market".to_string()),
            ],
            body: Vec::new(),
            formFields: Vec::new(),
            fileParts: Vec::new(),
            connectTimeoutSeconds: 15,
            readTimeoutSeconds: 15,
            followRedirects: true,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        })
        .ok()?;
    if !(200..300).contains(&response.statusCode) {
        return None;
    }
    let value = serde_json::from_slice::<serde_json::Value>(&response.body).ok()?;
    value
        .get("default_branch")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

#[allow(non_snake_case)]
fn downloadToFile(url: &str, outFile: &Path) -> Result<(), String> {
    let response = defaultHttpHost()
        .executeHttpRequest(HttpRequestData {
            url: url.to_string(),
            method: "GET".to_string(),
            headers: vec![(
                "User-Agent".to_string(),
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string(),
            )],
            body: Vec::new(),
            formFields: Vec::new(),
            fileParts: Vec::new(),
            connectTimeoutSeconds: 30,
            readTimeoutSeconds: 30,
            followRedirects: true,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        })
        .map_err(|error| error.to_string())?;
    if !(200..300).contains(&response.statusCode) {
        return Err(format!("HTTP {}", response.statusCode));
    }
    fs::write(outFile, response.body).map_err(|error| error.to_string())?;
    Ok(())
}

#[allow(non_snake_case)]
fn directoryNameSet(root: &Path) -> Vec<String> {
    fs::read_dir(root)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

#[allow(non_snake_case)]
fn encodePathSegment(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        let ch = *byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            out.push(ch);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[allow(non_snake_case)]
fn sanitizeTempPart(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}
