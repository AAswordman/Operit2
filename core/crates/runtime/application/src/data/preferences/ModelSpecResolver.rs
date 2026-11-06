#![allow(non_snake_case)]
//! Works out the pricing, context, capabilities and request shape a configured model runs with.
//!
//! What the user set wins, field by field. A field the user did not set takes what the model
//! catalog says of the model's id: the fixed list of a provider without a model-list
//! endpoint, otherwise the models.dev catalog downloaded on this device. A field the catalog
//! does not give takes what the provider's model list said when the model was added, then the
//! default. Installed local models are described by their local runtime alone, which takes an
//! on/off thinking switch for every model.
//!
//! Thinking controls are one field: the user's description of them replaces the catalog's
//! whole. A model's thinking controls come from models.dev even when a fixed list describes
//! the rest of it.

use std::sync::Arc;

use operit_model::ModelCatalog::ModelCatalog;
use operit_model::ModelConfigData::{
    ApiProviderType, AvailableProviderModel, AvailableProviderModelSource, ModelBuiltinTool,
    ModelCapabilities, ModelCapabilityOverrides, ModelCatalogEntry,
    ModelContextSpec, ModelListedSpec, ModelPricing, ModelProfile, ModelRequestOverrides,
    ModelRequestSpec, ProviderProfile,
};
use operit_model::ModelThinking::{ModelThinkingSpec, ThinkingSpecSource};
use operit_model::ModelsDevCatalog::{contextSpec, ModelsDevCatalog};

use crate::services::ModelCatalogService;

/// What the model catalog says of one model.
#[derive(Clone, Debug)]
pub(crate) struct CatalogSpec {
    pub(crate) catalogModelId: String,
    pub(crate) pricing: Option<ModelPricing>,
    pub(crate) context: Option<ModelContextSpec>,
    pub(crate) capabilities: ModelCapabilities,
    pub(crate) supportsStructuredTools: bool,
    pub(crate) builtinTools: Vec<ModelBuiltinTool>,
    /// The thinking controls a host serves the model with, `None` when the catalog names none.
    pub(crate) thinking: Option<ModelThinkingSpec>,
}

/// What a model takes before what the user set; `None` where nothing says.
#[derive(Clone, Debug)]
pub(crate) struct InheritedSpec {
    pub(crate) catalog: Option<CatalogSpec>,
    pub(crate) pricing: Option<ModelPricing>,
    pub(crate) context: Option<ModelContextSpec>,
    pub(crate) capabilities: Option<ModelCapabilities>,
    pub(crate) supportsStructuredTools: Option<bool>,
}

/// The thinking controls a configured model runs with, and where they come from.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedThinking {
    pub(crate) spec: ModelThinkingSpec,
    pub(crate) source: ThinkingSpecSource,
    /// What the catalog says, `None` when it does not describe the model's thinking.
    pub(crate) catalog: Option<ModelThinkingSpec>,
}

impl InheritedSpec {
    fn effectiveContext(&self) -> ModelContextSpec {
        self.context.clone().unwrap_or_default()
    }

    fn effectiveCapabilities(&self) -> ModelCapabilities {
        self.capabilities.clone().unwrap_or_default()
    }

    fn effectiveStructuredTools(&self) -> bool {
        self.supportsStructuredTools.unwrap_or(false)
    }
}

/// The settings a model runs with.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedSpec {
    pub(crate) pricing: Option<ModelPricing>,
    pub(crate) context: ModelContextSpec,
    pub(crate) capabilities: ModelCapabilities,
    pub(crate) request: ModelRequestSpec,
    pub(crate) thinking: ResolvedThinking,
}

/// Resolves the models of one provider against the model catalog.
pub(crate) struct ModelSpecResolver {
    recognizesModels: bool,
    fixedModels: Vec<ModelCatalogEntry>,
    catalog: Option<Arc<ModelsDevCatalog>>,
    catalogProviderIds: Vec<String>,
}

impl ModelSpecResolver {
    /// Prepares the catalog lists that describe what this provider serves.
    pub(crate) fn new(provider: &ProviderProfile) -> Self {
        let recognizesModels = provider.providerType != ApiProviderType::LOCAL_MODEL;
        let catalog = if recognizesModels {
            ModelCatalogService::currentCatalog()
        } else {
            None
        };
        let catalogProviderIds = catalog
            .as_ref()
            .map(|catalog| catalog.providerIdsFor(&provider.providerTypeId, &provider.endpoint))
            .unwrap_or_default();
        Self {
            recognizesModels,
            fixedModels: ModelCatalog::fixedModels(&provider.providerTypeId),
            catalog,
            catalogProviderIds,
        }
    }

