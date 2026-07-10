use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::toolpkg::ToolPkgCommonPluginConstants::{
    TOOLPKG_NAV_SURFACE_TOOLBOX, TOOLPKG_RUNTIME_COMPOSE_DSL,
};
use crate::toolpkg::ToolPkgPackageModels::{
    ToolPkgContainerDetails, ToolPkgDesktopWidget, ToolPkgNavigationActionHook,
    ToolPkgNavigationEntry, ToolPkgSubpackageInfo, ToolPkgToolboxUiModule, ToolPkgUiRoute,
    ToolPkgWorkspaceTemplate, ToolPkgWorkspaceTemplateImportResult,
};
use crate::toolpkg::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgContainerRuntime, ToolPkgResourceRuntime, ToolPkgSubpackageRuntime,
};

/// Supplies host-owned state, persistence, and resource access to ToolPkg package services.
pub trait ToolPkgPackageHost: Send + Sync {
    /// Ensures package state has been loaded before a public query.
    #[allow(non_snake_case)]
    fn ensureInitialized(&self);

    /// Normalizes a package name using the embedding application's naming rules.
    #[allow(non_snake_case)]
    fn normalizePackageName(&self, packageName: &str) -> String;

    /// Returns all registered ToolPkg container runtimes.
    #[allow(non_snake_case)]
    fn toolPkgContainersInternal(&self) -> BTreeMap<String, ToolPkgContainerRuntime>;

    /// Returns all registered ToolPkg subpackage runtimes keyed by package name.
    #[allow(non_snake_case)]
    fn toolPkgSubpackageByPackageNameInternal(&self) -> BTreeMap<String, ToolPkgSubpackageRuntime>;

    /// Resolves one ToolPkg subpackage runtime from a package name.
    #[allow(non_snake_case)]
    fn resolveToolPkgSubpackageRuntimeInternal(
        &self,
        packageName: &str,
    ) -> Option<ToolPkgSubpackageRuntime>;

    /// Returns the complete enabled package name set.
    #[allow(non_snake_case)]
    fn getEnabledPackageNameSetInternal(&self) -> BTreeSet<String>;

    /// Returns enabled package names in stable order.
    #[allow(non_snake_case)]
    fn getEnabledPackageNames(&self) -> Vec<String>;

    /// Returns persisted ToolPkg subpackage enabled states.
    #[allow(non_snake_case)]
    fn getToolPkgSubpackageStatesInternal(&self) -> BTreeMap<String, bool>;

    /// Persists the complete enabled package name list.
    #[allow(non_snake_case)]
    fn saveEnabledPackageNames(&self, packageNames: &[String]) -> Result<(), String>;

    /// Persists the complete ToolPkg subpackage state map.
    #[allow(non_snake_case)]
    fn saveToolPkgSubpackageStates(&self, states: &BTreeMap<String, bool>) -> Result<(), String>;

    /// Returns whether one package is currently enabled.
    #[allow(non_snake_case)]
    fn isPackageEnabled(&self, packageName: &str) -> bool;

    /// Resolves one ToolPkg resource to a host-readable file.
    #[allow(non_snake_case)]
    fn resolveToolPkgResourceFile(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Option<PathBuf>;

    /// Exports one ToolPkg resource to a destination file.
    #[allow(non_snake_case)]
    fn exportToolPkgResource(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resource: &ToolPkgResourceRuntime,
        destinationFile: &Path,
    ) -> bool;

    /// Reads one ToolPkg resource as raw bytes.
    #[allow(non_snake_case)]
    fn readToolPkgResourceBytes(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Option<Vec<u8>>;
}

/// Provides SDK-owned ToolPkg package queries and resource workflows.
pub struct ToolPkgPackageService<'a> {
    packageManager: &'a dyn ToolPkgPackageHost,
}

impl<'a> ToolPkgPackageService<'a> {
    /// Creates a ToolPkg package service backed by an embedding application host.
    #[allow(non_snake_case)]
    pub fn new(packageManager: &'a dyn ToolPkgPackageHost) -> Self {
        Self { packageManager }
    }

    /// Builds the serializable module specification consumed by ToolPkg UI hosts.
    #[allow(non_snake_case)]
    fn buildModuleSpec(
        id: &str,
        routeId: &str,
        runtime: &str,
        screen: &str,
        title: &str,
        toolPkgId: &str,
        keepAlive: bool,
    ) -> BTreeMap<String, Value> {
        BTreeMap::from([
            ("id".to_string(), Value::String(id.to_string())),
            ("routeId".to_string(), Value::String(routeId.to_string())),
            ("runtime".to_string(), Value::String(runtime.to_string())),
            ("screen".to_string(), Value::String(screen.to_string())),
            ("title".to_string(), Value::String(title.to_string())),
            (
                "toolPkgId".to_string(),
                Value::String(toolPkgId.to_string()),
            ),
            ("keepAlive".to_string(), Value::Bool(keepAlive)),
        ])
    }

