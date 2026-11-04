//! The models.dev catalog, indexed for recognizing the models a provider serves.
//!
//! models.dev publishes each provider's models (`api.json`) and each lab's models
//! (`models.json`). A model's context window, inputs and tool calling are taken as most
//! of the entries naming it say, keyed by its id without a vendor prefix, so a model one
//! provider lists is recognized under every provider and relay that serves it. Prices stay
//! with the provider that charges them. Reasoning controls are per host, so a model's are
//! taken from its provider's entry when there is one, else as most hosts serving it say.
//!
//! An id is looked up in this order, the first that names a known model winning: the
//! provider's own entry; the id as given; the id without a Bedrock region, a `(level)`
//! suffix or a `:tag`; the id with a relay's window mark (`[1m]`), `claude-` prefix or
//! channel word (`nyro-`) taken off; the id spelt as models.dev spells it (version dots
//! and dashes, case, gaps and snapshot dates aside); the id without a channel tag in
//! brackets. Variants stay apart: `-pro-max` is not `-pro`, `-mini` is not the model.

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

use crate::BillingMode::BillingMode;
use crate::ModelConfigData::{ModelCapabilities, ModelContextSpec, ModelPricing, PricingCurrency};
use crate::ModelThinking::ModelThinkingSpec;

/// Smallest context window taken at its word: smaller listings cannot hold a system prompt.
const LEAST_WINDOW: u64 = 2048;

/// Words a relay puts before a model's id so a Claude client accepts it.
const RELAY_PREFIXES: [&str; 1] = ["claude-"];

/// Brackets a relay or a user puts a channel tag in, before or after a model's id.
const TAG_PAIRS: [(&str, &str); 5] = [
    ("(", ")"),
    ("[", "]"),
    ("（", "）"),
    ("【", "】"),
    ("「", "」"),
];

/// Region prefixes of Bedrock inference profile ids.
const BEDROCK_REGIONS: [&str; 9] = [
    "global.", "us.", "us-gov.", "eu.", "apac.", "jp.", "au.", "ca.", "in.",
];

/// API resources an endpoint may name after the provider's API base.
const API_RESOURCES: [&str; 4] = ["/chat/completions", "/responses", "/messages", "/models"];

/// What the catalog says of one model.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelsDevFacts {
    /// Id the model was recognized as, without a vendor prefix.
    pub catalogModelId: String,
    /// Context window in tokens, when the catalog gives one.
    pub contextTokens: Option<u64>,
    pub capabilities: ModelCapabilities,
    /// The thinking controls a host serves the model with, when a host's entry gives them.
    pub thinking: Option<ModelThinkingSpec>,
}

/// One model a models.dev provider lists, with that provider's facts and price.
#[derive(Clone, Debug)]
pub struct ModelsDevProviderModel {
    /// Id the provider serves the model under.
    pub modelId: String,
    pub facts: ModelsDevFacts,
    pub pricing: Option<ModelPricing>,
}

/// Converts a token window into the kilo-token context spec models are configured with.
pub fn contextSpec(tokens: u64) -> ModelContextSpec {
    ModelContextSpec {
        maxContextLength: tokens as f32 / 1000.0,
    }
}

/// One chat model entry as read from the catalog.
struct Entry {
    window: Option<u64>,
    capabilities: ModelCapabilities,
    /// `None` for a lab's entry, which names no host's reasoning controls.
    thinking: Option<ModelThinkingSpec>,
    pricing: Option<ModelPricing>,
    canonical: Option<String>,
}

