use crate::runtime_support::ProviderRuntimeContext;
use operit_model::ApiKeyInfo::candidate_keys;
use operit_model::ModelConfigData::ProviderProfile;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Structured transport outcome used by the provider-wide credential decision policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiKeyOutcome {
    Success,
    Authentication,
    Quota,
    RateLimited { retry_after_ms: Option<i64> },
    Transient,
    Transport,
}

#[derive(Default)]
struct KeyHealth {
    blocked: bool,
    retry_at: i64,
    failures: u32,
    last_failure: i64,
}

/// Node-local health shared by every model and concurrent request in one runtime namespace.
#[derive(Default)]
pub struct ApiKeyPoolState {
    health: HashMap<(String, String), KeyHealth>,
}

/// Hashes credentials for health identities without retaining them in registry keys.
pub fn credential_id(key: &str) -> String {
    format!("{:x}", Sha256::digest(key.trim().as_bytes()))
}

impl ApiKeyPoolState {
    /// Keeps the current healthy key; selects another eligible key only when it cannot be used.
    pub fn reserve(&mut self, provider: &mut ProviderProfile, now: i64) -> Result<String, String> {
        if !provider.useMultipleApiKeys {
            return Ok(provider.apiKey.trim().to_string());
        }
        let eligible = candidate_keys(&provider.apiKeyPool);
        let usable = |entry: &&operit_model::ApiKeyInfo::ApiKeyInfo| {
            self.health
                .get(&(provider.id.clone(), credential_id(&entry.key)))
                .map_or(true, |health| !health.blocked && health.retry_at <= now)
        };
        let chosen = eligible
            .iter()
            .copied()
            .filter(usable)
            .find(|entry| {
                provider
                    .apiKeyPool
                    .get(usize::try_from(provider.currentKeyIndex).unwrap_or(usize::MAX))
                    .is_some_and(|current| current.id == entry.id)
            })
            .or_else(|| {
                eligible.iter().copied().filter(usable).min_by_key(|entry| {
                    (
                        self.health
                            .get(&(provider.id.clone(), credential_id(&entry.key)))
                            .map_or(0, |h| h.failures),
                        entry.availabilityStatus
                            != operit_model::ApiKeyInfo::ApiKeyAvailabilityStatus::AVAILABLE,
                        entry.errorCount,
                    )
                })
            })
            .ok_or_else(|| {
                "API key pool has no usable keys; keys are disabled, unavailable, or cooling down"
                    .to_string()
            })?;
        let position = provider
            .apiKeyPool
            .iter()
            .position(|entry| entry.id == chosen.id)
            .expect("selected entry exists");
        let key = chosen.key.trim().to_string();
        provider.currentKeyIndex = position as i32;
        let entry = &mut provider.apiKeyPool[position];
        entry.usageCount = entry.usageCount.saturating_add(1);
        entry.lastUsed = now;
        Ok(key)
    }

    /// Applies feedback to the exact credential, without letting a late success erase a newer failure.
    pub fn report(
        &mut self,
        provider: &mut ProviderProfile,
        key: &str,
        started_at: i64,
        outcome: ApiKeyOutcome,
        now: i64,
    ) {
        if !provider.useMultipleApiKeys {
            return;
        }
        let Some(entry) = provider
            .apiKeyPool
            .iter_mut()
            .find(|entry| entry.key.trim() == key.trim())
        else {
            return;
        };
        let health = self
            .health
            .entry((provider.id.clone(), credential_id(key)))
            .or_default();
        if outcome == ApiKeyOutcome::Success {
            if health.last_failure < started_at {
                health.failures = 0;
            }
            return;
        }
        entry.errorCount = entry.errorCount.saturating_add(1);
        health.failures = health.failures.saturating_add(1);
        health.last_failure = now;
        let factor = 1_i64 << health.failures.saturating_sub(1).min(4);
        let delay = match outcome {
            ApiKeyOutcome::Authentication => {
                health.blocked = true;
                0
            }
            ApiKeyOutcome::Quota => 300_000,
            ApiKeyOutcome::RateLimited { retry_after_ms } => {
                retry_after_ms.unwrap_or(30_000 * factor).max(1_000)
            }
            ApiKeyOutcome::Transient | ApiKeyOutcome::Transport => (10_000 * factor).min(120_000),
            ApiKeyOutcome::Success => unreachable!(),
        };
        health.retry_at = health.retry_at.max(now.saturating_add(delay));
    }

