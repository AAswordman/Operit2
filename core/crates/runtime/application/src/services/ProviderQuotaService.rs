#![allow(non_snake_case)]

use std::collections::BTreeMap;

use operit_host_api::TimeUtils::currentTimeMillis;
use operit_model::ModelConfigData::{ApiProviderType, ProviderProfile};
use operit_model::ProviderQuota::{
    mask_api_key_fingerprint, ProviderBalanceProtocol, ProviderLocalBudget,
    ProviderQuotaCenterState, ProviderQuotaProbeConfig, ProviderQuotaProbeOutcome,
    ProviderQuotaSnapshot, QuotaAccountKind, QuotaBalanceBreakdown, QuotaProbeStatus, QuotaWindow,
    QuotaWindowKind, KeyQuotaSnapshot, UsageResetScope,
};
use operit_providers::chat::llmprovider::ProviderQuotaProber::{
    console_url_for_provider, probe_single_key, resolve_effective_protocol, CodexQuotaCredentials,
};
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_store::repository::UsageStatisticsStore::{UsageRequestRecord, UsageStatisticsStore};

use crate::data::preferences::CodexAuthPreferences::CodexAuthPreferences;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::data::preferences::ProviderQuotaPreferences::ProviderQuotaPreferences;
use crate::services::CodexOAuthService::CodexOAuthService;

/// Manages provider balance/quota probing, local spending budgets, and usage reset operations.
pub struct ProviderQuotaService;

impl ProviderQuotaService {
    /// Returns the runtime-managed ProviderQuotaService singleton.
    pub fn getInstance() -> Self {
        Self
    }

    /// Reads the combined Space-synced snapshots, budgets, and probe configurations.
    pub fn getQuotaCenterState(&self) -> Result<ProviderQuotaCenterState, String> {
        let profiles = ModelConfigManager::default()
            .getProviderProfiles()
            .map_err(|error| error.to_string())?;
        let prefs = ProviderQuotaPreferences::default();
        let stored_snapshots: BTreeMap<String, ProviderQuotaSnapshot> = prefs
            .listSnapshots()?
            .into_iter()
            .map(|snapshot| (snapshot.providerId.clone(), snapshot))
            .collect();
        let budgets = prefs.listBudgets()?;
        let probe_configs = prefs.listProbeConfigs()?;
        let config_map: BTreeMap<String, ProviderQuotaProbeConfig> = probe_configs
            .iter()
            .map(|cfg| (cfg.providerId.clone(), cfg.clone()))
            .collect();

        let local_records = if profiles.iter().any(|p| {
            matches!(
                resolve_effective_protocol(p, config_map.get(&p.id)),
                ProviderBalanceProtocol::LOCAL_COMPUTE
            )
        }) {
            UsageStatisticsStore::new().getAllRequestRecords().unwrap_or_default()
        } else {
            Vec::new()
        };

        let mut snapshots = Vec::with_capacity(profiles.len());
        for profile in &profiles {
            let cfg = config_map.get(&profile.id);
            let effective_protocol = resolve_effective_protocol(profile, cfg);
            if effective_protocol == ProviderBalanceProtocol::LOCAL_COMPUTE {
                snapshots.push(build_local_compute_snapshot(profile, &local_records));
                continue;
            }
            if let Some(mut saved) = stored_snapshots.get(&profile.id).cloned() {
                saved.providerName = profile.name.clone();
                saved.providerType = profile.providerType.clone();
                saved.providerTypeId = profile.providerTypeId.clone();
                saved.protocol = effective_protocol;
                if saved.consoleUrl.is_none() {
                    saved.consoleUrl = console_url_for_provider(profile);
                }
                snapshots.push(saved);
            } else {
                snapshots.push(initial_unprobed_snapshot(profile, cfg));
            }
        }

        Ok(ProviderQuotaCenterState {
            snapshots,
            budgets,
            probeConfigs: probe_configs,
        })
    }

