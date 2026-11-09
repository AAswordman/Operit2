use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;
use zip::ZipArchive;

use operit_util::AppLogger::AppLogger;

use crate::data::archive::ArchiveSource::{ArchiveSource, ArchiveSourceReader};

const FORMAT_VERSION: i32 = 1;
pub(crate) const ENTRY_MANIFEST: &str = "manifest.json";
pub(crate) const ENTRY_MODEL_CONFIGS: &str = "payload/files/datastore/model_configs.preferences_pb";
pub(crate) const ENTRY_FUNCTIONAL_CONFIGS: &str =
    "payload/files/datastore/functional_configs.preferences_pb";
pub(crate) const ENTRY_DATASTORE_PREFIX: &str = "payload/files/datastore/";
pub(crate) const ENTRY_FILES_PREFIX: &str = "payload/files/";
pub(crate) const ENTRY_EXTERNAL_FILES_PREFIX: &str = "payload/external_files/";
const KEY_CONFIG_LIST: &str = "config_list";
const KEY_FUNCTION_CONFIG_MAPPING: &str = "function_config_mapping";
const MAX_ARCHIVE_ENTRY_COUNT: usize = 100_000;
const MAX_DATASTORE_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const ARCHIVE_ENTRY_COPY_BUFFER_BYTES: usize = 256 * 1024;

/// Describes one validated file entry stored by an Operit1 snapshot archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Operit1SnapshotEntry {
    pub(crate) uncompressedSize: u64,
    pub(crate) compressedSize: u64,
}

/// Stores the legacy Operit1 manifest fields required by import planning.
#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
pub(crate) struct Operit1SnapshotManifest {
    pub(crate) formatVersion: i32,
    pub(crate) packageName: String,
    pub(crate) createdAt: i64,
}

/// Stores one model configuration JSON value identified by its legacy ID.
#[derive(Clone, Debug)]
pub(crate) struct Operit1SnapshotModelConfigJson {
    pub(crate) id: String,
    pub(crate) value: Value,
}

/// Stores decoded Operit1 metadata and a reusable validated ZIP directory.
pub(crate) struct Operit1SnapshotArchive {
    archive: ZipArchive<ArchiveSourceReader>,
    pub(crate) manifest: Operit1SnapshotManifest,
    pub(crate) entries: BTreeMap<String, Operit1SnapshotEntry>,
    pub(crate) datastorePreferences: BTreeMap<String, HashMap<String, Operit1PreferenceValue>>,
    pub(crate) modelConfigJsons: Vec<Operit1SnapshotModelConfigJson>,
    pub(crate) chatMappingJson: Value,
}