    /// Builds toolbox modules for routes registered on the toolbox navigation surface.
    #[allow(non_snake_case)]
    fn buildToolPkgToolboxUiModules(
        &self,
        container: &ToolPkgContainerRuntime,
        useEnglish: bool,
        runtime: &str,
    ) -> Vec<ToolPkgToolboxUiModule> {
        let containerDisplayName = nonBlankOr(
            container.displayName.resolve(useEnglish).trim(),
            &container.packageName,
        );
        let containerDescription = container.description.resolve(useEnglish);
        let toolboxRouteIds = container
            .navigationEntries
            .iter()
            .filter(|entry| {
                entry
                    .surface
                    .eq_ignore_ascii_case(TOOLPKG_NAV_SURFACE_TOOLBOX)
            })
            .map(|entry| entry.routeId.to_ascii_lowercase())
            .collect::<std::collections::BTreeSet<_>>();
        let mut modules = container
            .uiRoutes
            .iter()
            .filter(|route| {
                route.runtime.eq_ignore_ascii_case(runtime)
                    && toolboxRouteIds.contains(&route.routeId.to_ascii_lowercase())
            })
            .map(|route| {
                let moduleTitle = nonBlankOr(
                    route.title.resolve(useEnglish).trim(),
                    &containerDisplayName,
                );
                ToolPkgToolboxUiModule {
                    containerPackageName: container.packageName.clone(),
                    toolPkgId: container.packageName.clone(),
                    routeId: route.routeId.clone(),
                    uiModuleId: route.id.clone(),
                    runtime: route.runtime.clone(),
                    screen: route.screen.clone(),
                    title: moduleTitle.clone(),
                    description: containerDescription.clone(),
                    keepAlive: route.keepAlive,
                    moduleSpec: Self::buildModuleSpec(
                        &route.id,
                        &route.routeId,
                        &route.runtime,
                        &route.screen,
                        &moduleTitle,
                        &container.packageName,
                        route.keepAlive,
                    ),
                }
            })
            .collect::<Vec<_>>();
        modules.sort_by(|left, right| {
            left.title
                .cmp(&right.title)
                .then_with(|| left.containerPackageName.cmp(&right.containerPackageName))
                .then_with(|| left.uiModuleId.cmp(&right.uiModuleId))
        });
        modules
    }

    /// Builds localized UI route descriptors for one ToolPkg runtime kind.
    #[allow(non_snake_case)]
    fn buildToolPkgUiRoutes(
        &self,
        container: &ToolPkgContainerRuntime,
        useEnglish: bool,
        runtime: &str,
    ) -> Vec<ToolPkgUiRoute> {
        let containerDisplayName = nonBlankOr(
            container.displayName.resolve(useEnglish).trim(),
            &container.packageName,
        );
        let containerDescription = container.description.resolve(useEnglish);
        let mut routes = container
            .uiRoutes
            .iter()
            .filter(|route| route.runtime.eq_ignore_ascii_case(runtime))
            .map(|route| {
                let routeTitle = nonBlankOr(
                    route.title.resolve(useEnglish).trim(),
                    &containerDisplayName,
                );
                ToolPkgUiRoute {
                    containerPackageName: container.packageName.clone(),
                    toolPkgId: container.packageName.clone(),
                    routeId: route.routeId.clone(),
                    uiModuleId: route.id.clone(),
                    runtime: route.runtime.clone(),
                    screen: route.screen.clone(),
                    title: routeTitle.clone(),
                    description: containerDescription.clone(),
                    keepAlive: route.keepAlive,
                    moduleSpec: Self::buildModuleSpec(
                        &route.id,
                        &route.routeId,
                        &route.runtime,
                        &route.screen,
                        &routeTitle,
                        &container.packageName,
                        route.keepAlive,
                    ),
                }
            })
            .collect::<Vec<_>>();
        routes.sort_by(|left, right| {
            left.title
                .cmp(&right.title)
                .then_with(|| left.containerPackageName.cmp(&right.containerPackageName))
                .then_with(|| left.uiModuleId.cmp(&right.uiModuleId))
        });
        routes
    }