    /// Probes one configured provider (and all enabled pool keys) on this device node.
    #[operit_route_macros::operit_core_route(target = targetNodeId, permission = "target:runtime.execute")]
    pub fn probeProviderQuota(&self, providerId: String, targetNodeId: Option<String>)
        -> impl std::future::Future<Output = Result<ProviderQuotaSnapshot, String>> + 'static {
        async move {
            let _ = targetNodeId;
            Self::getInstance().probeLocalProviderQuota(providerId).await
        }
    }

    /// Refreshes the selected node and retains its returned snapshot for this UI.
    pub async fn refreshProviderQuota(&self, providerId: String, targetNodeId: Option<String>) -> Result<ProviderQuotaSnapshot, String> {
        let snapshot = self.probeProviderQuota(providerId, targetNodeId).await?;
        ProviderQuotaPreferences::default().saveSnapshot(&snapshot)?;
        Ok(snapshot)
    }

    /// Queries credentials owned by this node after route authorization.
    async fn probeLocalProviderQuota(&self, providerId: String) -> Result<ProviderQuotaSnapshot, String> {
        let profile = ModelConfigManager::default()
            .getProviderProfile(&providerId)
            .map_err(|error| error.to_string())?;
        let prefs = ProviderQuotaPreferences::default();
        let cfg = prefs
            .listProbeConfigs()?
            .into_iter()
            .find(|item| item.providerId == providerId);
        let effective_protocol = resolve_effective_protocol(&profile, cfg.as_ref());
        let console_url = console_url_for_provider(&profile);
        let now_ms = currentTimeMillis();
        let local_node_id = CoreNodeIdentityStore::localNodeId().unwrap_or_default();
        let local_device_name = resolve_local_device_name();

        if effective_protocol == ProviderBalanceProtocol::LOCAL_COMPUTE {
            let records = UsageStatisticsStore::new().getAllRequestRecords().unwrap_or_default();
            let mut snapshot = build_local_compute_snapshot(&profile, &records);
            snapshot.probedByNodeId = local_node_id;
            snapshot.probedByDeviceName = local_device_name;
            prefs.saveSnapshot(&snapshot)?;
            return Ok(snapshot);
        }

        if effective_protocol == ProviderBalanceProtocol::CODEX_WHAM_USAGE {
            let codex_outcome = match CodexOAuthService::getInstance().accessForRequest().await {
                Ok((accessToken, accountId)) => {
                    let email = CodexAuthPreferences::getInstance()
                        .load()
                        .ok()
                        .flatten()
                        .map(|tokens| tokens.email)
                        .unwrap_or_default();
                    let creds = CodexQuotaCredentials {
                        accessToken,
                        accountId,
                        email,
                    };
                    probe_single_key(&profile, "", cfg.as_ref(), Some(&creds)).await
                }
                Err(error) => ProviderQuotaProbeOutcome {
                    accountKind: QuotaAccountKind::SUBSCRIPTION,
                    protocol: ProviderBalanceProtocol::CODEX_WHAM_USAGE,
                    status: QuotaProbeStatus::UNAUTHORIZED,
                    plan: Some("ChatGPT Codex".to_string()),
                    accountLabel: None,
                    balance: None,
                    windows: Vec::new(),
                    consoleUrl: console_url.clone(),
                    error: Some(error),
                },
            };
            let snapshot = ProviderQuotaSnapshot {
                providerId: profile.id.clone(),
                providerName: profile.name.clone(),
                providerType: profile.providerType,
                providerTypeId: profile.providerTypeId.clone(),
                accountKind: codex_outcome.accountKind,
                protocol: codex_outcome.protocol,
                status: codex_outcome.status,
                plan: codex_outcome.plan,
                accountLabel: codex_outcome.accountLabel,
                balance: codex_outcome.balance,
                windows: codex_outcome.windows,
                keySnapshots: Vec::new(),
                consoleUrl: codex_outcome.consoleUrl.or(console_url),
                error: codex_outcome.error,
                held: false,
                asOfMs: now_ms,
                probedByNodeId: local_node_id,
                probedByDeviceName: local_device_name,
            };
            prefs.saveSnapshot(&snapshot)?;
            return Ok(snapshot);
        }

        if effective_protocol == ProviderBalanceProtocol::UNSUPPORTED {
            let snapshot = ProviderQuotaSnapshot {
                providerId: profile.id.clone(),
                providerName: profile.name.clone(),
                providerType: profile.providerType,
                providerTypeId: profile.providerTypeId.clone(),
                accountKind: QuotaAccountKind::UNSUPPORTED,
                protocol: effective_protocol,
                status: QuotaProbeStatus::UNSUPPORTED,
                plan: None,
                accountLabel: None,
                balance: None,
                windows: Vec::new(),
                keySnapshots: Vec::new(),
                consoleUrl: console_url,
                error: Some(
                    "This provider does not expose a key-queryable balance API; configure a Monthly Budget Cap or open the console."
                        .to_string(),
                ),
                held: false,
                asOfMs: now_ms,
                probedByNodeId: local_node_id,
                probedByDeviceName: local_device_name,
            };
            prefs.saveSnapshot(&snapshot)?;
            return Ok(snapshot);
        }

        let key_entries = collect_probe_keys(&profile);
        let mut key_snapshots = Vec::with_capacity(key_entries.len());
        let mut outcomes = Vec::with_capacity(key_entries.len());
        for (key_id, key_name, raw_key, enabled) in key_entries {
            let fingerprint = mask_api_key_fingerprint(&raw_key);
            let outcome = if raw_key.trim().is_empty() {
                ProviderQuotaProbeOutcome {
                    accountKind: account_kind_for_protocol(effective_protocol),
                    protocol: effective_protocol,
                    status: QuotaProbeStatus::UNAUTHORIZED,
                    plan: None,
                    accountLabel: None,
                    balance: None,
                    windows: Vec::new(),
                    consoleUrl: console_url.clone(),
                    error: Some("API key is empty".to_string()),
                }
            } else {
                probe_single_key(&profile, &raw_key, cfg.as_ref(), None).await
            };
            key_snapshots.push(KeyQuotaSnapshot {
                keyId: key_id,
                keyName: key_name,
                keyFingerprint: fingerprint,
                isEnabled: enabled,
                status: outcome.status,
                balance: outcome.balance.clone(),
                windows: outcome.windows.clone(),
                error: outcome.error.clone(),
                asOfMs: now_ms,
            });
            outcomes.push(outcome);
        }

        let snapshot = aggregate_provider_snapshot(
            &profile,
            effective_protocol,
            console_url,
            now_ms,
            local_node_id,
            local_device_name,
            &outcomes,
            key_snapshots,
        );
        prefs.saveSnapshot(&snapshot)?;
        Ok(snapshot)
    }

    /// Probes all configured providers sequentially/concisely and returns updated snapshots.
    pub async fn probeAllProviders(&self) -> Result<Vec<ProviderQuotaSnapshot>, String> {
        let profiles = ModelConfigManager::default()
            .getProviderProfiles()
            .map_err(|error| error.to_string())?;
        let mut futures = Vec::with_capacity(profiles.len());
        for profile in profiles {
            futures.push(self.probeLocalProviderQuota(profile.id));
        }
        let results = futures_util::future::join_all(futures).await;
        let mut snapshots = Vec::with_capacity(results.len());
        for res in results {
            snapshots.push(res?);
        }
        Ok(snapshots)
    }

    /// Saves or updates a user-defined monthly/total spending cap for a provider.
    pub fn saveProviderLocalBudget(
        &self,
        mut budget: ProviderLocalBudget,
    ) -> Result<ProviderLocalBudget, String> {
        if budget.providerId.trim().is_empty() {
            return Err("providerId must not be empty".to_string());
        }
        if budget.monthlyBudget < 0.0 || !budget.monthlyBudget.is_finite() {
            return Err("monthlyBudget must be a finite non-negative number".to_string());
        }
        budget.resetDayOfMonth = budget.resetDayOfMonth.clamp(1, 28);
        if budget.alertThresholdPercent <= 0.0 || budget.alertThresholdPercent > 100.0 {
            budget.alertThresholdPercent = 80.0;
        }
        if budget.currency.trim().is_empty() {
            budget.currency = "USD".to_string();
        }
        budget.updatedAtMs = currentTimeMillis();
        ProviderQuotaPreferences::default().saveBudget(&budget)?;
        Ok(budget)
    }

    /// Deletes a provider's local budget configuration.
    pub fn deleteProviderLocalBudget(&self, providerId: String) -> Result<(), String> {
        ProviderQuotaPreferences::default().deleteBudget(&providerId)
    }

    /// Saves a provider's explicit quota protocol override or custom HTTP probe settings.
    pub fn saveProviderProbeConfig(
        &self,
        config: ProviderQuotaProbeConfig,
    ) -> Result<ProviderQuotaProbeConfig, String> {
        if config.providerId.trim().is_empty() {
            return Err("providerId must not be empty".to_string());
        }
        ProviderQuotaPreferences::default().saveProbeConfig(&config)?;
        Ok(config)
    }

    /// Clears usage statistics either locally on this device or synchronized across the Space.
    pub fn resetUsageStatistics(&self, scope: UsageResetScope) -> Result<(), String> {
        UsageStatisticsStore::new().clearUsageStatistics(scope)
    }
}

