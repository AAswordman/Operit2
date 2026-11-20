//! Route fingerprint supporting the single published peer protocol version.
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

/// Hashes route signatures and the authenticated session wire contract.
pub fn fingerprint(routes: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"operit-peer-route-signatures-v1\0");
    hash.update(b"operit-peer-session-fragments-v1\0");
    hash.update(routes.as_bytes());
    format!("{:x}", hash.finalize())
}

/// Requires the current route fingerprint to match the last published revision.
pub fn validate_revisions(revisions: &serde_json::Value, hash: &str) -> Result<u32, String> {
    let revisions = revisions.as_array().ok_or("Protocol revisions must be an array")?;
    let mut last = 0;
    for revision in revisions {
        let version = revision["version"].as_u64().filter(|v| *v <= u32::MAX as u64)
            .ok_or("Protocol version must be a positive u32")? as u32;
        let fingerprint = revision["hash"].as_str().ok_or("Protocol hash must be a string")?;
        if version <= last || fingerprint.len() != 64 || !fingerprint.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
            return Err("Protocol revisions must append increasing versions with lowercase SHA-256 hashes".into());
        }
        last = version;
    }
    if last == 0 || revisions.last().unwrap()["hash"].as_str() != Some(hash) {
        return Err(format!("Peer protocol changed. Append a revision with version greater than {last} and hash {hash} to core/peer-protocol.json. Do not replace a published revision."));
    }
    Ok(last)
}

/// Validates the route revision ledger and emits its runtime version constant.
pub fn generate(manifest_dir: &Path, out: &Path, routes: &str) {
    let root = manifest_dir.join("../../../..").canonicalize().expect("repository root");
    let manifest = root.join("core/peer-protocol.json");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let config: serde_json::Value = serde_json::from_slice(&fs::read(&manifest).expect("read peer protocol manifest")).expect("decode peer protocol manifest");
    let hash = fingerprint(routes);
    let version = validate_revisions(&config["revisions"], &hash).unwrap_or_else(|error| panic!("{error}"));
    fs::write(out.join("peer_protocol_identity.rs"), format!(
        "pub const VERSION: u32 = {version};\n")).expect("write peer protocol identity");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    /// Ensures that published versions remain append-only after narrowing coverage.
    #[test]
    fn changed_hash_requires_a_new_increasing_revision() {
        let first = "a".repeat(64);
        let next = "b".repeat(64);
        assert_eq!(validate_revisions(&json!([{ "version": 1, "hash": first }]), &first).unwrap(), 1);
        assert!(validate_revisions(&json!([{ "version": 1, "hash": first }]), &next).is_err());
        assert!(validate_revisions(&json!([{ "version": 1, "hash": first }, { "version": 1, "hash": next }]), &next).is_err());
        assert_eq!(validate_revisions(&json!([{ "version": 1, "hash": first }, { "version": 2, "hash": next }]), &next).unwrap(), 2);
    }

    /// Semantic wire changes can advance compatibility without changing a route signature.
    #[test]
    fn semantic_changes_advance_the_same_protocol_version_ledger() {
        let hash = "a".repeat(64);
        assert_eq!(validate_revisions(&json!([
            { "version": 25, "hash": hash }, { "version": 26, "hash": hash }
        ]), &hash).unwrap(), 26);
    }
}
