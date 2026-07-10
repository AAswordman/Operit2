use base64::engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD};
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use operit_host_api::RuntimeStorageHost;
use serde::{Deserialize, Serialize};

use crate::PreferencesDataStore::PreferencesDataStoreError;
use crate::RuntimeStorageHost::defaultHostSecretStoreOption;

const ENCRYPTION_KEY_PATH: &str = "secure/preferences_encryption_key.json";
const ENCRYPTION_HOST_SECRET_KEY: &str = "operit.preferences.encryption_key.v1";
const ENCRYPTION_KEY_FORMAT: &str = "operit.preferences.encryption.key";
const ENCRYPTION_KEY_VERSION: u32 = 1;
const ENCRYPTED_PREFERENCES_FORMAT: &str = "operit.preferences.encrypted";
const ENCRYPTED_PREFERENCES_VERSION: u32 = 1;
const ENCRYPTION_ALGORITHM: &str = "XChaCha20Poly1305";
const KEY_LENGTH: usize = 32;
const NONCE_LENGTH: usize = 24;

#[derive(Clone)]
/// XChaCha20-Poly1305 encryption helper for encrypted preferences files.
pub struct PreferencesEncryption {
    keyId: String,
    key: [u8; KEY_LENGTH],
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredPreferencesEncryptionKey {
    format: String,
    version: u32,
    algorithm: String,
    #[serde(rename = "keyId")]
    keyId: String,
    key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedPreferencesEnvelope {
    format: String,
    version: u32,
    algorithm: String,
    #[serde(rename = "keyId")]
    keyId: String,
    nonce: String,
    ciphertext: String,
}

impl PreferencesEncryption {
    /// Loads the existing preferences encryption key or creates and stores a new one.
    pub fn load_or_create(
        storageHost: &dyn RuntimeStorageHost,
    ) -> Result<Self, PreferencesDataStoreError> {
        Self::loadOrCreateWithSecretStore(storageHost, defaultHostSecretStoreOption().as_deref())
    }

    /// Loads or creates the encryption key using the supplied secret store.
    fn loadOrCreateWithSecretStore(
        storageHost: &dyn RuntimeStorageHost,
        secretStore: Option<&dyn operit_host_api::HostSecretStore>,
    ) -> Result<Self, PreferencesDataStoreError> {
        if let Some(secretStore) = secretStore {
            return Self::loadOrCreateFromHostSecret(storageHost, secretStore);
        }
        if storageHost.exists(ENCRYPTION_KEY_PATH)? {
            return Self::load(storageHost);
        }
        Self::create(storageHost)
    }

    /// Loads the key from host secrets, migrates an old stored key, or creates a host secret key.
    fn loadOrCreateFromHostSecret(
        storageHost: &dyn RuntimeStorageHost,
        secretStore: &dyn operit_host_api::HostSecretStore,
    ) -> Result<Self, PreferencesDataStoreError> {
        if let Some(content) = secretStore
            .readSecret(ENCRYPTION_HOST_SECRET_KEY)
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?
        {
            return Self::decodeKeyBytes(&content);
        }
        if storageHost.exists(ENCRYPTION_KEY_PATH)? {
            let content = storageHost.readBytes(ENCRYPTION_KEY_PATH)?;
            let encryption = Self::decodeKeyBytes(&content)?;
            secretStore
                .writeSecret(ENCRYPTION_HOST_SECRET_KEY, &content)
                .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
            storageHost.delete(ENCRYPTION_KEY_PATH, false)?;
            return Ok(encryption);
        }
        Self::createWithHostSecret(secretStore)
    }

    fn load(storageHost: &dyn RuntimeStorageHost) -> Result<Self, PreferencesDataStoreError> {
        let content = storageHost.readBytes(ENCRYPTION_KEY_PATH)?;
        Self::decodeKeyBytes(&content)
    }

    fn decodeKeyBytes(content: &[u8]) -> Result<Self, PreferencesDataStoreError> {
        let content = String::from_utf8(content.to_vec())
            .map_err(|error| PreferencesDataStoreError::Message(error.to_string()))?;
        let stored: StoredPreferencesEncryptionKey = serde_json::from_str(&content)?;
        if stored.format != ENCRYPTION_KEY_FORMAT {
            return Err(PreferencesDataStoreError::Encryption(format!(
                "unexpected preferences encryption key format: {}",
                stored.format
            )));
        }
        if stored.version != ENCRYPTION_KEY_VERSION {
            return Err(PreferencesDataStoreError::Encryption(format!(
                "unsupported preferences encryption key version: {}",
                stored.version
            )));
        }
        if stored.algorithm != ENCRYPTION_ALGORITHM {
            return Err(PreferencesDataStoreError::Encryption(format!(
                "unsupported preferences encryption key algorithm: {}",
                stored.algorithm
            )));
        }
        let keyBytes = STANDARD_NO_PAD
            .decode(stored.key.as_bytes())
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
        let key: [u8; KEY_LENGTH] = keyBytes
            .try_into()
            .map_err(|_| PreferencesDataStoreError::Encryption("invalid key length".to_string()))?;
        Ok(Self {
            keyId: stored.keyId,
            key,
        })
    }

    fn createWithHostSecret(
        secretStore: &dyn operit_host_api::HostSecretStore,
    ) -> Result<Self, PreferencesDataStoreError> {
        let stored = Self::newStoredKey()?;
        let content = serde_json::to_vec_pretty(&stored)?;
        secretStore
            .writeSecret(ENCRYPTION_HOST_SECRET_KEY, &content)
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
        Ok(Self {
            keyId: stored.keyId,
            key: decodeStoredKey(&stored.key)?,
        })
    }

    fn create(storageHost: &dyn RuntimeStorageHost) -> Result<Self, PreferencesDataStoreError> {
        let stored = Self::newStoredKey()?;
        let content = serde_json::to_vec_pretty(&stored)?;
        storageHost.writeBytes(ENCRYPTION_KEY_PATH, &content)?;
        Ok(Self {
            keyId: stored.keyId,
            key: decodeStoredKey(&stored.key)?,
        })
    }

    fn newStoredKey() -> Result<StoredPreferencesEncryptionKey, PreferencesDataStoreError> {
        let mut key = [0u8; KEY_LENGTH];
        getrandom::getrandom(&mut key)
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
        let mut keyIdBytes = [0u8; 16];
        getrandom::getrandom(&mut keyIdBytes)
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
        Ok(StoredPreferencesEncryptionKey {
            format: ENCRYPTION_KEY_FORMAT.to_string(),
            version: ENCRYPTION_KEY_VERSION,
            algorithm: ENCRYPTION_ALGORITHM.to_string(),
            keyId: URL_SAFE_NO_PAD.encode(keyIdBytes),
            key: STANDARD_NO_PAD.encode(key),
        })
    }

    /// Encrypts preference bytes using the storage path as authenticated data.
    pub fn encrypt(
        &self,
        storagePath: &str,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, PreferencesDataStoreError> {
        let cipher = XChaCha20Poly1305::new_from_slice(&self.key)
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
        let mut nonce = [0u8; NONCE_LENGTH];
        getrandom::getrandom(&mut nonce)
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
        let ciphertext = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: storagePath.as_bytes(),
                },
            )
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
        let envelope = EncryptedPreferencesEnvelope {
            format: ENCRYPTED_PREFERENCES_FORMAT.to_string(),
            version: ENCRYPTED_PREFERENCES_VERSION,
            algorithm: ENCRYPTION_ALGORITHM.to_string(),
            keyId: self.keyId.clone(),
            nonce: STANDARD_NO_PAD.encode(nonce),
            ciphertext: STANDARD_NO_PAD.encode(ciphertext),
        };
        Ok(serde_json::to_vec_pretty(&envelope)?)
    }

    /// Decrypts preference bytes and verifies the storage-path authenticated data.
    pub fn decrypt(
        &self,
        storagePath: &str,
        content: &[u8],
    ) -> Result<Vec<u8>, PreferencesDataStoreError> {
        let envelope: EncryptedPreferencesEnvelope = serde_json::from_slice(content)?;
        if envelope.format != ENCRYPTED_PREFERENCES_FORMAT {
            return Err(PreferencesDataStoreError::Encryption(format!(
                "unexpected encrypted preferences format: {}",
                envelope.format
            )));
        }
        if envelope.version != ENCRYPTED_PREFERENCES_VERSION {
            return Err(PreferencesDataStoreError::Encryption(format!(
                "unsupported encrypted preferences version: {}",
                envelope.version
            )));
        }
        if envelope.algorithm != ENCRYPTION_ALGORITHM {
            return Err(PreferencesDataStoreError::Encryption(format!(
                "unsupported encrypted preferences algorithm: {}",
                envelope.algorithm
            )));
        }
        if envelope.keyId != self.keyId {
            return Err(PreferencesDataStoreError::Encryption(
                "encrypted preferences key id mismatch".to_string(),
            ));
        }
        let nonceBytes = STANDARD_NO_PAD
            .decode(envelope.nonce.as_bytes())
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
        let nonce: [u8; NONCE_LENGTH] = nonceBytes.try_into().map_err(|_| {
            PreferencesDataStoreError::Encryption("invalid nonce length".to_string())
        })?;
        let ciphertext = STANDARD_NO_PAD
            .decode(envelope.ciphertext.as_bytes())
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
        let cipher = XChaCha20Poly1305::new_from_slice(&self.key)
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
        cipher
            .decrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &ciphertext,
                    aad: storagePath.as_bytes(),
                },
            )
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))
    }
}

/// Decodes a stored base64 key into the fixed key byte array.
#[allow(non_snake_case)]
fn decodeStoredKey(value: &str) -> Result<[u8; KEY_LENGTH], PreferencesDataStoreError> {
    let keyBytes = STANDARD_NO_PAD
        .decode(value.as_bytes())
        .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
    keyBytes
        .try_into()
        .map_err(|_| PreferencesDataStoreError::Encryption("invalid key length".to_string()))
}

#[cfg(test)]
pub(crate) mod tests {
    pub(crate) const ENCRYPTION_HOST_SECRET_KEY_FOR_TEST: &str = super::ENCRYPTION_HOST_SECRET_KEY;

    /// Loads or creates preferences encryption with an explicit secret store for tests.
    pub(crate) fn loadOrCreateWithSecretStoreForTest(
        storageHost: &dyn operit_host_api::RuntimeStorageHost,
        secretStore: Option<&dyn operit_host_api::HostSecretStore>,
    ) -> Result<super::PreferencesEncryption, crate::PreferencesDataStore::PreferencesDataStoreError>
    {
        super::PreferencesEncryption::loadOrCreateWithSecretStore(storageHost, secretStore)
    }
}