fn collect_probe_keys(profile: &ProviderProfile) -> Vec<(String, Option<String>, String, bool)> {
    if profile.useMultipleApiKeys && !profile.apiKeyPool.is_empty() {
        let mut list = Vec::new();
        for entry in &profile.apiKeyPool {
            if !entry.isEnabled {
                continue;
            }
            let name = if entry.name.trim().is_empty() {
                None
            } else {
                Some(entry.name.trim().to_string())
            };
            list.push((entry.id.clone(), name, entry.key.clone(), entry.isEnabled));
        }
        if !list.is_empty() {
            return list;
        }
    }
    vec![(
        "primary".to_string(),
        None,
        profile.apiKey.clone(),
        true,
    )]
}

fn aggregate_provider_snapshot(
    profile: &ProviderProfile,
    protocol: ProviderBalanceProtocol,
    console_url: Option<String>,
    now_ms: i64,
    local_node_id: String,
    local_device_name: String,
    outcomes: &[ProviderQuotaProbeOutcome],
    key_snapshots: Vec<KeyQuotaSnapshot>,
) -> ProviderQuotaSnapshot {
    let ok_outcomes: Vec<&ProviderQuotaProbeOutcome> = outcomes
        .iter()
        .filter(|o| o.status == QuotaProbeStatus::OK)
        .collect();
    if let Some(first_ok) = ok_outcomes.first() {
        let aggregate_balance = sum_balances(&ok_outcomes);
        return ProviderQuotaSnapshot {
            providerId: profile.id.clone(),
            providerName: profile.name.clone(),
            providerType: profile.providerType.clone(),
            providerTypeId: profile.providerTypeId.clone(),
            accountKind: first_ok.accountKind,
            protocol,
            status: QuotaProbeStatus::OK,
            plan: first_ok.plan.clone(),
            accountLabel: first_ok.accountLabel.clone(),
            balance: aggregate_balance.or_else(|| first_ok.balance.clone()),
            windows: first_ok.windows.clone(),
            keySnapshots: key_snapshots,
            consoleUrl: first_ok.consoleUrl.clone().or(console_url),
            error: None,
            held: false,
            asOfMs: now_ms,
            probedByNodeId: local_node_id,
            probedByDeviceName: local_device_name,
        };
    }

    let first = outcomes.first();
    ProviderQuotaSnapshot {
        providerId: profile.id.clone(),
        providerName: profile.name.clone(),
        providerType: profile.providerType.clone(),
        providerTypeId: profile.providerTypeId.clone(),
        accountKind: first
            .map(|o| o.accountKind)
            .unwrap_or_else(|| account_kind_for_protocol(protocol)),
        protocol,
        status: first.map(|o| o.status).unwrap_or(QuotaProbeStatus::ERROR),
        plan: first.and_then(|o| o.plan.clone()),
        accountLabel: None,
        balance: None,
        windows: Vec::new(),
        keySnapshots: key_snapshots,
        consoleUrl: console_url,
        error: first.and_then(|o| o.error.clone()),
        held: false,
        asOfMs: now_ms,
        probedByNodeId: local_node_id,
        probedByDeviceName: local_device_name,
    }
}