impl Entry {
    /// Reads a chat model entry; image, embedding, speech and other models are skipped.
    fn read(model: &Value) -> Option<Self> {
        let model = model.as_object()?;
        if let Some(kind) = model.get("type").and_then(Value::as_str) {
            if kind != "chat" {
                return None;
            }
        }
        let modalities = model.get("modalities").and_then(Value::as_object);
        let output = modalities.and_then(|modalities| strings(modalities.get("output")));
        if output.as_ref().is_some_and(|output| !output.iter().any(|item| item == "text")) {
            return None;
        }
        let input = modalities
            .and_then(|modalities| strings(modalities.get("input")))
            .unwrap_or_default();
        Some(Self {
            window: entryWindow(model.get("limit").and_then(Value::as_object)),
            capabilities: ModelCapabilities {
                directImage: input.iter().any(|item| item == "image"),
                directAudio: input.iter().any(|item| item == "audio"),
                directVideo: input.iter().any(|item| item == "video"),
                toolCall: model.get("tool_call").and_then(Value::as_bool) == Some(true),
            },
            thinking: entryThinking(model),
            pricing: entryPricing(model.get("cost").and_then(Value::as_object)),
            canonical: model
                .get("canonical_model_id")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }

    /// Describes the entry under the bare id it was recognized as.
    fn facts(&self, bare: &str) -> ModelsDevFacts {
        ModelsDevFacts {
            catalogModelId: bare.to_string(),
            contextTokens: self.window,
            capabilities: self.capabilities.clone(),
            thinking: self.thinking.clone(),
        }
    }
}

/// What the entries naming one model say of it, counted for a majority.
#[derive(Default)]
struct Votes {
    entries: u32,
    images: i32,
    audio: i32,
    video: i32,
    tools: i32,
    windows: HashMap<u64, u32>,
    thinking: HashMap<ModelThinkingSpec, u32>,
}

impl Votes {
    /// Counts one entry's word on the model.
    fn add(&mut self, entry: &Entry) {
        self.entries += 1;
        let vote = |value: bool| if value { 1 } else { -1 };
        self.images += vote(entry.capabilities.directImage);
        self.audio += vote(entry.capabilities.directAudio);
        self.video += vote(entry.capabilities.directVideo);
        self.tools += vote(entry.capabilities.toolCall);
        if let Some(window) = entry.window {
            *self.windows.entry(window).or_default() += 1;
        }
        if let Some(thinking) = &entry.thinking {
            *self.thinking.entry(thinking.clone()).or_default() += 1;
        }
    }

    /// Describes the model as most of its entries do; a tie in sizes goes to the smaller.
    fn decide(&self, bare: &str) -> ModelsDevFacts {
        let window = self
            .windows
            .iter()
            .max_by(|left, right| left.1.cmp(right.1).then_with(|| right.0.cmp(left.0)))
            .map(|(window, _)| *window);
        // A tie goes to the controls that read first, so the pick does not hang on hash order.
        let thinking = self
            .thinking
            .iter()
            .map(|(spec, count)| (*count, thinkingKey(spec), spec))
            .max_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(&left.1)))
            .map(|(_, _, spec)| spec.clone());
        ModelsDevFacts {
            catalogModelId: bare.to_string(),
            contextTokens: window,
            capabilities: ModelCapabilities {
                directImage: self.images > 0,
                directAudio: self.audio > 0,
                directVideo: self.video > 0,
                toolCall: self.tools > 0,
            },
            thinking,
        }
    }
}

/// One models.dev provider's models and the ids they are found by.
#[derive(Default)]
struct CatalogProvider {
    models: Vec<ModelsDevProviderModel>,
    byKey: HashMap<String, usize>,
    byBare: HashMap<String, usize>,
    byDashed: HashMap<String, usize>,
    byCanonical: HashMap<String, usize>,
}

impl CatalogProvider {
    /// Adds one entry; of entries found by the same id, the first in id order is kept.
    fn push(&mut self, providerId: &str, key: &str, bare: &str, entry: &Entry) {
        let index = self.models.len();
        self.models.push(ModelsDevProviderModel {
            modelId: key.to_string(),
            facts: entry.facts(bare),
            pricing: entry.pricing.clone(),
        });
        self.byKey.entry(key.to_lowercase()).or_insert(index);
        self.byBare.entry(bare.to_string()).or_insert(index);
        self.byDashed.entry(dashed(bare)).or_insert(index);
        // A vendor's own entry also answers for the id models.dev says it is, which
        // relays serve it under: DeepSeek's deepseek-flash for deepseek-v4.1-flash.
        if let Some((maker, model)) = entry.canonical.as_deref().and_then(|id| id.split_once('/')) {
            if ownApi(providerId, maker) {
                let canonical = bareId(model);
                self.byCanonical.entry(dashed(&canonical)).or_insert(index);
                self.byCanonical.entry(canonical).or_insert(index);
            }
        }
    }

    /// Finds the provider's entry for a model id.
    fn find(&self, modelId: &str) -> Option<&ModelsDevProviderModel> {
        let lowered = modelId.trim().to_lowercase();
        if let Some(index) = self.byKey.get(&lowered) {
            return Some(&self.models[*index]);
        }
        let bare = bareId(&lowered);
        let bare = unprofiled(&bare).unwrap_or(bare);
        for wanted in [bare.clone(), cutAt(&bare, '('), cutAt(&bare, ':')] {
            if let Some(index) = self.byBare.get(&wanted) {
                return Some(&self.models[*index]);
            }
        }
        let dashedBare = dashed(&bare);
        self.byDashed
            .get(&dashedBare)
            .or_else(|| self.byCanonical.get(&bare))
            .or_else(|| self.byCanonical.get(&dashedBare))
            .map(|index| &self.models[*index])
    }
}

/// The models.dev catalog indexed by provider and by model.
pub struct ModelsDevCatalog {
    providers: HashMap<String, CatalogProvider>,
    facts: HashMap<String, ModelsDevFacts>,
    /// The bare id models.dev spells a model by, by its `sameModel` spelling.
    normal: HashMap<String, String>,
    /// models.dev providers by the API base they serve at.
    endpoints: HashMap<String, Vec<String>>,
}