    /// Returns what the catalog says of the model this id names, `None` when it does not know it.
    pub(crate) fn catalogSpec(&self, modelId: &str) -> Option<CatalogSpec> {
        if !self.recognizesModels {
            return None;
        }
        if let Some(entry) = self
            .fixedModels
            .iter()
            .find(|entry| entry.modelId.eq_ignore_ascii_case(modelId.trim()))
        {
            return Some(CatalogSpec {
                catalogModelId: entry.modelId.clone(),
                pricing: entry.pricing.clone(),
                context: entry.context.clone(),
                capabilities: entry.capabilities.clone().unwrap_or_default(),
                supportsStructuredTools: entry
                    .request
                    .as_ref()
                    .is_some_and(|request| request.supportsStructuredTools),
                builtinTools: entry.builtinTools.clone(),
                thinking: self.catalog.as_ref().and_then(|catalog| {
                    catalog
                        .facts(&self.catalogProviderIds, modelId)
                        .and_then(|facts| facts.thinking)
                }),
            });
        }
        let catalog = self.catalog.as_ref()?;
        let facts = catalog.facts(&self.catalogProviderIds, modelId)?;
        Some(CatalogSpec {
            pricing: catalog.price(&self.catalogProviderIds, modelId),
            context: facts.contextTokens.map(contextSpec),
            supportsStructuredTools: facts.capabilities.toolCall,
            capabilities: facts.capabilities,
            catalogModelId: facts.catalogModelId,
            builtinTools: Vec::new(),
            thinking: facts.thinking,
        })
    }

    /// Returns what a configured model takes before what the user set.
    pub(crate) fn inherited(&self, model: &ModelProfile) -> InheritedSpec {
        let catalog = self.catalogSpec(&model.id);
        Self::inheritedFrom(catalog, model.listed.as_ref())
    }

    /// Returns the thinking controls a configured model runs with.
    pub(crate) fn thinking(&self, model: &ModelProfile, inherited: &InheritedSpec) -> ResolvedThinking {
        let catalog = inherited
            .catalog
            .as_ref()
            .and_then(|spec| spec.thinking.clone());
        let (spec, source) = match (&model.thinkingOverride, &catalog) {
            (Some(declared), _) => (declared.clone(), ThinkingSpecSource::Declared),
            (None, _) if !self.recognizesModels => {
                (ModelThinkingSpec::switchable(), ThinkingSpecSource::LocalRuntime)
            }
            (None, Some(spec)) => (spec.clone(), ThinkingSpecSource::Catalog),
            (None, None) => (ModelThinkingSpec::default(), ThinkingSpecSource::Unknown),
        };
        ResolvedThinking {
            spec,
            source,
            catalog,
        }
    }

    /// Returns the settings a configured model runs with.
    pub(crate) fn resolve(&self, model: &ModelProfile) -> ResolvedSpec {
        let inherited = self.inherited(model);
        let thinking = self.thinking(model, &inherited);
        let capabilities = inherited.effectiveCapabilities();
        let overrides = &model.capabilitiesOverride;
        ResolvedSpec {
            pricing: model.pricingOverride.clone().or(inherited.pricing.clone()),
            context: model
                .contextOverride
                .clone()
                .unwrap_or_else(|| inherited.effectiveContext()),
            capabilities: ModelCapabilities {
                directImage: overrides.directImage.unwrap_or(capabilities.directImage),
                directAudio: overrides.directAudio.unwrap_or(capabilities.directAudio),
                directVideo: overrides.directVideo.unwrap_or(capabilities.directVideo),
                toolCall: overrides.toolCall.unwrap_or(capabilities.toolCall),
            },
            request: ModelRequestSpec {
                supportsStructuredTools: model
                    .requestOverride
                    .supportsStructuredTools
                    .unwrap_or(inherited.effectiveStructuredTools()),
                enableClaude1hPromptCache: model
                    .requestOverride
                    .enableClaude1hPromptCache
                    .unwrap_or(false),
            },
            thinking,
        }
    }

    /// Describes a model the provider offers as it would be once added.
    pub(crate) fn available(
        &self,
        modelId: String,
        source: AvailableProviderModelSource,
        listed: Option<ModelListedSpec>,
    ) -> AvailableProviderModel {
        let catalog = self.catalogSpec(&modelId);
        let builtinTools = catalog
            .as_ref()
            .map(|spec| spec.builtinTools.clone())
            .unwrap_or_default();
        let inherited = Self::inheritedFrom(catalog, listed.as_ref());
        AvailableProviderModel {
            catalogModelId: inherited
                .catalog
                .as_ref()
                .map(|spec| spec.catalogModelId.clone()),
            modelId,
            source,
            pricing: inherited.pricing,
            context: inherited.context,
            capabilities: inherited.capabilities,
            builtinTools,
            request: inherited
                .supportsStructuredTools
                .map(|supportsStructuredTools| ModelRequestSpec {
                    supportsStructuredTools,
                    ..ModelRequestSpec::default()
                }),
            listed,
        }
    }