fn sum_balances(ok_outcomes: &[&ProviderQuotaProbeOutcome]) -> Option<QuotaBalanceBreakdown> {
    let first_bal = ok_outcomes.iter().find_map(|o| o.balance.as_ref())?;
    if ok_outcomes.len() <= 1 {
        return Some(first_bal.clone());
    }
    let currency = first_bal.currency.clone();
    let mut total = 0.0_f64;
    let mut has_total = false;
    let mut cash = 0.0_f64;
    let mut has_cash = false;
    let mut granted = 0.0_f64;
    let mut has_granted = false;
    let mut used = 0.0_f64;
    let mut has_used = false;
    let mut daily = 0.0_f64;
    let mut has_daily = false;
    let mut monthly = 0.0_f64;
    let mut has_monthly = false;
    let mut any_available = false;

    for outcome in ok_outcomes {
        let Some(bal) = outcome.balance.as_ref() else {
            continue;
        };
        if !bal.currency.eq_ignore_ascii_case(&currency) {
            return Some(first_bal.clone());
        }
        any_available |= bal.isAvailable;
        if let Some(v) = bal.totalBalance {
            total += v;
            has_total = true;
        }
        if let Some(v) = bal.cashBalance {
            cash += v;
            has_cash = true;
        }
        if let Some(v) = bal.grantedBalance {
            granted += v;
            has_granted = true;
        }
        if let Some(v) = bal.usedAmount {
            used += v;
            has_used = true;
        }
        if let Some(v) = bal.dailyUsed {
            daily += v;
            has_daily = true;
        }
        if let Some(v) = bal.monthlyUsed {
            monthly += v;
            has_monthly = true;
        }
    }

    Some(QuotaBalanceBreakdown {
        currency,
        totalBalance: has_total.then_some(total),
        cashBalance: has_cash.then_some(cash),
        grantedBalance: has_granted.then_some(granted),
        usedAmount: has_used.then_some(used),
        dailyUsed: has_daily.then_some(daily),
        monthlyUsed: has_monthly.then_some(monthly),
        isAvailable: any_available,
        isFreeTier: first_bal.isFreeTier,
    })
}