impl ModelsDevCatalog {
    /// Indexes models.dev's provider catalog (`api.json`) and lab catalog (`models.json`).
    pub fn parse(providerCatalog: &[u8], labCatalog: &[u8]) -> Result<Self, String> {
        let providerCatalog: Value = serde_json::from_slice(providerCatalog)
            .map_err(|error| format!("models.dev api.json is not valid JSON: {error}"))?;
        let providerCatalog = providerCatalog
            .as_object()
            .filter(|providers| !providers.is_empty())
            .ok_or_else(|| "models.dev api.json lists no providers".to_string())?;
        let labCatalog: Value = serde_json::from_slice(labCatalog)
            .map_err(|error| format!("models.dev models.json is not valid JSON: {error}"))?;
        let labCatalog = labCatalog
            .as_object()
            .ok_or_else(|| "models.dev models.json is not an object".to_string())?;

        let mut providers = HashMap::new();
        let mut endpoints: HashMap<String, Vec<String>> = HashMap::new();
        let mut votes: HashMap<String, Votes> = HashMap::new();
        let mut canonicalEntries = Vec::new();
        for (providerId, provider) in providerCatalog {
            let Some(provider) = provider.as_object() else {
                continue;
            };
            if let Some(api) = provider.get("api").and_then(Value::as_str) {
                let key = endpointKey(api);
                if !key.is_empty() {
                    endpoints.entry(key).or_default().push(providerId.clone());
                }
            }
            let mut entries = provider
                .get("models")
                .and_then(Value::as_object)
                .map(sortedEntries)
                .unwrap_or_default();
            entries.retain(|(key, _)| !key.trim().is_empty());
            let mut catalogProvider = CatalogProvider::default();
            for (key, model) in entries {
                let Some(entry) = Entry::read(model) else {
                    continue;
                };
                let bare = bareId(key);
                votes.entry(bare.clone()).or_default().add(&entry);
                catalogProvider.push(providerId, key, &bare, &entry);
                let canonical = entry
                    .canonical
                    .as_deref()
                    .filter(|id| id.contains('/'))
                    .map(bareId);
                if let Some(canonical) = canonical {
                    canonicalEntries.push((canonical, entry));
                }
            }
            providers.insert(providerId.clone(), catalogProvider);
        }
        // A model no provider lists by its own id, only as an entry's canonical id, takes
        // those entries' word: relays serve it under that id.
        let listed = votes.keys().cloned().collect::<HashSet<_>>();
        for (canonical, entry) in &canonicalEntries {
            if !listed.contains(canonical) {
                votes.entry(canonical.clone()).or_default().add(entry);
            }
        }
        // A model only its lab lists, as a gated one an account can still call, takes the
        // lab's word on it.
        for (key, model) in sortedEntries(labCatalog) {
            let bare = bareId(key);
            if bare.is_empty() || votes.contains_key(&bare) {
                continue;
            }
            if let Some(entry) = Entry::read(model) {
                votes.entry(bare.clone()).or_default().add(&entry);
            }
        }
        for ids in endpoints.values_mut() {
            ids.sort();
        }

        let mut normal: HashMap<String, (String, u32)> = HashMap::new();
        for (bare, vote) in &votes {
            let spelling = sameModel(bare);
            let better = match normal.get(&spelling) {
                Some((current, count)) => {
                    vote.entries > *count || (vote.entries == *count && bare < current)
                }
                None => true,
            };
            if better {
                normal.insert(spelling, (bare.clone(), vote.entries));
            }
        }
        let facts = votes
            .iter()
            .map(|(bare, vote)| (bare.clone(), vote.decide(bare)))
            .collect();
        Ok(Self {
            providers,
            facts,
            normal: normal
                .into_iter()
                .map(|(spelling, (bare, _))| (spelling, bare))
                .collect(),
            endpoints,
        })
    }

    /// Returns the models.dev providers whose lists describe what a provider of this type at
    /// this endpoint serves: those models.dev says serve at the endpoint's API base, or for an
    /// endpoint on the servers of a vendor whose models.dev entry names no API base, the
    /// vendor's own list. A relay or self-hosted server has none: what it serves is its own.
    pub fn providerIdsFor(&self, providerTypeId: &str, endpoint: &str) -> Vec<String> {
        let base = apiBase(endpoint);
        if !base.is_empty() {
            if let Some(ids) = self.endpoints.get(&base) {
                return ids.clone();
            }
            if let Some(ids) = withoutVersion(&base).and_then(|base| self.endpoints.get(&base)) {
                return ids.clone();
            }
        }
        vendorCatalogs(providerTypeId, endpoint)
            .iter()
            .map(|id| id.to_string())
            .collect()
    }