impl Operit1SnapshotArchive {
    /// Parses and validates an Operit1 snapshot from one range-readable source.
    pub(crate) fn fromSource(source: Arc<dyn ArchiveSource>) -> Result<Self, String> {
        let mut archive = Self::openZip(source)?;
        if archive.len() > MAX_ARCHIVE_ENTRY_COUNT {
            return Err(format!(
                "Operit1 snapshot contains more than {MAX_ARCHIVE_ENTRY_COUNT} entries"
            ));
        }
        let mut entries = BTreeMap::new();
        let mut datastorePreferences = BTreeMap::new();
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
            if file.is_dir() {
                continue;
            }
            let name = file.name().to_string();
            if let Err(error) = validateSnapshotEntryPath(&name) {
                AppLogger::w(
                    "Operit1SnapshotImport",
                    &format!("Skipping snapshot entry: {error}"),
                );
                continue;
            }
            let entry = Operit1SnapshotEntry {
                uncompressedSize: file.size(),
                compressedSize: file.compressed_size(),
            };
            if isDataStoreEntry(&name) {
                if file.size() > MAX_DATASTORE_ENTRY_BYTES {
                    return Err(format!(
                        "Operit1 DataStore entry exceeds the {MAX_DATASTORE_ENTRY_BYTES}-byte limit: {name}"
                    ));
                }
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)
                    .map_err(|error| error.to_string())?;
                datastorePreferences.insert(name.clone(), decodeDataStorePreferences(&bytes)?);
            }
            if entries.insert(name.clone(), entry).is_some() {
                return Err(format!(
                    "Operit1 snapshot contains a duplicate entry: {name}"
                ));
            }
        }
        let manifest: Operit1SnapshotManifest =
            serde_json::from_slice(&Self::readEntryFromZip(&mut archive, ENTRY_MANIFEST)?)
                .map_err(|error| format!("Operit1 snapshot manifest is invalid: {error}"))?;
        if manifest.formatVersion != FORMAT_VERSION {
            return Err(format!(
                "Unsupported Operit1 snapshot formatVersion: {}",
                manifest.formatVersion
            ));
        }
        let modelPreferences = datastorePreferences
            .get(ENTRY_MODEL_CONFIGS)
            .ok_or_else(|| "Operit1 snapshot is missing model configurations".to_string())?;
        let configIds: Vec<String> = serde_json::from_str(requiredPreferenceString(
            modelPreferences,
            KEY_CONFIG_LIST,
            "Operit1 snapshot is missing the model configuration list",
        )?)
        .map_err(|error| format!("Operit1 model configuration list is invalid: {error}"))?;
        if configIds.is_empty() {
            return Err("Operit1 snapshot model configuration list is empty".to_string());
        }
        let mut modelConfigJsons = Vec::new();
        for configId in configIds {
            let key = format!("config_{configId}");
            let value = serde_json::from_str(requiredPreferenceString(
                modelPreferences,
                &key,
                &format!("Operit1 snapshot is missing model configuration: {configId}"),
            )?)
            .map_err(|error| format!("Operit1 model configuration is invalid: {error}"))?;
            modelConfigJsons.push(Operit1SnapshotModelConfigJson {
                id: configId,
                value,
            });
        }
        let functionalPreferences = datastorePreferences
            .get(ENTRY_FUNCTIONAL_CONFIGS)
            .ok_or_else(|| "Operit1 snapshot is missing functional configurations".to_string())?;
        let functionalMapping: Value = serde_json::from_str(requiredPreferenceString(
            functionalPreferences,
            KEY_FUNCTION_CONFIG_MAPPING,
            "Operit1 snapshot is missing function model mappings",
        )?)
        .map_err(|error| format!("Operit1 function model mappings are invalid: {error}"))?;
        let chatMappingJson = functionalMapping
            .get("CHAT")
            .cloned()
            .ok_or_else(|| "Operit1 function model mappings are missing CHAT".to_string())?;
        Ok(Self {
            archive,
            manifest,
            entries,
            datastorePreferences,
            modelConfigJsons,
            chatMappingJson,
        })
    }

    /// Returns an independently seekable ZIP archive over the immutable source.
    fn openZip(source: Arc<dyn ArchiveSource>) -> Result<ZipArchive<ArchiveSourceReader>, String> {
        ZipArchive::new(ArchiveSourceReader::new(source)).map_err(|error| error.to_string())
    }

    /// Reads one named non-directory ZIP entry from an opened archive.
    fn readEntryFromZip(
        archive: &mut ZipArchive<ArchiveSourceReader>,
        name: &str,
    ) -> Result<Vec<u8>, String> {
        let mut file = archive
            .by_name(name)
            .map_err(|_| format!("Operit1 snapshot is missing required entry: {name}"))?;
        if file.is_dir() {
            return Err(format!("Operit1 snapshot entry is a directory: {name}"));
        }
        if name == ENTRY_MANIFEST && file.size() > MAX_MANIFEST_BYTES {
            return Err(format!(
                "Operit1 snapshot manifest exceeds the {MAX_MANIFEST_BYTES}-byte limit"
            ));
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        Ok(bytes)
    }

    /// Returns whether the archive contains one validated file entry.
    pub(crate) fn hasEntry(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Copies one named entry to a caller-owned writer without retaining its full payload.
    pub(crate) fn copyEntryTo<W: Write>(&self, name: &str, writer: &mut W) -> Result<(), String> {
        if !self.hasEntry(name) {
            return Err(format!(
                "Operit1 snapshot is missing required entry: {name}"
            ));
        }
        let mut archive = self.archive.clone();
        let mut entry = archive.by_name(name).map_err(|error| error.to_string())?;
        copyArchiveReaderToWriter(&mut entry, writer)?;
        Ok(())
    }

    /// Copies multiple named entries through one open ZIP directory.
    pub(crate) fn copyEntriesTo<F>(&self, names: &[String], mut copyEntry: F) -> Result<(), String>
    where
        F: FnMut(usize, &str, &mut dyn Read) -> Result<(), String>,
    {
        let mut archive = self.archive.clone();
        for (index, name) in names.iter().enumerate() {
            if !self.hasEntry(name) {
                return Err(format!(
                    "Operit1 snapshot is missing required entry: {name}"
                ));
            }
            let mut entry = archive.by_name(name).map_err(|error| error.to_string())?;
            copyEntry(index, name, &mut entry)?;
        }
        Ok(())
    }
}

/// Copies one archive reader into a writer with a bounded transfer buffer.
fn copyArchiveReaderToWriter<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
) -> Result<(), String> {
    let mut buffer = vec![0; ARCHIVE_ENTRY_COPY_BUFFER_BYTES];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if count == 0 {
            return Ok(());
        }
        writer
            .write_all(&buffer[..count])
            .map_err(|error| error.to_string())?;
    }
}

