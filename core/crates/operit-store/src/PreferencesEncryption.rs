use base64::engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD};
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use operit_host_api::RuntimeStorageHost;
use serde::{Deserialize, Serialize};

use crate::PreferencesDataStore::PreferencesDataStoreError;

const ENCRYPTION_KEY_PATH: &str = "secure/preferences_encryption_key.json";
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
        if storageHost.exists(ENCRYPTION_KEY_PATH)? {
            return Self::load(storageHost);
        }
        Self::create(storageHost)
    }

    fn load(storageHost: &dyn RuntimeStorageHost) -> Result<Self, PreferencesDataStoreError> {
        let content = String::from_utf8(storageHost.readBytes(ENCRYPTION_KEY_PATH)?)
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

    fn create(storageHost: &dyn RuntimeStorageHost) -> Result<Self, PreferencesDataStoreError> {
        let mut key = [0u8; KEY_LENGTH];
        getrandom::getrandom(&mut key)
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
        let mut keyIdBytes = [0u8; 16];
        getrandom::getrandom(&mut keyIdBytes)
            .map_err(|error| PreferencesDataStoreError::Encryption(error.to_string()))?;
        let stored = StoredPreferencesEncryptionKey {
            format: ENCRYPTION_KEY_FORMAT.to_string(),
            version: ENCRYPTION_KEY_VERSION,
            algorithm: ENCRYPTION_ALGORITHM.to_string(),
            keyId: URL_SAFE_NO_PAD.encode(keyIdBytes),
            key: STANDARD_NO_PAD.encode(key),
        };
        let content = serde_json::to_vec_pretty(&stored)?;
        storageHost.writeBytes(ENCRYPTION_KEY_PATH, &content)?;
        Ok(Self {
            keyId: stored.keyId,
            key,
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