    /// Describes a configured model with the settings it runs with.
    pub(crate) fn configured(
        &self,
        model: &ModelProfile,
        source: AvailableProviderModelSource,
    ) -> AvailableProviderModel {
        let resolved = self.resolve(model);
        AvailableProviderModel {
            modelId: model.id.clone(),
            source,
            catalogModelId: self.inherited(model).catalog.map(|spec| spec.catalogModelId),
            pricing: resolved.pricing,
            context: Some(resolved.context),
            capabilities: Some(resolved.capabilities),
            builtinTools: model.builtinToolsOverride.clone().unwrap_or_default(),
            request: Some(resolved.request),
            listed: model.listed.clone(),
        }
    }

    /// Returns the ids of the other models the catalog lists for this provider.
    pub(crate) fn catalogModelIds(&self) -> Vec<String> {
        if !self.fixedModels.is_empty() {
            return self
                .fixedModels
                .iter()
                .map(|entry| entry.modelId.clone())
                .collect();
        }
        match &self.catalog {
            Some(catalog) => catalog
                .providerModels(&self.catalogProviderIds)
                .into_iter()
                .map(|model| model.modelId)
                .collect(),
            None => Vec::new(),
        }
    }

    /// Returns what a provider's model list said of one of its models, `None` when it said nothing.
    pub(crate) fn listedSpec(model: &AvailableProviderModel) -> Option<ModelListedSpec> {
        let listed = ModelListedSpec {
            pricing: model.pricing.clone(),
            context: model.context.clone(),
            capabilities: model.capabilities.clone(),
            supportsStructuredTools: model
                .request
                .as_ref()
                .map(|request| request.supportsStructuredTools),
        };
        (listed != ModelListedSpec::default()).then_some(listed)
    }

    /// Keeps, of submitted capabilities, the values that differ from what the model inherits.
    pub(crate) fn capabilityOverrides(
        inherited: &InheritedSpec,
        capabilities: &ModelCapabilities,
    ) -> ModelCapabilityOverrides {
        let inheritedCapabilities = inherited.effectiveCapabilities();
        let differing = |value: bool, inheritedValue: bool| (value != inheritedValue).then_some(value);
        ModelCapabilityOverrides {
            directImage: differing(capabilities.directImage, inheritedCapabilities.directImage),
            directAudio: differing(capabilities.directAudio, inheritedCapabilities.directAudio),
            directVideo: differing(capabilities.directVideo, inheritedCapabilities.directVideo),
            toolCall: differing(capabilities.toolCall, inheritedCapabilities.toolCall),
        }
    }

    /// Keeps a submitted context window unless it is the inherited one. The settings page shows
    /// windows in whole thousands of tokens, so one it shows unchanged rounds to the inherited one.
    pub(crate) fn contextOverride(
        inherited: &InheritedSpec,
        context: ModelContextSpec,
    ) -> Option<ModelContextSpec> {
        let inheritedContext = inherited.effectiveContext();
        (context.maxContextLength.round() != inheritedContext.maxContextLength.round())
            .then_some(context)
    }

    /// Keeps, of submitted request settings, the values that differ from what the model inherits.
    pub(crate) fn requestOverrides(
        inherited: &InheritedSpec,
        request: &ModelRequestSpec,
    ) -> ModelRequestOverrides {
        ModelRequestOverrides {
            supportsStructuredTools: (request.supportsStructuredTools
                != inherited.effectiveStructuredTools())
            .then_some(request.supportsStructuredTools),
            enableClaude1hPromptCache: request.enableClaude1hPromptCache.then_some(true),
        }
    }

    /// Combines the catalog's word with the provider list's, field by field.
    fn inheritedFrom(catalog: Option<CatalogSpec>, listed: Option<&ModelListedSpec>) -> InheritedSpec {
        InheritedSpec {
            pricing: catalog
                .as_ref()
                .and_then(|spec| spec.pricing.clone())
                .or_else(|| listed.and_then(|listed| listed.pricing.clone())),
            context: catalog
                .as_ref()
                .and_then(|spec| spec.context.clone())
                .or_else(|| listed.and_then(|listed| listed.context.clone())),
            capabilities: catalog
                .as_ref()
                .map(|spec| spec.capabilities.clone())
                .or_else(|| listed.and_then(|listed| listed.capabilities.clone())),
            supportsStructuredTools: catalog
                .as_ref()
                .map(|spec| spec.supportsStructuredTools)
                .or_else(|| listed.and_then(|listed| listed.supportsStructuredTools)),
            catalog,
        }
    }
}
