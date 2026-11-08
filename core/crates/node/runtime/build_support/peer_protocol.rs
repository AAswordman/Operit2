//! Conservative source fingerprint plus the generated, server-owned route catalog.
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::{Path, PathBuf}};

fn collect(path: &Path, files: &mut BTreeSet<PathBuf>) {
    if path.is_dir() {
        println!("cargo:rerun-if-changed={}", path.display());
        for entry in fs::read_dir(path).expect("read protocol source directory") {
            let path = entry.expect("protocol source entry").path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name.to_ascii_lowercase().ends_with("tests") || name.to_ascii_lowercase().ends_with("tests.rs") || name.ends_with("_test.rs") { continue; }
            collect(&path, files);
        }
    } else if path.is_file() {
        if path.extension().is_some_and(|extension| extension == "rs") { files.insert(path.to_owned()); }
    } else {
        panic!("Missing or unsupported protocol source: {}", path.display());
    }
}

fn part(hash: &mut Sha256, name: &str, contents: &[u8]) {
    hash.update((name.len() as u64).to_be_bytes());
    hash.update(name.as_bytes());
    hash.update((contents.len() as u64).to_be_bytes());
    hash.update(contents);
}

pub fn validate_revisions(revisions: &serde_json::Value, hash: &str) -> Result<u32, String> {
    let revisions = revisions.as_array().ok_or("Protocol revisions must be an array")?;
    let mut last = 0;
    let mut hashes = BTreeSet::new();
    for revision in revisions {
        let version = revision["version"].as_u64().filter(|v| *v <= u32::MAX as u64)
            .ok_or("Protocol version must be a positive u32")? as u32;
        let fingerprint = revision["hash"].as_str().ok_or("Protocol hash must be a string")?;
        if version <= last || fingerprint.len() != 64 || !fingerprint.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || !hashes.insert(fingerprint) {
            return Err("Protocol revisions must append increasing versions with unique SHA-256 hashes".into());
        }
        last = version;
    }
    if last == 0 || revisions.last().unwrap()["hash"].as_str() != Some(hash) {
        return Err(format!("Peer protocol changed. Append a revision with version greater than {last} and hash {hash} to core/peer-protocol.json. Do not replace a published revision."));
    }
    Ok(last)
}

/// Serialized DTO fields and custom codecs are part of the wire contract even
/// when a route's Rust signature itself has not changed.
fn wire_schema(text: &str) -> Result<String, syn::Error> {
    use quote::ToTokens;
    fn collect_items(items: Vec<syn::Item>, shapes: &mut Vec<String>) {
        for item in items {
            let attributes = match &item {
                syn::Item::Struct(item) => Some(&item.attrs),
                syn::Item::Enum(item) => Some(&item.attrs),
                _ => None,
            };
            let serialized = attributes.is_some_and(|attributes| attributes.iter().any(|attribute| {
                let tokens = attribute.meta.to_token_stream().to_string();
                attribute.path().is_ident("derive") && (tokens.contains("Serialize") || tokens.contains("Deserialize"))
            }));
            let custom_codec = matches!(&item, syn::Item::Impl(item) if item.trait_.as_ref().is_some_and(|(_, path, _)|
                path.segments.last().is_some_and(|segment| segment.ident == "Serialize" || segment.ident == "Deserialize")));
            if serialized || custom_codec || matches!(&item, syn::Item::Type(_)) {
                shapes.push(item.to_token_stream().to_string());
            } else if let syn::Item::Mod(module) = item {
                if let Some((_, items)) = module.content { collect_items(items, shapes); }
            }
        }
    }
    let mut shapes = Vec::new();
    collect_items(syn::parse_file(text)?.items, &mut shapes);
    Ok(shapes.join("\n"))
}