fn build_local_compute_snapshot(
    profile: &ProviderProfile,
    records: &[UsageRequestRecord],
) -> ProviderQuotaSnapshot {
    let mut total_tokens: i64 = 0;
    let mut req_count: i64 = 0;
    for rec in records {
        if rec.providerId == profile.id
            || (rec.providerId.is_empty() && rec.provider == profile.providerTypeId)
        {
            req_count += 1;
            total_tokens += rec.inputTokens + rec.outputTokens;
        }
    }
    ProviderQuotaSnapshot {
        providerId: profile.id.clone(),
        providerName: profile.name.clone(),
        providerType: profile.providerType.clone(),
        providerTypeId: profile.providerTypeId.clone(),
        accountKind: QuotaAccountKind::LOCAL_COMPUTE,
        protocol: ProviderBalanceProtocol::LOCAL_COMPUTE,
        status: QuotaProbeStatus::OK,
        plan: Some("On-Device Compute".to_string()),
        accountLabel: Some(format!("{req_count} requests")),
        balance: None,
        windows: vec![QuotaWindow {
            name: "Local Tokens Processed".to_string(),
            kind: QuotaWindowKind::TOTAL,
            display: format!("{total_tokens} tok"),
            used: Some(total_tokens as f64),
            remaining: None,
            limit: None,
            usedPercent: None,
            unlimited: true,
            resetsAtMs: None,
            resetSecs: None,
            spanLabel: Some("local".to_string()),
            model: None,
            unit: Some("tokens".to_string()),
        }],
        keySnapshots: Vec::new(),
        consoleUrl: None,
        error: None,
        held: false,
        asOfMs: currentTimeMillis(),
        probedByNodeId: CoreNodeIdentityStore::localNodeId().unwrap_or_default(),
        probedByDeviceName: resolve_local_device_name(),
    }
}

