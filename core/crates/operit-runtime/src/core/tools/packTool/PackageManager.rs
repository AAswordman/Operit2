use std::collections::{BTreeMap, BTreeSet};

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::core::tools::ToolPackage::ToolPackage;
use crate::core::tools::skill::SkillManager::SkillManager;
use crate::data::preferences::SkillVisibilityPreferences::SkillVisibilityPreferences;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

const ENABLED_PACKAGES_KEY: &str = "imported_packages";

#[derive(Clone, Debug, Default)]
pub struct MCPServerConfig {
    pub description: String,
}

#[derive(Clone, Debug, Default)]
pub struct CachedMcpToolInfo {
    pub name: String,
    pub description: String,
    pub inputSchema: String,
}

#[derive(Clone, Debug)]
pub struct PackageManager {
    activatedPackages: BTreeSet<String>,
    availablePackages: BTreeMap<String, ToolPackage>,
    availableServerPackages: BTreeMap<String, MCPServerConfig>,
    cachedMcpTools: BTreeMap<String, Vec<CachedMcpToolInfo>>,
    toolPkgContainers: BTreeSet<String>,
    dataStore: PreferencesDataStore,
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new(RuntimeStorePaths::default())
    }
}

impl PackageManager {
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            activatedPackages: BTreeSet::new(),
            availablePackages: BTreeMap::new(),
            availableServerPackages: BTreeMap::new(),
            cachedMcpTools: BTreeMap::new(),
            toolPkgContainers: BTreeSet::new(),
            dataStore: PreferencesDataStore::new(paths.package_manager_preferences_path()),
        }
    }

    pub fn activatePackage(&mut self, packageName: &str) -> bool {
        let normalizedPackageName = self.normalizePackageName(packageName);
        self.activatedPackages.insert(normalizedPackageName)
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
            let Some(toolPackage) = self.availablePackages.get(&normalizedPackageName).cloned() else {
                return format!("Failed to load package data for: {}", normalizedPackageName);
            };
            self.activatePackage(&normalizedPackageName);
            return self.generatePackageSystemPrompt(&toolPackage);
        }

        let skillManager = SkillManager::getInstance();
        let skillVisibilityPreferences = SkillVisibilityPreferences::getInstance();
        if skillManager
            .getAvailableSkills()
            .contains_key(&normalizedPackageName)
            && !skillVisibilityPreferences.isSkillVisibleToAi(&normalizedPackageName)
        {
            return format!(
                "Skill '{}' is set to not show to AI",
                normalizedPackageName
            );
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
        self.decodeEnabledPackageNamesFromPrefs()
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
            return format!("Failed to enable package '{}': {}", normalizedPackageName, error);
        }
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
                return format!("Failed to disable package '{}': {}", normalizedPackageName, error);
            }
            return format!("Successfully disabled package: {}", normalizedPackageName);
        }
        format!("Package is already disabled: {}", normalizedPackageName)
    }

    #[allow(non_snake_case)]
    pub fn isToolPkgContainer(&self, packageName: &str) -> bool {
        let normalizedPackageName = self.normalizePackageName(packageName);
        self.toolPkgContainers.contains(&normalizedPackageName)
    }

    #[allow(non_snake_case)]
    pub fn getEffectivePackageTools(&self, packageName: &str) -> Option<ToolPackage> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        self.availablePackages.get(&normalizedPackageName).cloned()
    }

    #[allow(non_snake_case)]
    pub fn getAvailablePackages(&self) -> BTreeMap<String, ToolPackage> {
        self.availablePackages.clone()
    }

    #[allow(non_snake_case)]
    pub fn getAvailableServerPackages(&self) -> BTreeMap<String, MCPServerConfig> {
        self.availableServerPackages.clone()
    }

    #[allow(non_snake_case)]
    pub fn getCachedMcpTools(&self, serverName: &str) -> Vec<CachedMcpToolInfo> {
        self.cachedMcpTools
            .get(serverName)
            .cloned()
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn setAvailablePackage(&mut self, packageName: String, toolPackage: ToolPackage) {
        let normalizedPackageName = self.normalizePackageName(&packageName);
        self.availablePackages.insert(normalizedPackageName, toolPackage);
    }

    #[allow(non_snake_case)]
    pub fn setEnabledPackageNames(
        &self,
        packageNames: &[String],
    ) -> Result<(), PreferencesDataStoreError> {
        self.saveEnabledPackageNames(packageNames)
    }

    #[allow(non_snake_case)]
    pub fn setAvailableServerPackage(
        &mut self,
        serverName: String,
        serverConfig: MCPServerConfig,
    ) {
        self.availableServerPackages.insert(serverName, serverConfig);
    }

    #[allow(non_snake_case)]
    pub fn setCachedMcpTools(&mut self, serverName: String, tools: Vec<CachedMcpToolInfo>) {
        self.cachedMcpTools.insert(serverName, tools);
    }

    #[allow(non_snake_case)]
    pub fn useMCPServer(&mut self, serverName: &str) -> String {
        if !self.isRegisteredMCPServer(serverName) {
            return format!("MCP server '{}' does not exist or is not registered.", serverName);
        }
        let Some(serverConfig) = self.availableServerPackages.get(serverName).cloned() else {
            return format!("Cannot get MCP server configuration: {}", serverName);
        };
        self.activatePackage(serverName);
        self.generateMCPSystemPrompt(serverName, &serverConfig)
    }

    #[allow(non_snake_case)]
    fn isRegisteredMCPServer(&self, serverName: &str) -> bool {
        self.availableServerPackages.contains_key(serverName)
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
    fn generateMCPSystemPrompt(
        &self,
        serverName: &str,
        serverConfig: &MCPServerConfig,
    ) -> String {
        let mut prompt = String::new();
        prompt.push_str(&format!("Using MCP server: {}\n", serverName));
        prompt.push_str(&format!("Time: {}\n", currentUseTime()));
        prompt.push_str(&format!("Description: {}\n\n", serverConfig.description));
        prompt.push_str("Available tools:\n");

        for tool in self.getCachedMcpTools(serverName) {
            prompt.push_str(&format!("- {}:{}: {}\n\n", serverName, tool.name, tool.description));
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
