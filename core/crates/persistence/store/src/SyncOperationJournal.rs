//! The JSONL journal owns record locations and routing metadata, never payload trees.
//! Every reader uses bounded Host ranges; payloads are opened only by their owner.
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead, BufReader, Read};

use operit_host_api::RuntimeStorageHost;
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::RawValue;

use crate::PreferenceMerge::PLUGIN_MERGE_DECLARATION_KEY;
use crate::SyncOperationStore::{
    OperationLogLineFinding, SyncOperation, SyncOperationOrder, SyncOperationSemantics,
    SyncOperationStoreError,
};

pub(crate) const JOURNAL_READ_BYTES: usize = 64 * 1024;
const ENCRYPTED_FORMAT: &str = "operit.sync.encrypted_payload";

/// Adapts the portable Host range capability to a bounded sequential reader.
pub(crate) struct JournalReader<'a> {
    host: &'a dyn RuntimeStorageHost,
    path: &'a str,
    offset: u64,
    remaining: u64,
}

impl<'a> JournalReader<'a> {
    pub(crate) fn new(
        host: &'a dyn RuntimeStorageHost,
        path: &'a str,
        offset: u64,
        length: u64,
    ) -> Self {
        Self {
            host,
            path,
            offset,
            remaining: length,
        }
    }
}

impl Read for JournalReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let length = buffer
            .len()
            .min(JOURNAL_READ_BYTES)
            .min(self.remaining.min(usize::MAX as u64) as usize);
        if length == 0 {
            return Ok(0);
        }
        let bytes = self
            .host
            .readBytesRange(self.path, self.offset, length)
            .map_err(io::Error::other)?;
        if bytes.len() > length {
            return Err(io::Error::other(
                "Host returned more journal bytes than requested",
            ));
        }
        buffer[..bytes.len()].copy_from_slice(&bytes);
        self.offset += bytes.len() as u64;
        self.remaining -= bytes.len() as u64;
        Ok(bytes.len())
    }
}

/// Keeps line offsets identical for indexing, diagnostics, and reversible repair.
pub(crate) struct JournalLines<'a> {
    reader: BufReader<JournalReader<'a>>,
    pub(crate) offset: u64,
    pub(crate) line_number: u64,
    pub(crate) bytes: Vec<u8>,
}

impl<'a> JournalLines<'a> {
    pub(crate) fn new(host: &'a dyn RuntimeStorageHost, path: &'a str) -> Self {
        Self {
            reader: BufReader::with_capacity(
                JOURNAL_READ_BYTES,
                JournalReader::new(host, path, 0, u64::MAX),
            ),
            offset: 0,
            line_number: 0,
            bytes: Vec::new(),
        }
    }

    pub(crate) fn next(&mut self) -> Result<bool, SyncOperationStoreError> {
        self.offset += self.bytes.len() as u64;
        self.bytes.clear();
        let count = self.reader.read_until(b'\n', &mut self.bytes)?;
        if count == 0 {
            return Ok(false);
        }
        self.line_number += 1;
        Ok(true)
    }
}

/// Only these small payload fields participate in routing and conflict ordering.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct PayloadRoute {
    pub(crate) encrypted: bool,
    pub(crate) storage_path: Option<String>,
    pub(crate) merge_metadata: bool,
    contract_version: Option<u64>,
    logical_time: Option<i64>,
    initial: bool,
}

#[derive(Default)]
struct RouteFields<'a> {
    format: Option<&'a RawValue>,
    storage_path: Option<&'a RawValue>,
    key: Option<&'a RawValue>,
    entries: Option<&'a RawValue>,
    contract_version: Option<&'a RawValue>,
    logical_time: Option<&'a RawValue>,
    initial: Option<&'a RawValue>,
}

impl<'de> Deserialize<'de> for RouteFields<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = RouteFields<'de>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("payload routing fields")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut fields = RouteFields::default();
                while let Some(key) = map.next_key::<String>()? {
                    // Like Value, payload objects use their last duplicate key.
                    match key.as_str() {
                        "format" => fields.format = map.next_value()?,
                        "storagePath" => fields.storage_path = map.next_value()?,
                        "key" => fields.key = map.next_value()?,
                        "entries" => fields.entries = map.next_value()?,
                        "contractVersion" => fields.contract_version = map.next_value()?,
                        "logicalTime" => fields.logical_time = map.next_value()?,
                        "initial" => fields.initial = map.next_value()?,
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                Ok(fields)
            }
        }
        deserializer.deserialize_map(Fields)
    }
}

