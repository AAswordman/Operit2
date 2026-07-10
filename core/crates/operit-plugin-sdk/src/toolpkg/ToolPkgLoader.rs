use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use crate::javascript::JsExecutionEngine;
use crate::package::ToolPackage;
use crate::toolpkg::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgLoadResult, ToolPkgMainRegistrationParseResult, ToolPkgSourceType,
};
use crate::JsPackageLoader::JsPackageLoader;

/// Loads ToolPkg archives and executes their main registration scripts.
pub struct ToolPkgLoader;

impl ToolPkgLoader {
    /// Loads a ToolPkg archive from an external file.
    #[allow(non_snake_case)]
    pub fn loadToolPkgFromExternalFile<FReportPackageLoadError>(
        file: &Path,
        jsEngine: &dyn JsExecutionEngine,
        reportPackageLoadError: FReportPackageLoadError,
    ) -> Result<ToolPkgLoadResult, String>
    where
        FReportPackageLoadError: Fn(&str, &str),
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
            |jsContent, reportPackageLoadError| match JsPackageLoader::parse(jsContent) {
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
            |packageName, error| reportPackageLoadError(&packageName, &error),
        )
    }

    /// Loads a ToolPkg archive from embedded application asset bytes.
    #[allow(non_snake_case)]
    pub fn loadToolPkgFromBuiltInAsset<FReportPackageLoadError>(
        assetName: &str,
        bytes: &'static [u8],
        jsEngine: &dyn JsExecutionEngine,
        reportPackageLoadError: FReportPackageLoadError,
    ) -> Result<ToolPkgLoadResult, String>
    where
        FReportPackageLoadError: Fn(&str, &str),
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
            |jsContent, reportPackageLoadError| match JsPackageLoader::parse(jsContent) {
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
            |packageName, error| reportPackageLoadError(&packageName, &error),
        )
    }
}

/// Parses declarations exported by a ToolPkg main registration script.
#[allow(non_snake_case)]
fn parseMainRegistration(
    mainScriptText: &str,
    toolPkgId: &str,
    mainScriptPath: &str,
    jsEngine: &dyn JsExecutionEngine,
    textResources: Arc<std::collections::BTreeMap<String, String>>,
) -> ToolPkgMainRegistrationParseResult {
    crate::toolpkg::ToolPkgMainRegistrationScriptParser::ToolPkgMainRegistrationScriptParser::parseWithTextResources(
        mainScriptText,
        toolPkgId,
        mainScriptPath,
        jsEngine,
        Some(textResources),
    )
}

/// Reads every UTF-8 archive entry for registration-time resource access.
#[allow(non_snake_case)]
fn readToolPkgTextResources<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    entryIndex: &crate::toolpkg::ToolPkgParser::ToolPkgEntryIndex,
) -> std::collections::BTreeMap<String, String> {
    let mut resources = std::collections::BTreeMap::new();
    for entryName in &entryIndex.entryNames {
        if let Some(text) = ToolPkgArchiveParser::readZipEntryText(archive, entryIndex, entryName) {
            resources.insert(entryName.to_ascii_lowercase(), text);
        }
    }
    resources
}

/// Resolves and reads one normalized text resource from the registration cache.
#[allow(non_snake_case)]
fn readIndexedTextResource(
    textResources: &std::collections::BTreeMap<String, String>,
    entryIndex: &crate::toolpkg::ToolPkgParser::ToolPkgEntryIndex,
    rawPath: &str,
) -> Option<String> {
    let entryName = entryIndex.resolveEntryName(rawPath)?;
    textResources.get(&entryName.to_ascii_lowercase()).cloned()
}