pub fn generate(manifest_dir: &Path, out: &Path, routes: &str) {
    let root = manifest_dir.join("../../../..").canonicalize().expect("repository root");
    let manifest = root.join("core/peer-protocol.json");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let config: serde_json::Value = serde_json::from_slice(&fs::read(&manifest).expect("read peer protocol manifest")).expect("decode peer protocol manifest");
    let mut files = BTreeSet::new();
    for source in config["sources"].as_array().expect("protocol source list") {
        let source = source.as_str().expect("protocol source path");
        let path = Path::new(source);
        assert!(!path.is_absolute() && path.components().all(|c| matches!(c, std::path::Component::Normal(_))), "Protocol sources must be repository-relative paths");
        collect(&root.join(path), &mut files);
    }
    assert!(!files.is_empty(), "Protocol fingerprint must cover source files");
    let mut hash = Sha256::new();
    for file in files {
        println!("cargo:rerun-if-changed={}", file.display());
        let name = file.strip_prefix(&root).unwrap().to_str().unwrap().replace('\\', "/");
        let text = fs::read_to_string(&file).expect("read protocol source").replace("\r\n", "\n");
        part(&mut hash, &name, text.as_bytes());
    }
    let mut schema_files = BTreeSet::new();
    for source in config["schemaRoots"].as_array().expect("wire schema source roots") {
        let path = Path::new(source.as_str().expect("wire schema path"));
        assert!(!path.is_absolute() && path.components().all(|c| matches!(c, std::path::Component::Normal(_))), "Schema roots must be repository-relative");
        collect(&root.join(path), &mut schema_files);
    }
    for file in schema_files {
        println!("cargo:rerun-if-changed={}", file.display());
        let name = file.strip_prefix(&root).unwrap().to_str().unwrap().replace('\\', "/");
        let schema = wire_schema(&fs::read_to_string(&file).expect("read wire schema source").replace("\r\n", "\n"))
            .unwrap_or_else(|error| panic!("parse wire schema {name}: {error}"));
        if !schema.is_empty() { part(&mut hash, &format!("wire-schema:{name}"), schema.as_bytes()); }
    }
    part(&mut hash, "generated-route-catalog", routes.as_bytes());
    let hash = format!("{:x}", hash.finalize());
    let version = validate_revisions(&config["revisions"], &hash).unwrap_or_else(|error| panic!("{error}"));
    fs::write(out.join("peer_protocol_identity.rs"), format!(
        "pub const VERSION: u32 = {version};\n")).expect("write peer protocol identity");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn dto_field_changes_and_custom_codecs_are_protocol_changes() {
        let first = "#[derive(serde::Serialize)] struct Request { route: String }";
        let changed = "#[derive(serde::Serialize)] struct Request { target: String }";
        assert_ne!(wire_schema(first).unwrap(), wire_schema(changed).unwrap());
        assert_ne!(wire_schema("pub type Version = u32;").unwrap(), wire_schema("pub type Version = u64;").unwrap());
        assert_ne!(wire_schema("impl Serialize for Packet { fn serialize() { old(); } }").unwrap(),
            wire_schema("impl Serialize for Packet { fn serialize() { new(); } }").unwrap());
    }
    #[test]
    fn software_only_implementation_edits_do_not_change_wire_shapes() {
        let first = "#[derive(Serialize)] struct Request { route: String } impl Request { fn render() { old(); } }";
        let changed = "#[derive(Serialize)] struct Request { route: String } impl Request { fn render() { new(); } }";
        assert_eq!(wire_schema(first).unwrap(), wire_schema(changed).unwrap());
    }
    #[test]
    fn changed_hash_requires_a_new_increasing_revision() {
        let first = "a".repeat(64);
        let next = "b".repeat(64);
        assert_eq!(validate_revisions(&json!([{ "version": 1, "hash": first }]), &first).unwrap(), 1);
        assert!(validate_revisions(&json!([{ "version": 1, "hash": first }]), &next).is_err());
        assert!(validate_revisions(&json!([{ "version": 1, "hash": first }, { "version": 1, "hash": next }]), &next).is_err());
        assert_eq!(validate_revisions(&json!([{ "version": 1, "hash": first }, { "version": 2, "hash": next }]), &next).unwrap(), 2);
    }
    #[test]
    fn routing_and_sync_content_and_paths_are_fingerprinted_without_ambiguity() {
        let digest = |parts: &[(&str, &[u8])]| {
            let mut hash = Sha256::new();
            for (name, data) in parts { part(&mut hash, name, data); }
            format!("{:x}", hash.finalize())
        };
        assert_ne!(digest(&[("sync", b"v1"), ("routes", b"target")]), digest(&[("sync", b"v1"), ("routes", b"binding")]));
        assert_ne!(digest(&[("sync", b"v1")]), digest(&[("sync", b"v2")]));
        assert_ne!(digest(&[("ab", b"c")]), digest(&[("a", b"bc")]));
    }
}
