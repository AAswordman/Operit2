use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::tools::packTool::PackageManager::PublishablePackageSource;
use operit_tools::packTool::ToolPkgParser::ToolPkgArchiveParser;
use crate::data::api::MarketStatsApiService::{
    ArtifactProjectDetailResponse, ArtifactProjectNodeResponse,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalArtifactAuthorDeclaration {
    pub hasAuthorField: bool,
    pub declaredAuthorSlotCount: i32,
}

#[derive(Clone, Debug, Default)]
pub struct ArtifactAuthorValidation;

impl ArtifactAuthorValidation {
    pub fn new() -> Self {
        Self
    }

    #[allow(non_snake_case)]
    pub fn inspectLocalArtifactAuthorDeclaration(
        &self,
        source: PublishablePackageSource,
    ) -> Result<LocalArtifactAuthorDeclaration, String> {
        let sourceFile = PathBuf::from(&source.sourcePath);
        if !sourceFile.exists() || !sourceFile.is_file() {
            return Ok(LocalArtifactAuthorDeclaration {
                hasAuthorField: false,
                declaredAuthorSlotCount: 0,
            });
        }

        if source.isToolPkg {
            inspectToolPkgAuthorDeclaration(&sourceFile)
        } else {
            inspectJsAuthorDeclaration(&sourceFile)
        }
    }

    #[allow(non_snake_case)]
    pub fn collectArtifactPredecessorPublisherLogins(
        &self,
        project: ArtifactProjectDetailResponse,
        parentNodeIds: Vec<String>,
    ) -> Result<Vec<String>, String> {
        let nodeById = project
            .nodes
            .iter()
            .map(|node| (node.nodeId.as_str(), node))
            .collect::<BTreeMap<_, _>>();
        let mut publishers = Vec::new();

        for nodeId in parentNodeIds
            .iter()
            .map(|nodeId| nodeId.trim())
            .filter(|nodeId| !nodeId.is_empty())
        {
            let node = nodeById
                .get(nodeId)
                .ok_or_else(|| format!("找不到前驱节点 `{nodeId}`，无法校验作者数量。"))?;
            let publisherLogin = artifactNodePublisherLogin(node);
            if !publisherLogin.is_empty()
                && !publishers
                    .iter()
                    .any(|existing: &String| existing == &publisherLogin)
            {
                publishers.push(publisherLogin);
            }
        }

        Ok(publishers)
    }
}

#[allow(non_snake_case)]
fn inspectJsAuthorDeclaration(sourceFile: &Path) -> Result<LocalArtifactAuthorDeclaration, String> {
    let jsContent = fs::read_to_string(sourceFile).map_err(|error| error.to_string())?;
    let lowerPath = sourceFile.to_string_lossy().to_ascii_lowercase();
    let metadataString = if lowerPath.ends_with(".js") || lowerPath.ends_with(".ts") {
        extractJsMetadata(&jsContent)
    } else {
        jsContent
    };
    inspectAuthorDeclarationFromMetadata(&metadataString)
}

#[allow(non_snake_case)]
fn inspectToolPkgAuthorDeclaration(
    sourceFile: &Path,
) -> Result<LocalArtifactAuthorDeclaration, String> {
    let file = fs::File::open(sourceFile).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
    let manifestEntryName = findToolPkgManifestEntryName(&entryIndex.entryNames)
        .ok_or_else(|| "toolpkg 缺少 manifest.hjson 或 manifest.json".to_string())?;
    let manifestText =
        ToolPkgArchiveParser::readZipEntryText(&mut archive, &entryIndex, &manifestEntryName)
            .ok_or_else(|| "无法读取 toolpkg manifest".to_string())?;
    inspectAuthorDeclarationFromMetadata(&manifestText)
}

#[allow(non_snake_case)]
fn inspectAuthorDeclarationFromMetadata(
    metadataString: &str,
) -> Result<LocalArtifactAuthorDeclaration, String> {
    let normalized = normalizeHjsonLikeMetadata(metadataString);
    let value = json5::from_str::<serde_json::Value>(&normalized)
        .map_err(|error| format!("Package metadata parse error: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "Package metadata must be an object".to_string())?;
    let author = object.get("author");
    Ok(LocalArtifactAuthorDeclaration {
        hasAuthorField: author.is_some(),
        declaredAuthorSlotCount: countDeclaredAuthorSlots(author),
    })
}

#[allow(non_snake_case)]
fn extractJsMetadata(jsContent: &str) -> String {
    let metadataPattern =
        regex::Regex::new(r"/\*\s*METADATA\s*([\s\S]*?)\*/").expect("valid metadata regex");
    metadataPattern
        .captures(jsContent)
        .and_then(|captures| captures.get(1))
        .map(|metadata| metadata.as_str().trim().to_string())
        .unwrap_or_else(|| "{}".to_string())
}

#[allow(non_snake_case)]
fn findToolPkgManifestEntryName(entryNames: &BTreeSet<String>) -> Option<String> {
    entryNames
        .iter()
        .find(|entry| entry.eq_ignore_ascii_case("manifest.hjson"))
        .cloned()
        .or_else(|| {
            entryNames
                .iter()
                .find(|entry| entry.eq_ignore_ascii_case("manifest.json"))
                .cloned()
        })
        .or_else(|| {
            entryNames
                .iter()
                .find(|entry| {
                    Path::new(entry)
                        .file_name()
                        .and_then(|value| value.to_str())
                        .is_some_and(|fileName| fileName.eq_ignore_ascii_case("manifest.hjson"))
                })
                .cloned()
        })
        .or_else(|| {
            entryNames
                .iter()
                .find(|entry| {
                    Path::new(entry)
                        .file_name()
                        .and_then(|value| value.to_str())
                        .is_some_and(|fileName| fileName.eq_ignore_ascii_case("manifest.json"))
                })
                .cloned()
        })
}

#[allow(non_snake_case)]
fn countDeclaredAuthorSlots(value: Option<&serde_json::Value>) -> i32 {
    match value {
        None | Some(serde_json::Value::Null) => 0,
        Some(serde_json::Value::Array(items)) => items.len() as i32,
        Some(_) => 1,
    }
}

#[allow(non_snake_case)]
fn artifactNodePublisherLogin(node: &ArtifactProjectNodeResponse) -> String {
    let declaredPublisherLogin = node.publisherLogin.trim();
    if declaredPublisherLogin.is_empty() {
        node.issue.user.login.trim().to_string()
    } else {
        declaredPublisherLogin.to_string()
    }
}

#[allow(non_snake_case)]
fn normalizeHjsonLikeMetadata(input: &str) -> String {
    let mut lines = Vec::new();
    for rawLine in input.lines() {
        let line = stripInlineComment(rawLine).trim().to_string();
        if line.is_empty() {
            continue;
        }
        lines.push(normalizeBareWords(&line));
    }

    let mut output = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            let previous = lines[index - 1].trim_end();
            let current = line.trim_start();
            if needsCommaBetween(previous, current) {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str(line);
    }
    output
}

#[allow(non_snake_case)]
fn stripInlineComment(line: &str) -> String {
    let mut inString = false;
    let mut quote = '\0';
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if inString {
            if ch == quote && (index == 0 || chars[index - 1] != '\\') {
                inString = false;
            }
            index += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            inString = true;
            quote = ch;
            index += 1;
            continue;
        }
        if ch == '/' && index + 1 < chars.len() && chars[index + 1] == '/' {
            return chars[..index].iter().collect();
        }
        index += 1;
    }
    line.to_string()
}

#[allow(non_snake_case)]
fn normalizeBareWords(line: &str) -> String {
    let mut out = String::new();
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut inString = false;
    let mut quote = '\0';
    while index < chars.len() {
        let ch = chars[index];
        out.push(ch);
        if inString {
            if ch == quote && (index == 0 || chars[index - 1] != '\\') {
                inString = false;
            }
            index += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            inString = true;
            quote = ch;
            index += 1;
            continue;
        }
        if ch == ':' {
            let mut lookahead = index + 1;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                out.push(chars[lookahead]);
                lookahead += 1;
            }
            if lookahead >= chars.len() {
                index = lookahead;
                continue;
            }
            let next = chars[lookahead];
            if next == '"'
                || next == '\''
                || next == '{'
                || next == '['
                || next == '-'
                || next.is_ascii_digit()
            {
                index = lookahead;
                continue;
            }
            let mut end = lookahead;
            while end < chars.len() {
                let c = chars[end];
                if c == ',' || c == '}' || c == ']' {
                    break;
                }
                end += 1;
            }
            let rawValue = chars[lookahead..end].iter().collect::<String>();
            let value = rawValue.trim();
            let lower = value.to_ascii_lowercase();
            if matches!(lower.as_str(), "true" | "false" | "null") || value.is_empty() {
                out.push_str(value);
            } else {
                out.push('"');
                out.push_str(&value.replace('"', "\\\""));
                out.push('"');
            }
            index = end;
            continue;
        }
        index += 1;
    }
    out
}

#[allow(non_snake_case)]
fn needsCommaBetween(previous: &str, current: &str) -> bool {
    if previous.is_empty()
        || previous.ends_with(',')
        || previous.ends_with('{')
        || previous.ends_with('[')
        || current.starts_with('}')
        || current.starts_with(']')
    {
        return false;
    }
    true
}