/// Validate Value's number/depth rules without allocating its object/array tree.
struct ValidatePayload;
impl<'de> DeserializeSeed<'de> for ValidatePayload {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        struct Value;
        impl<'de> Visitor<'de> for Value {
            type Value = ();
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a JSON value")
            }
            fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
                Ok(())
            }
            fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
                Ok(())
            }
            fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
                Ok(())
            }
            fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<(), E> {
                Ok(())
            }
            fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<(), E> {
                Ok(())
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
                Ok(())
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
                while seq.next_element_seed(ValidatePayload)?.is_some() {}
                Ok(())
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
                while map.next_key::<serde::de::IgnoredAny>()?.is_some() {
                    map.next_value_seed(ValidatePayload)?;
                }
                Ok(())
            }
        }
        deserializer.deserialize_any(Value)
    }
}

struct ValidateOperationPayload;
impl<'de> DeserializeSeed<'de> for ValidateOperationPayload {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        struct Operation;
        impl<'de> Visitor<'de> for Operation {
            type Value = ();
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a synchronization operation")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
                while let Some(key) = map.next_key::<String>()? {
                    if key == "payload" {
                        map.next_value_seed(ValidatePayload)?;
                    } else {
                        map.next_value::<serde::de::IgnoredAny>()?;
                    }
                }
                Ok(())
            }
        }
        deserializer.deserialize_map(Operation)
    }
}

fn string_field(raw: Option<&RawValue>) -> Option<String> {
    raw.and_then(|value| serde_json::from_str(value.get()).ok())
}

struct MergeEntries;
impl<'de> DeserializeSeed<'de> for MergeEntries {
    type Value = bool;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<bool, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = bool;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("preference transaction entries")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut entries: A) -> Result<bool, A::Error> {
                let mut found = false;
                while let Some(raw) = entries.next_element::<&RawValue>()? {
                    let fields =
                        serde_json::from_str::<RouteFields<'_>>(raw.get()).unwrap_or_default();
                    found |=
                        string_field(fields.key).as_deref() == Some(PLUGIN_MERGE_DECLARATION_KEY);
                }
                Ok(found)
            }
        }
        deserializer.deserialize_seq(Entries)
    }
}

impl PayloadRoute {
    fn parse(raw: &RawValue) -> Self {
        let fields = serde_json::from_str::<RouteFields<'_>>(raw.get()).unwrap_or_default();
        let merge_entries = fields.entries.is_some_and(|entries| {
            MergeEntries
                .deserialize(&mut serde_json::Deserializer::from_str(entries.get()))
                .unwrap_or(false)
        });
        Self {
            encrypted: string_field(fields.format).as_deref() == Some(ENCRYPTED_FORMAT),
            storage_path: string_field(fields.storage_path),
            merge_metadata: string_field(fields.key).as_deref()
                == Some(PLUGIN_MERGE_DECLARATION_KEY)
                || merge_entries,
            contract_version: fields
                .contract_version
                .and_then(|v| serde_json::from_str(v.get()).ok()),
            logical_time: fields
                .logical_time
                .and_then(|v| serde_json::from_str(v.get()).ok()),
            initial: fields.initial.is_some_and(|v| v.get() == "true"),
        }
    }

