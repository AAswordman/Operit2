use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;
use zip::ZipArchive;

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

/// Stores decoded Operit1 archive metadata and a source for later entry reads.
pub(crate) struct Operit1SnapshotArchive {
    source: Arc<dyn ArchiveSource>,
    pub(crate) manifest: Operit1SnapshotManifest,
    pub(crate) entries: BTreeMap<String, Operit1SnapshotEntry>,
    pub(crate) datastorePreferences: BTreeMap<String, HashMap<String, Operit1PreferenceValue>>,
    pub(crate) modelConfigJsons: Vec<Operit1SnapshotModelConfigJson>,
    pub(crate) chatMappingJson: Value,
}

impl Operit1SnapshotArchive {
    /// Parses and validates an Operit1 snapshot from one range-readable source.
    pub(crate) fn fromSource(source: Arc<dyn ArchiveSource>) -> Result<Self, String> {
        let mut archive = Self::openZip(source.clone())?;
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
            validateSnapshotEntryPath(&name)?;
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
                return Err(format!("Operit1 snapshot contains a duplicate entry: {name}"));
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
            source,
            manifest,
            entries,
            datastorePreferences,
            modelConfigJsons,
            chatMappingJson,
        })
    }

    /// Returns an independently seekable ZIP archive over the immutable source.
    fn openZip(
        source: Arc<dyn ArchiveSource>,
    ) -> Result<ZipArchive<ArchiveSourceReader>, String> {
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
        let mut archive = Self::openZip(self.source.clone())?;
        let mut entry = archive.by_name(name).map_err(|error| error.to_string())?;
        copyArchiveReaderToWriter(&mut entry, writer)?;
        Ok(())
    }

    /// Copies multiple named entries through one open ZIP directory.
    pub(crate) fn copyEntriesTo<F>(&self, names: &[String], mut copyEntry: F) -> Result<(), String>
    where
        F: FnMut(usize, &str, &mut dyn Read) -> Result<(), String>,
    {
        let mut archive = Self::openZip(self.source.clone())?;
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
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Operit1PreferenceValue {
    Boolean(bool),
    Float(f32),
    Double(f64),
    Int(i32),
    String(String),
    StringSet(Vec<String>),
    Long(i64),
}

impl Operit1PreferenceValue {
    /// Returns the contained string when this preference uses the string variant.
    pub(crate) fn asString(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the contained string set when this preference uses that variant.
    pub(crate) fn asStringSet(&self) -> Option<&[String]> {
        match self {
            Self::StringSet(value) => Some(value),
            _ => None,
        }
    }
}

/// Checks whether one entry stores an Operit1 DataStore preference payload.
pub(crate) fn isDataStoreEntry(entry: &str) -> bool {
    entry.starts_with(ENTRY_DATASTORE_PREFIX) && entry.ends_with(".preferences_pb")
}

/// Validates a ZIP entry path before it can be indexed or copied.
pub(crate) fn validateSnapshotEntryPath(path: &str) -> Result<(), String> {
    if path.is_empty() || path.starts_with('/') || path.starts_with('\\') || path.contains('\\') {
        return Err(format!("Operit1 snapshot contains an invalid path: {path}"));
    }
    validateRelativePath(path)
}

/// Validates a path that must remain relative to one controlled output root.
pub(crate) fn validateRelativePath(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.split('/').any(|segment| {
            segment.is_empty() || segment == "." || segment == ".." || segment.contains(':')
        })
    {
        return Err(format!(
            "Operit1 snapshot contains an invalid relative path: {path}"
        ));
    }
    Ok(())
}

/// Returns a required string-valued preference with a caller-specific missing message.
fn requiredPreferenceString<'a>(
    preferences: &'a HashMap<String, Operit1PreferenceValue>,
    key: &str,
    missingMessage: &str,
) -> Result<&'a str, String> {
    preferences
        .get(key)
        .ok_or_else(|| missingMessage.to_string())?
        .asString()
        .ok_or_else(|| format!("Operit1 DataStore key is not a string: {key}"))
}

/// Decodes one legacy AndroidX DataStore preferences protobuf payload.
fn decodeDataStorePreferences(
    bytes: &[u8],
) -> Result<HashMap<String, Operit1PreferenceValue>, String> {
    let mut decoder = ProtoDecoder::new(bytes);
    let mut preferences = HashMap::new();
    while !decoder.isComplete() {
        let (fieldNumber, wireType) = decoder.readTag()?;
        if fieldNumber != 1 || wireType != 2 {
            return Err(format!(
                "Operit1 DataStore preferences contain an unknown field: {fieldNumber}/{wireType}"
            ));
        }
        let entryBytes = decoder.readLengthDelimited()?;
        if let Some((key, value)) = decodePreferenceEntry(entryBytes)? {
            preferences.insert(key, value);
        }
    }
    Ok(preferences)
}

/// Decodes one key/value entry from a legacy DataStore preferences protobuf payload.
fn decodePreferenceEntry(bytes: &[u8]) -> Result<Option<(String, Operit1PreferenceValue)>, String> {
    let mut decoder = ProtoDecoder::new(bytes);
    let mut key = None;
    let mut value = None;
    while !decoder.isComplete() {
        let (fieldNumber, wireType) = decoder.readTag()?;
        match (fieldNumber, wireType) {
            (1, 2) => key = Some(decoder.readString()?),
            (2, 2) => value = decodePreferenceValue(decoder.readLengthDelimited()?)?,
            _ => decoder.skipField(wireType)?,
        }
    }
    let key =
        key.ok_or_else(|| "Operit1 DataStore preference entry is missing its key".to_string())?;
    Ok(value.map(|value| (key, value)))
}

