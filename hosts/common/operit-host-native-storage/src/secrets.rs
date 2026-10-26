//! File-backed secrets for explicitly configured native host profiles.
use operit_host_api::{HostError, HostResult, HostSecretStore};
use std::{fs, io::Write, path::PathBuf};
/// Stores secrets below a dedicated profile directory.
/// Unix permissions are private; other platforms inherit the profile directory ACL.
pub struct NativeFileSecretStore(pub PathBuf);
impl NativeFileSecretStore {
    fn path(&self, key: &str) -> HostResult<PathBuf> {
        if matches!(key, "" | "." | "..")
            || key.len() > 160
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err(HostError::new("Invalid secret key"));
        }
        Ok(self.0.join(key))
    }
}
impl HostSecretStore for NativeFileSecretStore {
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        let path = self.path(key)?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                fs::read(path).map(Some).map_err(Into::into)
            }
            Ok(_) => Err(HostError::new("Secret is not a regular file")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        let path = self.path(key)?;
        fs::create_dir_all(&self.0)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700))?;
        }
        let staging = self.0.join(format!(".{}", uuid::Uuid::new_v4()));
        let result = (|| -> HostResult<()> {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&staging)?;
            file.write_all(content)?;
            file.sync_all()?;
            drop(file);
            super::atomicReplace(&staging, &path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(staging);
        }
        result
    }
    fn deleteSecret(&self, key: &str) -> HostResult<()> {
        match fs::remove_file(self.path(key)?) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn isolated_secrets_persist_with_private_permissions_and_reject_paths() {
        let root =
            std::env::temp_dir().join(format!("operit-file-secrets-{}", uuid::Uuid::new_v4()));
        let store = NativeFileSecretStore(root.clone());
        assert_eq!(store.readSecret("key").unwrap(), None);
        store.writeSecret("key", b"temporary-test-secret").unwrap();
        assert_eq!(
            NativeFileSecretStore(root.clone())
                .readSecret("key")
                .unwrap()
                .unwrap(),
            b"temporary-test-secret"
        );
        store.writeSecret("key", b"replacement").unwrap();
        assert_eq!(
            NativeFileSecretStore(root.clone())
                .readSecret("key")
                .unwrap(),
            Some(b"replacement".to_vec())
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&root).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(root.join("key")).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        for key in ["", ".", "..", "../outside", "a/b", "a\\b"] {
            assert!(store.writeSecret(key, b"no").is_err());
        }
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        store.deleteSecret("key").unwrap();
        assert!(store.readSecret("key").unwrap().is_none());
        store.deleteSecret("key").unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