    /// Recognizes a model: by the provider's own entry first, then as most of the entries
    /// naming it describe it.
    pub fn facts(&self, providerIds: &[String], modelId: &str) -> Option<ModelsDevFacts> {
        for providerId in providerIds {
            if let Some(model) = self
                .providers
                .get(providerId)
                .and_then(|provider| provider.find(modelId))
            {
                return Some(model.facts.clone());
            }
        }
        self.lookup(&bareId(modelId))
    }

    /// Returns the list price the first of the providers listing a model gives it.
    pub fn price(&self, providerIds: &[String], modelId: &str) -> Option<ModelPricing> {
        for providerId in providerIds {
            if let Some(model) = self
                .providers
                .get(providerId)
                .and_then(|provider| provider.find(modelId))
            {
                if model.pricing.is_some() {
                    return model.pricing.clone();
                }
            }
        }
        let (renamed, _) = self.renamed(&bareId(modelId))?;
        self.price(providerIds, &renamed)
    }

    /// Returns the models the providers list, each id once.
    pub fn providerModels(&self, providerIds: &[String]) -> Vec<ModelsDevProviderModel> {
        let mut seen = HashSet::new();
        let mut models = Vec::new();
        for providerId in providerIds {
            let Some(provider) = self.providers.get(providerId) else {
                continue;
            };
            for model in &provider.models {
                if seen.insert(model.modelId.to_lowercase()) {
                    models.push(model.clone());
                }
            }
        }
        models
    }

    /// Looks a bare id up across every provider.
    fn lookup(&self, bare: &str) -> Option<ModelsDevFacts> {
        if let Some(facts) = self.found(bare) {
            return Some(facts.clone());
        }
        if let Some((renamed, window)) = self.renamed(bare) {
            if let Some(facts) = self.found(&renamed) {
                let mut facts = facts.clone();
                // A relay's window mark is the window it serves the model at.
                if window.is_some() {
                    facts.contextTokens = window;
                }
                return Some(facts);
            }
        }
        if let Some(spelling) = self.normal.get(&sameModel(bare)) {
            if spelling != bare {
                return self.found(spelling).cloned();
            }
        }
        let untagged = untagged(bare)?;
        self.lookup(&untagged)
    }

    /// Finds a bare id as given, without a Bedrock region, a `(level)` suffix or a `:tag`.
    fn found(&self, bare: &str) -> Option<&ModelsDevFacts> {
        if let Some(facts) = self.facts.get(bare) {
            return Some(facts);
        }
        if let Some(unprofiled) = unprofiled(bare) {
            return self.found(&unprofiled);
        }
        let mut bare = bare;
        if let Some(index) = bare.find('(').filter(|index| *index > 0 && bare.ends_with(')')) {
            if let Some(facts) = self.facts.get(&bare[..index]) {
                return Some(facts);
            }
            bare = &bare[..index];
        }
        let index = bare.find(':').filter(|index| *index > 0)?;
        self.facts.get(&bare[..index])
    }

    /// The known model a relay renamed to this bare id, and the window the rename names:
    /// a window mark (`[1m]`), a `claude-` prefix or a channel word taken off. An id the
    /// catalog knows is always its own model, and a prefix comes off only when what is left
    /// is a known model with a version in its id.
    fn renamed(&self, bare: &str) -> Option<(String, Option<u64>)> {
        if self.found(bare).is_some() {
            return None;
        }
        let (rest, window) = unmarked(bare);
        if rest != bare && self.found(&rest).is_some() {
            return Some((rest, window));
        }
        for prefix in RELAY_PREFIXES {
            if let Some(model) = rest.strip_prefix(prefix) {
                if hasDigit(model) && self.found(model).is_some() {
                    return Some((model.to_string(), window));
                }
            }
        }
        let model = channelStripped(&rest)?;
        (hasDigit(model) && self.found(model).is_some()).then(|| (model.to_string(), window))
    }
}

/// Returns the models.dev lists of a vendor whose models.dev entry names no API base, for an
/// endpoint on that vendor's own servers.
fn vendorCatalogs(providerTypeId: &str, endpoint: &str) -> &'static [&'static str] {
    let host = endpointHost(endpoint);
    match (providerTypeId.trim().to_ascii_uppercase().as_str(), host.as_str()) {
        ("OPENAI" | "OPENAI_RESPONSES", "api.openai.com") => &["openai"],
        ("ANTHROPIC", "api.anthropic.com") => &["anthropic"],
        ("GOOGLE", "generativelanguage.googleapis.com") => &["google"],
        ("MISTRAL", "api.mistral.ai" | "codestral.mistral.ai") => &["mistral"],
        ("XAI", "api.x.ai") => &["xai"],
        ("DEEPSEEK", "api.deepseek.com") => &["deepseek"],
        ("NOVITA", "api.novita.ai") => &["novita-ai"],
        ("MINIMAX", "api.minimax.io") => &["minimax"],
        ("MINIMAX", "api.minimaxi.com") => &["minimax-cn"],
        ("OPENCODE", "opencode.ai") if endpoint.to_ascii_lowercase().contains("/zen/go") => {
            &["opencode-go"]
        }
        ("OPENCODE", "opencode.ai") => &["opencode"],
        _ => &[],
    }
}