    /// Builds localized navigation entries declared by one ToolPkg container.
    #[allow(non_snake_case)]
    fn buildToolPkgNavigationEntries(
        &self,
        container: &ToolPkgContainerRuntime,
        useEnglish: bool,
    ) -> Vec<ToolPkgNavigationEntry> {
        let containerDescription = container.description.resolve(useEnglish);
        let mut entries = container
            .navigationEntries
            .iter()
            .map(|entry| ToolPkgNavigationEntry {
                containerPackageName: container.packageName.clone(),
                toolPkgId: container.packageName.clone(),
                entryId: entry.id.clone(),
                routeId: entry.routeId.clone(),
                surface: entry.surface.clone(),
                title: nonBlankOr(entry.title.resolve(useEnglish).trim(), &entry.id),
                description: containerDescription.clone(),
                action: entry
                    .action
                    .as_ref()
                    .map(|action| ToolPkgNavigationActionHook {
                        functionName: action.function.clone(),
                        functionSource: action.functionSource.clone(),
                    }),
                icon: entry.icon.clone(),
                order: entry.order,
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            left.surface
                .cmp(&right.surface)
                .then_with(|| left.order.cmp(&right.order))
                .then_with(|| left.title.cmp(&right.title))
        });
        entries
    }

    /// Builds localized desktop widget descriptors declared by one ToolPkg container.
    #[allow(non_snake_case)]
    fn buildToolPkgDesktopWidgets(
        &self,
        container: &ToolPkgContainerRuntime,
        useEnglish: bool,
    ) -> Vec<ToolPkgDesktopWidget> {
        let mut widgets = container
            .desktopWidgets
            .iter()
            .map(|widget| ToolPkgDesktopWidget {
                containerPackageName: container.packageName.clone(),
                toolPkgId: container.packageName.clone(),
                widgetId: widget.id.clone(),
                routeId: widget.routeId.clone(),
                renderRouteId: widget.renderRouteId.clone(),
                title: nonBlankOr(widget.title.resolve(useEnglish).trim(), &widget.id),
                subtitle: widget.subtitle.resolve(useEnglish).trim().to_string(),
                description: widget.description.resolve(useEnglish).trim().to_string(),
                icon: widget.icon.clone(),
                order: widget.order,
            })
            .collect::<Vec<_>>();
        widgets.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.title.cmp(&right.title))
                .then_with(|| left.widgetId.cmp(&right.widgetId))
        });
        widgets
    }

    /// Builds localized workspace template descriptors declared by one ToolPkg container.
    #[allow(non_snake_case)]
    fn buildToolPkgWorkspaceTemplates(
        &self,
        container: &ToolPkgContainerRuntime,
        useEnglish: bool,
    ) -> Vec<ToolPkgWorkspaceTemplate> {
        let mut templates = container
            .workspaceTemplates
            .iter()
            .map(|template| ToolPkgWorkspaceTemplate {
                containerPackageName: container.packageName.clone(),
                toolPkgId: container.packageName.clone(),
                templateId: template.id.clone(),
                displayName: nonBlankOr(
                    template.display_name.resolve(useEnglish).trim(),
                    &template.id,
                ),
                description: template.description.resolve(useEnglish),
                resourceKey: template.resource_key.clone(),
                projectType: template.project_type.clone(),
            })
            .collect::<Vec<_>>();
        templates.sort_by(|left, right| {
            left.displayName
                .cmp(&right.displayName)
                .then_with(|| left.templateId.cmp(&right.templateId))
        });
        templates
    }

    /// Returns whether a package name identifies a registered ToolPkg container.
    #[allow(non_snake_case)]
    pub fn isToolPkgContainer(&self, packageName: &str) -> bool {
        self.packageManager.ensureInitialized();
        let normalizedPackageName = self.packageManager.normalizePackageName(packageName);
        self.packageManager
            .toolPkgContainersInternal()
            .contains_key(&normalizedPackageName)
    }

    /// Returns whether a package name identifies a registered ToolPkg subpackage.
    #[allow(non_snake_case)]
    pub fn isToolPkgSubpackage(&self, packageName: &str) -> bool {
        self.packageManager.ensureInitialized();
        self.packageManager
            .resolveToolPkgSubpackageRuntimeInternal(packageName)
            .is_some()
    }

    /// Returns localized public details for one ToolPkg container.
    #[allow(non_snake_case)]
    pub fn getToolPkgContainerDetails(
        &self,
        packageName: &str,
        useEnglish: bool,
    ) -> Option<ToolPkgContainerDetails> {
        self.packageManager.ensureInitialized();
        let normalizedPackageName = self.packageManager.normalizePackageName(packageName);
        let container = self
            .packageManager
            .toolPkgContainersInternal()
            .get(&normalizedPackageName)
            .cloned()?;
        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        let containerEnabled = enabledSet.contains(&container.packageName);
        let toolboxUiModules = if containerEnabled {
            self.buildToolPkgToolboxUiModules(&container, useEnglish, TOOLPKG_RUNTIME_COMPOSE_DSL)
        } else {
            Vec::new()
        };
        let subpackages = container
            .subpackages
            .iter()
            .map(|subpackage| ToolPkgSubpackageInfo {
                packageName: subpackage.packageName.clone(),
                subpackageId: subpackage.subpackageId.clone(),
                displayName: subpackage.displayName.resolve(useEnglish),
                description: subpackage.description.resolve(useEnglish),
                enabledByDefault: subpackage.enabledByDefault,
                toolCount: subpackage.toolCount,
                enabled: containerEnabled && enabledSet.contains(&subpackage.packageName),
            })
            .collect::<Vec<_>>();
        let workspaceTemplates = self.buildToolPkgWorkspaceTemplates(&container, useEnglish);
        Some(ToolPkgContainerDetails {
            packageName: container.packageName.clone(),
            displayName: container.displayName.resolve(useEnglish),
            description: container.description.resolve(useEnglish),
            version: container.version.clone(),
            author: container.author.clone(),
            resourceCount: container.resources.len(),
            workspaceTemplateCount: workspaceTemplates.len(),
            uiModuleCount: container.uiModules.len(),
            toolboxUiModules,
            subpackages,
            workspaceTemplates,
        })
    }

    /// Returns localized UI routes exposed by enabled containers for one runtime kind.
    #[allow(non_snake_case)]
    pub fn getToolPkgUiRoutes(&self, runtime: &str, useEnglish: bool) -> Vec<ToolPkgUiRoute> {
        self.packageManager.ensureInitialized();
        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        self.packageManager
            .toolPkgContainersInternal()
            .values()
            .filter(|container| enabledSet.contains(&container.packageName))
            .flat_map(|container| self.buildToolPkgUiRoutes(container, useEnglish, runtime))
            .collect()
    }

    /// Returns localized navigation entries exposed by enabled ToolPkg containers.
    #[allow(non_snake_case)]
    pub fn getToolPkgNavigationEntries(&self, useEnglish: bool) -> Vec<ToolPkgNavigationEntry> {
        self.packageManager.ensureInitialized();
        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        self.packageManager
            .toolPkgContainersInternal()
            .values()
            .filter(|container| enabledSet.contains(&container.packageName))
            .flat_map(|container| self.buildToolPkgNavigationEntries(container, useEnglish))
            .collect()
    }

    /// Returns localized desktop widgets exposed by enabled ToolPkg containers.
    #[allow(non_snake_case)]
    pub fn getToolPkgDesktopWidgets(&self, useEnglish: bool) -> Vec<ToolPkgDesktopWidget> {
        self.packageManager.ensureInitialized();
        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        let mut widgets = self
            .packageManager
            .toolPkgContainersInternal()
            .values()
            .filter(|container| enabledSet.contains(&container.packageName))
            .flat_map(|container| self.buildToolPkgDesktopWidgets(container, useEnglish))
            .collect::<Vec<_>>();
        widgets.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.title.cmp(&right.title))
                .then_with(|| left.containerPackageName.cmp(&right.containerPackageName))
                .then_with(|| left.widgetId.cmp(&right.widgetId))
        });
        widgets
    }

    /// Returns localized workspace templates exposed by enabled ToolPkg containers.
    #[allow(non_snake_case)]
    pub fn getToolPkgWorkspaceTemplates(&self, useEnglish: bool) -> Vec<ToolPkgWorkspaceTemplate> {
        self.packageManager.ensureInitialized();
        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        self.packageManager
            .toolPkgContainersInternal()
            .values()
            .filter(|container| enabledSet.contains(&container.packageName))
            .flat_map(|container| self.buildToolPkgWorkspaceTemplates(container, useEnglish))
            .collect()
    }

    /// Imports one directory-backed ToolPkg workspace template into an empty destination.
    #[allow(non_snake_case)]
    pub fn importToolPkgWorkspaceTemplate(
        &self,
        containerPackageName: &str,
        templateId: &str,
        destinationDir: &Path,
    ) -> Result<ToolPkgWorkspaceTemplateImportResult, String> {
        self.packageManager.ensureInitialized();
        let normalizedContainerPackageName = self
            .packageManager
            .normalizePackageName(containerPackageName);
        let runtime = self
            .packageManager
            .toolPkgContainersInternal()
            .get(&normalizedContainerPackageName)
            .cloned()
            .ok_or_else(|| format!("ToolPkg container not found: {containerPackageName}"))?;
        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        if !enabledSet.contains(&runtime.packageName) {
            return Err(format!(
                "ToolPkg container is not enabled: {}",
                runtime.packageName
            ));
        }
        let template = runtime
            .workspaceTemplates
            .iter()
            .find(|template| template.id.eq_ignore_ascii_case(templateId.trim()))
            .ok_or_else(|| format!("Workspace template not found: {templateId}"))?;
        let resource = runtime
            .resources
            .iter()
            .find(|resource| resource.key.eq_ignore_ascii_case(&template.resource_key))
            .ok_or_else(|| {
                format!(
                    "Workspace template resource not found: {}",
                    template.resource_key
                )
            })?;
        if !ToolPkgArchiveParser::isDirectoryResourceMime(Some(&resource.mime)) {
            return Err(format!(
                "Workspace template resource must be a directory: {}",
                template.resource_key
            ));
        }
        if destinationDir.exists() {
            if !destinationDir.is_dir() {
                return Err(format!(
                    "Workspace destination is not a directory: {}",
                    destinationDir.display()
                ));
            }
            if destinationDir
                .read_dir()
                .map_err(|error| error.to_string())?
                .next()
                .is_some()
            {
                return Err(format!(
                    "Workspace destination must be empty: {}",
                    destinationDir.display()
                ));
            }
        } else {
            fs::create_dir_all(destinationDir).map_err(|error| {
                format!(
                    "Failed to create workspace destination: {}: {}",
                    destinationDir.display(),
                    error
                )
            })?;
        }
        let resourceDir = self
            .packageManager
            .resolveToolPkgResourceFile(&runtime, &resource.path)
            .ok_or_else(|| {
                format!(
                    "Workspace template directory is unavailable: {}",
                    template.resource_key
                )
            })?;
        if !resourceDir.is_dir() {
            return Err(format!(
                "Workspace template directory is invalid: {}",
                template.resource_key
            ));
        }
        for entry in fs::read_dir(&resourceDir).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            copyRecursively(&entry.path(), &destinationDir.join(entry.file_name()))?;
        }
        let configPath = destinationDir.join(".operit").join("config.json");
        if !configPath.is_file() {
            return Err(format!(
                "Workspace template is missing .operit/config.json: {}",
                template.id
            ));
        }
        let workspaceConfig = fs::read_to_string(&configPath)
            .map_err(|error| error.to_string())
            .and_then(|text| {
                serde_json::from_str::<Value>(&text).map_err(|error| error.to_string())
            })?;
        Ok(ToolPkgWorkspaceTemplateImportResult {
            containerPackageName: runtime.packageName.clone(),
            toolPkgId: runtime.packageName.clone(),
            templateId: template.id.clone(),
            workspacePath: destinationDir.to_string_lossy().to_string(),
            workspaceConfig,
        })
    }

    /// Updates and persists the enabled state of one ToolPkg subpackage.
    #[allow(non_snake_case)]
    pub fn setToolPkgSubpackageEnabled(&self, subpackagePackageName: &str, enabled: bool) -> bool {
        self.packageManager.ensureInitialized();
        let normalizedPackageName = self
            .packageManager
            .normalizePackageName(subpackagePackageName);
        let subpackageRuntime = self
            .packageManager
            .toolPkgSubpackageByPackageNameInternal()
            .get(&normalizedPackageName)
            .cloned();
        let Some(subpackageRuntime) = subpackageRuntime else {
            return false;
        };
        let mut enabledPackageNames =
            std::collections::BTreeSet::from_iter(self.packageManager.getEnabledPackageNames());
        let mut subpackageStates = self.packageManager.getToolPkgSubpackageStatesInternal();
        let containerEnabled =
            enabledPackageNames.contains(&subpackageRuntime.containerPackageName);
        subpackageStates.insert(normalizedPackageName.clone(), enabled);
        if containerEnabled && enabled {
            enabledPackageNames.insert(normalizedPackageName.clone());
        } else {
            enabledPackageNames.remove(&normalizedPackageName);
        }
        let names = enabledPackageNames.into_iter().collect::<Vec<_>>();
        if self.packageManager.saveEnabledPackageNames(&names).is_err() {
            return false;
        }
        if self
            .packageManager
            .saveToolPkgSubpackageStates(&subpackageStates)
            .is_err()
        {
            return false;
        }
        let stateSaved = self
            .packageManager
            .getToolPkgSubpackageStatesInternal()
            .get(&normalizedPackageName)
            .copied()
            == Some(enabled);
        let importedMatches = if containerEnabled {
            self.packageManager
                .getEnabledPackageNames()
                .contains(&normalizedPackageName)
                == enabled
        } else {
            !self
                .packageManager
                .getEnabledPackageNames()
                .contains(&normalizedPackageName)
        };
        stateSaved && importedMatches
    }

    /// Resolves a subpackage id to a concrete package name.
    #[allow(non_snake_case)]
    pub fn findPreferredPackageNameForSubpackageId(
        &self,
        subpackageId: &str,
        preferEnabled: bool,
    ) -> Option<String> {
        self.packageManager.ensureInitialized();
        if subpackageId.trim().is_empty() {
            return None;
        }
        if let Some(directRuntime) = self
            .packageManager
            .resolveToolPkgSubpackageRuntimeInternal(subpackageId)
        {
            if preferEnabled
                && self
                    .packageManager
                    .isPackageEnabled(&directRuntime.packageName)
            {
                return Some(directRuntime.packageName);
            }
            return Some(directRuntime.packageName);
        }
        let candidates = self
            .packageManager
            .toolPkgSubpackageByPackageNameInternal()
            .values()
            .filter(|subpackage| subpackage.subpackageId.eq_ignore_ascii_case(subpackageId))
            .cloned()
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        if preferEnabled {
            if let Some(enabledCandidate) = candidates.iter().find(|subpackage| {
                self.packageManager
                    .isPackageEnabled(&subpackage.packageName)
            }) {
                return Some(enabledCandidate.packageName.clone());
            }
        }
        candidates
            .first()
            .map(|subpackage| subpackage.packageName.clone())
    }

    /// Copies a ToolPkg resource selected by subpackage id to a host file.
    #[allow(non_snake_case)]
    pub fn copyToolPkgResourceToFileBySubpackageId(
        &self,
        subpackageId: &str,
        resourceKey: &str,
        destinationFile: &Path,
        preferEnabledContainer: bool,
    ) -> bool {
        self.packageManager.ensureInitialized();
        if subpackageId.trim().is_empty() || resourceKey.trim().is_empty() {
            return false;
        }

        let directSubpackage = self
            .packageManager
            .resolveToolPkgSubpackageRuntimeInternal(subpackageId);
        let subpackages = if let Some(directSubpackage) = directSubpackage {
            vec![directSubpackage]
        } else {
            self.packageManager
                .toolPkgSubpackageByPackageNameInternal()
                .values()
                .filter(|subpackage| subpackage.subpackageId.eq_ignore_ascii_case(subpackageId))
                .cloned()
                .collect::<Vec<_>>()
        };

        if subpackages.is_empty() {
            return false;
        }

        let candidateContainers = if preferEnabledContainer {
            let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
            let enabledContainers = distinctContainerNames(
                subpackages
                    .iter()
                    .filter(|subpackage| enabledSet.contains(&subpackage.containerPackageName)),
            );
            if !enabledContainers.is_empty() {
                enabledContainers
            } else {
                distinctContainerNames(subpackages.iter())
            }
        } else {
            distinctContainerNames(subpackages.iter())
        };

        for containerName in candidateContainers {
            if self.copyToolPkgResourceToFile(&containerName, resourceKey, destinationFile) {
                return true;
            }
        }

        false
    }

    /// Copies a resource from one enabled ToolPkg container to a host file.
    #[allow(non_snake_case)]
    pub fn copyToolPkgResourceToFile(
        &self,
        containerPackageName: &str,
        resourceKey: &str,
        destinationFile: &Path,
    ) -> bool {
        self.packageManager.ensureInitialized();
        let normalizedContainerPackageName = self
            .packageManager
            .normalizePackageName(containerPackageName);
        let toolPkgContainers = self.packageManager.toolPkgContainersInternal();
        let Some(runtime) = toolPkgContainers.get(&normalizedContainerPackageName) else {
            return false;
        };
        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        if !enabledSet.contains(&runtime.packageName) {
            return false;
        }
        let Some(resource) = runtime
            .resources
            .iter()
            .find(|resource| resource.key.eq_ignore_ascii_case(resourceKey))
        else {
            return false;
        };

        self.packageManager
            .exportToolPkgResource(runtime, resource, destinationFile)
    }

    /// Returns the output file name declared by a ToolPkg resource.
    #[allow(non_snake_case)]
    pub fn getToolPkgResourceOutputFileName(
        &self,
        packageNameOrSubpackageId: &str,
        resourceKey: &str,
        preferEnabledContainer: bool,
    ) -> Option<String> {
        self.packageManager.ensureInitialized();
        let target = packageNameOrSubpackageId.trim();
        let key = resourceKey.trim();
        if target.is_empty() || key.is_empty() {
            return None;
        }

        let toolPkgContainers = self.packageManager.toolPkgContainersInternal();
        let resolveFromContainer = |containerName: &str| -> Option<String> {
            let normalizedContainerName = self.packageManager.normalizePackageName(containerName);
            let runtime = toolPkgContainers.get(&normalizedContainerName)?;
            let resource = runtime
                .resources
                .iter()
                .find(|resource| resource.key.eq_ignore_ascii_case(key))?;
            let baseName = resource
                .path
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or_default()
                .trim();
            if baseName.is_empty() {
                return None;
            }
            if ToolPkgArchiveParser::isDirectoryResourceMime(Some(&resource.mime)) {
                if baseName.to_ascii_lowercase().ends_with(".zip") {
                    Some(baseName.to_string())
                } else {
                    Some(format!("{baseName}.zip"))
                }
            } else {
                Some(baseName.to_string())
            }
        };

        if let Some(outputFileName) = resolveFromContainer(target) {
            return Some(outputFileName);
        }

        let directSubpackage = self
            .packageManager
            .resolveToolPkgSubpackageRuntimeInternal(target);
        if let Some(directSubpackage) = directSubpackage {
            if let Some(outputFileName) =
                resolveFromContainer(&directSubpackage.containerPackageName)
            {
                return Some(outputFileName);
            }
        }

        let subpackages = self
            .packageManager
            .toolPkgSubpackageByPackageNameInternal()
            .values()
            .filter(|subpackage| subpackage.subpackageId.eq_ignore_ascii_case(target))
            .cloned()
            .collect::<Vec<_>>();
        if subpackages.is_empty() {
            return None;
        }

        let candidateContainers = if preferEnabledContainer {
            let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
            let enabledContainers = distinctContainerNames(
                subpackages
                    .iter()
                    .filter(|subpackage| enabledSet.contains(&subpackage.containerPackageName)),
            );
            if !enabledContainers.is_empty() {
                enabledContainers
            } else {
                distinctContainerNames(subpackages.iter())
            }
        } else {
            distinctContainerNames(subpackages.iter())
        };

        for containerName in candidateContainers {
            if let Some(outputFileName) = resolveFromContainer(&containerName) {
                return Some(outputFileName);
            }
        }

        None
    }

    /// Reads a Compose DSL script selected by ToolPkg subpackage id.
    #[allow(non_snake_case)]
    pub fn getToolPkgComposeDslScriptBySubpackageId(
        &self,
        subpackageId: &str,
        uiModuleId: Option<&str>,
        preferEnabledContainer: bool,
    ) -> Option<String> {
        self.packageManager.ensureInitialized();
        if subpackageId.trim().is_empty() {
            return None;
        }

        let directSubpackage = self
            .packageManager
            .resolveToolPkgSubpackageRuntimeInternal(subpackageId);
        let subpackages = if let Some(directSubpackage) = directSubpackage {
            vec![directSubpackage]
        } else {
            self.packageManager
                .toolPkgSubpackageByPackageNameInternal()
                .values()
                .filter(|subpackage| subpackage.subpackageId.eq_ignore_ascii_case(subpackageId))
                .cloned()
                .collect::<Vec<_>>()
        };

        if subpackages.is_empty() {
            return None;
        }

        let candidateContainers = if preferEnabledContainer {
            let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
            distinctContainerNames(
                subpackages
                    .iter()
                    .filter(|subpackage| enabledSet.contains(&subpackage.containerPackageName)),
            )
        } else {
            distinctContainerNames(subpackages.iter())
        };

        for containerName in candidateContainers {
            let script = self.getToolPkgComposeDslScript(&containerName, uiModuleId);
            if let Some(script) = script.filter(|script| !script.trim().is_empty()) {
                return Some(script);
            }
        }

        None
    }

    /// Reads the Compose DSL script for one enabled ToolPkg UI module.
    #[allow(non_snake_case)]
    pub fn getToolPkgComposeDslScript(
        &self,
        containerPackageName: &str,
        uiModuleId: Option<&str>,
    ) -> Option<String> {
        self.packageManager.ensureInitialized();
        let normalizedContainerPackageName = self
            .packageManager
            .normalizePackageName(containerPackageName);
        let runtime = self
            .packageManager
            .toolPkgContainersInternal()
            .get(&normalizedContainerPackageName)
            .cloned()?;
        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        if !enabledSet.contains(&runtime.packageName) {
            return None;
        }

        let uiModule =
            if let Some(uiModuleId) = uiModuleId.map(str::trim).filter(|value| !value.is_empty()) {
                runtime.uiModules.iter().find(|module| {
                    module.id.eq_ignore_ascii_case(uiModuleId)
                        && module
                            .runtime
                            .eq_ignore_ascii_case(TOOLPKG_RUNTIME_COMPOSE_DSL)
                })
            } else {
                runtime.uiModules.iter().find(|module| {
                    module
                        .runtime
                        .eq_ignore_ascii_case(TOOLPKG_RUNTIME_COMPOSE_DSL)
                })
            }?;

        if uiModule.screen.trim().is_empty() {
            return None;
        }

        let bytes = self
            .packageManager
            .readToolPkgResourceBytes(&runtime, &uiModule.screen)?;
        String::from_utf8(bytes).ok()
    }

    /// Returns the Compose DSL screen path for one enabled ToolPkg UI module.
    #[allow(non_snake_case)]
    pub fn getToolPkgComposeDslScreenPath(
        &self,
        containerPackageName: &str,
        uiModuleId: Option<&str>,
    ) -> Option<String> {
        self.packageManager.ensureInitialized();
        let normalizedContainerPackageName = self
            .packageManager
            .normalizePackageName(containerPackageName);
        let runtime = self
            .packageManager
            .toolPkgContainersInternal()
            .get(&normalizedContainerPackageName)
            .cloned()?;
        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        if !enabledSet.contains(&runtime.packageName) {
            return None;
        }

        let uiModule =
            if let Some(uiModuleId) = uiModuleId.map(str::trim).filter(|value| !value.is_empty()) {
                runtime.uiModules.iter().find(|module| {
                    module.id.eq_ignore_ascii_case(uiModuleId)
                        && module
                            .runtime
                            .eq_ignore_ascii_case(TOOLPKG_RUNTIME_COMPOSE_DSL)
                })
            } else {
                runtime.uiModules.iter().find(|module| {
                    module
                        .runtime
                        .eq_ignore_ascii_case(TOOLPKG_RUNTIME_COMPOSE_DSL)
                })
            }?;

        let screen = uiModule.screen.trim().to_string();
        if screen.is_empty() {
            return None;
        }
        Some(screen)
    }

    /// Reads a UTF-8 text resource from an enabled ToolPkg container or subpackage.
    #[allow(non_snake_case)]
    pub fn readToolPkgTextResource(
        &self,
        packageNameOrSubpackageId: &str,
        resourcePath: &str,
        preferEnabledContainer: bool,
    ) -> Option<String> {
        self.packageManager.ensureInitialized();
        let target = packageNameOrSubpackageId.trim();
        let normalizedPath = resourcePath
            .trim()
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string();

        if target.is_empty() || normalizedPath.is_empty() {
            return None;
        }

        let toolPkgContainers = self.packageManager.toolPkgContainersInternal();
        if let Some(containerRuntime) = toolPkgContainers.get(target).cloned() {
            let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
            if !enabledSet.contains(&containerRuntime.packageName) {
                return None;
            }
            return self
                .packageManager
                .readToolPkgResourceBytes(&containerRuntime, &normalizedPath)
                .and_then(|bytes| String::from_utf8(bytes).ok());
        }

        let directSubpackageRuntime = self
            .packageManager
            .resolveToolPkgSubpackageRuntimeInternal(target);
        if let Some(directSubpackageRuntime) = directSubpackageRuntime {
            if let Some(directContainer) = toolPkgContainers
                .get(&directSubpackageRuntime.containerPackageName)
                .cloned()
            {
                let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
                if !enabledSet.contains(&directContainer.packageName) {
                    return None;
                }
                return self
                    .packageManager
                    .readToolPkgResourceBytes(&directContainer, &normalizedPath)
                    .and_then(|bytes| String::from_utf8(bytes).ok());
            }
        }

        let subpackages = self
            .packageManager
            .toolPkgSubpackageByPackageNameInternal()
            .values()
            .filter(|subpackage| subpackage.subpackageId.eq_ignore_ascii_case(target))
            .cloned()
            .collect::<Vec<_>>();
        if subpackages.is_empty() {
            return None;
        }

        let candidateContainers = if preferEnabledContainer {
            let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
            distinctContainerNames(
                subpackages
                    .iter()
                    .filter(|subpackage| enabledSet.contains(&subpackage.containerPackageName)),
            )
        } else {
            distinctContainerNames(subpackages.iter())
        };

        for containerName in candidateContainers {
            let Some(runtime) = toolPkgContainers.get(&containerName) else {
                continue;
            };
            let text = self
                .packageManager
                .readToolPkgResourceBytes(runtime, &normalizedPath)
                .and_then(|bytes| String::from_utf8(bytes).ok());
            if let Some(text) = text.filter(|text| !text.is_empty()) {
                return Some(text);
            }
        }

        None
    }
}

/// Returns distinct container package names while preserving subpackage iteration order.
#[allow(non_snake_case)]
fn distinctContainerNames<'a, I>(subpackages: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a ToolPkgSubpackageRuntime>,
{
    let mut seen = BTreeSet::new();
    subpackages
        .into_iter()
        .filter_map(|subpackage| {
            let container_name = subpackage.containerPackageName.clone();
            seen.insert(container_name.clone())
                .then_some(container_name)
        })
        .collect()
}

/// Returns a trimmed string or the supplied default when it is blank.
#[allow(non_snake_case)]
fn nonBlankOr(value: &str, defaultValue: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        defaultValue.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Copies one file or directory tree into a workspace template destination.
#[allow(non_snake_case)]
fn copyRecursively(source: &Path, destination: &Path) -> Result<(), String> {
    if source.is_dir() {
        fs::create_dir_all(destination).map_err(|error| error.to_string())?;
        for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            copyRecursively(&entry.path(), &destination.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if source.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::copy(source, destination)
            .map(|_| ())
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    Err(format!(
        "Workspace template source is invalid: {}",
        source.display()
    ))
}