/// Represents one value decoded from the legacy DataStore protobuf payload.

include!("Operit1DataStorePreferences.rs");

#[cfg(test)]
mod archive_tests {
    use super::*;

    use std::io::Cursor;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Counts range reads over an immutable snapshot used to verify ZIP directory reuse.
    struct CountingSnapshotSource {
        bytes: Vec<u8>,
        reads: AtomicUsize,
    }

    impl ArchiveSource for CountingSnapshotSource {
        /// Returns the fixture snapshot's immutable length.
        fn len(&self) -> Result<u64, String> {
            Ok(self.bytes.len() as u64)
        }

        /// Reads the requested range and records each host-boundary call.
        fn readAt(&self, offset: u64, length: usize) -> Result<Vec<u8>, String> {
            self.reads.fetch_add(1, Ordering::Relaxed);
            let start = offset as usize;
            Ok(self.bytes[start..(start + length).min(self.bytes.len())].to_vec())
        }
    }

    /// Encodes one length-delimited protobuf field for independent AndroidX fixtures.
    fn appendField(bytes: &mut Vec<u8>, tag: u8, value: &[u8]) {
        bytes.push(tag);
        let mut length = value.len();
        while length >= 128 {
            bytes.push((length as u8 & 127) | 128);
            length >>= 7;
        }
        bytes.push(length as u8);
        bytes.extend_from_slice(value);
    }

