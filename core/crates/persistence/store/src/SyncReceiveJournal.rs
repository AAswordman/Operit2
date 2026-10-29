//! Durable replay intent references exact records, including independently committed SQL revisions.
//! Legacy inline payloads are streamed out before publishing the new ledger.
use operit_host_api::RuntimeStorageHost;
use serde::de::{DeserializeSeed, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{BufRead, BufReader};

use crate::SyncOperationJournal::{JournalReader, JournalRecord, JOURNAL_READ_BYTES};
use crate::SyncOperationStore::{ChatOperationRevision, SyncOperation, SyncOperationStoreError, SyncReceipt};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PendingRecord {
    pub(crate) record: JournalRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    revision: Option<String>,
}

impl PendingRecord {
    /// Keeps a replacement separate from the file referenced by the previously committed ledger.
    pub(crate) fn path(&self, root: &str) -> String {
        let key = match &self.revision {
            Some(revision) => format!("{}:{revision}", self.record.op_id),
            None => self.record.op_id.clone(),
        };
        pending_path(root, &key)
    }
}

#[derive(Clone, Serialize)]
pub(crate) struct ReceiveState {
    #[serde(rename = "formatVersion")]
    format_version: u32,
    pub(crate) receipt: SyncReceipt,
    pub(crate) pending: BTreeMap<String, PendingRecord>,
    pub(crate) fingerprints: BTreeMap<String, String>,
    #[serde(rename = "chatRevisions")]
    pub(crate) chat_revisions: BTreeMap<String, ChatOperationRevision>,
}

impl Default for ReceiveState {
    fn default() -> Self {
        Self {
            format_version: 2,
            receipt: SyncReceipt::default(),
            pending: BTreeMap::new(),
            fingerprints: BTreeMap::new(),
            chat_revisions: BTreeMap::new(),
        }
    }
}

pub(crate) fn pending_path(root: &str, op_id: &str) -> String {
    format!("{root}/pending/{:x}.json", Sha256::digest(op_id.as_bytes()))
}

pub(crate) fn persist_pending(
    host: &dyn RuntimeStorageHost,
    root: &str,
    operation: &SyncOperation,
    replacement: bool,
) -> Result<PendingRecord, SyncOperationStoreError> {
    let bytes = serde_json::to_vec(operation)?;
    let record = JournalRecord::parse(&bytes, 0)?;
    let pending = PendingRecord {
        record,
        revision: replacement.then(|| format!("{:x}", Sha256::digest(&bytes))),
    };
    host.writeBytesAtomically(&pending.path(root), &bytes)?;
    Ok(pending)
}

struct StateSeed<'a> {
    host: &'a dyn RuntimeStorageHost,
    root: &'a str,
    migrated: &'a mut bool,
}
struct PendingSeed<'a> {
    host: &'a dyn RuntimeStorageHost,
    root: &'a str,
    migrated: &'a mut bool,
}

impl<'de> DeserializeSeed<'de> for PendingSeed<'_> {
    type Value = BTreeMap<String, PendingRecord>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct PendingVisitor<'a>(PendingSeed<'a>);
        impl<'de> Visitor<'de> for PendingVisitor<'_> {
            type Value = BTreeMap<String, PendingRecord>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("pending operation references")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut pending = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    // Only one legacy record, rather than the entire pending map, is materialized.
                    let raw = map.next_value::<Box<RawValue>>()?;
                    let entry = match serde_json::from_str::<PendingRecord>(raw.get()) {
                        Ok(entry) => entry,
                        Err(_) => {
                            let record = JournalRecord::parse(raw.get().as_bytes(), 0)
                                .map_err(serde::de::Error::custom)?;
                            if key != record.op_id {
                                return Err(serde::de::Error::custom(
                                    "Pending identity does not match its key",
                                ));
                            }
                            self.0
                                .host
                                .writeBytesAtomically(
                                    &pending_path(self.0.root, &key),
                                    raw.get().as_bytes(),
                                )
                                .map_err(serde::de::Error::custom)?;
                            let entry = PendingRecord { record, revision: None };
                            *self.0.migrated = true;
                            entry
                        }
                    };
                    if key != entry.record.op_id {
                        return Err(serde::de::Error::custom(
                            "Pending reference identity does not match its key",
                        ));
                    }
                    pending.insert(key, entry);
                }
                Ok(pending)
            }
        }
        deserializer.deserialize_map(PendingVisitor(self))
    }
}

