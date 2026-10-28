//! Bounded NVS entry model and one-shot faults for the simulator carrier.
//! This models entry pressure, not ESP-IDF page placement or garbage collection.
use operit_host_api::{HostError, HostResult};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

pub const TOTAL_ENTRIES: usize = 6 * 126;
pub const GC_ENTRIES: usize = 126;
pub const CAPACITY: usize = TOTAL_ENTRIES - GC_ENTRIES;
const OTHER_ENTRIES: usize = 213;

#[derive(Clone)]
pub struct StorageControl(Arc<Mutex<State>>);
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct State {
    total_entries: usize,
    reserved_gc_entries: usize,
    other_namespace_entries: usize,
    runtime_entries: usize,
    logical_bytes: usize,
    logical_budget_bytes: usize,
    fail_after_mutations: Option<usize>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Configuration {
    other_namespace_entries: Option<usize>,
    fail_after_mutations: Option<i64>,
}
impl Default for StorageControl {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(State {
            total_entries: TOTAL_ENTRIES, reserved_gc_entries: GC_ENTRIES,
            other_namespace_entries: OTHER_ENTRIES, runtime_entries: 0,
            logical_bytes: 0, logical_budget_bytes: crate::runtime_storage_codec::STORAGE_BUDGET,
            fail_after_mutations: None,
        })))
    }
}
impl StorageControl {
    pub fn snapshot(&self) -> serde_json::Value {
        let state = self.0.lock().unwrap();
        let mut result = serde_json::to_value(&*state).unwrap();
        result["availableEntries"] = CAPACITY.saturating_sub(state.other_namespace_entries + state.runtime_entries).into();
        result["model"] = "bounded-nvs-entries".into();
        result
    }
    /// Process-local test controls, reachable only through the editor's authenticated IPC.
    /// A restart clears injected faults and restores the modeled external namespace load.
    pub fn configure(&self, input: serde_json::Value) -> HostResult<serde_json::Value> {
        let input: Configuration = serde_json::from_value(input).map_err(|e| HostError::new(e.to_string()))?;
        if input.other_namespace_entries.is_some_and(|value| value > CAPACITY)
            || input.fail_after_mutations.is_some_and(|value| !(-1..=10000).contains(&value)) {
            return Err(HostError::new("Invalid simulator NVS pressure configuration"));
        }
        let mut state = self.0.lock().unwrap();
        if let Some(entries) = input.other_namespace_entries { state.other_namespace_entries = entries; }
        if let Some(after) = input.fail_after_mutations { state.fail_after_mutations = usize::try_from(after).ok(); }
        drop(state);
        Ok(self.snapshot())
    }
    pub fn update(&self, runtime_entries: usize, logical_bytes: Option<usize>) {
        let mut state = self.0.lock().unwrap();
        state.runtime_entries = runtime_entries;
        if let Some(bytes) = logical_bytes { state.logical_bytes = bytes; }
    }
    pub fn check_capacity(&self, needed: usize) -> HostResult<()> {
        let state = self.0.lock().unwrap();
        let available = CAPACITY.saturating_sub(state.other_namespace_entries + state.runtime_entries);
        if needed > available {
            return Err(HostError::new(format!("ESP32 NVS capacity exhausted: need {needed} entries, available {available}")));
        }
        Ok(())
    }
    /// Fail once; cleanup and subsequent retries can complete normally.
    pub fn before_mutation(&self) -> HostResult<()> {
        let mut state = self.0.lock().unwrap();
        if let Some(after) = state.fail_after_mutations {
            if after == 0 {
                state.fail_after_mutations = None;
                return Err(HostError::new("Injected NVS write failure"));
            }
            state.fail_after_mutations = Some(after - 1);
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn invalid_configuration_is_atomic_and_faults_are_one_shot() {
        let control = StorageControl::default();
        let before = control.snapshot();
        for value in [json!({"otherNamespaceEntries": 1, "failAfterMutations": -2}),
            json!({"otherNamespaceEntries": 631}), json!({"unknown": 1}),
            json!({"failAfterMutations": 10001}), json!({"otherNamespaceEntries": -1})] {
            assert!(control.configure(value).is_err());
            assert_eq!(control.snapshot(), before);
        }
        control.configure(json!({"failAfterMutations": 1})).unwrap();
        control.before_mutation().unwrap();
        assert!(control.before_mutation().is_err());
        control.before_mutation().unwrap();
        control.configure(json!({"failAfterMutations": 0})).unwrap();
        control.configure(json!({"failAfterMutations": -1})).unwrap();
        control.before_mutation().unwrap();
    }
}