/// Returns the lowercase host an endpoint names, without its scheme, credentials or port.
fn endpointHost(endpoint: &str) -> String {
    let endpoint = endpoint.trim();
    let rest = endpoint
        .split_once("://")
        .map_or(endpoint, |(_, rest)| rest);
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .expect("split always produces one authority segment");
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let host = host.split_once(':').map_or(host, |(host, _)| host);
    host.to_ascii_lowercase()
}

/// Reads the reasoning controls a host's entry gives; a lab's entry gives none for a model
/// that reasons.
fn entryThinking(model: &Map<String, Value>) -> Option<ModelThinkingSpec> {
    match model.get("reasoning").and_then(Value::as_bool) {
        Some(false) => Some(ModelThinkingSpec::default()),
        Some(true) => model
            .get("reasoning_options")
            .map(ModelThinkingSpec::fromReasoningOptions),
        None => None,
    }
}

/// Orders reasoning controls for a tie between them.
fn thinkingKey(spec: &ModelThinkingSpec) -> String {
    serde_json::to_string(spec).expect("thinking controls encode")
}

/// Returns a JSON object's entries ordered by key.
fn sortedEntries(object: &Map<String, Value>) -> Vec<(&String, &Value)> {
    let mut entries = object.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(right.0));
    entries
}

/// Reads an array of strings, `None` when the value is not one.
fn strings(value: Option<&Value>) -> Option<Vec<String>> {
    value?.as_array().map(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_lowercase)
            .collect()
    })
}

/// Reads the tokens a prompt may hold: the input limit where given, never above the whole
/// context, else the context. A limit under the least window says nothing.
fn entryWindow(limit: Option<&Map<String, Value>>) -> Option<u64> {
    let limit = limit?;
    let read = |name: &str| {
        limit
            .get(name)
            .and_then(Value::as_u64)
            .filter(|tokens| *tokens >= LEAST_WINDOW)
    };
    match (read("input"), read("context")) {
        (Some(input), Some(context)) => Some(input.min(context)),
        (Some(input), None) => Some(input),
        (None, context) => context,
    }
}

/// Reads a list price in USD per million tokens; one without input and output prices is none.
fn entryPricing(cost: Option<&Map<String, Value>>) -> Option<ModelPricing> {
    let cost = cost?;
    let read = |name: &str| cost.get(name).and_then(Value::as_f64);
    Some(ModelPricing {
        billingMode: BillingMode::TOKEN,
        inputPricePerMillion: read("input")?,
        cachedInputPricePerMillion: read("cache_read"),
        cacheWritePricePerMillion: read("cache_write"),
        outputPricePerMillion: read("output")?,
        pricePerRequest: 0.0,
        currency: PricingCurrency::USD,
    })
}

/// Returns a model id lowercased and without its vendor's path prefix.
fn bareId(id: &str) -> String {
    let id = id.trim().to_lowercase();
    match id.rfind('/') {
        Some(index) => id[index + 1..].to_string(),
        None => id,
    }
}

/// Returns an id with its version dots written as dashes.
fn dashed(id: &str) -> String {
    id.replace('.', "-")
}

/// Returns the text before a separator, or all of it.
fn cutAt(text: &str, separator: char) -> String {
    match text.find(separator) {
        Some(index) if index > 0 => text[..index].to_string(),
        _ => text.to_string(),
    }
}

/// Returns whether an id holds a digit, as every versioned model id does.
fn hasDigit(text: &str) -> bool {
    text.chars().any(|character| character.is_ascii_digit())
}

/// Returns whether a vendor-scoped models.dev provider is the maker's own API: its id, or
/// its id and a region or edition, but not a membership plan's list.
fn ownApi(providerId: &str, maker: &str) -> bool {
    let providerId = providerId.to_lowercase();
    let maker = maker.to_lowercase();
    (providerId == maker || providerId.starts_with(&format!("{maker}-")))
        && !providerId.contains("-plan")
}

/// Returns a Bedrock inference profile's id without its region.
fn unprofiled(bare: &str) -> Option<String> {
    BEDROCK_REGIONS.iter().find_map(|region| {
        bare.strip_prefix(*region)
            .filter(|rest| rest.contains('.'))
            .map(str::to_string)
    })
}

