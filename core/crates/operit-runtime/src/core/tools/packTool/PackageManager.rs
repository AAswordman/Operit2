use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::javascript::JsEngine::JsEngine;
use crate::core::tools::mcp::MCPManager::MCPManager;
use crate::core::tools::mcp::MCPPackage::MCPPackage;
use crate::core::tools::mcp::MCPServerConfig::MCPServerConfig;
use crate::core::tools::packTool::PackageManagerToolPkgFacade::PackageManagerToolPkgFacade;
use crate::core::tools::packTool::ToolPkgLoader::ToolPkgLoader;
use crate::core::tools::packTool::ToolPkgManager::{ToolPkgManager, ToolPkgRuntimeChangeListener};
use crate::core::tools::packTool::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgContainerRuntime, ToolPkgLoadResult, ToolPkgSourceType,
    ToolPkgSubpackageRuntime,
};
use crate::core::tools::skill::SkillManager::SkillManager;
use crate::core::tools::ToolPackage::{
    EnvVar, LocalizedText, PackageTool, PackageToolParameter, ToolPackage, ToolPackageState,
};
use crate::data::mcp::MCPLocalServer::MCPLocalServer;
use crate::data::preferences::SkillVisibilityPreferences::SkillVisibilityPreferences;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use serde::{Deserialize, Serialize};

const ENABLED_PACKAGES_KEY: &str = "imported_packages";
const DISABLED_PACKAGES_KEY: &str = "disabled_packages";

pub type CachedMcpToolInfo = crate::data::mcp::MCPLocalServer::CachedToolInfo;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct PublishablePackageSource {
    pub packageName: String,
    pub displayName: String,
    pub description: String,
    pub author: Vec<String>,
    pub sourcePath: String,
    pub sourceFileName: String,
    pub fileExtension: String,
    pub isToolPkg: bool,
    pub hasDeclaredAuthorField: bool,
    pub declaredAuthorSlotCount: usize,
    pub inferredVersion: Option<String>,
}

#[derive(Clone, Default)]
struct PackageScanSnapshot {
    availablePackages: BTreeMap<String, ToolPackage>,
    toolPkgContainers: BTreeMap<String, ToolPkgContainerRuntime>,
    toolPkgSubpackages: BTreeMap<String, ToolPkgSubpackageRuntime>,
}

#[derive(Clone, Default)]
struct PackageScanCandidateResult {
    phase: String,
    toolPackage: Option<ToolPackage>,
    toolPkgLoadResult: Option<ToolPkgLoadResult>,
    sourcePath: String,
}

#[derive(Clone, Default)]
struct ExternalPackageScanCacheEntry {
    signature: String,
    result: PackageScanCandidateResult,
}

#[derive(Clone)]
pub struct PackageManager {
    activatedPackages: BTreeSet<String>,
    availablePackages: BTreeMap<String, ToolPackage>,
    cachedMcpTools: BTreeMap<String, Vec<CachedMcpToolInfo>>,
    toolPkgManager: ToolPkgManager,
    activePackageStateIds: BTreeMap<String, Option<String>>,
    externalPackageScanCache: BTreeMap<String, ExternalPackageScanCacheEntry>,
    dataStore: PreferencesDataStore,
    storePaths: RuntimeStorePaths,
    context: OperitApplicationContext,
    mcpManager: MCPManager,
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new(RuntimeStorePaths::default())
    }
}