    /// Clears health only when the user explicitly edits a credential or resets its test status.
    pub fn clear(&mut self, provider_id: &str, key: &str) {
        self.health
            .remove(&(provider_id.to_string(), credential_id(key)));
    }
}

/// The sole credential source passed to providers, for both single keys and managed pools.
#[derive(Clone)]
pub struct KeyGetter {
    acquire: std::sync::Arc<dyn Fn() -> Result<String, String> + Send + Sync>,
    report: std::sync::Arc<dyn Fn(&str, i64, ApiKeyOutcome) -> Result<(), String> + Send + Sync>,
}

impl KeyGetter {
    /// Creates a fixed credential source with no pool health or retry side effects.
    pub fn fixed(key: String) -> Self {
        Self::from_callbacks(
            std::sync::Arc::new(move || Ok(key.trim().to_string())),
            std::sync::Arc::new(|_, _, _| Ok(())),
        )
    }

    /// Enforces protocols that require a credential before dispatching their request.
    pub fn required(self) -> Self {
        let report = self.report.clone();
        Self::from_callbacks(
            std::sync::Arc::new(move || {
                let key = self.acquire()?;
                if key.is_empty() {
                    Err("provider api key is required".into())
                } else {
                    Ok(key)
                }
            }),
            report,
        )
    }
    /// Binds a request family to one provider's shared credential health.
    pub fn new(provider_id: String, context: ProviderRuntimeContext) -> Self {
        let id = provider_id.clone();
        let context_copy = context.clone();
        Self::from_callbacks(
            std::sync::Arc::new(move || context.support().reserveProviderApiKey(&provider_id)),
            std::sync::Arc::new(move |key, started, outcome| {
                context_copy
                    .support()
                    .reportProviderApiKey(&id, key, started, outcome)
            }),
        )
    }
    /// Binds isolated or synchronous callers to the same atomic credential decisions.
    pub fn from_callbacks(
        acquire: std::sync::Arc<dyn Fn() -> Result<String, String> + Send + Sync>,
        report: std::sync::Arc<
            dyn Fn(&str, i64, ApiKeyOutcome) -> Result<(), String> + Send + Sync,
        >,
    ) -> Self {
        Self { acquire, report }
    }
    /// Acquires the current healthy credential through the runtime boundary.
    pub fn acquire(&self) -> Result<String, String> {
        (self.acquire)()
    }
    /// Records an HTTP or transport outcome for the exact credential used.
    pub fn report(&self, key: &str, started_at: i64, outcome: ApiKeyOutcome) -> Result<(), String> {
        (self.report)(key, started_at, outcome)
    }
}

impl From<String> for KeyGetter {
    /// Wraps an explicit credential without exposing a second provider constructor path.
    fn from(key: String) -> Self {
        Self::fixed(key)
    }
}