/// Splits a window mark (`[1m]`, `[200k]`) off an id, with the window it names in tokens.
fn unmarked(bare: &str) -> (String, Option<u64>) {
    let Some(open) = bare.strip_suffix(']').and_then(|text| text.rfind('[')) else {
        return (bare.to_string(), None);
    };
    if open == 0 {
        return (bare.to_string(), None);
    }
    let mark = &bare[open + 1..bare.len() - 1];
    let (number, unit) = if let Some(number) = mark.strip_suffix('k') {
        (number, 1_000.0)
    } else if let Some(number) = mark.strip_suffix('m') {
        (number, 1_000_000.0)
    } else {
        return (bare.to_string(), None);
    };
    match number.parse::<f64>() {
        Ok(value) if value > 0.0 && number.chars().all(|c| c.is_ascii_digit() || c == '.') => {
            (bare[..open].to_string(), Some((value * unit) as u64))
        }
        _ => (bare.to_string(), None),
    }
}

/// Returns an id without a channel word a relay put before it (`nyro-gpt-5.5`): two to
/// twenty-four letters and a dash.
fn channelStripped(bare: &str) -> Option<&str> {
    let dash = bare.find('-')?;
    let word = &bare[..dash];
    ((2..=24).contains(&dash) && word.chars().all(|c| c.is_ascii_lowercase()))
        .then(|| &bare[dash + 1..])
}

/// Returns an id without a channel tag in brackets at its start or its end
/// (`deepseek-v4.1-flash (wb)`). A `[...]` written onto the id with no space before it
/// may be a variant and stays, as does a window mark.
fn untagged(bare: &str) -> Option<String> {
    let trimmed = bare.trim();
    if unmarked(trimmed).0 != trimmed {
        return None;
    }
    let mut text = trimmed.to_string();
    for (open, close) in TAG_PAIRS {
        if text.ends_with(close) {
            if let Some(index) = text.rfind(open) {
                if index > 0 && (open != "[" || text[..index].ends_with(' ')) {
                    text = text[..index].trim().to_string();
                    break;
                }
            }
        }
        if text.starts_with(open) {
            if let Some(index) = text.find(close) {
                if index > 0 && index + close.len() < text.len() {
                    text = text[index + close.len()..].trim().to_string();
                    break;
                }
            }
        }
    }
    let text = text.trim_matches(|c: char| matches!(c, ' ' | '-' | '_' | '·' | '.'));
    (text != bare && !text.is_empty() && text.chars().any(char::is_alphabetic))
        .then(|| text.to_string())
}

/// Spells a model id as vendors agree on it: lowercase, without a vendor prefix, a version's
/// dot (or Fireworks' `p`) as a dash, runs of spaces, `_` and `-` as one `-`, and without a
/// snapshot date (`-20260801`, `@20260801`, Ark's `-261024`). Variants after `:`, `-flash`,
/// `-thinking` and four-digit releases (`-2507`) stay.
pub fn sameModel(id: &str) -> String {
    let lowered = bareId(id);
    let mut collapsed = String::with_capacity(lowered.len());
    let mut gap = false;
    for character in lowered.chars() {
        if character.is_whitespace() || character == '_' || character == '-' {
            gap = true;
            continue;
        }
        if gap && !collapsed.is_empty() {
            collapsed.push('-');
        }
        gap = false;
        collapsed.push(character);
    }
    let mut characters = collapsed.chars().collect::<Vec<_>>();
    for index in 1..characters.len().saturating_sub(1) {
        if matches!(characters[index], '.' | 'p')
            && characters[index - 1].is_ascii_digit()
            && characters[index + 1].is_ascii_digit()
        {
            characters[index] = '-';
        }
    }
    let mut spelling = characters.into_iter().collect::<String>();
    if let Some(index) = spelling.rfind(|c: char| c == '-' || c == '@') {
        if index > 0 && snapshotDate(&spelling[index + 1..]) {
            spelling.truncate(index);
        }
    }
    spelling
}

/// Returns whether text is a snapshot date: YYYYMMDD from 2000 on, or Ark's YYMMDD.
fn snapshotDate(text: &str) -> bool {
    if text.is_empty() || !text.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let number = |range: std::ops::Range<usize>| text[range].parse::<u32>().unwrap_or(0);
    match text.len() {
        8 => text.starts_with("20"),
        6 => {
            (23..=39).contains(&number(0..2))
                && (1..=12).contains(&number(2..4))
                && (1..=31).contains(&number(4..6))
        }
        _ => false,
    }
}

/// Keys an API base: lowercase, without a trailing slash.
fn endpointKey(url: &str) -> String {
    url.trim().trim_end_matches('/').to_lowercase()
}

