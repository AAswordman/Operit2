//! Prepares package registrations independently of the canonical runtime manager.
use super::*;

/// Owns scan capabilities and parse caches, never a second runtime registry or execution owner.
pub(super) struct PackageRegistryScanner {
    pub(super) runtimeSupport: Arc<dyn ToolRuntimeSupport>,
    pub(super) registrationExecutor: Arc<dyn operit_plugin_sdk::javascript::JsPackageExecutor>,
    pub(super) resources: resource_reader::ToolPkgResourceReader,
    pub(super) dataStore: CoreNodeStateStore,
    pub(super) filesystem: Arc<VisualFileSystem>,
    pub(super) storePaths: RuntimeStorePaths,
    pub(super) externalPackageScanCache: Arc<BTreeMap<String, ExternalPackageScanCacheEntry>>,
    pub(super) builtInPackageScanCache: Arc<BTreeMap<String, ExternalPackageScanCacheEntry>>,
    pub(super) runtimes: ToolPkgManager,
}

/// Contains only declarations and ownership records ready to publish to the existing manager.
pub(super) struct PreparedPackageRegistry {
    pub(super) snapshot: PackageScanSnapshot,
    pub(super) scopes: BTreeMap<String, ExtensionRecord>,
    pub(super) externalCache: Arc<BTreeMap<String, ExternalPackageScanCacheEntry>>,
    pub(super) builtinCache: Arc<BTreeMap<String, ExternalPackageScanCacheEntry>>,
    pub(super) runtimes: operit_plugin_sdk::toolpkg::ToolPkgManager::PreparedToolPkgRuntimeMaps,
}

impl PackageRegistryScanner {
    /// Runs filesystem reads and package-owned registration code outside the registry mutex.
    pub(super) async fn scan(mut self) -> Result<PreparedPackageRegistry, String> {
        let clock = operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost();
        let mut last = clock.monotonicTimeMillis().expect("Registry scanning requires the Host clock");
        // Keep only scan boundaries; detailed investigation logging is not part of each package call.
        let mut phase = |stage: &str| {
            let now = clock.monotonicTimeMillis().expect("Registry scanning requires the Host clock");
            AppLogger::trace("PackageLoadTiming", &format!("source=registry stage={stage} elapsedMs={}", now.saturating_sub(last)));
            last = clock.monotonicTimeMillis().expect("Registry scanning requires the Host clock");
        };
        let activation = PackageScanActivation::read()?;
        phase("activation_read");
        let assets = self.scanBuiltInPackageAssets(&activation).await;
        phase("builtin_scan");
        let snapshot = self.scanExternalPackages(assets, &activation).await?;
        phase("external_scan");
        let (mut snapshot, scopes) = self.prepareOwnership(snapshot, &activation);
        phase("ownership_prepare");
        let runtimes = self.runtimes.prepareRuntimeMaps(
            std::mem::take(&mut snapshot.toolPkgContainers),
            std::mem::take(&mut snapshot.toolPkgSubpackages),
        );
        phase("revision_prepare");
        Ok(PreparedPackageRegistry {
            snapshot, scopes, externalCache: self.externalPackageScanCache,
            builtinCache: self.builtInPackageScanCache, runtimes,
        })
    }
    /// Prepares package declarations without holding the live registry owner.
    async fn scanBuiltInPackageAssets(&mut self, activation: &PackageScanActivation) -> PackageScanSnapshot {
        let mut results = Vec::new();
        let mut nextCache = BTreeMap::new();
        for asset in self.runtimeSupport.builtinPluginAssets() {
            let signature = buildBundledPluginAssetSignature(asset);
            let mut result = match self.builtInPackageScanCache.get(asset.name)
                .filter(|entry| entry.signature == signature && activation.matches(&entry.result)) {
                Some(entry) => entry.result.clone(),
                None => self.parseCandidate(
                    asset.name, Ok(asset.bytes), ToolPkgSourceType::ASSET,
                    Ok(ToolPkgConfigScope::Device), Some(activation),
                ).await,
            };
            let packageName = RuntimePackageManager::packageNameFromScanResult(&result);
            let selected = match packageName.as_deref() {
                Some(name) => activation.acceptsBuiltin(name),
                None => Ok(true),
            };
            match selected {
                Ok(false) => continue,
                Ok(true) => {}
                Err(error) => {
                    result.toolPackage = None;
                    result.toolPkgLoadResult = None;
                    result.issues.push(newToolPkgLoadIssue(
                        asset.name.to_string(), packageName, packageProgressDisplayName(asset.name),
                        "scope_failed", error, "package",
                    ));
                }
            }
            nextCache.insert(asset.name.to_string(), ExternalPackageScanCacheEntry {
                signature, result: result.clone(),
            });
            self.reportEnabledPackageScanProgress(&result, activation);
            results.push(result);
        }
        self.builtInPackageScanCache = Arc::new(nextCache);
        self.mergePackageScanCandidateResults(results, None)
    }

