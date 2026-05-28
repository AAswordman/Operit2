use std::fs;
use std::io::Cursor;
use std::path::Path;

use crate::core::tools::packTool::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgLoadResult, ToolPkgMainRegistrationParseResult, ToolPkgSourceType,
};
use crate::core::tools::ToolPackage::ToolPackage;

pub struct ToolPkgLoader;

impl ToolPkgLoader {
    #[allow(non_snake_case)]
    pub fn loadToolPkgFromExternalFile<FParseJsPackage>(
        file: &Path,
        parseJsPackage: FParseJsPackage,
    ) -> Result<ToolPkgLoadResult, String>
    where
        FParseJsPackage: Fn(&str) -> Result<ToolPackage, String>,
    {
        let zipFile = fs::File::open(file).map_err(|error| error.to_string())?;
        let mut archive = zip::ZipArchive::new(zipFile).map_err(|error| error.to_string())?;
        let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
        ToolPkgArchiveParser::parseToolPkgFromIndexedEntries(
            &entryIndex,
            |entryName| {
                ToolPkgArchiveParser::readZipEntryText(&mut archive, &entryIndex, entryName)
            },
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
                parseMainRegistration(mainScriptText, toolPkgId, mainScriptPath)
            },
            |packageName, error| {
                eprintln!("ToolPkg package load error [{packageName}]: {error}");
            },
        )
    }

    #[allow(non_snake_case)]
    pub fn loadToolPkgFromBuiltInAsset<FParseJsPackage>(
        assetName: &str,
        bytes: &'static [u8],
        parseJsPackage: FParseJsPackage,
    ) -> Result<ToolPkgLoadResult, String>
    where
        FParseJsPackage: Fn(&str) -> Result<ToolPackage, String>,
    {
        let cursor = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|error| error.to_string())?;
        let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
        ToolPkgArchiveParser::parseToolPkgFromIndexedEntries(
            &entryIndex,
            |entryName| {
                ToolPkgArchiveParser::readZipEntryText(&mut archive, &entryIndex, entryName)
            },
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
                parseMainRegistration(mainScriptText, toolPkgId, mainScriptPath)
            },
            |packageName, error| {
                eprintln!("Built-in ToolPkg package load error [{packageName}]: {error}");
            },
        )
    }
}

#[allow(non_snake_case)]
fn parseMainRegistration(
    mainScriptText: &str,
    toolPkgId: &str,
    mainScriptPath: &str,
) -> ToolPkgMainRegistrationParseResult {
    let jsEngine =
        crate::core::tools::javascript::JsEngine::JsEngine::newToolPkgRegistrationEngine();
    crate::core::tools::packTool::ToolPkgMainRegistrationScriptParser::ToolPkgMainRegistrationScriptParser::parse(
        mainScriptText,
        toolPkgId,
        mainScriptPath,
        &jsEngine,
    )
}
