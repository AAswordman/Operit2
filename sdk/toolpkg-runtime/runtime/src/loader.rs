use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use operit_tools::packTool::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgEntryIndex, ToolPkgSourceType,
};

use crate::javascript::JsEngine::JsEngine;
use crate::models::{ToolPkgLoadOutcome, ToolPkgPackageLoadError};
use crate::pack::ToolPkgJsPackageParser::parseToolPkgJsPackage;
use crate::pack::ToolPkgMainRegistrationScriptParser::ToolPkgMainRegistrationScriptParser;

#[derive(Clone)]
pub struct LoadedToolPkg {
    pub outcome: ToolPkgLoadOutcome,
    pub mainScript: String,
    pub mainScriptPath: String,
    pub textResources: Arc<BTreeMap<String, String>>,
}

#[allow(non_snake_case)]
pub(crate) fn loadToolPkgFileWithEngine(
    path: &Path,
    engine: &JsEngine,
    languageCode: &str,
) -> Result<LoadedToolPkg, String> {
    let zipFile = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(zipFile).map_err(|error| error.to_string())?;
    let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
    let textResources = Arc::new(readToolPkgTextResources(&mut archive, &entryIndex));
    let mut packageLoadErrors = Vec::new();
    let package = ToolPkgArchiveParser::parseToolPkgFromIndexedEntries(
        &entryIndex,
        |entryName| readIndexedTextResource(&textResources, &entryIndex, entryName),
        ToolPkgSourceType::EXTERNAL,
        &path.to_string_lossy(),
        false,
        |jsContent, reportPackageLoadError| match parseToolPkgJsPackage(jsContent) {
            Ok(package) => Some(package),
            Err(error) => {
                reportPackageLoadError(String::new(), error);
                None
            }
        },
        |mainScriptText, toolPkgId, mainScriptPath| {
            ToolPkgMainRegistrationScriptParser::parseWithLanguageAndTextResources(
                mainScriptText,
                toolPkgId,
                mainScriptPath,
                engine,
                languageCode,
                Some(textResources.clone()),
            )
        },
        |packageName, message| {
            packageLoadErrors.push(ToolPkgPackageLoadError {
                packageName,
                message,
            });
        },
    )?;
    let mainScriptPath = package.containerRuntime.mainEntry.clone();
    let mainScript = readIndexedTextResource(&textResources, &entryIndex, &mainScriptPath)
        .ok_or_else(|| format!("ToolPkg main script is unavailable: {mainScriptPath}"))?;
    Ok(LoadedToolPkg {
        outcome: ToolPkgLoadOutcome {
            package,
            packageLoadErrors,
        },
        mainScript,
        mainScriptPath,
        textResources,
    })
}

#[allow(non_snake_case)]
pub(crate) fn readToolPkgTextResourceFromMap(
    textResources: &BTreeMap<String, String>,
    resourcePath: &str,
) -> Option<String> {
    let normalized = resourcePath
        .trim()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_ascii_lowercase();
    textResources.get(&normalized).cloned()
}

#[allow(non_snake_case)]
fn readToolPkgTextResources<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    entryIndex: &ToolPkgEntryIndex,
) -> BTreeMap<String, String> {
    let mut resources = BTreeMap::new();
    for entryName in &entryIndex.entryNames {
        if let Some(text) = ToolPkgArchiveParser::readZipEntryText(archive, entryIndex, entryName) {
            resources.insert(entryName.to_ascii_lowercase(), text);
        }
    }
    resources
}

#[allow(non_snake_case)]
fn readIndexedTextResource(
    textResources: &BTreeMap<String, String>,
    entryIndex: &ToolPkgEntryIndex,
    rawPath: &str,
) -> Option<String> {
    let entryName = entryIndex.resolveEntryName(rawPath)?;
    textResources.get(&entryName.to_ascii_lowercase()).cloned()
}