/// Returns the API base of a configured endpoint: without its query and the resource it names.
fn apiBase(endpoint: &str) -> String {
    let endpoint = endpoint.trim();
    let endpoint = match endpoint.find(|c: char| c == '?' || c == '#') {
        Some(index) => &endpoint[..index],
        None => endpoint,
    };
    let mut base = endpointKey(endpoint);
    for resource in API_RESOURCES {
        if let Some(stripped) = base.strip_suffix(resource) {
            base = stripped.to_string();
            break;
        }
    }
    base
}

/// Returns an API base without a trailing version segment (`/v1`), when it has one.
fn withoutVersion(base: &str) -> Option<String> {
    let (rest, segment) = base.rsplit_once('/')?;
    let version = segment.strip_prefix('v')?;
    (!version.is_empty() && version.chars().all(|c| c.is_ascii_digit()) && rest.contains("://"))
        .then(|| rest.to_string())
}

#[cfg(test)]
mod tests {
    use super::{sameModel, ModelsDevCatalog};

    const PROVIDERS: &str = r#"{
        "zhipuai": {
            "api": "https://open.bigmodel.cn/api/paas/v4",
            "models": {
                "glm-5": {
                    "id": "glm-5", "name": "GLM-5", "tool_call": true,
                    "reasoning": true, "reasoning_options": [{"type": "toggle"}],
                    "modalities": {"input": ["text"], "output": ["text"]},
                    "limit": {"context": 204800, "output": 131072},
                    "cost": {"input": 0.6, "output": 2.2, "cache_read": 0.11}
                }
            }
        },
        "openrouter": {
            "api": "https://openrouter.ai/api/v1",
            "models": {
                "z-ai/glm-5": {
                    "id": "z-ai/glm-5", "name": "Z.ai: GLM 5", "tool_call": true,
                    "reasoning": true,
                    "reasoning_options": [{"type": "effort", "values": ["none", "high"]}],
                    "modalities": {"input": ["text"], "output": ["text"]},
                    "limit": {"context": 204800, "output": 131072},
                    "cost": {"input": 0.65, "output": 2.4}
                },
                "openai/gpt-5": {
                    "id": "openai/gpt-5", "name": "GPT-5", "tool_call": true,
                    "modalities": {"input": ["text", "image"], "output": ["text"]},
                    "limit": {"context": 400000, "input": 272000, "output": 128000},
                    "cost": {"input": 1.25, "output": 10}
                },
                "moonshotai/kimi-k3.1-pro": {
                    "id": "moonshotai/kimi-k3.1-pro", "name": "Kimi K3.1 Pro", "tool_call": true,
                    "modalities": {"input": ["text"], "output": ["text"]},
                    "limit": {"context": 262144, "output": 65536}
                },
                "black-forest-labs/flux": {
                    "id": "black-forest-labs/flux", "name": "FLUX", "type": "image",
                    "modalities": {"input": ["text"], "output": ["image"]},
                    "limit": {"context": 0, "output": 0}
                }
            }
        },
        "deepseek": {
            "api": "https://api.deepseek.com",
            "models": {
                "deepseek-flash": {
                    "id": "deepseek-flash", "name": "DeepSeek Flash", "tool_call": true,
                    "reasoning": true,
                    "reasoning_options": [
                        {"type": "toggle"},
                        {"type": "effort", "values": ["low", "high", "max"]}
                    ],
                    "modalities": {"input": ["text"], "output": ["text"]},
                    "limit": {"context": 1048576, "output": 393216},
                    "cost": {"input": 0.15, "output": 0.6},
                    "canonical_model_id": "deepseek/deepseek-v4.1-flash"
                }
            }
        }
    }"#;

    const LABS: &str = r#"{
        "anthropic/claude-mythos-5.1": {
            "id": "anthropic/claude-mythos-5.1", "name": "Claude Mythos 5.1", "tool_call": true,
            "modalities": {"input": ["text", "image"], "output": ["text"]},
            "limit": {"context": 1000000, "output": 128000}
        }
    }"#;

    fn catalog() -> ModelsDevCatalog {
        ModelsDevCatalog::parse(PROVIDERS.as_bytes(), LABS.as_bytes()).expect("catalog parses")
    }

    #[test]
    fn recognizes_models_across_providers_by_bare_id() {
        let catalog = catalog();
        let other = catalog.facts(&[], "glm-5").expect("glm-5 from any provider");
        assert_eq!(other.catalogModelId, "glm-5");
        assert_eq!(other.contextTokens, Some(204_800));
        assert!(other.capabilities.toolCall);

        let gpt = catalog.facts(&[], "GPT-5").expect("gpt-5 known through openrouter");
        assert!(gpt.capabilities.directImage);
        assert_eq!(gpt.contextTokens, Some(272_000));
    }

    #[test]
    fn reads_relay_renames_tags_and_spellings() {
        let catalog = catalog();
        let marked = catalog.facts(&[], "glm-5[1m]").expect("window mark");
        assert_eq!(marked.catalogModelId, "glm-5");
        assert_eq!(marked.contextTokens, Some(1_000_000));
        assert!(catalog.facts(&[], "claude-glm-5").is_some());
        assert!(catalog.facts(&[], "nyro-gpt-5").is_some());
        assert!(catalog.facts(&[], "glm-5:free").is_some());
        assert!(catalog.facts(&[], "glm-5 (WB)").is_some());
        let spelt = catalog
            .facts(&[], "kimi-k3-1-pro-261024")
            .expect("Ark's spelling and snapshot date");
        assert_eq!(spelt.catalogModelId, "kimi-k3.1-pro");
        assert!(catalog.facts(&[], "glm-5-pro-max").is_none());
        assert!(catalog.facts(&[], "flux").is_none());
    }

    #[test]
    fn knows_canonical_and_lab_models() {
        let catalog = catalog();
        let canonical = catalog
            .facts(&[], "deepseek-v4.1-flash")
            .expect("canonical id relays serve");
        assert_eq!(canonical.contextTokens, Some(1_048_576));
        let lab = catalog.facts(&[], "claude-mythos-5.1").expect("lab-only model");
        assert!(lab.capabilities.directImage);
    }

    #[test]
    fn prices_stay_with_the_provider() {
        let catalog = catalog();
        let zhipu = catalog.providerIdsFor("ZHIPU", "https://open.bigmodel.cn/api/paas/v4/chat/completions");
        assert_eq!(zhipu, vec!["zhipuai".to_string()]);
        let price = catalog.price(&zhipu, "glm-5").expect("zhipu price");
        assert!((price.inputPricePerMillion - 0.6).abs() < 1e-9);
        assert!(catalog.price(&[], "glm-5").is_none());

        let deepseek = catalog.providerIdsFor("DEEPSEEK", "https://api.deepseek.com/v1/chat/completions");
        assert_eq!(deepseek, vec!["deepseek".to_string()]);
        assert!(catalog.price(&deepseek, "deepseek-v4.1-flash").is_some());
        assert_eq!(
            catalog.providerIdsFor("OPENAI", "https://api.openai.com/v1/chat/completions"),
            vec!["openai".to_string()]
        );
    }

    #[test]
    fn relays_do_not_take_vendor_lists() {
        let catalog = catalog();
        assert!(catalog
            .providerIdsFor("DEEPSEEK", "https://relay.example.com/v1/chat/completions")
            .is_empty());
        assert!(catalog
            .providerIdsFor("OPENAI", "https://proxy.example.com/v1/chat/completions")
            .is_empty());
        assert_eq!(
            catalog.providerIdsFor("ANTHROPIC", "https://API.anthropic.com:443/v1/messages"),
            vec!["anthropic".to_string()]
        );
        // A relay still recognizes the model it serves, by id across every provider.
        assert!(catalog.facts(&[], "deepseek-flash").is_some());
        assert!(catalog.price(&[], "deepseek-flash").is_none());
    }

    #[test]
    fn reads_thinking_controls_per_host() {
        use crate::ModelThinking::ThinkingLevel::{High, Low, Max, Off};
        let catalog = catalog();
        let deepseek = catalog
            .facts(&["deepseek".to_string()], "deepseek-flash")
            .and_then(|facts| facts.thinking)
            .expect("deepseek's controls");
        assert!(deepseek.toggle);
        assert_eq!(deepseek.efforts, vec![Low, High, Max]);

        let zhipu = catalog
            .facts(&["zhipuai".to_string()], "glm-5")
            .and_then(|facts| facts.thinking)
            .expect("zhipu's controls");
        assert!(zhipu.toggle && zhipu.efforts.is_empty());
        let relay = catalog
            .facts(&["openrouter".to_string()], "z-ai/glm-5")
            .and_then(|facts| facts.thinking)
            .expect("openrouter's controls");
        assert_eq!(relay.efforts, vec![Off, High]);

        // A lab's entry names no host's controls.
        let lab = catalog.facts(&[], "claude-mythos-5.1").expect("lab-only model");
        assert_eq!(lab.thinking, None);
        // An entry that says nothing of reasoning gives no controls either.
        let gpt = catalog.facts(&[], "gpt-5").expect("gpt-5 through openrouter");
        assert_eq!(gpt.thinking, None);
    }

    #[test]
    fn same_model_spelling() {
        assert_eq!(sameModel("anthropic/Claude-Opus-5.5-20260801"), "claude-opus-5-5");
        assert_eq!(sameModel("GPT-6 Sol"), "gpt-6-sol");
        assert_eq!(sameModel("deepseek-v4p1-flash"), "deepseek-v4-1-flash");
        assert_eq!(sameModel("qwen3-2507"), "qwen3-2507");
    }
}
