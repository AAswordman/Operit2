//! Bounded JSON I/O for verified synchronization blobs. Publication remains atomic.
use std::io::{self, BufReader, BufWriter, Read, Write};
use operit_host_api::{RuntimeStorageHost, RuntimeStorageWriteSession};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::SyncOperationStore::SyncOperation;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeFileSyncReference {
    pub contentHash: String,
    pub size: i64,
}

/// Validates one content-addressed blob reference.
pub(crate) fn validateBlobReference(reference: &RuntimeFileSyncReference) -> Result<(), String> {
    validateContentHash(&reference.contentHash)?;
    if reference.size < 0 {
        return Err("runtime file blob size must not be negative".to_string());
    }
    Ok(())
}

/// Validates a lowercase SHA-256 storage key.
pub(crate) fn validateContentHash(contentHash: &str) -> Result<(), String> {
    if contentHash.len() != 64 || !contentHash.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err("runtime file blob hash must be lowercase SHA-256 hex".to_string());
    }
    Ok(())
}

/// Verifies declared blob size and content identity.
pub(crate) fn verifyBlob(reference: &RuntimeFileSyncReference, content: &[u8]) -> Result<(), String> {
    validateBlobReference(reference)?;
    let size = i64::try_from(content.len())
        .map_err(|_| "runtime file size does not fit i64".to_string())?;
    if size != reference.size {
        return Err(format!("runtime file blob size mismatch: expected {}, got {size}", reference.size));
    }
    let actualHash = format!("{:x}",Sha256::digest(content));
    if actualHash != reference.contentHash {
        return Err(format!("runtime file blob hash mismatch: expected {}, got {actualHash}", reference.contentHash));
    }
    Ok(())
}

pub(crate) fn initialization_reference(operation:&SyncOperation)->Result<Option<RuntimeFileSyncReference>,String> {
    if operation.operation!="initialize" || operation.entityType!="initialization" { return Ok(None); }
    operation.payload.get("transactionBlob").map(|value| {
        let reference:RuntimeFileSyncReference=serde_json::from_value(value.clone()).map_err(|e|e.to_string())?;
        validateBlobReference(&reference)?;
        Ok(reference)
    }).transpose()
}
/// Validate the immutable header before reserving a downloaded initializer's journal identity.
pub(crate) fn verify_initialization(host:&dyn RuntimeStorageHost,root:&str,operation:&SyncOperation)->Result<(),String> {
    let Some(reference)=initialization_reference(operation)? else { return Ok(()); };
    let path=format!("{root}/blobs/{}",reference.contentHash);
    if !host.exists(&path).map_err(|e|e.to_string())? { return Ok(()); }
    let header:SyncOperation<serde::de::IgnoredAny>=read_json(host,&path,&reference)?;
    if header.with_payload(serde_json::Value::Null)!=operation.with_payload(serde_json::Value::Null) { return Err("Initialization blob differs from its envelope".into()); }
    Ok(())
}


const BUFFER_BYTES: usize = 64 * 1024;

struct SessionWriter {
    session: Option<Box<dyn RuntimeStorageWriteSession>>,
    hash: Sha256,
    bytes: u64,
}
impl Write for SessionWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.session.as_mut().expect("open blob writer").writeChunk(bytes).map_err(io::Error::other)?;
        self.hash.update(bytes);
        self.bytes += bytes.len() as u64;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
impl Drop for SessionWriter {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() { let _ = session.discard(); }
    }
}

pub(crate) fn write_json<T: Serialize>(host: &dyn RuntimeStorageHost, path: &str, value: &T,
    reference: &RuntimeFileSyncReference) -> Result<(), String> {
    if reference.size<=BUFFER_BYTES as i64 {
        let bytes=serde_json::to_vec(value).map_err(|e|e.to_string())?;
        verifyBlob(reference,&bytes)?;
        return host.writeBytesAtomically(path,&bytes).map_err(|e|e.to_string());
    }
    write_stream(host,path,reference,|writer|serde_json::to_writer(writer,value).map_err(|e|e.to_string()))
}

pub(crate) fn write_stream(host:&dyn RuntimeStorageHost,path:&str,reference:&RuntimeFileSyncReference,
    write:impl FnOnce(&mut dyn Write)->Result<(),String>)->Result<(),String> {
    let session=host.openWriteSession(path).map_err(|e|e.to_string())?.ok_or("Storage host cannot stream synchronization blobs")?;
    let mut writer = BufWriter::with_capacity(BUFFER_BYTES, SessionWriter { session: Some(session), hash: Sha256::new(), bytes: 0 });
    write(&mut writer)?;
    writer.flush().map_err(|e|e.to_string())?;
    let mut writer = writer.into_inner().map_err(|e|e.to_string())?;
    if writer.bytes != reference.size as u64 || format!("{:x}", writer.hash.clone().finalize()) != reference.contentHash {
        return Err("Synchronization blob changed during serialization".into());
    }
    writer.session.take().expect("open blob writer").commit().map_err(|e|e.to_string())
}

pub(crate) struct VerifiedReader<'a> {
    reader: crate::LegacyTransactionImport::JournalReader<'a>,
    hash: Sha256,
    bytes: u64,
}
impl Read for VerifiedReader<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let count = self.reader.read(bytes)?;
        self.hash.update(&bytes[..count]);
        self.bytes += count as u64;
        Ok(count)
    }
}

pub(crate) fn reader<'a>(host:&'a dyn RuntimeStorageHost,path:&'a str,reference:&RuntimeFileSyncReference)->VerifiedReader<'a> {
    VerifiedReader { reader:crate::LegacyTransactionImport::JournalReader::new(host,path,0,reference.size as u64+1),hash:Sha256::new(),bytes:0 }
}
pub(crate) fn verify_reader(reader:VerifiedReader<'_>,reference:&RuntimeFileSyncReference)->Result<(),String> {
    if reader.bytes!=reference.size as u64 || format!("{:x}",reader.hash.finalize())!=reference.contentHash { return Err("Synchronization blob content hash or size differs".into()); }
    Ok(())
}

pub(crate) fn read_json<T: DeserializeOwned>(host: &dyn RuntimeStorageHost, path: &str,
    reference: &RuntimeFileSyncReference) -> Result<T, String> {
    let inner = reader(host,path,reference);
    let mut reader = BufReader::with_capacity(BUFFER_BYTES, inner);
    let value = serde_json::from_reader(&mut reader).map_err(|e|e.to_string())?;
    verify_reader(reader.into_inner(),reference)?;
    Ok(value)
}