    fn from_operation(operation: &SyncOperation, encrypted: bool) -> Self {
        Self {
            encrypted,
            storage_path: operation
                .payload
                .get("storagePath")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            merge_metadata: crate::PreferenceMerge::isMergeMetadata(operation),
            contract_version: operation
                .payload
                .get("contractVersion")
                .and_then(|v| v.as_u64()),
            logical_time: operation
                .payload
                .get("logicalTime")
                .and_then(|v| v.as_i64()),
            initial: operation.payload.get("initial").and_then(|v| v.as_bool()) == Some(true),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordFields<'a> {
    op_id: String,
    origin_device_id: String,
    sequence: i64,
    domain: String,
    entity_type: String,
    entity_id: String,
    operation: String,
    semantics: SyncOperationSemantics,
    #[serde(borrow)]
    payload: &'a RawValue,
    created_at: i64,
    // Required even though the index does not need to retain it.
    #[serde(rename = "schemaVersion")]
    _schema_version: i32,
}

/// An immutable location; no JSON business value is retained in an index.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct JournalRecord {
    pub(crate) offset: u64,
    pub(crate) length: u64,
    pub(crate) op_id: String,
    pub(crate) origin_device_id: String,
    pub(crate) sequence: i64,
    pub(crate) domain: String,
    pub(crate) entity_type: String,
    pub(crate) entity_id: String,
    pub(crate) operation: String,
    pub(crate) semantics: SyncOperationSemantics,
    pub(crate) route: PayloadRoute,
    created_at: i64,
}

impl JournalRecord {
    pub(crate) fn parse(bytes: &[u8], offset: u64) -> Result<Self, serde_json::Error> {
        let fields: RecordFields<'_> = serde_json::from_slice(bytes)?;
        // Validate in the enclosing operation so payload recursion has exactly
        // the same limit as decoding SyncOperation, including its outer object.
        ValidateOperationPayload.deserialize(&mut serde_json::Deserializer::from_slice(bytes))?;
        Ok(Self {
            offset,
            length: bytes.len() as u64,
            op_id: fields.op_id,
            origin_device_id: fields.origin_device_id,
            sequence: fields.sequence,
            domain: fields.domain,
            entity_type: fields.entity_type,
            entity_id: fields.entity_id,
            operation: fields.operation,
            semantics: fields.semantics,
            route: PayloadRoute::parse(fields.payload),
            created_at: fields.created_at,
        })
    }

    pub(crate) fn order(&self) -> Result<SyncOperationOrder, SyncOperationStoreError> {
        let created_at = match (self.domain.as_str(), self.route.contract_version) {
            ("preferences", Some(1)) => self.route.logical_time.ok_or_else(|| {
                SyncOperationStoreError::Message(format!(
                    "Missing preference logical time op={}",
                    self.op_id
                ))
            })?,
            ("runtime_file", _) if self.route.initial => 1,
            _ => self.created_at,
        };
        Ok(SyncOperationOrder {
            createdAt: created_at,
            originDeviceId: self.origin_device_id.clone(),
            sequence: self.sequence,
        })
    }

    pub(crate) fn preference_path_matches(&self, path: &str) -> bool {
        if self.route.encrypted {
            serde_json::from_str::<Vec<serde_json::Value>>(&self.entity_id)
                .is_ok_and(|identity| identity.first().and_then(|v| v.as_str()) == Some(path))
        } else {
            self.route.storage_path.as_deref() == Some(path)
        }
    }

    pub(crate) fn is_merge_candidate(&self) -> bool {
        self.domain == "preferences"
            && (self.route.encrypted || self.operation == "alias" || self.route.merge_metadata)
    }
}

#[derive(Default)]
pub(crate) struct JournalIndex {
    pub(crate) loaded: bool,
    pub(crate) records: Vec<JournalRecord>,
    pub(crate) operation_ids: BTreeSet<String>,
    pub(crate) sequences: BTreeMap<i64, usize>,
    pub(crate) highest_sequence: i64,
    pub(crate) byte_length: u64,
    pub(crate) needs_separator: bool,
    pub(crate) findings: Vec<OperationLogLineFinding>,
}

impl JournalIndex {
    fn push(&mut self, record: JournalRecord) {
        self.highest_sequence = self.highest_sequence.max(record.sequence);
        self.operation_ids.insert(record.op_id.clone());
        self.sequences.insert(record.sequence, self.records.len());
        self.records.push(record);
    }

    pub(crate) fn appended(
        &mut self,
        operation: &SyncOperation,
        offset: u64,
        length: u64,
        encrypted: bool,
    ) {
        self.push(JournalRecord {
            offset,
            length,
            op_id: operation.opId.clone(),
            origin_device_id: operation.originDeviceId.clone(),
            sequence: operation.sequence,
            domain: operation.domain.clone(),
            entity_type: operation.entityType.clone(),
            entity_id: operation.entityId.clone(),
            operation: operation.operation.clone(),
            semantics: operation.semantics,
            route: PayloadRoute::from_operation(operation, encrypted),
            created_at: operation.createdAt,
        });
    }

