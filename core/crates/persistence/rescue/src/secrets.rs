//! The portable Host secret contract shared by normal snapshots and offline rescue.
use std::collections::BTreeMap;

use base64::{engine::general_purpose::STANDARD, Engine};
use operit_host_api::HostSecretStore;

pub const SYSTEM_SECRETS: &str = "system/host-secrets.json";
pub const SECRET_KEYS: &[&str] = &[
    "operit.preferences.encryption_key.v1",
    "market.github-oauth-pending",
];
pub type PortableSecrets = BTreeMap<String, String>;

/// Reads registered secrets without exporting platform wrapping keys or creating new keys.
pub fn read_secrets(store: Option<&dyn HostSecretStore>) -> Result<PortableSecrets, String> {
    let mut values = BTreeMap::new();
    if let Some(store) = store {
        for key in SECRET_KEYS {
            if let Some(value) = store.readSecret(key).map_err(|error| error.to_string())? {
                values.insert((*key).into(), STANDARD.encode(value));
            }
        }
    }
    Ok(values)
}

/// Rejects unknown secret names and invalid encodings before any destination changes.
pub fn validate_secrets(values: &PortableSecrets) -> Result<(), String> {
    for (key, value) in values {
        if !SECRET_KEYS.contains(&key.as_str()) {
            return Err(format!("Unregistered Host secret: {key}"));
        }
        STANDARD.decode(value).map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Checks that every imported secret can be published by the destination Host.
pub fn validate_destination(
    values: &PortableSecrets,
    store: Option<&dyn HostSecretStore>,
) -> Result<(), String> {
    validate_secrets(values)?;
    if !values.is_empty() && store.is_none() {
        return Err("This Host does not support importing the archive's system secrets".into());
    }
    Ok(())
}

/// Restores one registered secret so durable publishers can checkpoint each step.
pub fn restore_secret(
    store: &dyn HostSecretStore,
    values: &PortableSecrets,
    key: &str,
) -> Result<(), String> {
    if !SECRET_KEYS.contains(&key) {
        return Err(format!("Unregistered Host secret: {key}"));
    }
    match values.get(key) {
        Some(value) => store.writeSecret(
            key,
            &STANDARD.decode(value).map_err(|error| error.to_string())?,
        ),
        None => store.deleteSecret(key),
    }
    .map_err(|error| error.to_string())
}

/// Replaces the registered secret set with the verified archive's exact values.
pub fn restore_secrets(
    values: &PortableSecrets,
    store: Option<&dyn HostSecretStore>,
) -> Result<(), String> {
    validate_destination(values, store)?;
    if let Some(store) = store {
        for key in SECRET_KEYS {
            restore_secret(store, values, key)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::{HostError, HostResult};
    use std::sync::Mutex;

    #[derive(Default)]
    struct Secrets(Mutex<BTreeMap<String, Vec<u8>>>);
    impl HostSecretStore for Secrets {
        /// Reads one deterministic registered fixture secret.
        fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
            Ok(self.0.lock().unwrap().get(key).cloned())
        }
        /// Records writes so validation failures can prove that originals survived.
        fn writeSecret(&self, key: &str, bytes: &[u8]) -> HostResult<()> {
            self.0.lock().unwrap().insert(key.into(), bytes.to_vec());
            Ok(())
        }
        /// Removes only the declared fixture secret.
        fn deleteSecret(&self, key: &str) -> HostResult<()> {
            self.0.lock().unwrap().remove(key);
            Ok(())
        }
    }

    /// Rejects unknown keys and malformed base64 before deleting or writing any secret.
    #[test]
    fn invalid_secret_sets_preserve_destination() {
        let store = Secrets::default();
        store.writeSecret(SECRET_KEYS[0], b"original").unwrap();
        for values in [
            BTreeMap::from([("unknown-key".into(), STANDARD.encode(b"unknown"))]),
            BTreeMap::from([(SECRET_KEYS[0].into(), "invalid-base64!".into())]),
        ] {
            assert!(restore_secrets(&values, Some(&store)).is_err());
            assert_eq!(
                store.readSecret(SECRET_KEYS[0]).unwrap().unwrap(),
                b"original"
            );
        }
    }

    /// Replaces only the registered portable set and preserves unrelated Host secrets.
    #[test]
    fn portable_secrets_replace_registered_values_only() {
        let store = Secrets::default();
        store.writeSecret(SECRET_KEYS[0], b"old").unwrap();
        store.writeSecret(SECRET_KEYS[1], b"stale").unwrap();
        store.writeSecret("unrelated", b"keep").unwrap();
        let imported = BTreeMap::from([(SECRET_KEYS[0].into(), STANDARD.encode(b"restored"))]);
        restore_secrets(&imported, Some(&store)).unwrap();
        assert_eq!(read_secrets(Some(&store)).unwrap(), imported);
        assert_eq!(store.readSecret("unrelated").unwrap().unwrap(), b"keep");
        assert!(validate_destination(&imported, None).is_err());
    }

    struct UnreadableSecrets;
    impl HostSecretStore for UnreadableSecrets {
        /// Reports the actual Host read failure instead of pretending that no key exists.
        fn readSecret(&self, _: &str) -> HostResult<Option<Vec<u8>>> {
            Err(HostError::new("injected read failure"))
        }
        /// Fails the test if exporting tries to create a replacement key.
        fn writeSecret(&self, _: &str, _: &[u8]) -> HostResult<()> {
            panic!("export must not write keys")
        }
        /// Fails the test if exporting tries to remove an unreadable key.
        fn deleteSecret(&self, _: &str) -> HostResult<()> {
            panic!("export must not delete keys")
        }
    }

    /// Propagates secret read errors without exporting an empty replacement set.
    #[test]
    fn secret_export_preserves_host_errors() {
        assert!(read_secrets(Some(&UnreadableSecrets))
            .unwrap_err()
            .contains("injected read failure"));
    }
}
