use std::collections::{HashMap, HashSet};

use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use operit_host_api::HostManager::HostManager;
use operit_tools::runtime_support::ResolvedCharacterCardToolAccess;
use operit_tools::tools::packTool::PackageManager::PackageManager;
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;

pub struct CharacterCardToolAccessResolver;

impl CharacterCardToolAccessResolver {
    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self
    }

    #[allow(non_snake_case)]
    pub fn resolve(
        &self,
        roleCardId: Option<&str>,
        packageManager: &PackageManager,
        globalToolVisibility: Option<HashMap<String, bool>>,
    ) -> ResolvedCharacterCardToolAccess {
        let effectiveGlobalToolVisibility = globalToolVisibility.unwrap_or_default();

        let globalPackageNames = packageManager
            .getEnabledPackageNames()
            .into_iter()
            .filter(|packageName| !packageManager.isToolPkgContainer(packageName))
            .collect::<HashSet<_>>();
        let globalSkillNames = SkillRepository::getInstance(&HostManager::new())
            .getAiVisibleSkillPackages()
            .keys()
            .cloned()
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

        let manageableBuiltinNames = manageable_tool_names();
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

fn manageable_tool_names() -> HashSet<String> {
    [
        "use_package",
        "package_proxy",
        "read_file",
        "read_file_full",
        "read_file_part",
        "write_file",
        "edit_file",
        "create_file",
        "delete_file",
        "list_files",
        "find_files",
        "file_info",
        "file_exists",
        "move_file",
        "copy_file",
        "make_directory",
        "grep_code",
        "search",
        "proxy",
        "sleep",
    ]
    .into_iter()
    .map(|value| value.to_string())
    .collect()
}

fn normalize_entries(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}