    pub(crate) fn load(
        &mut self,
        host: &dyn RuntimeStorageHost,
        path: &str,
        device_id: &str,
    ) -> Result<(), SyncOperationStoreError> {
        let exists = host.exists(path)?;
        if self.loaded && (exists || self.byte_length == 0) {
            return Ok(());
        }
        // Publish a complete index only after all Host reads succeeded.
        let mut next = Self::default();
        if exists {
            let mut lines = JournalLines::new(host, path);
            while lines.next()? {
                next.byte_length = lines.offset + lines.bytes.len() as u64;
                next.needs_separator = !lines.bytes.ends_with(b"\n");
                if lines.bytes.iter().all(u8::is_ascii_whitespace) {
                    continue;
                }
                match JournalRecord::parse(&lines.bytes, lines.offset) {
                    Ok(record) => next.push(record),
                    Err(error) => next.findings.push(OperationLogLineFinding {
                        deviceId: device_id.to_owned(),
                        lineNumber: lines.line_number,
                        byteOffset: lines.offset,
                        byteLength: lines.bytes.len() as u64,
                        error: error.to_string(),
                    }),
                }
            }
        }
        next.loaded = true;
        *self = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn lightweight_routes_match_decoded_payload_semantics() {
        let declaration =
            serde_json::json!({"entries":[{"key":PLUGIN_MERGE_DECLARATION_KEY}]}).to_string();
        for payload in [
            declaration.as_str(),
            "null",
            "42",
            "[]",
            "true",
            "{\"storagePath\":null}",
            "{\"storagePath\":\"wrong\",\"storagePath\":\"right\"}",
            "{\"contractVersion\":1,\"logicalTime\":17}",
            "{\"initial\":true}",
            "{\"key\":null,\"entries\":[null,2,{\"key\":\"__operit_merge_declaration_v1\"}]}",
        ] {
            for domain in ["preferences", "runtime_file"] {
                let operation: SyncOperation = serde_json::from_value(json!({
                    "opId":"a:1", "originDeviceId":"a", "sequence":1, "domain":domain,
                    "entityType":"entry", "entityId":"id", "operation":"set", "semantics":"entity_state",
                    "payload":serde_json::from_str::<Value>(payload).unwrap(), "createdAt":99, "schemaVersion":1,
                })).unwrap();
                let raw = serde_json::to_vec(&operation).unwrap();
                let record = JournalRecord::parse(&raw, 0).unwrap();
                assert_eq!(
                    record.order().unwrap(),
                    SyncOperationOrder::fromOperation(&operation)
                );
                assert_eq!(
                    record.is_merge_candidate(),
                    crate::PreferenceMerge::isMergeMetadata(&operation)
                );
                let raw_route =
                    PayloadRoute::parse(serde_json::from_str::<&RawValue>(payload).unwrap());
                assert_eq!(
                    raw_route.storage_path.as_deref(),
                    operation.payload.get("storagePath").and_then(Value::as_str)
                );
            }
        }
    }

    #[test]
    fn metadata_index_has_the_same_payload_depth_limit_as_decoding() {
        for depth in 120..132 {
            let payload = format!("{}null{}", "[".repeat(depth), "]".repeat(depth));
            let raw = format!(
                r#"{{"opId":"a:1","originDeviceId":"a","sequence":1,"domain":"chat","entityType":"message","entityId":"m","operation":"upsert","semantics":"entity_state","payload":{payload},"createdAt":1,"schemaVersion":1}}"#
            );
            assert_eq!(
                JournalRecord::parse(raw.as_bytes(), 0).is_ok(),
                serde_json::from_str::<SyncOperation>(&raw).is_ok(),
                "depth={depth}"
            );
        }
    }

    #[test]
    fn metadata_index_rejects_numbers_that_cannot_be_materialized_as_values() {
        let raw = br#"{"opId":"a:1","originDeviceId":"a","sequence":1,"domain":"chat","entityType":"message","entityId":"m","operation":"upsert","semantics":"entity_state","payload":{"bad":1e999},"createdAt":1,"schemaVersion":1}"#;
        assert!(serde_json::from_slice::<SyncOperation>(raw).is_err());
        assert!(JournalRecord::parse(raw, 0).is_err());
    }
}