impl PackageManager {
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self::newWithContext(paths, OperitApplicationContext::new())
    }

    #[allow(non_snake_case)]
    pub fn newWithContext(paths: RuntimeStorePaths, context: OperitApplicationContext) -> Self {
        let mut manager = Self {
            activatedPackages: BTreeSet::new(),
            availablePackages: BTreeMap::new(),
            cachedMcpTools: BTreeMap::new(),
            toolPkgManager: ToolPkgManager::new(),
            activePackageStateIds: BTreeMap::new(),
            externalPackageScanCache: BTreeMap::new(),
            dataStore: PreferencesDataStore::new(paths.package_manager_preferences_path()),
            storePaths: paths,
            mcpManager: MCPManager::getInstance(context.clone()),
            context,
        };
        manager.loadAvailablePackages();
        manager
    }

    #[allow(non_snake_case)]
    pub fn getInstance(context: OperitApplicationContext) -> Self {
        let packageManager = crate::core::tools::AIToolHandler::AIToolHandler::getInstance(context)
            .getOrCreatePackageManager();
        let manager = packageManager
            .lock()
            .expect("PackageManager mutex poisoned")
            .clone();
        manager
    }

    pub fn activatePackage(&mut self, packageName: &str) -> bool {
        let normalizedPackageName = self.normalizePackageName(packageName);
        self.activatedPackages.insert(normalizedPackageName)
    }

    #[allow(non_snake_case)]
    pub fn updateContext(&mut self, context: OperitApplicationContext) {
        self.mcpManager = MCPManager::getInstance(context.clone());
        self.context = context;
    }

    #[allow(non_snake_case)]
    pub fn releaseToolPkgExecutionEngine(&self, contextKey: &str) {
        self.toolPkgManager
            .releaseToolPkgExecutionEngine(contextKey);
    }

    #[allow(non_snake_case)]
    pub fn getToolPkgExecutionEngine(&self, contextKey: &str) -> JsEngine {
        self.toolPkgManager
            .getToolPkgExecutionEngine(&self.context, contextKey)
    }

    #[allow(non_snake_case)]
    pub fn findToolPkgExecutionEngine(&self, contextKey: &str) -> Option<JsEngine> {
        self.toolPkgManager.findToolPkgExecutionEngine(contextKey)
    }

    pub fn isPackageActivated(&self, packageName: &str) -> bool {
        let normalizedPackageName = self.normalizePackageName(packageName);
        self.activatedPackages.contains(&normalizedPackageName)
    }

    #[allow(non_snake_case)]
    pub fn usePackage(&mut self, packageName: &str) -> String {
        let normalizedPackageName = self.normalizePackageName(packageName);

        if self.isToolPkgContainer(&normalizedPackageName) {
            return format!(
                "ToolPkg container '{}' is not a package and cannot be activated.",
                normalizedPackageName
            );
        }

        let enabledPackageNames = self.getEnabledPackageNames();
        if enabledPackageNames.contains(&normalizedPackageName) {
            let Some(toolPackage) = self.availablePackages.get(&normalizedPackageName).cloned()
            else {
                return format!("Failed to load package data for: {}", normalizedPackageName);
            };
            let selectedPackage = self.selectToolPackageState(&toolPackage);
            self.activatePackage(&normalizedPackageName);
            return self.generatePackageSystemPrompt(&selectedPackage);
        }

        let skillManager = SkillManager::getInstance();
        let skillVisibilityPreferences = SkillVisibilityPreferences::getInstance();
        if skillManager
            .getAvailableSkills()
            .contains_key(&normalizedPackageName)
            && !skillVisibilityPreferences.isSkillVisibleToAi(&normalizedPackageName)
        {
            return format!("Skill '{}' is set to not show to AI", normalizedPackageName);
        }

        if let Some(skillPrompt) = skillManager.getSkillSystemPrompt(&normalizedPackageName) {
            return skillPrompt;
        }

        if self.isRegisteredMCPServer(&normalizedPackageName) {
            return self.useMCPServer(&normalizedPackageName);
        }

        format!(
            "Package not found: {}. Please import it first or register it as an MCP server.",
            normalizedPackageName
        )
    }

    #[allow(non_snake_case)]
    pub fn executeUsePackageTool(&mut self, toolName: &str, packageName: &str) -> ToolResult {
        if packageName.trim().is_empty() {
            return ToolResult {
                toolName: toolName.to_string(),
                success: false,
                result: String::new(),
                error: Some("Missing required parameter: package_name".to_string()),
            };
        }

        let normalizedPackageName = self.normalizePackageName(packageName);
        if self.isToolPkgContainer(&normalizedPackageName) {
            return ToolResult {
                toolName: toolName.to_string(),
                success: false,
                result: String::new(),
                error: Some(format!(
                    "ToolPkg container '{}' is not a package and cannot be activated.",
                    normalizedPackageName
                )),
            };
        }

        let skillManager = SkillManager::getInstance();
        let skillVisibilityPreferences = SkillVisibilityPreferences::getInstance();
        if skillManager
            .getAvailableSkills()
            .contains_key(&normalizedPackageName)
            && !skillVisibilityPreferences.isSkillVisibleToAi(&normalizedPackageName)
        {
            return ToolResult {
                toolName: toolName.to_string(),
                success: false,
                result: String::new(),
                error: Some(format!(
                    "Skill '{}' is set to not show to AI",
                    normalizedPackageName
                )),
            };
        }

        ToolResult {
            toolName: toolName.to_string(),
            success: true,
            result: self.usePackage(&normalizedPackageName),
            error: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn getEnabledPackageNames(&self) -> Vec<String> {
        let mut enabledPackageNames =
            BTreeSet::from_iter(self.decodeEnabledPackageNamesFromPrefs());
        let disabledPackageNames = BTreeSet::from_iter(self.decodeDisabledPackageNamesFromPrefs());
        for toolPackage in self.availablePackages.values() {
            if toolPackage.is_built_in
                && toolPackage.enabled_by_default
                && !disabledPackageNames.contains(&toolPackage.name)
            {
                enabledPackageNames.insert(toolPackage.name.clone());
            }
        }
        enabledPackageNames.into_iter().collect()
    }

    #[allow(non_snake_case)]
    pub fn getActivePackageNames(&self) -> Vec<String> {
        self.activatedPackages.iter().cloned().collect()
    }

    #[allow(non_snake_case)]
    pub fn enablePackage(&mut self, packageName: &str) -> String {
        let normalizedPackageName = self.normalizePackageName(packageName);
        if normalizedPackageName.trim().is_empty() {
            return "Package name cannot be empty".to_string();
        }

        if self.isToolPkgContainer(&normalizedPackageName) {
            return format!(
                "ToolPkg container '{}' is not a package. Enable its subpackages instead.",
                normalizedPackageName
            );
        }

        if !self.availablePackages.contains_key(&normalizedPackageName) {
            return format!("Package not found: {}", normalizedPackageName);
        }

        let mut enabledPackageNames = BTreeSet::from_iter(self.getEnabledPackageNames());
        if enabledPackageNames.contains(&normalizedPackageName) {
            return format!("Package is already enabled: {}", normalizedPackageName);
        }
        enabledPackageNames.insert(normalizedPackageName.clone());
        let names = enabledPackageNames.into_iter().collect::<Vec<_>>();
        if let Err(error) = self.saveEnabledPackageNames(&names) {
            return format!(
                "Failed to enable package '{}': {}",
                normalizedPackageName, error
            );
        }
        if let Err(error) = self.removeFromDisabledPackages(&normalizedPackageName) {
            return format!(
                "Failed to enable package '{}': {}",
                normalizedPackageName, error
            );
        }
        self.notifyToolPkgRuntimeChangeListeners();
        format!("Successfully enabled package: {}", normalizedPackageName)
    }

    #[allow(non_snake_case)]
    pub fn disablePackage(&mut self, packageName: &str) -> String {
        let normalizedPackageName = self.normalizePackageName(packageName);
        let mut enabledPackageNames = BTreeSet::from_iter(self.getEnabledPackageNames());
        self.activatedPackages.remove(&normalizedPackageName);
        if enabledPackageNames.remove(&normalizedPackageName) {
            let names = enabledPackageNames.into_iter().collect::<Vec<_>>();
            if let Err(error) = self.saveEnabledPackageNames(&names) {
                return format!(
                    "Failed to disable package '{}': {}",
                    normalizedPackageName, error
                );
            }
            if let Err(error) = self.addToDisabledIfDefaultEnabled(&normalizedPackageName) {
                return format!(
                    "Failed to disable package '{}': {}",
                    normalizedPackageName, error
                );
            }
            self.notifyToolPkgRuntimeChangeListeners();
            return format!("Successfully disabled package: {}", normalizedPackageName);
        }
        format!("Package is already disabled: {}", normalizedPackageName)
    }

    #[allow(non_snake_case)]
    pub fn deletePackage(&mut self, packageName: &str) -> bool {
        let normalizedPackageName = self.normalizePackageName(packageName);

        if self
            .toolPkgManager
            .resolveToolPkgSubpackageRuntimeInternal(&normalizedPackageName)
            .is_some()
        {
            self.disablePackage(&normalizedPackageName);
            return true;
        }

        let packageFile = self.findPackageFile(&normalizedPackageName);

        if packageFile.as_ref().is_none_or(|file| !file.exists()) {
            self.disablePackage(&normalizedPackageName);
            self.removeFromCachesAfterDelete(&normalizedPackageName);
            return true;
        }

        let packageFile = packageFile.expect("checked package file presence");
        match fs::remove_file(&packageFile) {
            Ok(_) => {
                self.disablePackage(&normalizedPackageName);
                self.removeFromCachesAfterDelete(&normalizedPackageName);
                true
            }
            Err(_) => false,
        }
    }

    #[allow(non_snake_case)]
    pub fn enableToolPkgContainer(&mut self, containerPackageName: &str) -> String {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        let Some(runtime) = self
            .toolPkgManager
            .getToolPkgContainerRuntime(&normalizedContainerPackageName)
        else {
            return format!(
                "ToolPkg container not found: {}",
                normalizedContainerPackageName
            );
        };
        let mut enabledPackageNames = BTreeSet::from_iter(self.getEnabledPackageNames());
        let mut changed = enabledPackageNames.insert(runtime.packageName.clone());
        for subpackage in runtime.subpackages {
            changed |= enabledPackageNames.insert(subpackage.packageName);
        }
        if !changed {
            return format!(
                "ToolPkg container is already enabled: {}",
                normalizedContainerPackageName
            );
        }
        let names = enabledPackageNames.into_iter().collect::<Vec<_>>();
        if let Err(error) = self.saveEnabledPackageNames(&names) {
            return format!(
                "Failed to enable ToolPkg container '{}': {}",
                normalizedContainerPackageName, error
            );
        }
        if let Err(error) = self.removeFromDisabledPackages(&normalizedContainerPackageName) {
            return format!(
                "Failed to enable ToolPkg container '{}': {}",
                normalizedContainerPackageName, error
            );
        }
        self.notifyToolPkgRuntimeChangeListeners();
        format!(
            "Successfully enabled ToolPkg container: {}",
            normalizedContainerPackageName
        )
    }

    #[allow(non_snake_case)]
    pub fn disableToolPkgContainer(&mut self, containerPackageName: &str) -> String {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        let Some(runtime) = self
            .toolPkgManager
            .getToolPkgContainerRuntime(&normalizedContainerPackageName)
        else {
            return format!(
                "ToolPkg container not found: {}",
                normalizedContainerPackageName
            );
        };
        let mut enabledPackageNames = BTreeSet::from_iter(self.getEnabledPackageNames());
        let mut changed = enabledPackageNames.remove(&runtime.packageName);
        for subpackage in runtime.subpackages {
            self.activatedPackages.remove(&subpackage.packageName);
            changed |= enabledPackageNames.remove(&subpackage.packageName);
        }
        if !changed {
            return format!(
                "ToolPkg container is already disabled: {}",
                normalizedContainerPackageName
            );
        }
        let names = enabledPackageNames.into_iter().collect::<Vec<_>>();
        if let Err(error) = self.saveEnabledPackageNames(&names) {
            return format!(
                "Failed to disable ToolPkg container '{}': {}",
                normalizedContainerPackageName, error
            );
        }
        if let Err(error) = self.addToDisabledIfDefaultEnabled(&normalizedContainerPackageName) {
            return format!(
                "Failed to disable ToolPkg container '{}': {}",
                normalizedContainerPackageName, error
            );
        }
        self.notifyToolPkgRuntimeChangeListeners();
        format!(
            "Successfully disabled ToolPkg container: {}",
            normalizedContainerPackageName
        )
    }

    #[allow(non_snake_case)]
    pub fn isToolPkgContainer(&self, packageName: &str) -> bool {
        let normalizedPackageName = self.normalizePackageName(packageName);
        self.toolPkgManager
            .isToolPkgContainer(&normalizedPackageName)
    }

    #[allow(non_snake_case)]
    pub fn getEnabledToolPkgContainerRuntimes(&self) -> Vec<ToolPkgContainerRuntime> {
        self.toolPkgManager
            .getEnabledToolPkgContainerRuntimes(&self.getEnabledPackageNames())
    }

    #[allow(non_snake_case)]
    pub fn getToolPkgContainerRuntimes(&self) -> Vec<ToolPkgContainerRuntime> {
        self.toolPkgManager.getToolPkgContainerRuntimes()
    }

    #[allow(non_snake_case)]
    pub fn getToolPkgContainerRuntime(
        &self,
        containerPackageName: &str,
    ) -> Option<ToolPkgContainerRuntime> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        self.toolPkgManager
            .getToolPkgContainerRuntime(&normalizedContainerPackageName)
    }

    #[allow(non_snake_case)]
    pub fn resolveToolPkgSubpackageRuntimeInternal(
        &self,
        packageName: &str,
    ) -> Option<ToolPkgSubpackageRuntime> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        self.toolPkgManager
            .resolveToolPkgSubpackageRuntimeInternal(&normalizedPackageName)
    }

    #[allow(non_snake_case)]
    pub fn getEffectivePackageTools(&self, packageName: &str) -> Option<ToolPackage> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        let toolPackage = self.availablePackages.get(&normalizedPackageName)?;
        Some(self.selectToolPackageStateSnapshot(toolPackage))
    }

    #[allow(non_snake_case)]
    pub fn getPackageTools(&self, packageName: &str) -> Option<ToolPackage> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        self.availablePackages.get(&normalizedPackageName).cloned()
    }

    #[allow(non_snake_case)]
    pub fn getPackageScript(&self, packageName: &str) -> Option<String> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        self.availablePackages
            .get(&normalizedPackageName)
            .and_then(|toolPackage| toolPackage.tools.first())
            .map(|tool| tool.script.clone())
    }

    #[allow(non_snake_case)]
    pub fn getActivePackageStateId(&self, packageName: &str) -> Option<String> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        self.activePackageStateIds
            .get(&normalizedPackageName)
            .cloned()
            .flatten()
    }

    #[allow(non_snake_case)]
    pub fn getAvailablePackages(&self) -> BTreeMap<String, ToolPackage> {
        self.availablePackages.clone()
    }

    #[allow(non_snake_case)]
    pub fn getAvailableServerPackages(&self) -> BTreeMap<String, MCPServerConfig> {
        self.mcpManager.getRegisteredServers()
    }

    #[allow(non_snake_case)]
    pub fn getCachedMcpTools(&self, serverName: &str) -> Vec<CachedMcpToolInfo> {
        MCPLocalServer::getInstance(&self.context)
            .getCachedTools(serverName)
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn setAvailablePackage(&mut self, packageName: String, toolPackage: ToolPackage) {
        let normalizedPackageName = self.normalizePackageName(&packageName);
        self.availablePackages
            .insert(normalizedPackageName, toolPackage);
    }

    #[allow(non_snake_case)]
    pub fn registerToolPkg(&mut self, loadResult: ToolPkgLoadResult) -> bool {
        if !self
            .toolPkgManager
            .canRegisterToolPkg(&loadResult, &self.availablePackages)
        {
            return false;
        }

        self.availablePackages.insert(
            loadResult.containerPackage.name.clone(),
            loadResult.containerPackage.clone(),
        );
        for subpackage in self.toolPkgManager.registerToolPkg(loadResult) {
            self.availablePackages
                .insert(subpackage.name.clone(), subpackage);
        }
        self.notifyToolPkgRuntimeChangeListeners();
        true
    }

    #[allow(non_snake_case)]
    pub fn addToolPkgRuntimeChangeListener(&self, listener: ToolPkgRuntimeChangeListener) {
        self.toolPkgManager
            .addToolPkgRuntimeChangeListener(listener, self.getEnabledToolPkgContainerRuntimes());
    }

    #[allow(non_snake_case)]
    fn notifyToolPkgRuntimeChangeListeners(&self) {
        self.toolPkgManager
            .notifyToolPkgRuntimeChangeListeners(self.getEnabledToolPkgContainerRuntimes());
    }

    #[allow(non_snake_case)]
    pub fn getExternalPackagesPath(&self) -> String {
        self.storePaths.packages_dir().to_string_lossy().to_string()
    }

    #[allow(non_snake_case)]
    pub fn loadAvailablePackages(&mut self) {
        let previousContainerNames = self
            .toolPkgManager
            .getToolPkgContainerRuntimes()
            .into_iter()
            .map(|runtime| runtime.packageName)
            .collect::<BTreeSet<_>>();
        let assetSnapshot = self.scanBuiltInPackageAssets();
        let mergedSnapshot = self.scanExternalPackages(&assetSnapshot);
        let nextContainerNames = mergedSnapshot
            .toolPkgContainers
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        for packageName in previousContainerNames.union(&nextContainerNames) {
            self.toolPkgManager
                .destroyDefaultToolPkgExecutionEngine(packageName);
        }
        self.applyPackageScanSnapshot(mergedSnapshot);
        self.notifyToolPkgRuntimeChangeListeners();
    }

    #[allow(non_snake_case)]
    pub fn getPublishablePackageSources(&mut self) -> Vec<PublishablePackageSource> {
        let mut sources = Vec::new();

        for (packageName, toolPackage) in self.availablePackages.clone() {
            if toolPackage.is_built_in
                || self.toolPkgManager.hasSubpackage(&packageName)
                || self.toolPkgManager.isToolPkgContainer(&packageName)
            {
                continue;
            }
            let Some(sourceFile) = self.findPackageFile(&packageName) else {
                continue;
            };
            if !sourceFile.exists() || !sourceFile.is_file() {
                continue;
            }
            let displayName = {
                let resolved = toolPackage.display_name.resolve(false);
                if resolved.trim().is_empty() {
                    packageName.clone()
                } else {
                    resolved
                }
            };
            let authorDeclaration = self.inspectStandalonePackageAuthorDeclaration(&sourceFile);
            sources.push(PublishablePackageSource {
                packageName,
                displayName,
                description: toolPackage.description.resolve(false),
                author: toolPackage.author,
                sourcePath: sourceFile.to_string_lossy().to_string(),
                sourceFileName: sourceFile
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_default(),
                fileExtension: sourceFile
                    .extension()
                    .map(|value| value.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default(),
                isToolPkg: false,
                hasDeclaredAuthorField: authorDeclaration.hasDeclaredAuthorField,
                declaredAuthorSlotCount: authorDeclaration.declaredAuthorSlotCount,
                inferredVersion: None,
            });
        }

        for runtime in self.toolPkgManager.getToolPkgContainerRuntimes() {
            if runtime.sourceType != ToolPkgSourceType::EXTERNAL {
                continue;
            }
            let sourceFile = PathBuf::from(&runtime.sourcePath);
            if !sourceFile.exists() || !sourceFile.is_file() {
                continue;
            }
            let displayName = {
                let resolved = runtime.displayName.resolve(false);
                if resolved.trim().is_empty() {
                    runtime.packageName.clone()
                } else {
                    resolved
                }
            };
            let authorDeclaration = self.inspectToolPkgAuthorDeclaration(&sourceFile);
            sources.push(PublishablePackageSource {
                packageName: runtime.packageName,
                displayName,
                description: runtime.description.resolve(false),
                author: runtime.author,
                sourcePath: sourceFile.to_string_lossy().to_string(),
                sourceFileName: sourceFile
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_default(),
                fileExtension: sourceFile
                    .extension()
                    .map(|value| value.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default(),
                isToolPkg: true,
                hasDeclaredAuthorField: authorDeclaration.hasDeclaredAuthorField,
                declaredAuthorSlotCount: authorDeclaration.declaredAuthorSlotCount,
                inferredVersion: if runtime.version.trim().is_empty() {
                    None
                } else {
                    Some(runtime.version)
                },
            });
        }

        sources.sort_by(|left, right| {
            left.isToolPkg.cmp(&right.isToolPkg).then_with(|| {
                left.displayName
                    .to_lowercase()
                    .cmp(&right.displayName.to_lowercase())
            })
        });
        sources
    }

    #[allow(non_snake_case)]
    fn inspectStandalonePackageAuthorDeclaration(&self, sourceFile: &Path) -> AuthorDeclaration {
        let Ok(content) = fs::read_to_string(sourceFile) else {
            return AuthorDeclaration::default();
        };
        let lowerPath = sourceFile.to_string_lossy().to_ascii_lowercase();
        let metadataString = if lowerPath.ends_with(".js") || lowerPath.ends_with(".ts") {
            self.extractMetadataFromJs(&content)
        } else {
            content
        };
        inspectAuthorDeclarationFromMetadata(&metadataString)
    }

    #[allow(non_snake_case)]
    fn inspectToolPkgAuthorDeclaration(&self, sourceFile: &Path) -> AuthorDeclaration {
        let Ok(file) = fs::File::open(sourceFile) else {
            return AuthorDeclaration::default();
        };
        let Ok(mut archive) = zip::ZipArchive::new(file) else {
            return AuthorDeclaration::default();
        };
        let entryIndex = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
        let Some(manifestEntryName) = findToolPkgManifestEntryName(&entryIndex.entryNames) else {
            return AuthorDeclaration::default();
        };
        let Some(manifestText) =
            ToolPkgArchiveParser::readZipEntryText(&mut archive, &entryIndex, &manifestEntryName)
        else {
            return AuthorDeclaration::default();
        };
        inspectAuthorDeclarationFromMetadata(&manifestText)
    }

    #[allow(non_snake_case)]
    fn scanBuiltInPackageAssets(&self) -> PackageScanSnapshot {
        let sourceDir = builtInPackageAssetsDir();
        let Ok(entries) = fs::read_dir(&sourceDir) else {
            eprintln!(
                "Built-in package asset directory is unavailable: {}",
                sourceDir.display()
            );
            return PackageScanSnapshot::default();
        };
        let mut files = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        files.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));
        let results = files
            .into_iter()
            .filter_map(|path| {
                let fileName = path.file_name()?.to_string_lossy().to_string();
                Some(self.parseBuiltInPackageCandidate(&fileName, &path))
            })
            .collect::<Vec<_>>();
        self.mergePackageScanCandidateResults(results, None)
    }

    #[allow(non_snake_case)]
    fn scanExternalPackages(&mut self, baseSnapshot: &PackageScanSnapshot) -> PackageScanSnapshot {
        let packagesDir = self.storePaths.packages_dir();
        if let Err(error) = fs::create_dir_all(&packagesDir) {
            eprintln!(
                "External package directory creation failed: {}, error={error}",
                packagesDir.display()
            );
            return baseSnapshot.clone();
        }
        let Ok(entries) = fs::read_dir(&packagesDir) else {
            eprintln!(
                "External package directory is unreadable: {}",
                packagesDir.display()
            );
            return baseSnapshot.clone();
        };
        let mut files = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        files.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));

        let previousCache = self.externalPackageScanCache.clone();
        let mut nextCache = BTreeMap::new();
        let mut results = Vec::new();
        for file in files {
            let cacheKey = file.to_string_lossy().to_string();
            let signature = self.buildExternalPackageScanSignature(&file);
            let result = previousCache
                .get(&cacheKey)
                .filter(|entry| entry.signature == signature)
                .map(|entry| entry.result.clone())
                .unwrap_or_else(|| self.parseExternalPackageCandidate(&file));
            nextCache.insert(
                cacheKey,
                ExternalPackageScanCacheEntry {
                    signature,
                    result: result.clone(),
                },
            );
            results.push(result);
        }
        self.externalPackageScanCache = nextCache;
        self.mergePackageScanCandidateResults(results, Some(baseSnapshot))
    }

    #[allow(non_snake_case)]
    fn parseBuiltInPackageCandidate(
        &self,
        fileName: &str,
        path: &Path,
    ) -> PackageScanCandidateResult {
        let mut result = PackageScanCandidateResult {
            phase: "asset".to_string(),
            sourcePath: fileName.to_string(),
            ..Default::default()
        };
        let lowerName = fileName.to_ascii_lowercase();
        if lowerName.ends_with(".js") || lowerName.ends_with(".ts") {
            match fs::read_to_string(path).and_then(|script| {
                self.parseJsPackage(&script)
                    .map(|package| ToolPackage {
                        is_built_in: true,
                        ..package
                    })
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
            }) {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => eprintln!(
                    "Built-in JavaScript package load error [{}]: {error}",
                    path.display()
                ),
            }
        } else if lowerName.ends_with(".hjson") || lowerName.ends_with(".json") {
            match fs::read_to_string(path).and_then(|content| {
                self.parsePackageMetadata(&content, "")
                    .map(|package| ToolPackage {
                        is_built_in: true,
                        ..package
                    })
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
            }) {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => eprintln!(
                    "Built-in package metadata load error [{}]: {error}",
                    path.display()
                ),
            }
        } else if lowerName.ends_with(".toolpkg") {
            match self.loadToolPkgFromBuiltInAssetFile(fileName, path) {
                Ok(loadResult) => result.toolPkgLoadResult = Some(loadResult),
                Err(error) => eprintln!(
                    "Built-in ToolPkg package load error [{}]: {error}",
                    path.display()
                ),
            }
        }
        result
    }

    #[allow(non_snake_case)]
    fn parseExternalPackageCandidate(&self, path: &Path) -> PackageScanCandidateResult {
        let sourcePath = path.to_string_lossy().to_string();
        let mut result = PackageScanCandidateResult {
            phase: "external".to_string(),
            sourcePath: sourcePath.clone(),
            ..Default::default()
        };
        let lowerPath = sourcePath.to_ascii_lowercase();
        if lowerPath.ends_with(".js") || lowerPath.ends_with(".ts") {
            result.toolPackage = self.loadPackageFromJsFile(path);
        } else if lowerPath.ends_with(".hjson") {
            match fs::read_to_string(path).and_then(|content| {
                self.parsePackageMetadata(&content, "")
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
            }) {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => {
                    eprintln!("External package metadata load error [{sourcePath}]: {error}")
                }
            }
        } else if lowerPath.ends_with(".toolpkg") {
            match self.loadToolPkgFromExternalFile(path) {
                Ok(loadResult) => result.toolPkgLoadResult = Some(loadResult),
                Err(error) => {
                    eprintln!("External ToolPkg package load error [{sourcePath}]: {error}")
                }
            }
        }
        result
    }

    #[allow(non_snake_case)]
    fn mergePackageScanCandidateResults(
        &self,
        candidateResults: Vec<PackageScanCandidateResult>,
        baseSnapshot: Option<&PackageScanSnapshot>,
    ) -> PackageScanSnapshot {
        let mut stagedAvailablePackages = baseSnapshot
            .map(|snapshot| snapshot.availablePackages.clone())
            .unwrap_or_default();
        let mut stagedToolPkgContainers = baseSnapshot
            .map(|snapshot| snapshot.toolPkgContainers.clone())
            .unwrap_or_default();
        let mut stagedToolPkgSubpackages = baseSnapshot
            .map(|snapshot| snapshot.toolPkgSubpackages.clone())
            .unwrap_or_default();

        for result in candidateResults {
            if let Some(packageMetadata) = result.toolPackage {
                if result.phase == "external"
                    && !Self::prepareExternalStandalonePackageOverride(
                        &packageMetadata.name,
                        &mut stagedAvailablePackages,
                        &mut stagedToolPkgContainers,
                        &mut stagedToolPkgSubpackages,
                    )
                {
                    eprintln!(
                        "Duplicate package name: {}, source={}",
                        packageMetadata.name, result.sourcePath
                    );
                    continue;
                }
                if stagedAvailablePackages.contains_key(&packageMetadata.name) {
                    eprintln!(
                        "Duplicate package name: {}, source={}",
                        packageMetadata.name, result.sourcePath
                    );
                } else {
                    stagedAvailablePackages.insert(packageMetadata.name.clone(), packageMetadata);
                }
            }
            if let Some(loadResult) = result.toolPkgLoadResult {
                if result.phase == "external"
                    && !Self::prepareExternalToolPkgOverride(
                        &loadResult,
                        &mut stagedAvailablePackages,
                        &mut stagedToolPkgContainers,
                        &mut stagedToolPkgSubpackages,
                    )
                {
                    eprintln!(
                        "Duplicate ToolPkg package name: {}, source={}",
                        loadResult.containerPackage.name, result.sourcePath
                    );
                    continue;
                }
                if !Self::registerToolPkgInto(
                    loadResult,
                    &mut stagedAvailablePackages,
                    &mut stagedToolPkgContainers,
                    &mut stagedToolPkgSubpackages,
                ) {
                    eprintln!(
                        "ToolPkg package registration failed, source={}",
                        result.sourcePath
                    );
                }
            }
        }

        PackageScanSnapshot {
            availablePackages: stagedAvailablePackages,
            toolPkgContainers: stagedToolPkgContainers,
            toolPkgSubpackages: stagedToolPkgSubpackages,
        }
    }

    #[allow(non_snake_case)]
    fn applyPackageScanSnapshot(&mut self, snapshot: PackageScanSnapshot) {
        self.availablePackages = snapshot.availablePackages;
        self.toolPkgManager
            .replaceRuntimeMaps(snapshot.toolPkgContainers, snapshot.toolPkgSubpackages);
    }

    #[allow(non_snake_case)]
    fn registerToolPkgInto(
        loadResult: ToolPkgLoadResult,
        availablePackagesTarget: &mut BTreeMap<String, ToolPackage>,
        toolPkgContainersTarget: &mut BTreeMap<String, ToolPkgContainerRuntime>,
        toolPkgSubpackageByPackageNameTarget: &mut BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) -> bool {
        let containerName = loadResult.containerPackage.name.clone();
        if availablePackagesTarget.contains_key(&containerName) {
            return false;
        }
        if loadResult
            .subpackagePackages
            .iter()
            .any(|subpackage| availablePackagesTarget.contains_key(&subpackage.name))
        {
            return false;
        }
        availablePackagesTarget.insert(containerName.clone(), loadResult.containerPackage);
        toolPkgContainersTarget.insert(containerName, loadResult.containerRuntime.clone());
        for subpackage in loadResult.subpackagePackages {
            availablePackagesTarget.insert(subpackage.name.clone(), subpackage);
        }
        for runtime in loadResult.containerRuntime.subpackages {
            toolPkgSubpackageByPackageNameTarget.insert(runtime.packageName.clone(), runtime);
        }
        true
    }

    #[allow(non_snake_case)]
    fn removeToolPkgContainerFromTargets(
        containerPackageName: &str,
        availablePackagesTarget: &mut BTreeMap<String, ToolPackage>,
        toolPkgContainersTarget: &mut BTreeMap<String, ToolPkgContainerRuntime>,
        toolPkgSubpackageByPackageNameTarget: &mut BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) {
        let Some(runtime) = toolPkgContainersTarget.remove(containerPackageName) else {
            return;
        };
        availablePackagesTarget.remove(containerPackageName);
        for subpackage in runtime.subpackages {
            availablePackagesTarget.remove(&subpackage.packageName);
            toolPkgSubpackageByPackageNameTarget.remove(&subpackage.packageName);
        }
    }

    #[allow(non_snake_case)]
    fn prepareExternalStandalonePackageOverride(
        packageName: &str,
        availablePackagesTarget: &mut BTreeMap<String, ToolPackage>,
        toolPkgContainersTarget: &mut BTreeMap<String, ToolPkgContainerRuntime>,
        toolPkgSubpackageByPackageNameTarget: &mut BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) -> bool {
        if let Some(existingContainer) = toolPkgContainersTarget.get(packageName).cloned() {
            if existingContainer.sourceType != ToolPkgSourceType::ASSET {
                return false;
            }
            Self::removeToolPkgContainerFromTargets(
                &existingContainer.packageName,
                availablePackagesTarget,
                toolPkgContainersTarget,
                toolPkgSubpackageByPackageNameTarget,
            );
            return true;
        }

        if let Some(existingSubpackage) = toolPkgSubpackageByPackageNameTarget
            .get(packageName)
            .cloned()
        {
            let Some(ownerContainer) = toolPkgContainersTarget
                .get(&existingSubpackage.containerPackageName)
                .cloned()
            else {
                return false;
            };
            if ownerContainer.sourceType != ToolPkgSourceType::ASSET {
                return false;
            }
            Self::removeToolPkgContainerFromTargets(
                &ownerContainer.packageName,
                availablePackagesTarget,
                toolPkgContainersTarget,
                toolPkgSubpackageByPackageNameTarget,
            );
            return true;
        }

        let Some(existingPackage) = availablePackagesTarget.get(packageName) else {
            return true;
        };
        if !existingPackage.is_built_in {
            return false;
        }
        availablePackagesTarget.remove(packageName);
        true
    }

    #[allow(non_snake_case)]
    fn prepareExternalToolPkgOverride(
        loadResult: &ToolPkgLoadResult,
        availablePackagesTarget: &mut BTreeMap<String, ToolPackage>,
        toolPkgContainersTarget: &mut BTreeMap<String, ToolPkgContainerRuntime>,
        toolPkgSubpackageByPackageNameTarget: &mut BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) -> bool {
        let mut builtInContainersToRemove = BTreeSet::new();
        let mut builtInStandalonePackagesToRemove = BTreeSet::new();
        let mut conflictingNames = Vec::new();
        conflictingNames.push(loadResult.containerPackage.name.clone());
        conflictingNames.extend(
            loadResult
                .subpackagePackages
                .iter()
                .map(|subpackage| subpackage.name.clone()),
        );

        for packageName in conflictingNames {
            if let Some(existingContainer) = toolPkgContainersTarget.get(&packageName) {
                if existingContainer.sourceType != ToolPkgSourceType::ASSET {
                    return false;
                }
                builtInContainersToRemove.insert(existingContainer.packageName.clone());
                continue;
            }

            if let Some(existingSubpackage) = toolPkgSubpackageByPackageNameTarget.get(&packageName)
            {
                let Some(ownerContainer) =
                    toolPkgContainersTarget.get(&existingSubpackage.containerPackageName)
                else {
                    return false;
                };
                if ownerContainer.sourceType != ToolPkgSourceType::ASSET {
                    return false;
                }
                builtInContainersToRemove.insert(ownerContainer.packageName.clone());
                continue;
            }

            if let Some(existingPackage) = availablePackagesTarget.get(&packageName) {
                if !existingPackage.is_built_in {
                    return false;
                }
                builtInStandalonePackagesToRemove.insert(packageName);
            }
        }

        for containerPackageName in builtInContainersToRemove {
            Self::removeToolPkgContainerFromTargets(
                &containerPackageName,
                availablePackagesTarget,
                toolPkgContainersTarget,
                toolPkgSubpackageByPackageNameTarget,
            );
        }
        for packageName in builtInStandalonePackagesToRemove {
            availablePackagesTarget.remove(&packageName);
        }
        true
    }

    #[allow(non_snake_case)]
    fn buildExternalPackageScanSignature(&self, file: &Path) -> String {
        let metadata = fs::metadata(file).ok();
        format!(
            "{}|{}|{}",
            file.to_string_lossy(),
            metadata.as_ref().map(|value| value.len()).unwrap_or(0),
            metadata
                .and_then(|value| value.modified().ok())
                .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|value| value.as_millis())
                .unwrap_or(0)
        )
    }

    #[allow(non_snake_case)]
    pub fn addPackageFileFromExternalStorage(&mut self, filePath: &str) -> String {
        let file = PathBuf::from(filePath);
        if !file.exists() || !file.is_file() {
            return format!("Cannot access file at path: {filePath}");
        }

        let lowerPath = filePath.to_ascii_lowercase();
        let isJsLike = lowerPath.ends_with(".js") || lowerPath.ends_with(".ts");
        let isHjson = lowerPath.ends_with(".hjson");
        let isToolPkg = lowerPath.ends_with(".toolpkg");
        if !isJsLike && !isHjson && !isToolPkg {
            return "Only HJSON, JavaScript (.js), TypeScript (.ts) and ToolPkg (.toolpkg) package files are supported"
                .to_string();
        }

        if isToolPkg {
            let loadResult = match self.loadToolPkgFromExternalFile(&file) {
                Ok(value) => value,
                Err(error) => return format!("Error importing package: {error}"),
            };
            let packageName = loadResult.containerPackage.name.clone();
            if !self
                .toolPkgManager
                .canRegisterToolPkg(&loadResult, &self.availablePackages)
            {
                return format!(
                    "A package with name '{}' already exists in available packages",
                    packageName
                );
            }
            if let Err(error) = self.storePaths.ensure_packages_dir() {
                return format!("Error importing package: {error}");
            }
            let Some(fileName) = file.file_name() else {
                return "Error importing package: invalid file name".to_string();
            };
            let destinationFile = self.storePaths.packages_dir().join(fileName);
            if file != destinationFile {
                if let Err(error) = fs::copy(&file, &destinationFile) {
                    return format!("Error importing package: {error}");
                }
            }
            let importedLoadResult = match self.loadToolPkgFromExternalFile(&destinationFile) {
                Ok(value) => value,
                Err(error) => return format!("Error importing package: {error}"),
            };
            if !self.registerToolPkg(importedLoadResult) {
                return format!(
                    "A package with name '{}' already exists in available packages",
                    packageName
                );
            }
            return format!(
                "Successfully imported package: {}\nStored at: {}",
                packageName,
                destinationFile.to_string_lossy()
            );
        }

        let packageMetadata = if isHjson {
            let content = match fs::read_to_string(&file) {
                Ok(value) => value,
                Err(error) => return format!("Error importing package: {error}"),
            };
            match self.parsePackageMetadata(&content, "") {
                Ok(value) => value,
                Err(error) => return format!("Error importing package: {error}"),
            }
        } else {
            match self.loadPackageFromJsFile(&file) {
                Some(value) => value,
                None => {
                    return format!(
                        "Failed to parse {} package file",
                        if lowerPath.ends_with(".ts") {
                            "TypeScript"
                        } else {
                            "JavaScript"
                        }
                    )
                }
            }
        };

        if self.availablePackages.contains_key(&packageMetadata.name)
            || self
                .toolPkgManager
                .isToolPkgContainer(&packageMetadata.name)
            || self.toolPkgManager.hasSubpackage(&packageMetadata.name)
        {
            return format!(
                "A package with name '{}' already exists in available packages",
                packageMetadata.name
            );
        }

        if let Err(error) = self.storePaths.ensure_packages_dir() {
            return format!("Error importing package: {error}");
        }
        let Some(fileName) = file.file_name() else {
            return "Error importing package: invalid file name".to_string();
        };
        let destinationFile = self.storePaths.packages_dir().join(fileName);
        if file != destinationFile {
            if let Err(error) = fs::copy(&file, &destinationFile) {
                return format!("Error importing package: {error}");
            }
        }

        self.availablePackages.insert(
            packageMetadata.name.clone(),
            ToolPackage {
                is_built_in: false,
                ..packageMetadata.clone()
            },
        );
        format!(
            "Successfully imported package: {}\nStored at: {}",
            packageMetadata.name,
            destinationFile.to_string_lossy()
        )
    }

    #[allow(non_snake_case)]
    pub fn setEnabledPackageNames(
        &self,
        packageNames: &[String],
    ) -> Result<(), PreferencesDataStoreError> {
        let result = self.saveEnabledPackageNames(packageNames);
        if result.is_ok() {
            self.notifyToolPkgRuntimeChangeListeners();
        }
        result
    }

    #[allow(non_snake_case)]
    pub fn setAvailableServerPackage(&mut self, serverName: String, serverConfig: MCPServerConfig) {
        self.mcpManager.registerServer(serverName, serverConfig);
    }

    #[allow(non_snake_case)]
    pub fn setCachedMcpTools(&mut self, serverName: String, tools: Vec<CachedMcpToolInfo>) {
        self.cachedMcpTools.insert(serverName, tools);
    }

    #[allow(non_snake_case)]
    pub fn getToolPkgMainScriptInternal(&self, containerPackageName: &str) -> Option<String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        self.toolPkgManager.getToolPkgMainScriptInternal(
            &normalizedContainerPackageName,
            &self.getEnabledPackageNames(),
        )
    }

    #[allow(non_snake_case)]
    pub fn readToolPkgTextResource(
        &self,
        packageNameOrSubpackageId: &str,
        resourcePath: &str,
    ) -> Option<String> {
        let normalizedPackageName = self.normalizePackageName(packageNameOrSubpackageId);
        self.toolPkgManager.readToolPkgTextResource(
            &normalizedPackageName,
            resourcePath,
            &self.getEnabledPackageNames(),
        )
    }

    #[allow(non_snake_case)]
    pub fn getToolPkgComposeDslScript(
        &self,
        containerPackageName: &str,
        uiModuleIdOrRouteId: Option<&str>,
    ) -> Option<String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        let runtime = self
            .toolPkgManager
            .getToolPkgContainerRuntime(&normalizedContainerPackageName)?;
        let enabledPackageNames = self.getEnabledPackageNames();
        let enabledSet = BTreeSet::from_iter(enabledPackageNames.iter().cloned());
        if !enabledSet.contains(&runtime.packageName) {
            return None;
        }
        let requested = uiModuleIdOrRouteId
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let screen = requested
            .and_then(|id| {
                runtime
                    .uiRoutes
                    .iter()
                    .find(|route| {
                        route.routeId.eq_ignore_ascii_case(id) || route.id.eq_ignore_ascii_case(id)
                    })
                    .map(|route| route.screen.clone())
                    .or_else(|| {
                        runtime
                            .uiModules
                            .iter()
                            .find(|module| module.id.eq_ignore_ascii_case(id))
                            .map(|module| module.screen.clone())
                    })
            })
            .or_else(|| runtime.uiRoutes.first().map(|route| route.screen.clone()))
            .or_else(|| {
                runtime
                    .uiModules
                    .first()
                    .map(|module| module.screen.clone())
            })?;
        self.toolPkgManager.readToolPkgTextResource(
            &normalizedContainerPackageName,
            &screen,
            &enabledPackageNames,
        )
    }

    #[allow(non_snake_case)]
    pub fn getToolPkgComposeDslScreenPath(
        &self,
        containerPackageName: &str,
        uiModuleIdOrRouteId: Option<&str>,
    ) -> Option<String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        let runtime = self
            .toolPkgManager
            .getToolPkgContainerRuntime(&normalizedContainerPackageName)?;
        let requested = uiModuleIdOrRouteId
            .map(str::trim)
            .filter(|value| !value.is_empty());
        requested
            .and_then(|id| {
                runtime
                    .uiRoutes
                    .iter()
                    .find(|route| {
                        route.routeId.eq_ignore_ascii_case(id) || route.id.eq_ignore_ascii_case(id)
                    })
                    .map(|route| route.screen.clone())
                    .or_else(|| {
                        runtime
                            .uiModules
                            .iter()
                            .find(|module| module.id.eq_ignore_ascii_case(id))
                            .map(|module| module.screen.clone())
                    })
            })
            .or_else(|| runtime.uiRoutes.first().map(|route| route.screen.clone()))
            .or_else(|| {
                runtime
                    .uiModules
                    .first()
                    .map(|module| module.screen.clone())
            })
            .map(|screen| screen.trim().to_string())
            .filter(|screen| !screen.is_empty())
    }

    #[allow(non_snake_case)]
    pub fn executeToolPkgComposeDslScript(
        &self,
        containerPackageName: &str,
        uiModuleIdOrRouteId: Option<&str>,
        runtimeOptions: serde_json::Value,
    ) -> Option<String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        let script =
            self.getToolPkgComposeDslScript(&normalizedContainerPackageName, uiModuleIdOrRouteId)?;
        let params = self.buildToolPkgComposeDslRuntimeOptions(
            &normalizedContainerPackageName,
            uiModuleIdOrRouteId,
            runtimeOptions,
        );
        let contextKey = params
            .get("__operit_compose_execution_context_key")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("toolpkg_compose");
        self.toolPkgManager
            .getToolPkgExecutionEngine(&self.context, contextKey)
            .executeComposeDslScript(&script, &params)
    }

    #[allow(non_snake_case)]
    pub fn executeToolPkgComposeDslAction(
        &self,
        containerPackageName: &str,
        uiModuleIdOrRouteId: Option<&str>,
        actionId: &str,
        payload: Option<serde_json::Value>,
        runtimeOptions: serde_json::Value,
    ) -> Option<String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        let params = self.buildToolPkgComposeDslRuntimeOptions(
            &normalizedContainerPackageName,
            uiModuleIdOrRouteId,
            runtimeOptions,
        );
        let contextKey = params
            .get("__operit_compose_execution_context_key")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("toolpkg_compose");
        self.toolPkgManager
            .getToolPkgExecutionEngine(&self.context, contextKey)
            .executeComposeDslAction(actionId, payload, &params, None)
    }

    #[allow(non_snake_case)]
    pub fn rerenderToolPkgComposeDslTree(
        &self,
        containerPackageName: &str,
        uiModuleIdOrRouteId: Option<&str>,
        runtimeOptions: serde_json::Value,
    ) -> Option<String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        let params = self.buildToolPkgComposeDslRuntimeOptions(
            &normalizedContainerPackageName,
            uiModuleIdOrRouteId,
            runtimeOptions,
        );
        let contextKey = params
            .get("__operit_compose_execution_context_key")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("toolpkg_compose");
        self.toolPkgManager
            .getToolPkgExecutionEngine(&self.context, contextKey)
            .rerenderComposeDslTree(&params)
    }

    #[allow(non_snake_case)]
    fn buildToolPkgComposeDslRuntimeOptions(
        &self,
        containerPackageName: &str,
        uiModuleIdOrRouteId: Option<&str>,
        runtimeOptions: serde_json::Value,
    ) -> BTreeMap<String, serde_json::Value> {
        let mut params = match runtimeOptions {
            serde_json::Value::Object(object) => object.into_iter().collect(),
            _ => BTreeMap::new(),
        };
        let runtime = self
            .toolPkgManager
            .getToolPkgContainerRuntime(containerPackageName);
        let requested = uiModuleIdOrRouteId
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let route = runtime.as_ref().and_then(|runtime| {
            requested.as_deref().and_then(|id| {
                runtime
                    .uiRoutes
                    .iter()
                    .find(|route| {
                        route.routeId.eq_ignore_ascii_case(id) || route.id.eq_ignore_ascii_case(id)
                    })
                    .cloned()
            })
        });
        let module = runtime.as_ref().and_then(|runtime| {
            route
                .as_ref()
                .and_then(|route| {
                    runtime
                        .uiModules
                        .iter()
                        .find(|module| module.id.eq_ignore_ascii_case(&route.id))
                        .cloned()
                })
                .or_else(|| {
                    requested.as_deref().and_then(|id| {
                        runtime
                            .uiModules
                            .iter()
                            .find(|module| module.id.eq_ignore_ascii_case(id))
                            .cloned()
                    })
                })
                .or_else(|| runtime.uiModules.first().cloned())
        });
        let uiModuleId = route
            .as_ref()
            .map(|route| route.id.clone())
            .or_else(|| module.as_ref().map(|module| module.id.clone()))
            .or_else(|| requested.clone())
            .unwrap_or_default();
        let routeId = route
            .as_ref()
            .map(|route| route.routeId.clone())
            .unwrap_or_else(|| uiModuleId.clone());
        let screen = route
            .as_ref()
            .map(|route| route.screen.clone())
            .or_else(|| module.as_ref().map(|module| module.screen.clone()))
            .unwrap_or_default();
        let contextKey = format!("toolpkg_compose:{containerPackageName}:{routeId}");
        params.insert(
            "packageName".to_string(),
            serde_json::Value::String(containerPackageName.to_string()),
        );
        params.insert(
            "toolPkgId".to_string(),
            serde_json::Value::String(containerPackageName.to_string()),
        );
        params.insert(
            "uiModuleId".to_string(),
            serde_json::Value::String(uiModuleId.clone()),
        );
        params.insert(
            "routeInstanceId".to_string(),
            serde_json::Value::String(routeId.clone()),
        );
        params.insert(
            "__operit_ui_package_name".to_string(),
            serde_json::Value::String(containerPackageName.to_string()),
        );
        params.insert(
            "__operit_ui_toolpkg_id".to_string(),
            serde_json::Value::String(containerPackageName.to_string()),
        );
        params.insert(
            "__operit_ui_module_id".to_string(),
            serde_json::Value::String(uiModuleId.clone()),
        );
        params.insert(
            "__operit_route_instance_id".to_string(),
            serde_json::Value::String(routeId.clone()),
        );
        params.insert(
            "__operit_script_screen".to_string(),
            serde_json::Value::String(screen.clone()),
        );
        params.insert(
            "__operit_compose_execution_context_key".to_string(),
            serde_json::Value::String(contextKey),
        );
        params.insert(
            "__operit_toolpkg_runtime_kind".to_string(),
            serde_json::Value::String("ui".to_string()),
        );
        let mut moduleSpec = serde_json::Map::new();
        moduleSpec.insert(
            "id".to_string(),
            serde_json::Value::String(uiModuleId.clone()),
        );
        moduleSpec.insert("routeId".to_string(), serde_json::Value::String(routeId));
        moduleSpec.insert(
            "runtime".to_string(),
            serde_json::Value::String(
                route
                    .as_ref()
                    .map(|route| route.runtime.clone())
                    .or_else(|| module.as_ref().map(|module| module.runtime.clone()))
                    .unwrap_or_else(|| "compose_dsl".to_string()),
            ),
        );
        moduleSpec.insert("screen".to_string(), serde_json::Value::String(screen));
        moduleSpec.insert(
            "toolPkgId".to_string(),
            serde_json::Value::String(containerPackageName.to_string()),
        );
        params.insert(
            "moduleSpec".to_string(),
            serde_json::Value::Object(moduleSpec),
        );
        params
    }

    #[allow(non_snake_case)]
    pub fn runToolPkgMainHook(
        &self,
        containerPackageName: &str,
        functionName: &str,
        event: &str,
        eventName: Option<&str>,
        pluginId: Option<&str>,
        inlineFunctionSource: Option<&str>,
        eventPayload: serde_json::Value,
        executionContextKey: Option<&str>,
        runtimeKind: Option<&str>,
        onIntermediateResult: Option<std::sync::Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Result<Option<String>, String> {
        PackageManagerToolPkgFacade::runToolPkgMainHook(
            &self.toolPkgManager,
            &self.context,
            &self.getEnabledPackageNames(),
            containerPackageName,
            functionName,
            event,
            eventName,
            pluginId,
            inlineFunctionSource,
            eventPayload,
            executionContextKey,
            runtimeKind,
            onIntermediateResult,
        )
    }

    #[allow(non_snake_case)]
    pub fn useMCPServer(&mut self, serverName: &str) -> String {
        if !self.isRegisteredMCPServer(serverName) {
            return format!(
                "MCP server '{}' does not exist or is not registered.",
                serverName
            );
        }
        let Some(serverConfig) = self
            .mcpManager
            .getRegisteredServers()
            .get(serverName)
            .cloned()
        else {
            return format!("Cannot get MCP server configuration: {}", serverName);
        };
        let mcpLoadResult = MCPPackage::loadFromServer(&self.context, serverConfig);
        let Some(mcpPackage) = mcpLoadResult.mcpPackage else {
            return mcpLoadResult
                .errorMessage
                .map(|message| {
                    format!("Cannot connect to MCP server '{}': {}", serverName, message)
                })
                .unwrap_or_else(|| format!("Cannot connect to MCP server: {}", serverName));
        };
        let toolPackage = mcpPackage.toToolPackage();
        self.availablePackages
            .insert(toolPackage.name.clone(), toolPackage.clone());
        self.activatePackage(serverName);
        self.generateMCPSystemPrompt(&toolPackage, serverName)
    }

    #[allow(non_snake_case)]
    fn isRegisteredMCPServer(&self, serverName: &str) -> bool {
        self.mcpManager.isServerRegistered(serverName)
    }

    #[allow(non_snake_case)]
    fn normalizePackageName(&self, packageName: &str) -> String {
        packageName.trim().to_string()
    }

    #[allow(non_snake_case)]
    fn normalizeEnabledPackageNames(&self, packageNames: &[String]) -> Vec<String> {
        let mut normalized = BTreeSet::new();
        for original in packageNames {
            let canonical = self.normalizePackageName(original);
            if !canonical.trim().is_empty() {
                normalized.insert(canonical);
            }
        }
        normalized.into_iter().collect()
    }

    #[allow(non_snake_case)]
    fn decodeEnabledPackageNamesFromPrefs(&self) -> Vec<String> {
        let key = stringPreferencesKey(ENABLED_PACKAGES_KEY);
        let preferences = match self.dataStore.data() {
            Ok(preferences) => preferences,
            Err(_) => return Vec::new(),
        };
        let Some(packagesJson) = preferences.get(&key) else {
            return Vec::new();
        };
        let rawPackages = match serde_json::from_str::<Vec<String>>(packagesJson) {
            Ok(rawPackages) => rawPackages,
            Err(_) => return Vec::new(),
        };
        self.normalizeEnabledPackageNames(&rawPackages)
    }

    #[allow(non_snake_case)]
    fn decodeDisabledPackageNamesFromPrefs(&self) -> Vec<String> {
        let key = stringPreferencesKey(DISABLED_PACKAGES_KEY);
        let preferences = match self.dataStore.data() {
            Ok(preferences) => preferences,
            Err(_) => return Vec::new(),
        };
        let Some(packagesJson) = preferences.get(&key) else {
            return Vec::new();
        };
        let rawPackages = match serde_json::from_str::<Vec<String>>(packagesJson) {
            Ok(rawPackages) => rawPackages,
            Err(_) => return Vec::new(),
        };
        self.normalizeEnabledPackageNames(&rawPackages)
    }

    #[allow(non_snake_case)]
    fn saveEnabledPackageNames(
        &self,
        enabledPackageNames: &[String],
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedPackages = self.normalizeEnabledPackageNames(enabledPackageNames);
        let updatedJson = serde_json::to_string(&normalizedPackages)?;
        self.dataStore.edit(|preferences| {
            preferences.set(&stringPreferencesKey(ENABLED_PACKAGES_KEY), updatedJson);
        })
    }

    #[allow(non_snake_case)]
    fn saveDisabledPackageNames(
        &self,
        disabledPackageNames: &[String],
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedPackages = self.normalizeEnabledPackageNames(disabledPackageNames);
        let updatedJson = serde_json::to_string(&normalizedPackages)?;
        self.dataStore.edit(|preferences| {
            preferences.set(&stringPreferencesKey(DISABLED_PACKAGES_KEY), updatedJson);
        })
    }

    #[allow(non_snake_case)]
    fn removeFromDisabledPackages(
        &self,
        packageName: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        let mut disabledPackageNames =
            BTreeSet::from_iter(self.decodeDisabledPackageNamesFromPrefs());
        if disabledPackageNames.remove(&normalizedPackageName) {
            let names = disabledPackageNames.into_iter().collect::<Vec<_>>();
            self.saveDisabledPackageNames(&names)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    fn addToDisabledIfDefaultEnabled(
        &self,
        packageName: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        let Some(toolPackage) = self.availablePackages.get(&normalizedPackageName) else {
            return Ok(());
        };
        if !toolPackage.is_built_in || !toolPackage.enabled_by_default {
            return Ok(());
        }
        let mut disabledPackageNames =
            BTreeSet::from_iter(self.decodeDisabledPackageNamesFromPrefs());
        if disabledPackageNames.insert(normalizedPackageName) {
            let names = disabledPackageNames.into_iter().collect::<Vec<_>>();
            self.saveDisabledPackageNames(&names)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    fn removeFromCachesAfterDelete(&mut self, packageName: &str) {
        if let Some(container) = self.toolPkgManager.removeToolPkgContainer(packageName) {
            self.availablePackages.remove(packageName);
            for subpackage in container.subpackages {
                self.availablePackages.remove(&subpackage.packageName);
            }
            return;
        }

        self.availablePackages.remove(packageName);
    }

    #[allow(non_snake_case)]
    fn findPackageFile(&mut self, packageName: &str) -> Option<PathBuf> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        let packagesDir = self.storePaths.packages_dir();
        if !packagesDir.exists() {
            return None;
        }

        if let Some(containerRuntime) = self
            .toolPkgManager
            .getToolPkgContainerRuntime(&normalizedPackageName)
        {
            if containerRuntime.sourceType == ToolPkgSourceType::EXTERNAL {
                let candidate = PathBuf::from(containerRuntime.sourcePath);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        let jsFile = packagesDir.join(format!("{}.js", normalizedPackageName));
        if jsFile.exists() {
            return Some(jsFile);
        }

        let entries = fs::read_dir(&packagesDir).ok()?;
        for entry in entries.flatten() {
            let file = entry.path();
            if !file.is_file() {
                continue;
            }
            let lowerName = file.to_string_lossy().to_ascii_lowercase();
            if lowerName.ends_with(".js") {
                if let Some(loadedPackage) = self.loadPackageFromJsFile(&file) {
                    if loadedPackage.name == normalizedPackageName {
                        return Some(file);
                    }
                }
            }
        }

        None
    }

    #[allow(non_snake_case)]
    fn generatePackageSystemPrompt(&self, toolPackage: &ToolPackage) -> String {
        let mut prompt = String::new();
        prompt.push_str(&format!("Using package: {}\n", toolPackage.name));
        prompt.push_str(&format!("Use Time: {}\n", currentUseTime()));
        prompt.push_str(&format!(
            "Description: {}\n\n",
            toolPackage.description.resolve(false)
        ));
        prompt.push_str("Available tools in this package:\n");

        for tool in &toolPackage.tools {
            if tool.advice {
                prompt.push_str(&format!(
                    "- (advice): {}\n",
                    tool.description.resolve(false)
                ));
            } else {
                prompt.push_str(&format!(
                    "- {}:{}: {}\n",
                    toolPackage.name,
                    tool.name,
                    tool.description.resolve(false)
                ));
            }
            if !tool.parameters.is_empty() {
                prompt.push_str("  Parameters:\n");
                for parameter in &tool.parameters {
                    let requiredText = if parameter.required {
                        "(required)"
                    } else {
                        "(optional)"
                    };
                    prompt.push_str(&format!(
                        "  - {} {}: {}\n",
                        parameter.name,
                        requiredText,
                        parameter.description.resolve(false)
                    ));
                }
            }
            prompt.push('\n');
        }

        prompt
    }

    #[allow(non_snake_case)]
    fn loadPackageFromJsFile(&self, file: &Path) -> Option<ToolPackage> {
        let jsContent = fs::read_to_string(file).ok()?;
        self.parseJsPackage(&jsContent).ok()
    }

    #[allow(non_snake_case)]
    fn loadToolPkgFromExternalFile(&self, file: &Path) -> Result<ToolPkgLoadResult, String> {
        ToolPkgLoader::loadToolPkgFromExternalFile(file, &self.context, |jsContent| {
            self.parseJsPackage(jsContent)
        })
    }

    #[allow(non_snake_case)]
    fn loadToolPkgFromBuiltInAsset(
        &self,
        assetName: &str,
        bytes: &'static [u8],
    ) -> Result<ToolPkgLoadResult, String> {
        ToolPkgLoader::loadToolPkgFromBuiltInAsset(assetName, bytes, &self.context, |jsContent| {
            self.parseJsPackage(jsContent)
        })
    }

    #[allow(non_snake_case)]
    fn loadToolPkgFromBuiltInAssetFile(
        &self,
        assetName: &str,
        file: &Path,
    ) -> Result<ToolPkgLoadResult, String> {
        ToolPkgLoader::loadToolPkgFromBuiltInAssetFile(
            assetName,
            file,
            &self.context,
            |jsContent| self.parseJsPackage(jsContent),
        )
    }

    #[allow(non_snake_case)]
    fn parseJsPackage(&self, jsContent: &str) -> Result<ToolPackage, String> {
        let metadataString = self.extractMetadataFromJs(jsContent);
        let packageMetadata = self.parsePackageMetadata(&metadataString, jsContent)?;
        let tools = packageMetadata
            .tools
            .into_iter()
            .map(|tool| PackageTool {
                script: jsContent.to_string(),
                ..tool
            })
            .collect::<Vec<_>>();
        let states = packageMetadata
            .states
            .into_iter()
            .map(|state| ToolPackageState {
                tools: state
                    .tools
                    .into_iter()
                    .map(|tool| PackageTool {
                        script: jsContent.to_string(),
                        ..tool
                    })
                    .collect(),
                ..state
            })
            .collect();
        Ok(ToolPackage {
            tools,
            states,
            ..packageMetadata
        })
    }

    #[allow(non_snake_case)]
    fn parsePackageMetadata(
        &self,
        metadataString: &str,
        script: &str,
    ) -> Result<ToolPackage, String> {
        let normalized = normalizeHjsonLikeMetadata(metadataString);
        let value: serde_json::Value =
            json5::from_str(&normalized).map_err(|error| error.to_string())?;
        let object = value
            .as_object()
            .ok_or_else(|| "Package metadata must be an object".to_string())?;

        let name = stringField(object, "name");
        if name.is_empty() {
            return Err("Package metadata must have a name".to_string());
        }
        let toolsValue = object
            .get("tools")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let tools = toolsValue
            .iter()
            .filter_map(|value| parsePackageTool(value, script).ok())
            .collect::<Vec<_>>();
        let states = object
            .get("states")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|value| parsePackageState(value, script).ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let env = object
            .get("env")
            .and_then(serde_json::Value::as_array)
            .map(|items| items.iter().filter_map(parseEnvVar).collect::<Vec<_>>())
            .unwrap_or_default();

        Ok(ToolPackage {
            name,
            description: localizedTextField(object.get("description")),
            tools,
            states,
            env,
            is_built_in: boolField(object, "isBuiltIn") || boolField(object, "is_built_in"),
            enabled_by_default: boolField(object, "enabledByDefault")
                || boolField(object, "enabled_by_default"),
            display_name: localizedTextField(
                object
                    .get("display_name")
                    .or_else(|| object.get("displayName")),
            ),
            category: stringField(object, "category").if_empty_then("Other"),
            author: stringListField(object.get("author")),
        })
    }

    #[allow(non_snake_case)]
    fn extractMetadataFromJs(&self, jsContent: &str) -> String {
        let metadataPattern =
            regex::Regex::new(r"/\*\s*METADATA\s*([\s\S]*?)\*/").expect("valid metadata regex");
        metadataPattern
            .captures(jsContent)
            .and_then(|captures| captures.get(1))
            .map(|metadata| metadata.as_str().trim().to_string())
            .unwrap_or_else(|| "{}".to_string())
    }

    #[allow(non_snake_case)]
    fn selectToolPackageState(&mut self, toolPackage: &ToolPackage) -> ToolPackage {
        if toolPackage.states.is_empty() {
            self.activePackageStateIds.remove(&toolPackage.name);
            return toolPackage.clone();
        }
        let capabilities = buildConditionCapabilitiesSnapshot();
        let selectedState = toolPackage
            .states
            .iter()
            .find(|state| evaluateCondition(&state.condition, &capabilities));
        let Some(selectedState) = selectedState else {
            self.activePackageStateIds.remove(&toolPackage.name);
            return toolPackage.clone();
        };
        self.activePackageStateIds
            .insert(toolPackage.name.clone(), Some(selectedState.id.clone()));

        let tools = mergeToolsForState(&toolPackage.tools, selectedState);
        ToolPackage {
            tools,
            ..toolPackage.clone()
        }
    }

    #[allow(non_snake_case)]
    fn selectToolPackageStateSnapshot(&self, toolPackage: &ToolPackage) -> ToolPackage {
        if toolPackage.states.is_empty() {
            return toolPackage.clone();
        }
        let capabilities = buildConditionCapabilitiesSnapshot();
        let selectedState = toolPackage
            .states
            .iter()
            .find(|state| evaluateCondition(&state.condition, &capabilities));
        let Some(selectedState) = selectedState else {
            return toolPackage.clone();
        };
        ToolPackage {
            tools: mergeToolsForState(&toolPackage.tools, selectedState),
            ..toolPackage.clone()
        }
    }

    #[allow(non_snake_case)]
    fn generateMCPSystemPrompt(&self, toolPackage: &ToolPackage, serverName: &str) -> String {
        let mut prompt = String::new();
        prompt.push_str(&format!("Using MCP server: {}\n", serverName));
        prompt.push_str(&format!("Time: {}\n", currentUseTime()));
        prompt.push_str(&format!(
            "Description: {}\n\n",
            toolPackage.description.resolve(false)
        ));
        prompt.push_str("Available tools:\n");

        for tool in &toolPackage.tools {
            prompt.push_str(&format!(
                "- {}:{}: {}\n",
                serverName,
                tool.name,
                tool.description.resolve(false)
            ));
            if !tool.parameters.is_empty() {
                prompt.push_str("  Parameters:\n");
                for parameter in &tool.parameters {
                    let requiredText = if parameter.required {
                        "(required)"
                    } else {
                        "(optional)"
                    };
                    prompt.push_str(&format!(
                        "  - {} {}: {}\n",
                        parameter.name,
                        requiredText,
                        parameter.description.resolve(false)
                    ));
                }
            }
            prompt.push('\n');
        }

        prompt
    }
}

#[allow(non_snake_case)]
fn currentUseTime() -> String {
    chrono::Local::now()
        .naive_local()
        .format("%Y-%m-%dT%H:%M:%S%.f")
        .to_string()
}

#[allow(non_snake_case)]
fn builtInPackageAssetsDir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("plugins")
        .join("buildin")
}

#[derive(Clone, Debug, Default)]
#[allow(non_snake_case)]
struct AuthorDeclaration {
    hasDeclaredAuthorField: bool,
    declaredAuthorSlotCount: usize,
}

#[allow(non_snake_case)]
fn inspectAuthorDeclarationFromMetadata(metadataString: &str) -> AuthorDeclaration {
    let normalized = normalizeHjsonLikeMetadata(metadataString);
    let Ok(value) = json5::from_str::<serde_json::Value>(&normalized) else {
        return AuthorDeclaration::default();
    };
    let Some(object) = value.as_object() else {
        return AuthorDeclaration::default();
    };
    let author = object.get("author");
    AuthorDeclaration {
        hasDeclaredAuthorField: author.is_some(),
        declaredAuthorSlotCount: countDeclaredAuthorSlots(author),
    }
}

#[allow(non_snake_case)]
fn countDeclaredAuthorSlots(value: Option<&serde_json::Value>) -> usize {
    match value {
        None | Some(serde_json::Value::Null) => 0,
        Some(serde_json::Value::Array(items)) => items.len(),
        Some(_) => 1,
    }
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
fn mergeToolsForState(baseTools: &[PackageTool], state: &ToolPackageState) -> Vec<PackageTool> {
    let mut toolMap = BTreeMap::new();
    if state.inherit_tools {
        for tool in baseTools {
            toolMap.insert(tool.name.clone(), tool.clone());
        }
    }
    for toolName in &state.exclude_tools {
        toolMap.remove(toolName);
    }
    for tool in &state.tools {
        toolMap.insert(tool.name.clone(), tool.clone());
    }
    toolMap.into_values().collect()
}

#[allow(non_snake_case)]
fn buildConditionCapabilitiesSnapshot() -> BTreeMap<String, ConditionValue> {
    BTreeMap::from([
        (
            "ui.virtual_display".to_string(),
            ConditionValue::Bool(false),
        ),
        (
            "android.permission_level".to_string(),
            ConditionValue::Str("STANDARD".to_string()),
        ),
        (
            "android.shizuku_available".to_string(),
            ConditionValue::Bool(false),
        ),
        ("ui.shower_display".to_string(), ConditionValue::Bool(false)),
    ])
}

#[derive(Clone, Debug, PartialEq)]
enum ConditionValue {
    Bool(bool),
    Num(f64),
    Str(String),
    Null,
    Array(Vec<ConditionValue>),
}

impl ConditionValue {
    fn isTruthy(&self) -> bool {
        match self {
            Self::Bool(value) => *value,
            Self::Num(value) => *value != 0.0 && !value.is_nan(),
            Self::Str(value) => !value.is_empty(),
            Self::Null => false,
            Self::Array(items) => !items.is_empty(),
        }
    }

    fn toNumberOrNull(&self) -> Option<f64> {
        match self {
            Self::Num(value) => Some(*value),
            Self::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    fn compareTo(&self, other: &ConditionValue) -> Result<std::cmp::Ordering, String> {
        match (self, other) {
            (Self::Str(left), Self::Str(right)) => Ok(left.cmp(right)),
            _ => {
                let left = self
                    .toNumberOrNull()
                    .ok_or_else(|| "Cannot compare non-number".to_string())?;
                let right = other
                    .toNumberOrNull()
                    .ok_or_else(|| "Cannot compare non-number".to_string())?;
                left.partial_cmp(&right)
                    .ok_or_else(|| "Cannot compare NaN".to_string())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ConditionToken {
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(f64),
    BooleanLiteral(bool),
    NullLiteral,
    Operator(String),
    Punct(char),
    Eof,
}

#[allow(non_snake_case)]
fn evaluateCondition(expression: &str, capabilities: &BTreeMap<String, ConditionValue>) -> bool {
    let trimmed = expression.trim();
    if trimmed.is_empty() {
        return true;
    }
    let tokens = match ConditionTokenizer::new(trimmed).tokenize() {
        Ok(tokens) => tokens,
        Err(_) => return false,
    };
    let mut parser = ConditionParser::new(tokens, capabilities);
    match parser.parseExpression() {
        Ok(value) => value.isTruthy(),
        Err(_) => false,
    }
}

struct ConditionTokenizer<'a> {
    input: &'a str,
    chars: Vec<char>,
    i: usize,
}

impl<'a> ConditionTokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().collect(),
            i: 0,
        }
    }

    fn tokenize(&mut self) -> Result<Vec<ConditionToken>, String> {
        let mut out = Vec::new();
        loop {
            self.skipWs();
            if self.i >= self.chars.len() {
                out.push(ConditionToken::Eof);
                return Ok(out);
            }
            let c = self.chars[self.i];
            if matches!(c, '(' | ')' | '[' | ']' | ',') {
                out.push(ConditionToken::Punct(c));
                self.i += 1;
            } else if c == '"' || c == '\'' {
                out.push(ConditionToken::StringLiteral(self.readString(c)?));
            } else if c.is_ascii_digit()
                || (c == '.'
                    && self.i + 1 < self.chars.len()
                    && self.chars[self.i + 1].is_ascii_digit())
            {
                out.push(ConditionToken::NumberLiteral(self.readNumber()?));
            } else if isConditionIdentStart(c) {
                let ident = self.readIdentifier();
                match ident.as_str() {
                    "true" => out.push(ConditionToken::BooleanLiteral(true)),
                    "false" => out.push(ConditionToken::BooleanLiteral(false)),
                    "null" => out.push(ConditionToken::NullLiteral),
                    "in" => out.push(ConditionToken::Operator("in".to_string())),
                    _ => out.push(ConditionToken::Identifier(ident)),
                }
            } else if let Some(op) = self.readOperator() {
                out.push(ConditionToken::Operator(op));
            } else {
                return Err(format!("Unexpected character '{c}'"));
            }
        }
    }

    fn skipWs(&mut self) {
        while self.i < self.chars.len() && self.chars[self.i].is_whitespace() {
            self.i += 1;
        }
    }

    fn readIdentifier(&mut self) -> String {
        let start = self.i;
        self.i += 1;
        while self.i < self.chars.len() && isConditionIdentPart(self.chars[self.i]) {
            self.i += 1;
        }
        self.chars[start..self.i].iter().collect()
    }

    fn readString(&mut self, quote: char) -> Result<String, String> {
        self.i += 1;
        let mut out = String::new();
        while self.i < self.chars.len() {
            let c = self.chars[self.i];
            if c == quote {
                self.i += 1;
                return Ok(out);
            }
            if c == '\\' {
                if self.i + 1 >= self.chars.len() {
                    return Err("Unterminated escape".to_string());
                }
                let n = self.chars[self.i + 1];
                out.push(match n {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '\'' => '\'',
                    '"' => '"',
                    _ => n,
                });
                self.i += 2;
            } else {
                out.push(c);
                self.i += 1;
            }
        }
        Err("Unterminated string".to_string())
    }

    fn readNumber(&mut self) -> Result<f64, String> {
        let start = self.i;
        let mut hasDot = false;
        while self.i < self.chars.len() {
            let c = self.chars[self.i];
            if c.is_ascii_digit() {
                self.i += 1;
            } else if c == '.' && !hasDot {
                hasDot = true;
                self.i += 1;
            } else {
                break;
            }
        }
        self.input
            .chars()
            .skip(start)
            .take(self.i - start)
            .collect::<String>()
            .parse::<f64>()
            .map_err(|error| error.to_string())
    }

    fn readOperator(&mut self) -> Option<String> {
        for op in ["&&", "||", "==", "!=", ">=", "<=", ">", "<", "!"] {
            if self.input[self.byteIndex(self.i)..].starts_with(op) {
                self.i += op.chars().count();
                return Some(op.to_string());
            }
        }
        None
    }

    fn byteIndex(&self, charIndex: usize) -> usize {
        self.input
            .char_indices()
            .nth(charIndex)
            .map(|(index, _)| index)
            .unwrap_or(self.input.len())
    }
}

fn isConditionIdentStart(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn isConditionIdentPart(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

struct ConditionParser<'a> {
    tokens: Vec<ConditionToken>,
    pos: usize,
    capabilities: &'a BTreeMap<String, ConditionValue>,
}

impl<'a> ConditionParser<'a> {
    fn new(
        tokens: Vec<ConditionToken>,
        capabilities: &'a BTreeMap<String, ConditionValue>,
    ) -> Self {
        Self {
            tokens,
            pos: 0,
            capabilities,
        }
    }

    fn parseExpression(&mut self) -> Result<ConditionValue, String> {
        self.parseOr()
    }

    fn parseOr(&mut self) -> Result<ConditionValue, String> {
        let mut left = self.parseAnd()?;
        while self.matchOp("||") {
            if left.isTruthy() {
                let _ = self.parseAnd()?;
                left = ConditionValue::Bool(true);
            } else {
                left = ConditionValue::Bool(self.parseAnd()?.isTruthy());
            }
        }
        Ok(left)
    }

    fn parseAnd(&mut self) -> Result<ConditionValue, String> {
        let mut left = self.parseEquality()?;
        while self.matchOp("&&") {
            if !left.isTruthy() {
                let _ = self.parseEquality()?;
                left = ConditionValue::Bool(false);
            } else {
                left = ConditionValue::Bool(self.parseEquality()?.isTruthy());
            }
        }
        Ok(left)
    }

    fn parseEquality(&mut self) -> Result<ConditionValue, String> {
        let mut left = self.parseRelational()?;
        loop {
            if self.matchOp("==") {
                left = ConditionValue::Bool(left == self.parseRelational()?);
            } else if self.matchOp("!=") {
                left = ConditionValue::Bool(left != self.parseRelational()?);
            } else {
                return Ok(left);
            }
        }
    }

    fn parseRelational(&mut self) -> Result<ConditionValue, String> {
        let mut left = self.parseUnary()?;
        loop {
            if self.matchOp(">=") {
                left = ConditionValue::Bool(
                    left.compareTo(&self.parseUnary()?)? != std::cmp::Ordering::Less,
                );
            } else if self.matchOp("<=") {
                left = ConditionValue::Bool(
                    left.compareTo(&self.parseUnary()?)? != std::cmp::Ordering::Greater,
                );
            } else if self.matchOp(">") {
                left = ConditionValue::Bool(
                    left.compareTo(&self.parseUnary()?)? == std::cmp::Ordering::Greater,
                );
            } else if self.matchOp("<") {
                left = ConditionValue::Bool(
                    left.compareTo(&self.parseUnary()?)? == std::cmp::Ordering::Less,
                );
            } else if self.matchOp("in") {
                let right = self.parseUnary()?;
                let ok = matches!(right, ConditionValue::Array(items) if items.iter().any(|item| item == &left));
                left = ConditionValue::Bool(ok);
            } else {
                return Ok(left);
            }
        }
    }

    fn parseUnary(&mut self) -> Result<ConditionValue, String> {
        if self.matchOp("!") {
            return Ok(ConditionValue::Bool(!self.parseUnary()?.isTruthy()));
        }
        self.parsePrimary()
    }

    fn parsePrimary(&mut self) -> Result<ConditionValue, String> {
        match self.peek().clone() {
            ConditionToken::BooleanLiteral(value) => {
                self.pos += 1;
                Ok(ConditionValue::Bool(value))
            }
            ConditionToken::NullLiteral => {
                self.pos += 1;
                Ok(ConditionValue::Null)
            }
            ConditionToken::NumberLiteral(value) => {
                self.pos += 1;
                Ok(ConditionValue::Num(value))
            }
            ConditionToken::StringLiteral(value) => {
                self.pos += 1;
                Ok(ConditionValue::Str(value))
            }
            ConditionToken::Identifier(name) => {
                self.pos += 1;
                Ok(self
                    .capabilities
                    .get(&name)
                    .cloned()
                    .unwrap_or(ConditionValue::Null))
            }
            ConditionToken::Punct('(') => {
                self.pos += 1;
                let inner = self.parseExpression()?;
                self.expectPunct(')')?;
                Ok(inner)
            }
            ConditionToken::Punct('[') => {
                self.pos += 1;
                let mut elements = Vec::new();
                if !self.checkPunct(']') {
                    elements.push(self.parseExpression()?);
                    while self.matchPunct(',') {
                        elements.push(self.parseExpression()?);
                    }
                }
                self.expectPunct(']')?;
                Ok(ConditionValue::Array(elements))
            }
            token => Err(format!("Unexpected token: {token:?}")),
        }
    }

    fn peek(&self) -> &ConditionToken {
        self.tokens.get(self.pos).unwrap_or(&ConditionToken::Eof)
    }

    fn matchOp(&mut self, op: &str) -> bool {
        if matches!(self.peek(), ConditionToken::Operator(value) if value == op) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn matchPunct(&mut self, ch: char) -> bool {
        if matches!(self.peek(), ConditionToken::Punct(value) if *value == ch) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn checkPunct(&self, ch: char) -> bool {
        matches!(self.peek(), ConditionToken::Punct(value) if *value == ch)
    }

    fn expectPunct(&mut self, ch: char) -> Result<(), String> {
        if self.matchPunct(ch) {
            Ok(())
        } else {
            Err(format!("Expected '{ch}'"))
        }
    }
}

trait EmptyStringExt {
    fn if_empty_then(self, value: &str) -> String;
}

impl EmptyStringExt for String {
    fn if_empty_then(self, value: &str) -> String {
        if self.trim().is_empty() {
            value.to_string()
        } else {
            self
        }
    }
}

fn stringField(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    object
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn boolField(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> bool {
    match object.get(key) {
        Some(serde_json::Value::Bool(value)) => *value,
        Some(serde_json::Value::Number(value)) => value.as_i64().unwrap_or(0) != 0,
        Some(serde_json::Value::String(value)) => {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "true" | "1" | "yes" | "on"
            )
        }
        _ => false,
    }
}

fn localizedTextField(value: Option<&serde_json::Value>) -> LocalizedText {
    match value {
        Some(serde_json::Value::String(text)) => {
            let mut values = std::collections::HashMap::new();
            values.insert("default".to_string(), text.clone());
            LocalizedText { values }
        }
        Some(serde_json::Value::Object(object)) => {
            let mut values = std::collections::HashMap::new();
            for (key, value) in object {
                if let Some(text) = value.as_str() {
                    values.insert(key.clone(), text.to_string());
                }
            }
            LocalizedText { values }
        }
        _ => LocalizedText::default(),
    }
}

fn stringListField(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::String(text)) => vec![text.trim().to_string()]
            .into_iter()
            .filter(|item| !item.is_empty())
            .collect(),
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn parsePackageTool(value: &serde_json::Value, script: &str) -> Result<PackageTool, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "Package tool must be an object".to_string())?;
    let parameters = object
        .get("parameters")
        .and_then(serde_json::Value::as_array)
        .map(|items| items.iter().filter_map(parsePackageToolParameter).collect())
        .unwrap_or_default();
    Ok(PackageTool {
        name: stringField(object, "name"),
        description: localizedTextField(object.get("description")),
        parameters,
        script: script.to_string(),
        advice: boolField(object, "advice"),
    })
}

fn parsePackageToolParameter(value: &serde_json::Value) -> Option<PackageToolParameter> {
    let object = value.as_object()?;
    Some(PackageToolParameter {
        name: stringField(object, "name"),
        description: localizedTextField(object.get("description")),
        parameter_type: stringField(object, "type").if_empty_then("string"),
        required: object
            .get("required")
            .map(|_| boolField(object, "required"))
            .unwrap_or(true),
    })
}

fn parsePackageState(value: &serde_json::Value, script: &str) -> Result<ToolPackageState, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "Package state must be an object".to_string())?;
    let tools = object
        .get("tools")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| parsePackageTool(item, script).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(ToolPackageState {
        id: stringField(object, "id"),
        condition: stringField(object, "condition").if_empty_then("true"),
        inherit_tools: object
            .get("inheritTools")
            .or_else(|| object.get("inherit_tools"))
            .and_then(|_| {
                Some(boolField(object, "inheritTools") || boolField(object, "inherit_tools"))
            })
            .unwrap_or(false),
        exclude_tools: object
            .get("excludeTools")
            .or_else(|| object.get("exclude_tools"))
            .and_then(|value| Some(stringListField(Some(value))))
            .unwrap_or_default(),
        tools,
    })
}

fn parseEnvVar(value: &serde_json::Value) -> Option<EnvVar> {
    match value {
        serde_json::Value::String(name) => Some(EnvVar {
            name: name.trim().to_string(),
            description: LocalizedText::default(),
            required: true,
            default_value: None,
        }),
        serde_json::Value::Object(object) => Some(EnvVar {
            name: stringField(object, "name"),
            description: localizedTextField(object.get("description")),
            required: object
                .get("required")
                .map(|_| boolField(object, "required"))
                .unwrap_or(true),
            default_value: object
                .get("defaultValue")
                .or_else(|| object.get("default_value"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
        }),
        _ => None,
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
