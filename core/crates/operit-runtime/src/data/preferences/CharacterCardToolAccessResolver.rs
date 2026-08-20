use std::collections::{HashMap, HashSet};

use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::SkillVisibilityPreferences::SkillVisibilityPreferences;
use operit_providers::chat::config::SystemToolPrompts::{ManageableToolPrompt, SystemToolPrompts};
use operit_tools::runtime_support::ResolvedCharacterCardToolAccess;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::skill::SkillManager::SkillManager;

pub struct CharacterCardToolAccessResolver;

impl CharacterCardToolAccessResolver {
    /// Creates the stateless character-card tool access resolver.
    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self
    }

    /// Returns the canonical built-in tool options used by character-card access control.
    #[allow(non_snake_case)]
    pub fn getManageableBuiltinToolOptions(&self, useEnglish: bool) -> Vec<ManageableToolPrompt> {
        SystemToolPrompts::getManageableToolPrompts(useEnglish)
    }

    /// Resolves effective built-in and external tool access for one character card.
    #[allow(non_snake_case)]
    pub fn resolve(
        &self,
        roleCardId: Option<&str>,
        packageManager: &RuntimePackageManager,
        globalToolVisibility: Option<HashMap<String, bool>>,
    ) -> ResolvedCharacterCardToolAccess {
        let effectiveGlobalToolVisibility = globalToolVisibility.unwrap_or_default();

        let globalPackageNames = packageManager
            .getEnabledPackageNames()
            .into_iter()
            .map(|packageName| packageName.trim().to_string())
            .filter(|packageName| {
                !packageName.is_empty()
                    && packageManager.getPackageTools(packageName).is_some()
                    && !packageManager.isToolPkgContainer(packageName)
            })
            .collect::<HashSet<_>>();
        let globalSkillNames = SkillManager::fromDefaultPaths(packageManager.fileSystemHost())
            .getAvailableSkills()
            .into_keys()
            .filter(|name| SkillVisibilityPreferences::getInstance().isSkillVisibleToAi(name))
            .collect::<HashSet<_>>();
        let globalMcpServerNames = packageManager
            .getAvailableServerPackages()
            .keys()
            .cloned()
            .collect::<HashSet<_>>();

        let roleCardConfig = roleCardId
            .filter(|id| !id.trim().is_empty())
            .and_then(|cardId| {
                CharacterCardManager::getInstance()
                    .getCharacterCard(cardId)
                    .ok()
                    .map(|card| card.toolAccessConfig.normalized())
            })
            .unwrap_or_default();

        if !roleCardConfig.enabled {
            let hasAnyGlobalExternalSource = !globalPackageNames.is_empty()
                || !globalSkillNames.is_empty()
                || !globalMcpServerNames.is_empty();
            return ResolvedCharacterCardToolAccess {
                customEnabled: false,
                effectiveBuiltinToolVisibility: effectiveGlobalToolVisibility,
                allowedPackageNames: globalPackageNames,
                allowedSkillNames: globalSkillNames,
                allowedMcpServerNames: globalMcpServerNames,
                canUsePackageSystem: true,
                hasAnyAllowedExternalSource: hasAnyGlobalExternalSource,
            };
        }

        let manageableBuiltinNames = SystemToolPrompts::getManageableToolPrompts(false)
            .into_iter()
            .map(|tool| tool.name)
            .collect::<HashSet<_>>();
        let allowedBuiltinTools = normalize_entries(&roleCardConfig.allowedBuiltinTools);
        let effectiveBuiltinToolVisibility = manageableBuiltinNames
            .into_iter()
            .map(|toolName| {
                let visible = effectiveGlobalToolVisibility
                    .get(&toolName)
                    .copied()
                    .unwrap_or(true)
                    && allowedBuiltinTools.contains(&toolName);
                (toolName, visible)
            })
            .collect::<HashMap<_, _>>();

        let canUsePackageSystem = effectiveBuiltinToolVisibility
            .get("use_package")
            .copied()
            .unwrap_or(false);
        let allowedPackages = if canUsePackageSystem {
            globalPackageNames
                .iter()
                .filter(|name| {
                    roleCardConfig
                        .allowedPackages
                        .iter()
                        .any(|allowed| allowed == *name)
                })
                .cloned()
                .collect::<HashSet<_>>()
        } else {
            HashSet::new()
        };
        let allowedSkills = if canUsePackageSystem {
            globalSkillNames
                .iter()
                .filter(|name| {
                    roleCardConfig
                        .allowedSkills
                        .iter()
                        .any(|allowed| allowed == *name)
                })
                .cloned()
                .collect::<HashSet<_>>()
        } else {
            HashSet::new()
        };
        let allowedMcpServers = if canUsePackageSystem {
            globalMcpServerNames
                .iter()
                .filter(|name| {
                    roleCardConfig
                        .allowedMcpServers
                        .iter()
                        .any(|allowed| allowed == *name)
                })
                .cloned()
                .collect::<HashSet<_>>()
        } else {
            HashSet::new()
        };
        let hasAnyAllowedExternalSource = !allowedPackages.is_empty()
            || !allowedSkills.is_empty()
            || !allowedMcpServers.is_empty();

        ResolvedCharacterCardToolAccess {
            customEnabled: true,
            effectiveBuiltinToolVisibility,
            allowedPackageNames: allowedPackages,
            allowedSkillNames: allowedSkills,
            allowedMcpServerNames: allowedMcpServers,
            canUsePackageSystem,
            hasAnyAllowedExternalSource,
        }
    }
}

/// Trims and deduplicates persisted tool-access entries.
fn normalize_entries(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}
