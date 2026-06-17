use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use crate::core::tools::javascript::JsEngine::JsEngine;
use crate::core::tools::packTool::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgLoadResult, ToolPkgMainRegistrationParseResult, ToolPkgSourceType,
};
use crate::core::tools::ToolPackage::ToolPackage;
use crate::util::AppLogger::AppLogger;

const TAG: &str = "ToolPkg";

pub struct ToolPkgLoader;

impl ToolPkgLoader {
    #[allow(non_snake_case)]
    pub fn loadToolPkgFromExternalFile<FParseJsPackage>(
        file: &Path,
        jsEngine: &JsEngine,
        parseJsPackage: FParseJsPackage,
    ) -> Result<ToolPkgLoadResult, String>
    where
        FParseJsPackage: Fn(&str) -> Result<ToolPackage, String>,
    {
        let zipFile = fs::File::open(file).map_err(|error| error.to_string())?;
        let mut archive = zip::ZipArchive::new(zipFile).map_err(|error| error.to_string())?;
        let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
        let textResources = Arc::new(readToolPkgTextResources(&mut archive, &entryIndex));
        ToolPkgArchiveParser::parseToolPkgFromIndexedEntries(
            &entryIndex,
            |entryName| readIndexedTextResource(&textResources, &entryIndex, entryName),
            ToolPkgSourceType::EXTERNAL,
            &file.to_string_lossy(),
            false,
            |jsContent, reportPackageLoadError| match parseJsPackage(jsContent) {
                Ok(package) => Some(package),
                Err(error) => {
                    reportPackageLoadError(String::new(), error);
                    None
                }
            },
            |mainScriptText, toolPkgId, mainScriptPath| {
                parseMainRegistration(
                    mainScriptText,
                    toolPkgId,
                    mainScriptPath,
                    jsEngine,
                    textResources.clone(),
                )
            },
            |packageName, error| {
                AppLogger::e(
                    TAG,
                    &format!("ToolPkg package load error [{packageName}]: {error}"),
                );
            },
        )
    }

    #[allow(non_snake_case)]
    pub fn loadToolPkgFromBuiltInAsset<FParseJsPackage>(
        assetName: &str,
        bytes: &'static [u8],
        jsEngine: &JsEngine,
        parseJsPackage: FParseJsPackage,
    ) -> Result<ToolPkgLoadResult, String>
    where
        FParseJsPackage: Fn(&str) -> Result<ToolPackage, String>,
    {
        let cursor = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|error| error.to_string())?;
        let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
        let textResources = Arc::new(readToolPkgTextResources(&mut archive, &entryIndex));
        ToolPkgArchiveParser::parseToolPkgFromIndexedEntries(
            &entryIndex,
            |entryName| readIndexedTextResource(&textResources, &entryIndex, entryName),
            ToolPkgSourceType::ASSET,
            assetName,
            true,
            |jsContent, reportPackageLoadError| match parseJsPackage(jsContent) {
                Ok(package) => Some(package),
                Err(error) => {
                    reportPackageLoadError(String::new(), error);
                    None
                }
            },
            |mainScriptText, toolPkgId, mainScriptPath| {
                parseMainRegistration(
                    mainScriptText,
                    toolPkgId,
                    mainScriptPath,
                    jsEngine,
                    textResources.clone(),
                )
            },
            |packageName, error| {
                AppLogger::e(
                    TAG,
                    &format!("Built-in ToolPkg package load error [{packageName}]: {error}"),
                );
            },
        )
    }

    #[allow(non_snake_case)]
    pub fn loadToolPkgFromBuiltInAssetFile<FParseJsPackage>(
        assetName: &str,
        file: &Path,
        jsEngine: &JsEngine,
        parseJsPackage: FParseJsPackage,
    ) -> Result<ToolPkgLoadResult, String>
    where
        FParseJsPackage: Fn(&str) -> Result<ToolPackage, String>,
    {
        let zipFile = fs::File::open(file).map_err(|error| error.to_string())?;
        let mut archive = zip::ZipArchive::new(zipFile).map_err(|error| error.to_string())?;
        let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
        let textResources = Arc::new(readToolPkgTextResources(&mut archive, &entryIndex));
        ToolPkgArchiveParser::parseToolPkgFromIndexedEntries(
            &entryIndex,
            |entryName| readIndexedTextResource(&textResources, &entryIndex, entryName),
            ToolPkgSourceType::ASSET,
            assetName,
            true,
            |jsContent, reportPackageLoadError| match parseJsPackage(jsContent) {
                Ok(package) => Some(package),
                Err(error) => {
                    reportPackageLoadError(String::new(), error);
                    None
                }
            },
            |mainScriptText, toolPkgId, mainScriptPath| {
                parseMainRegistration(
                    mainScriptText,
                    toolPkgId,
                    mainScriptPath,
                    jsEngine,
                    textResources.clone(),
                )
            },
            |packageName, error| {
                AppLogger::e(
                    TAG,
                    &format!("Built-in ToolPkg package load error [{packageName}]: {error}"),
                );
            },
        )
    }
}

#[allow(non_snake_case)]
fn parseMainRegistration(
    mainScriptText: &str,
    toolPkgId: &str,
    mainScriptPath: &str,
    jsEngine: &JsEngine,
    textResources: Arc<std::collections::BTreeMap<String, String>>,
) -> ToolPkgMainRegistrationParseResult {
    crate::core::tools::packTool::ToolPkgMainRegistrationScriptParser::ToolPkgMainRegistrationScriptParser::parseWithTextResources(
        mainScriptText,
        toolPkgId,
        mainScriptPath,
        jsEngine,
        Some(textResources),
    )
}

#[allow(non_snake_case)]
fn readToolPkgTextResources<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    entryIndex: &crate::core::tools::packTool::ToolPkgParser::ToolPkgEntryIndex,
) -> std::collections::BTreeMap<String, String> {
    let mut resources = std::collections::BTreeMap::new();
    for entryName in &entryIndex.entryNames {
        if let Some(text) = ToolPkgArchiveParser::readZipEntryText(archive, entryIndex, entryName) {
            resources.insert(entryName.to_ascii_lowercase(), text);
        }
    }
    resources
}

#[allow(non_snake_case)]
fn readIndexedTextResource(
    textResources: &std::collections::BTreeMap<String, String>,
    entryIndex: &crate::core::tools::packTool::ToolPkgParser::ToolPkgEntryIndex,
    rawPath: &str,
) -> Option<String> {
    let entryName = entryIndex.resolveEntryName(rawPath)?;
    textResources.get(&entryName.to_ascii_lowercase()).cloned()
}