impl From<&str> for KeyGetter {
    /// Wraps a literal credential for callers constructing an isolated provider.
    fn from(key: &str) -> Self {
        Self::fixed(key.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_model::ApiKeyInfo::{ApiKeyAvailabilityStatus::*, ApiKeyInfo};
    use operit_model::ModelConfigData::ApiProviderType;
    /// Creates an independent provider and shared decision state.
    fn fixture() -> (ProviderProfile, ApiKeyPoolState) {
        let mut provider = ProviderProfile::new(
            "p".into(),
            "Pool".into(),
            ApiProviderType::OPENAI_GENERIC,
            String::new(),
        );
        provider.useMultipleApiKeys = true;
        provider.apiKeyPool = (0..3)
            .map(|i| ApiKeyInfo::new(i.to_string(), format!("key-{i}")))
            .collect();
        (provider, ApiKeyPoolState::default())
    }
    /// Retains affinity on success and switches only after the current credential fails.
    #[test]
    fn retains_healthy_key_and_switches_after_failure() {
        let (mut provider, mut state) = fixture();
        for now in 1..5 {
            assert_eq!(state.reserve(&mut provider, now).unwrap(), "key-0");
        }
        state.report(&mut provider, "key-0", 1, ApiKeyOutcome::Authentication, 5);
        assert_eq!(state.reserve(&mut provider, 6).unwrap(), "key-1");
        state.report(&mut provider, "key-0", 1, ApiKeyOutcome::Success, 7);
        assert_eq!(state.reserve(&mut provider, 8).unwrap(), "key-1");
        state.report(
            &mut provider,
            "key-1",
            8,
            ApiKeyOutcome::RateLimited {
                retry_after_ms: Some(2000),
            },
            9,
        );
        assert_eq!(state.reserve(&mut provider, 10).unwrap(), "key-2");
        assert_eq!(state.reserve(&mut provider, 4000).unwrap(), "key-2");
        provider.apiKeyPool[2].isEnabled = false;
        assert_eq!(state.reserve(&mut provider, 4001).unwrap(), "key-1");
        assert_eq!(provider.apiKeyPool[0].errorCount, 1);
    }
    /// Rejects exhausted pools while permitting untested alternatives after a tested key fails.
    #[test]
    fn cooldown_exhaustion_and_untested_candidates() {
        let (mut provider, mut state) = fixture();
        provider.apiKeyPool[0].availabilityStatus = AVAILABLE;
        provider.apiKeyPool[2].availabilityStatus = UNAVAILABLE;
        state.report(&mut provider, "key-0", 1, ApiKeyOutcome::Transport, 2);
        assert_eq!(state.reserve(&mut provider, 3).unwrap(), "key-1");
        state.report(&mut provider, "key-1", 3, ApiKeyOutcome::Transient, 4);
        assert!(state.reserve(&mut provider, 5).is_err());
        assert!(state.reserve(&mut provider, 100_000).is_ok());
        provider.apiKeyPool.clear();
        provider.apiKey = "single".into();
        assert!(state.reserve(&mut provider, 100_001).is_err());
    }

    /// Respects cooldown deadlines, avoids preempting a healthy key, and honors an explicit reset.
    #[test]
    fn cooldown_recovery_keeps_affinity_and_reset_unblocks_authentication() {
        let (mut provider, mut state) = fixture();
        provider.apiKeyPool[2].isEnabled = false;
        assert_eq!(state.reserve(&mut provider, 1).unwrap(), "key-0");
        state.report(&mut provider, "key-0", 1, ApiKeyOutcome::Authentication, 2);
        assert_eq!(state.reserve(&mut provider, 3).unwrap(), "key-1");
        state.report(
            &mut provider,
            "key-1",
            3,
            ApiKeyOutcome::RateLimited {
                retry_after_ms: Some(5000),
            },
            4,
        );
        assert!(state.reserve(&mut provider, 5003).is_err());
        assert_eq!(state.reserve(&mut provider, 5004).unwrap(), "key-1");
        state.clear(&provider.id, "key-0");
        assert_eq!(state.reserve(&mut provider, 5005).unwrap(), "key-1");
        provider.apiKeyPool[1].isEnabled = false;
        assert_eq!(state.reserve(&mut provider, 5006).unwrap(), "key-0");
    }

    /// Prefers a tested healthy replacement while keeping the current untested key until it fails.
    #[test]
    fn replacement_prefers_tested_keys_without_interrupting_affinity() {
        let (mut provider, mut state) = fixture();
        provider.apiKeyPool[2].availabilityStatus = AVAILABLE;
        assert_eq!(state.reserve(&mut provider, 1).unwrap(), "key-0");
        state.report(&mut provider, "key-0", 1, ApiKeyOutcome::Transient, 2);
        assert_eq!(state.reserve(&mut provider, 3).unwrap(), "key-2");
        state.report(&mut provider, "key-2", 3, ApiKeyOutcome::Transient, 4);
        assert_eq!(state.reserve(&mut provider, 5).unwrap(), "key-1");
        assert_eq!(state.reserve(&mut provider, 100_000).unwrap(), "key-1");
    }
}