impl<'de> DeserializeSeed<'de> for StateSeed<'_> {
    type Value = ReceiveState;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct StateVisitor<'a>(StateSeed<'a>);
        impl<'de> Visitor<'de> for StateVisitor<'_> {
            type Value = ReceiveState;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("receive ledger")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut state = ReceiveState::default();
                let mut version = None;
                let mut fields = BTreeSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !fields.insert(key.clone()) {
                        return Err(serde::de::Error::custom("Duplicate receive ledger field"));
                    }
                    match key.as_str() {
                        "formatVersion" => version = Some(map.next_value::<u32>()?),
                        "receipt" => state.receipt = map.next_value()?,
                        "fingerprints" => state.fingerprints = map.next_value()?,
                        "chatRevisions" => state.chat_revisions = map.next_value()?,
                        "pending" => {
                            state.pending = map.next_value_seed(PendingSeed {
                                host: self.0.host,
                                root: self.0.root,
                                migrated: self.0.migrated,
                            })?
                        }
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                if !fields.contains("receipt") || !fields.contains("pending") {
                    return Err(serde::de::Error::custom("Incomplete receive ledger"));
                }
                match version {
                    Some(2) => {}
                    None => *self.0.migrated = true,
                    Some(_) => {
                        return Err(serde::de::Error::custom(
                            "Unsupported receive ledger version",
                        ))
                    }
                }
                state.receipt.validate().map_err(serde::de::Error::custom)?;
                Ok(state)
            }
        }
        deserializer.deserialize_map(StateVisitor(self))
    }
}

pub(crate) fn load_receive_state(
    host: &dyn RuntimeStorageHost,
    root: &str,
) -> Result<ReceiveState, SyncOperationStoreError> {
    let path = format!("{root}/receive_state.json");
    if !host.exists(&path)? {
        return Ok(ReceiveState::default());
    }
    let mut reader = BufReader::with_capacity(
        JOURNAL_READ_BYTES,
        JournalReader::new(host, &path, 0, u64::MAX),
    );
    // Preserve the old empty-document convention without accepting truncated JSON.
    loop {
        let bytes = reader.fill_buf()?;
        if bytes.is_empty() {
            return Ok(ReceiveState::default());
        }
        let whitespace = bytes
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();
        if whitespace != bytes.len() {
            reader.consume(whitespace);
            break;
        }
        reader.consume(whitespace);
    }
    let mut migrated = false;
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    let state = StateSeed {
        host,
        root,
        migrated: &mut migrated,
    }
    .deserialize(&mut deserializer)?;
    deserializer.end()?;
    if migrated {
        // All payload files are durably published first. Failure leaves the original
        // inline ledger untouched and safe to migrate again after restart.
        host.writeBytesAtomically(&path, &serde_json::to_vec(&state)?)?;
    }
    // Unreferenced files can result from an interrupted metadata commit or cleanup.
    // This private namespace contains only content owned by this ledger.
    let referenced = state
        .pending
        .values()
        .map(|pending| pending.path(root))
        .collect::<BTreeSet<_>>();
    for entry in host.list(&format!("{root}/pending/"))? {
        if entry.isDirectory || referenced.contains(&entry.path) {
            continue;
        }
        let Some(name) = entry
            .path
            .rsplit('/')
            .next()
            .and_then(|name| name.strip_suffix(".json"))
        else {
            continue;
        };
        if name.len() == 64 && name.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            // A cleanup failure does not undo the already durable receive ledger.
            if let Err(error) = host.delete(&entry.path, false) {
                operit_util::AppLogger::AppLogger::w(
                    "SyncReceiveJournal",
                    &format!("pending cleanup: {error}"),
                );
            }
        }
    }
    Ok(state)
}
