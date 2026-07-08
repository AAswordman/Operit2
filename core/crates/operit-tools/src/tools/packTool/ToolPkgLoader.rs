use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use operit_tools::tools::javascript::JsExecutionEngine;
use operit_tools::tools::packTool::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgLoadResult, ToolPkgMainRegistrationParseResult, ToolPkgSourceType,
};
use operit_tools::tools::ToolPackage::ToolPackage;
use operit_util::AppLogger::AppLogger;

const TAG: &str = "ToolPkg";

pub struct ToolPkgLoader;

impl ToolPkgLoader {
    #[allow(non_snake_case)]
    pub fn loadToolPkgFromExternalFile<FParseJsPackage>(
        file: &Path,
        jsEngine: &dyn JsExecutionEngine,
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
        jsEngine: &dyn JsExecutionEngine,
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

}

#[allow(non_snake_case)]
fn parseMainRegistration(
    mainScriptText: &str,
    toolPkgId: &str,
    mainScriptPath: &str,
    jsEngine: &dyn JsExecutionEngine,
    textResources: Arc<std::collections::BTreeMap<String, String>>,
) -> ToolPkgMainRegistrationParseResult {
    operit_tools::tools::packTool::ToolPkgMainRegistrationScriptParser::ToolPkgMainRegistrationScriptParser::parseWithTextResources(
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
    entryIndex: &operit_tools::tools::packTool::ToolPkgParser::ToolPkgEntryIndex,
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
    entryIndex: &operit_tools::tools::packTool::ToolPkgParser::ToolPkgEntryIndex,
    rawPath: &str,
) -> Option<String> {
    let entryName = entryIndex.resolveEntryName(rawPath)?;
    textResources.get(&entryName.to_ascii_lowercase()).cloned()
}