fn initial_unprobed_snapshot(
    profile: &ProviderProfile,
    cfg: Option<&ProviderQuotaProbeConfig>,
) -> ProviderQuotaSnapshot {
    let protocol = resolve_effective_protocol(profile, cfg);
    let account_kind = account_kind_for_protocol(protocol);
    let status = if protocol == ProviderBalanceProtocol::UNSUPPORTED {
        QuotaProbeStatus::UNSUPPORTED
    } else {
        QuotaProbeStatus::STALE
    };
    let key_snapshots = collect_probe_keys(profile)
        .into_iter()
        .filter(|(_, _, raw, _)| !raw.trim().is_empty())
        .map(|(key_id, key_name, raw, enabled)| KeyQuotaSnapshot {
            keyId: key_id,
            keyName: key_name,
            keyFingerprint: mask_api_key_fingerprint(&raw),
            isEnabled: enabled,
            status,
            balance: None,
            windows: Vec::new(),
            error: None,
            asOfMs: 0,
        })
        .collect();
    ProviderQuotaSnapshot {
        providerId: profile.id.clone(),
        providerName: profile.name.clone(),
        providerType: profile.providerType.clone(),
        providerTypeId: profile.providerTypeId.clone(),
        accountKind: account_kind,
        protocol,
        status,
        plan: None,
        accountLabel: None,
        balance: None,
        windows: Vec::new(),
        keySnapshots: key_snapshots,
        consoleUrl: console_url_for_provider(profile),
        error: if status == QuotaProbeStatus::UNSUPPORTED {
            Some(
                "No public balance API; track with a Monthly Budget Cap or open console."
                    .to_string(),
            )
        } else {
            None
        },
        held: false,
        asOfMs: 0,
        probedByNodeId: String::new(),
        probedByDeviceName: String::new(),
    }
}

fn account_kind_for_protocol(protocol: ProviderBalanceProtocol) -> QuotaAccountKind {
    match protocol {
        ProviderBalanceProtocol::DEEPSEEK_BALANCE
        | ProviderBalanceProtocol::OPENROUTER_KEY
        | ProviderBalanceProtocol::OPENROUTER_CREDITS
        | ProviderBalanceProtocol::ZHIPU_ACCOUNT_REPORT
        | ProviderBalanceProtocol::MOONSHOT_BALANCE
        | ProviderBalanceProtocol::SILICONFLOW_USER_INFO
        | ProviderBalanceProtocol::NEW_API_TOKEN
        | ProviderBalanceProtocol::OPENAI_LEGACY_BILLING
        | ProviderBalanceProtocol::CUSTOM_HTTP_JSON => QuotaAccountKind::BALANCE,
        ProviderBalanceProtocol::ZHIPU_CODING_PLAN
        | ProviderBalanceProtocol::MINIMAX_TOKEN_PLAN
        | ProviderBalanceProtocol::KIMI_CODING_USAGES
        | ProviderBalanceProtocol::CODEX_WHAM_USAGE => QuotaAccountKind::SUBSCRIPTION,
        ProviderBalanceProtocol::LOCAL_COMPUTE => QuotaAccountKind::LOCAL_COMPUTE,
        ProviderBalanceProtocol::UNSUPPORTED | ProviderBalanceProtocol::AUTO => {
            QuotaAccountKind::UNSUPPORTED
        }
    }
}

fn resolve_local_device_name() -> String {
    let Ok(storage) = std::panic::catch_unwind(operit_store::RuntimeStorageHost::defaultRuntimeStorageHost)
    else {
        return "Local Node".to_string();
    };
    if let Some(local_id) = CoreNodeIdentityStore::localNodeId() {
        if let Ok(Some(profile)) = operit_store::CoreSpaceStore::CoreSpaceStore::new(storage).deviceProfile(&local_id) {
            if !profile.displayName.trim().is_empty() {
                return profile.displayName;
            }
        }
    }
    "Local Node".to_string()
}
