use std::io::Cursor;
use std::sync::Arc;

use crate::javascript::{ToolPkgRegistrationExecutor, ToolPkgTextResourceHost, ToolPkgConfigScope};
use crate::toolpkg::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgLoadResult, ToolPkgSourceType,
};
use crate::JsPackageLoader::JsPackageLoader;

/// Loads ToolPkg archives and executes their main registration scripts.
pub struct ToolPkgLoader;

impl ToolPkgLoader {
    /// Parses one already-unwrapped ToolPkg ZIP using its trusted source classification.
    #[allow(non_snake_case)]
    pub async fn loadToolPkgFromArchiveBytes<FReportPackageLoadError>(
        archiveBytes: &[u8],
        sourceType: ToolPkgSourceType,
        sourcePath: &str,
        isBuiltIn: bool,
        configScope: ToolPkgConfigScope,
        jsEngine: &dyn ToolPkgRegistrationExecutor,
        textResources: Arc<dyn ToolPkgTextResourceHost>,
        reportPackageLoadError: FReportPackageLoadError,
    ) -> Result<ToolPkgLoadResult, String>
    where
        FReportPackageLoadError: Fn(&str, &str),
    {
        let requiredScope = ToolPkgArchiveParser::readInstallationScope(archiveBytes)?;
        let configScope = match (isBuiltIn, requiredScope) {
            (true, Some(scope)) => scope,
            (false, Some(scope)) if scope != configScope => {
                return Err(format!("ToolPkg requires {} installation scope, not {}", scope.as_str(), configScope.as_str()));
            }
            _ => configScope,
        };
        ToolPkgArchiveParser::readThemeTemplates(archiveBytes)?;
        let cursor = Cursor::new(archiveBytes);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|error| error.to_string())?;
        let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
        let archive = std::cell::RefCell::new(archive);
        let result = ToolPkgArchiveParser::parseToolPkgFromIndexedEntries(
            &entryIndex,
            |entryName| ToolPkgArchiveParser::readZipEntryText(&mut *archive.borrow_mut(), &entryIndex, entryName),
            |entryName| {
                ToolPkgArchiveParser::readZipEntryPrefix(
                    &mut *archive.borrow_mut(),
                    &entryIndex,
                    entryName,
                    crate::toolpkg::ToolPkgProtection::MARKET_ONLY_PROTECTION_HEADER_SIZE,
                )
            },
            sourceType,
            sourcePath,
            isBuiltIn,
            |jsContent, apiVersion, reportPackageLoadError| match JsPackageLoader::parse_with_default_api_version(jsContent, apiVersion) {
                Ok(package) => Some(package),
                Err(error) => {
                    reportPackageLoadError(String::new(), error);
                    None
                }
            },
            |mainScriptText, toolPkgId, mainScriptPath, apiVersion, environmentNames| {
                let textResources = textResources.clone();
                async move { crate::toolpkg::ToolPkgMainRegistrationScriptParser::ToolPkgMainRegistrationScriptParser::parseWithTextResources(
                    &mainScriptText,
                    &toolPkgId,
                    &mainScriptPath,
                    &apiVersion,
                    &environmentNames,
                    configScope,
                    jsEngine,
                    Some(textResources),
                ).await }
            },
            |packageName, error| reportPackageLoadError(&packageName, &error),
        ).await;
        result
    }
}