    /// Prepares package declarations without holding the live registry owner.
    async fn scanExternalPackages(&mut self, baseSnapshot: PackageScanSnapshot, activation: &PackageScanActivation) -> Result<PackageScanSnapshot, String> {
        let files = activation.selection.records.iter().filter(|record| !record.sourceName.is_empty())
            .map(|record| Ok((record.scope.as_str(), self.storePaths.runtime_storage_path(&format!("{}/{}",
                ExtensionStore::root("package", &record.scope)?, record.sourceName)))))
            .collect::<Result<Vec<_>, String>>()?;

        let previousCache = self.externalPackageScanCache.clone();
        let mut nextCache = BTreeMap::new();
        let mut results = Vec::new();
        for (scope, file) in files {
            if !isExternalPackageCandidateFile(&file) {
                continue;
            }
            let cacheKey = file.to_string_lossy().replace('\\', "/");
            let signature = self.buildExternalPackageScanSignature(&file);
            let mut result = match previousCache.get(&cacheKey)
                .filter(|entry| entry.signature == signature && activation.matches(&entry.result)) {
                Some(entry) => entry.result.clone(),
                None => {
                    let bytes = self.filesystem.readFileBytes(&cacheKey);
                    // Installed archives passed authentication at import/update; startup reads identity only.
                    let sourceType = if file.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("toolpkg")) {
                        bytes.as_ref().map_err(Clone::clone).and_then(|bytes| {
                            let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|error| error.to_string())?;
                            Ok(if archive.file_names().any(|name| name.eq_ignore_ascii_case(ToolPkgProtection::MARKET_INSTALL_SEAL_ENTRY_NAME)) {
                                ToolPkgSourceType::MARKET
                            } else { ToolPkgSourceType::EXTERNAL })
                        })
                    } else { Ok(ToolPkgSourceType::EXTERNAL) };
                    let (bytes, sourceType) = match sourceType {
                        Ok(sourceType) => (bytes, sourceType),
                        Err(error) => (Err(error), ToolPkgSourceType::EXTERNAL),
                    };
                    self.parseCandidate(
                        &cacheKey, bytes.as_deref().map_err(Clone::clone), sourceType,
                        self.installedPackageConfigScope(&file), Some(activation),
                    ).await
                }
            };
            if let Some(name) = RuntimePackageManager::packageNameFromScanResult(&result) {
                if let Some(issue) = activation.shadowedIssue(&name, scope, &cacheKey) {
                    result = PackageScanCandidateResult { issues: vec![issue], ..Default::default() };
                }
            }
            self.reportEnabledPackageScanProgress(&result, activation);
            nextCache.insert(
                cacheKey,
                ExternalPackageScanCacheEntry {
                    signature,
                    result: result.clone(),
                },
            );
            results.push(result);
        }
        self.externalPackageScanCache = Arc::new(nextCache);
        Ok(self.mergePackageScanCandidateResults(results, Some(baseSnapshot)))
    }

    /// Parses all source kinds through one path; catalog previews never execute registration code.
    pub(super) async fn parseCandidate(
        &self,
        sourcePath: &str,
        bytes: Result<&[u8], String>,
        sourceType: ToolPkgSourceType,
        scope: Result<ToolPkgConfigScope, String>,
        activation: Option<&PackageScanActivation>,
    ) -> PackageScanCandidateResult {
        let archive = Path::new(sourcePath).extension().is_some_and(|extension| extension.eq_ignore_ascii_case("toolpkg"));
        if let (true, Some(activation), Ok(bytes), Ok(scope)) = (archive, activation, &bytes, &scope) {
            let builtin = sourceType == ToolPkgSourceType::ASSET;
            let mut result = self.scanToolPkgArchive(bytes, sourceType, sourcePath, builtin, *scope, activation).await;
            result.phase = if builtin { "asset" } else { "external" }.into();
            return result;
        }
        self.parseCatalogCandidate(sourcePath, bytes, sourceType, scope, activation)
    }

    /// Reads metadata without executing package-owned registration code.
    pub(super) fn parseCatalogCandidate(
        &self,
        sourcePath: &str,
        bytes: Result<&[u8], String>,
        sourceType: ToolPkgSourceType,
        scope: Result<ToolPkgConfigScope, String>,
        activation: Option<&PackageScanActivation>,
    ) -> PackageScanCandidateResult {
        let builtin = sourceType == ToolPkgSourceType::ASSET;
        let extension = Path::new(sourcePath).extension().and_then(|value| value.to_str())
            .unwrap_or_default().to_ascii_lowercase();
        let archive = extension == "toolpkg";
        let mut result = PackageScanCandidateResult {
            phase: if activation.is_none() { "bundled_external" } else if builtin { "asset" } else { "external" }.into(),
            sourcePath: sourcePath.into(),
            ..Default::default()
        };
        let parsed = (|| -> Result<(), String> {
            let bytes = bytes?;
            if archive {
                scope?;
                result.toolPkgLoadResult = Some(ToolPkgArchiveParser::readToolPkgCatalog(
                    bytes, sourceType.clone(), sourcePath, builtin,
                )?);
            } else {
                let package = match extension.as_str() {
                    "js" | "ts" => {
                        let script = ToolPkgProtection::decodeUtf8(bytes)?;
                        match activation {
                            Some(activation) => scanStandalonePackageScript(&script, builtin, activation)?,
                            None => JsPackageLoader::parse_metadata(&JsPackageLoader::extract_metadata(&script), "")?,
                        }
                    }
                    "hjson" | "json" => JsPackageLoader::parse_metadata(
                        std::str::from_utf8(bytes).map_err(|error| error.to_string())?, "",
                    )?,
                    _ => return Ok(()),
                };
                let package = ToolPackage { is_built_in: builtin, ..package };
                result.registrationEnabled = activation.is_some_and(|activation| activation.isEnabled(&package));
                result.toolPackage = Some(package);
            }
            Ok(())
        })();
        if let Err(error) = parsed {
            result.issues.push(newToolPkgLoadIssue(
                sourcePath.into(), None, packageProgressDisplayName(sourcePath),
                if archive { "toolpkg_load" } else { "package_parse" }, error,
                if archive { packageLoadIssueKind(sourcePath, true, sourceType == ToolPkgSourceType::MARKET) }
                    else if matches!(extension.as_str(), "js" | "ts") { "javascript" } else { "metadata" },
            ));
        }
        result
    }

    /// Discovers manifests and registers only enabled installations.
    async fn scanToolPkgArchive(
        &self,
        bytes: &[u8],
        sourceType: ToolPkgSourceType,
        sourcePath: &str,
        isBuiltIn: bool,
        configScope: ToolPkgConfigScope,
        activation: &PackageScanActivation,
    ) -> PackageScanCandidateResult {
        let mut result = PackageScanCandidateResult {
            phase: "external".into(), sourcePath: sourcePath.into(), ..Default::default()
        };
        let mut packageName = None;
        let mut displayName = packageProgressDisplayName(sourcePath);
        let scanned = async {
            let manifest = ToolPkgArchiveParser::readToolPkgManifest(bytes)?;
            packageName = Some(manifest.toolpkgId.clone());
            displayName = manifest.displayName.resolve(false);
            if !isBuiltIn {
                if let Some(issue) = activation.shadowedIssue(&manifest.toolpkgId, configScope.as_str(), sourcePath) {
                    result.issues.push(issue);
                    return Ok(None);
                }
            }
            let package = ToolPackage {
                name: manifest.toolpkgId, is_built_in: isBuiltIn,
                enabled_by_default: manifest.enabledByDefault, ..Default::default()
            };
            let requiredScope = manifest.installationScope;
            if !isBuiltIn && requiredScope.is_some_and(|required| required != configScope) {
                return Err("ToolPkg installation scope differs from its manifest".into());
            }
            result.registrationEnabled = activation.isEnabled(&package)
                && (!isBuiltIn || activation.acceptsBuiltin(&package.name)?);
            if !result.registrationEnabled {
                return ToolPkgArchiveParser::readToolPkgCatalog(
                    bytes, sourceType.clone(), sourcePath, isBuiltIn,
                ).map(Some);
            }
            beginPackageLoadProgress(sourcePath);
            let issues = RefCell::new(Vec::new());
            let catalog = ToolPkgArchiveParser::readToolPkgCatalog(bytes, sourceType.clone(), sourcePath, isBuiltIn)?;
            let resources = self.resources.registrationResources(&catalog.containerRuntime, bytes)?;
            let loaded = {
                ToolPkgLoader::loadToolPkgFromArchiveBytes(
                    bytes, sourceType.clone(), sourcePath, isBuiltIn,
                    configScope, self.registrationExecutor.as_ref(), resources, |packageName, error| {
                        issues.borrow_mut().push(newToolPkgLoadIssue(
                            sourcePath.into(), nonEmptyString(packageName), packageName.into(),
                            "toolpkg_registration", error.into(),
                            packageLoadIssueKind(sourcePath, true, sourceType == ToolPkgSourceType::MARKET),
                        ));
                    },
                ).await
            };
            result.issues.extend(issues.into_inner());
            loaded.map(Some)
        }.await;
        match scanned {
            Ok(loaded) => result.toolPkgLoadResult = loaded,
            Err(error) => result.issues.push(newToolPkgLoadIssue(
                sourcePath.into(), packageName, displayName,
                "toolpkg_load", error,
                packageLoadIssueKind(sourcePath, true, sourceType == ToolPkgSourceType::MARKET),
            )),
        }
        result
    }

    /// Prepares package declarations without holding the live registry owner.
    fn reportEnabledPackageScanProgress(
        &self,
        result: &PackageScanCandidateResult,
        activation: &PackageScanActivation,
    ) {
        let enabled = result.registrationEnabled || result.toolPackage.as_ref()
            .is_some_and(|package| activation.isEnabled(package));
        if enabled && isProgressTrackedPackageSource(&result.sourcePath) {
            beginPackageLoadProgress(&result.sourcePath);
            reportPackageScanProgress(result);
        }
    }

    /// Prepares package declarations without holding the live registry owner.
    fn mergePackageScanCandidateResults(
        &self,
        candidateResults: Vec<PackageScanCandidateResult>,
        baseSnapshot: Option<PackageScanSnapshot>,
    ) -> PackageScanSnapshot {
        let mut snapshot = baseSnapshot.unwrap_or_default();
        for result in candidateResults {
            appendUniqueToolPkgLoadIssues(&mut snapshot.toolPkgLoadIssues, &result.issues);
            if let Some(packageMetadata) = result.toolPackage {
                if (result.phase == "external"
                    && !RuntimePackageManager::prepareExternalStandalonePackageOverride(
                        &packageMetadata.name,
                        &mut snapshot.availablePackages,
                        &mut snapshot.toolPkgContainers,
                        &mut snapshot.toolPkgSubpackages,
                    )) || snapshot.availablePackages.contains_key(&packageMetadata.name)
                {
                    snapshot.toolPkgLoadIssues.push(newToolPkgLoadIssue(
                        result.sourcePath.clone(),
                        Some(packageMetadata.name.clone()),
                        packageMetadata.display_name.resolve(false),
                        "duplicate_package",
                        format!(
                            "A package with name '{}' is already registered",
                            packageMetadata.name
                        ),
                        "package",
                    ));
                } else {
                    snapshot.availablePackages.insert(packageMetadata.name.clone(), packageMetadata);
                }
            }
            if let Some(loadResult) = result.toolPkgLoadResult {
                if result.phase == "external" {
                    if let Err(error) = RuntimePackageManager::prepareExternalToolPkgOverride(
                        &loadResult,
                        &mut snapshot.availablePackages,
                        &mut snapshot.toolPkgContainers,
                        &mut snapshot.toolPkgSubpackages,
                    ) {
                        snapshot.toolPkgLoadIssues.push(newToolPkgLoadIssue(
                            result.sourcePath.clone(),
                            Some(loadResult.containerPackage.name.clone()),
                            loadResult.containerRuntime.displayName.resolve(false),
                            "duplicate_package",
                            error,
                            "toolpkg",
                        ));
                        continue;
                    }
                }
                let packageName = loadResult.containerPackage.name.clone();
                let displayName = loadResult.containerRuntime.displayName.resolve(false);
                if !RuntimePackageManager::registerToolPkgInto(
                    loadResult,
                    &mut snapshot.availablePackages,
                    &mut snapshot.toolPkgContainers,
                    &mut snapshot.toolPkgSubpackages,
                ) {
                    snapshot.toolPkgLoadIssues.push(newToolPkgLoadIssue(
                        result.sourcePath.clone(),
                        Some(packageName),
                        displayName,
                        "toolpkg_registration",
                        "ToolPkg package registration failed".to_string(),
                        "toolpkg",
                    ));
                }
            }
        }

        snapshot
    }

    /// Prepares package declarations without holding the live registry owner.
    fn buildExternalPackageScanSignature(&self, file: &Path) -> String {
        let metadata = self.filesystem.fileInfo(&vfsPathString(file)).ok();
        format!(
            "{}|{}|{}",
            file.to_string_lossy(),
            metadata.as_ref().map(|value| value.size).unwrap_or(0),
            metadata.map(|value| value.lastModified).unwrap_or_default()
        )
    }

    /// Prepares package declarations without holding the live registry owner.
    pub(super) fn installedPackageConfigScope(&self, path: &Path) -> Result<ToolPkgConfigScope, String> {
        for scope in [ToolPkgConfigScope::Device, ToolPkgConfigScope::Space] {
            let root = ExtensionStore::root("package", scope.as_str())?;
            let directory = self.storePaths.runtime_storage_path(&root);
            if path.parent() == Some(directory.as_path()) {
                return Ok(scope);
            }
        }
        Err(format!(
            "Package source is outside installed scope directories: {}",
            vfsPathString(path)
        ))
    }

    /// Registers an installation using the same settings schema as catalog discovery.
    pub(super) fn registerScope(
        &self, id: &str, source: &str, builtin: bool, members: Vec<String>,
        scope: ToolPkgConfigScope, requiredScope: Option<ToolPkgConfigScope>,
    ) -> Result<ExtensionRecord, String> {
        if requiredScope.is_some_and(|required| required != scope) {
            return Err(format!(
                "Package {id} cannot be installed in {} scope",
                scope.as_str()
            ));
        }
        let store = ExtensionStore::default();
        let mut settings = serde_json::json!({
            "builtin": builtin, "members": members, "enabledNames": [], "disabledNames": [],
            "subpackageStates": {}, "order": 0,
            "installationId": encodeMarketToolPkgInstallationId(&marketToolPkgInstallationId(&self.dataStore)?)
        });
        if let Some(required) = requiredScope {
            settings["installationScope"] =
                serde_json::to_value(required).map_err(|error| error.to_string())?;
        }
        store.registerInScope("package", id, source, settings, scope.as_str())?;
        store.record("package", id)
    }

    /// Validates and persists source ownership before publishing any live declarations.
    pub(super) fn prepareOwnership(&self, mut snapshot: PackageScanSnapshot, activation: &PackageScanActivation) -> (PackageScanSnapshot, BTreeMap<String, ExtensionRecord>) {
        let mut scopes = BTreeMap::new();
        let mut rejected = Vec::new();
        let mut records = activation.selection.records.iter().map(|record| (record.id.clone(), record.clone())).collect::<BTreeMap<_, _>>();
        for (name, package) in &snapshot.availablePackages {
            if snapshot.toolPkgSubpackages.contains_key(name) {
                continue;
            }
            let registration = (|| -> Result<(), String> {
                let container = snapshot.toolPkgContainers.get(name);
                let builtin = package.is_built_in;
                let sourcePath = if builtin {
                    None
                } else if let Some(container) = container {
                    Some(container.sourcePath.as_str())
                } else {
                    Some(self.externalPackageScanCache.values()
                        .find(|entry| entry.result.toolPackage.as_ref()
                            .is_some_and(|candidate| candidate.name == *name))
                        .ok_or("Scanned package lost its concrete source")?.result.sourcePath.as_str())
                };
                let sourceName = sourcePath.map(|path| Path::new(path).file_name()
                    .ok_or("Package source filename is missing")
                    .map(|name| name.to_string_lossy().into_owned()))
                    .transpose()?.unwrap_or_default();
                let mut members = vec![name.clone()];
                if let Some(container) = container {
                    members.extend(container.subpackages.iter().map(|child| child.packageName.clone()));
                }
                let store = ExtensionStore::default();
                let requiredScope = snapshot
                    .toolPkgContainers
                    .get(name)
                    .map(RuntimePackageManager::requiredInstallationScope)
                    .transpose()?
                    .flatten();
                let mut record = match records.remove(name) {
                    Some(record) => record,
                    None => {
                        let scope = match container {
                            Some(container) if !builtin => self.installedPackageConfigScope(Path::new(&container.sourcePath))?,
                            _ => requiredScope.unwrap_or(ToolPkgConfigScope::Device),
                        };
                        self.registerScope(name, &sourceName, builtin, members.clone(), scope, requiredScope)?
                    }
                };
                if requiredScope.is_some_and(|required| required.as_str() != record.scope) {
                    return Err(format!(
                        "Package {name} is installed outside its required scope"
                    ));
                }
                if record.sourceName != sourceName
                    || record.settings["builtin"].as_bool() != Some(builtin)
                {
                    return Err(format!("Package source does not match its owner: {name}"));
                }
                if let Some(sourcePath) = sourcePath {
                    let actual = self.storePaths.runtime_storage_key(Path::new(sourcePath))?;
                    let expected = format!(
                        "{}/{}",
                        ExtensionStore::root("package", &record.scope)?,
                        record.sourceName
                    );
                    if actual != expected {
                        return Err(format!("Package location conflicts with its owner: {name}"));
                    }
                }
                let declaredEnvironment = members.iter()
                    .filter_map(|member| snapshot.availablePackages.get(member))
                    .flat_map(|package| package.env.iter().map(|variable| variable.name.clone()))
                    .collect::<Vec<_>>();
                store.initializePackageEnvironmentForScope(&record.id, &record.scope, &declaredEnvironment)?;
                let mut settings = record.settings.clone();
                settings["members"] =
                    serde_json::to_value(members).map_err(|error| error.to_string())?;
                let fields = settings
                    .as_object_mut()
                    .ok_or("Package settings must be an object")?;
                match requiredScope {
                    Some(scope) => {
                        fields.insert(
                            "installationScope".into(),
                            serde_json::to_value(scope).map_err(|error| error.to_string())?,
                        );
                    }
                    None => {
                        fields.remove("installationScope");
                    }
                }
                if record.settings != settings {
                    store.setSettings("package", name, settings.clone())?;
                    record.settings = settings;
                }
                scopes.insert(name.clone(), record);
                Ok(())
            })();
            if let Err(error) = registration {
                let ownsToolPkg = snapshot.toolPkgContainers.contains_key(name);
                let market = snapshot
                    .toolPkgContainers
                    .get(name)
                    .is_some_and(|container| container.sourceType == ToolPkgSourceType::MARKET);
                snapshot.toolPkgLoadIssues.push(newToolPkgLoadIssue(
                    name.clone(),
                    Some(name.clone()),
                    name.clone(),
                    "scope_failed",
                    error,
                    packageLoadIssueKind(name, ownsToolPkg, market),
                ));
                rejected.push(name.clone());
            }
        }
        for name in rejected {
            if snapshot.toolPkgContainers.contains_key(&name) {
                RuntimePackageManager::removeToolPkgContainerFromTargets(
                    &name,
                    &mut snapshot.availablePackages,
                    &mut snapshot.toolPkgContainers,
                    &mut snapshot.toolPkgSubpackages,
                );
            } else {
                snapshot.availablePackages.remove(&name);
            }
        }
        (snapshot, scopes)
    }
}