    /// Encodes string preferences using the actual AndroidX PreferenceMap field numbers.
    fn stringPreferences(values: &[(&str, &str)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (key, value) in values {
            let mut encodedValue = Vec::new();
            appendField(&mut encodedValue, 0x2a, value.as_bytes());
            let mut entry = Vec::new();
            appendField(&mut entry, 0x0a, key.as_bytes());
            appendField(&mut entry, 0x12, &encodedValue);
            appendField(&mut bytes, 0x0a, &entry);
        }
        bytes
    }

    /// Builds a valid small snapshot containing one caller-specified resource path.
    fn snapshotFixture(resourcePath: &str) -> Arc<CountingSnapshotSource> {
        snapshotFixtureWithResources(&[(resourcePath, b"test resource")])
    }

    /// Builds a valid snapshot with independently supplied resource names and contents.
    fn snapshotFixtureWithResources(resources: &[(&str, &[u8])]) -> Arc<CountingSnapshotSource> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in [
            (
                ENTRY_MANIFEST,
                br#"{"formatVersion":1,"packageName":"test.operit","createdAt":1}"#.to_vec(),
            ),
            (
                ENTRY_MODEL_CONFIGS,
                stringPreferences(&[("config_list", "[\"test\"]"), ("config_test", "{}")]),
            ),
            (
                ENTRY_FUNCTIONAL_CONFIGS,
                stringPreferences(&[("function_config_mapping", "{\"CHAT\":{}}")]),
            ),
        ] {
            writer.start_file(name, options).unwrap();
            writer.write_all(&bytes).unwrap();
        }
        for (name, bytes) in resources {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        Arc::new(CountingSnapshotSource {
            bytes: writer.finish().unwrap().into_inner(),
            reads: AtomicUsize::new(0),
        })
    }

    /// Reuses the parsed ZIP directory and cached bytes for repeated validated entry reads.
    #[test]
    fn entry_copies_reuse_the_validated_zip_directory() {
        let entryName = "payload/files/resource.txt";
        let source = snapshotFixture(entryName);
        let archive = Operit1SnapshotArchive::fromSource(source.clone()).unwrap();
        let initialReads = source.reads.load(Ordering::Relaxed);
        for _ in 0..10 {
            let mut bytes = Vec::new();
            archive.copyEntryTo(entryName, &mut bytes).unwrap();
            assert_eq!(bytes, b"test resource");
        }
        let mut copied = 0;
        archive
            .copyEntriesTo(&[entryName.to_string()], |index, name, reader| {
                assert_eq!(index, 0);
                assert_eq!(name, entryName);
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes).unwrap();
                assert_eq!(bytes, b"test resource");
                copied += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(copied, 1);
        assert_eq!(source.reads.load(Ordering::Relaxed), initialReads);
        assert!(archive
            .copyEntryTo("payload/files/missing.txt", &mut Vec::new())
            .is_err());
    }

    /// Excludes unsafe resource entries while retaining metadata and safe resource copies.
    #[test]
    fn invalid_resource_paths_do_not_abort_snapshot_inspection() {
        let invalidPaths = [
            r#"payload/external_files/tide_bpm_now.json,{"bpm":80,"ts":1789306692035}"#,
            "payload/files/../invalid.txt",
            "payload/files//invalid.txt",
            "/payload/files/absolute.txt",
            "C:/payload/files/drive.txt",
        ];
        for invalidPath in invalidPaths {
            let validPath = "payload/external_files/document.txt";
            let source = snapshotFixtureWithResources(&[
                (invalidPath, b"excluded resource"),
                (validPath, b"preserved resource"),
            ]);
            let archive = Operit1SnapshotArchive::fromSource(source).unwrap();
            assert_eq!(archive.entries.len(), 4);
            assert_eq!(archive.modelConfigJsons.len(), 1);
            assert!(archive
                .datastorePreferences
                .contains_key(ENTRY_FUNCTIONAL_CONFIGS));
            assert!(!archive.hasEntry(invalidPath));
            assert!(archive.copyEntryTo(invalidPath, &mut Vec::new()).is_err());
            let mut bytes = Vec::new();
            archive.copyEntryTo(validPath, &mut bytes).unwrap();
            assert_eq!(bytes, b"preserved resource");
        }
    }

    /// Keeps JSON file contents separate from the ZIP entry name during inspection and copying.
    #[test]
    fn json_resource_contents_are_not_interpreted_as_paths() {
        let path = "payload/external_files/tide_bpm_now.json";
        let contents = br#"{"bpm":80,"ts":1789306692035}"#;
        let source = snapshotFixtureWithResources(&[(path, contents)]);
        let archive = Operit1SnapshotArchive::fromSource(source).unwrap();
        assert!(archive.hasEntry(path));
        let mut bytes = Vec::new();
        archive.copyEntryTo(path, &mut bytes).unwrap();
        assert_eq!(bytes, contents);
    }
}