/// Decodes one typed value from a legacy DataStore preferences protobuf payload.
fn decodePreferenceValue(bytes: &[u8]) -> Result<Option<Operit1PreferenceValue>, String> {
    let mut decoder = ProtoDecoder::new(bytes);
    let mut value = None;
    while !decoder.isComplete() {
        let (fieldNumber, wireType) = decoder.readTag()?;
        match (fieldNumber, wireType) {
            (1, 0) => value = Some(Operit1PreferenceValue::Boolean(decoder.readVarint()? != 0)),
            (2, 5) => {
                value = Some(Operit1PreferenceValue::Float(f32::from_le_bytes(
                    decoder.readFixed32()?.to_le_bytes(),
                )))
            }
            (3, 1) => {
                value = Some(Operit1PreferenceValue::Double(f64::from_le_bytes(
                    decoder.readFixed64()?.to_le_bytes(),
                )))
            }
            (4, 0) => {
                value = Some(Operit1PreferenceValue::Int(
                    decoder.readVarint()? as u32 as i32
                ))
            }
            (5, 2) => value = Some(Operit1PreferenceValue::String(decoder.readString()?)),
            (6, 2) => {
                value = Some(Operit1PreferenceValue::StringSet(
                    decodePreferenceStringSet(decoder.readLengthDelimited()?)?,
                ))
            }
            (7, 0) => value = Some(Operit1PreferenceValue::Long(decoder.readVarint()? as i64)),
            _ => decoder.skipField(wireType)?,
        }
    }
    Ok(value)
}

/// Decodes one legacy DataStore string-set payload.
fn decodePreferenceStringSet(bytes: &[u8]) -> Result<Vec<String>, String> {
    let mut decoder = ProtoDecoder::new(bytes);
    let mut values = Vec::new();
    while !decoder.isComplete() {
        let (fieldNumber, wireType) = decoder.readTag()?;
        match (fieldNumber, wireType) {
            (1, 2) => values.push(decoder.readString()?),
            _ => decoder.skipField(wireType)?,
        }
    }
    Ok(values)
}

/// Decodes the protobuf primitives used by the legacy DataStore preferences format.
struct ProtoDecoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> ProtoDecoder<'a> {
    /// Creates a decoder over one bounded protobuf buffer.
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    /// Reports whether every input byte has been consumed.
    fn isComplete(&self) -> bool {
        self.position == self.bytes.len()
    }

    /// Decodes one protobuf field tag.
    fn readTag(&mut self) -> Result<(u64, u64), String> {
        let tag = self.readVarint()?;
        Ok((tag >> 3, tag & 0x07))
    }

    /// Decodes one length-delimited protobuf field.
    fn readLengthDelimited(&mut self) -> Result<&'a [u8], String> {
        let length = self.readVarint()? as usize;
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| "Operit1 DataStore protobuf length overflowed".to_string())?;
        if end > self.bytes.len() {
            return Err("Operit1 DataStore protobuf is truncated".to_string());
        }
        let bytes = &self.bytes[self.position..end];
        self.position = end;
        Ok(bytes)
    }

    /// Decodes one UTF-8 protobuf string field.
    fn readString(&mut self) -> Result<String, String> {
        String::from_utf8(self.readLengthDelimited()?.to_vec()).map_err(|error| error.to_string())
    }

    /// Decodes one fixed-width 32-bit protobuf field.
    fn readFixed32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.readFixedBytes::<4>()?))
    }

    /// Decodes one fixed-width 64-bit protobuf field.
    fn readFixed64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.readFixedBytes::<8>()?))
    }

    /// Skips one unknown protobuf field using its wire type.
    fn skipField(&mut self, wireType: u64) -> Result<(), String> {
        match wireType {
            0 => {
                self.readVarint()?;
                Ok(())
            }
            1 => self.skipBytes(8),
            2 => {
                self.readLengthDelimited()?;
                Ok(())
            }
            5 => self.skipBytes(4),
            _ => Err(format!(
                "Operit1 DataStore protobuf has an unknown wire type: {wireType}"
            )),
        }
    }

    /// Advances by one bounded byte count.
    fn skipBytes(&mut self, count: usize) -> Result<(), String> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| "Operit1 DataStore protobuf length overflowed".to_string())?;
        if end > self.bytes.len() {
            return Err("Operit1 DataStore protobuf is truncated".to_string());
        }
        self.position = end;
        Ok(())
    }

    /// Decodes one fixed-size protobuf byte sequence.
    fn readFixedBytes<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self
            .position
            .checked_add(N)
            .ok_or_else(|| "Operit1 DataStore protobuf length overflowed".to_string())?;
        if end > self.bytes.len() {
            return Err("Operit1 DataStore protobuf is truncated".to_string());
        }
        let bytes = self.bytes[self.position..end]
            .try_into()
            .map_err(|_| "Operit1 DataStore protobuf fixed field is truncated".to_string())?;
        self.position = end;
        Ok(bytes)
    }

    /// Decodes one protobuf varint.
    fn readVarint(&mut self) -> Result<u64, String> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            if self.position >= self.bytes.len() {
                return Err("Operit1 DataStore protobuf varint is truncated".to_string());
            }
            let byte = self.bytes[self.position];
            self.position += 1;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err("Operit1 DataStore protobuf varint is invalid".to_string())
    }
}
