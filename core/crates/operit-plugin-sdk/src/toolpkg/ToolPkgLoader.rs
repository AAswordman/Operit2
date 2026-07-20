use std::io::Cursor;
use std::sync::Arc;

use crate::javascript::JsExecutionEngine;
use crate::toolpkg::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgLoadResult, ToolPkgMainRegistrationParseResult, ToolPkgSourceType,
};
use crate::JsPackageLoader::JsPackageLoader;
use operit_host_api::FileSystemHost;

/// Loads ToolPkg archives and executes their main registration scripts.
pub struct ToolPkgLoader;

impl ToolPkgLoader {
    /// Loads a ToolPkg archive through the supplied file-system host.
    #[allow(non_snake_case)]
    pub fn loadToolPkgFromExternalFile<FReportPackageLoadError>(
        fileSystemHost: &dyn FileSystemHost,
        sourcePath: &str,
        jsEngine: &dyn JsExecutionEngine,
        reportPackageLoadError: FReportPackageLoadError,
    ) -> Result<ToolPkgLoadResult, String>
    where
        FReportPackageLoadError: Fn(&str, &str),
    {
        let archiveBytes = fileSystemHost
            .readFileBytes(sourcePath)
            .map_err(|error| error.to_string())?;
        Self::loadToolPkgFromArchiveBytes(
            &archiveBytes,
            ToolPkgSourceType::EXTERNAL,
            sourcePath,
            false,
            jsEngine,
            reportPackageLoadError,
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
        Self::loadToolPkgFromArchiveBytes(
            bytes,
            ToolPkgSourceType::ASSET,
            assetName,
            true,
            jsEngine,
            reportPackageLoadError,
        )
    }

    /// Loads a raw protected ToolPkg ZIP after its local market installation seal has verified.
    #[allow(non_snake_case)]
    pub fn loadToolPkgFromMarketFile<FReportPackageLoadError>(
        fileSystemHost: &dyn FileSystemHost,
        sourcePath: &str,
        jsEngine: &dyn JsExecutionEngine,
        reportPackageLoadError: FReportPackageLoadError,
    ) -> Result<ToolPkgLoadResult, String>
    where
        FReportPackageLoadError: Fn(&str, &str),
    {
        let archiveBytes = fileSystemHost
            .readFileBytes(sourcePath)
            .map_err(|error| error.to_string())?;
        Self::loadToolPkgFromArchiveBytes(
            &archiveBytes,
            ToolPkgSourceType::MARKET,
            sourcePath,
            false,
            jsEngine,
            reportPackageLoadError,
        )
    }

    /// Parses one already-unwrapped ToolPkg ZIP using its trusted source classification.
    #[allow(non_snake_case)]
    fn loadToolPkgFromArchiveBytes<FReportPackageLoadError>(
        archiveBytes: &[u8],
        sourceType: ToolPkgSourceType,
        sourcePath: &str,
        isBuiltIn: bool,
        jsEngine: &dyn JsExecutionEngine,
        reportPackageLoadError: FReportPackageLoadError,
    ) -> Result<ToolPkgLoadResult, String>
    where
        FReportPackageLoadError: Fn(&str, &str),
    {
        let cursor = Cursor::new(archiveBytes);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|error| error.to_string())?;
        let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
        let textResources = Arc::new(readToolPkgTextResources(&mut archive, &entryIndex));
        ToolPkgArchiveParser::parseToolPkgFromIndexedEntries(
            &entryIndex,
            |entryName| readIndexedTextResource(&textResources, &entryIndex, entryName),
            |entryName| {
                ToolPkgArchiveParser::readZipEntryPrefix(
                    &mut archive,
                    &entryIndex,
                    entryName,
                    crate::toolpkg::ToolPkgProtection::MARKET_ONLY_PROTECTION_HEADER_SIZE,
                )
            },
            sourceType,
            sourcePath,
            isBuiltIn,
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
