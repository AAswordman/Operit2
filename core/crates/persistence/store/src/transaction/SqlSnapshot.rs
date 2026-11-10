//! Disk-backed logical SQL snapshots. Only a row and fixed I/O buffers enter the Rust heap.
use std::{collections::BTreeMap, sync::Arc};
use operit_host_api::{HostError, HostResult, RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeStorageHost, SqliteValue};
use serde::{Serialize, Serializer, ser::{SerializeMap, SerializeSeq}};
use serde_json::Value;
use crate::{SqliteStore::{SqliteStore, SqliteTransaction, SqliteStoreError, SqliteRowGet}, RuntimeFileSyncStore::RuntimeFileSyncStore};
use super::{Cell, Data, Result, sql_identifier};

fn fail(error: impl std::fmt::Display) -> SqliteStoreError { SqliteStoreError::Message(error.to_string()) }
fn host_error(error: impl std::fmt::Display) -> HostError { HostError::new(error.to_string()) }
struct StagingHost(Arc<dyn RuntimeStorageHost>);
impl RuntimeSqliteHost for StagingHost {
    fn runtimeStorage(&self) -> Option<Arc<dyn RuntimeStorageHost>> { Some(self.0.clone()) }
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> { self.0.openTransactionDatabase(path) }
}

pub struct SqlSnapshot {
    store: Option<SqliteStore>,
    pub(crate) storage: Arc<dyn RuntimeStorageHost>,
    path: String,
    pub(crate) columns: BTreeMap<String, Vec<String>>,
}
impl std::fmt::Debug for SqlSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("SqlSnapshot").field("path", &self.path).finish() }
}
impl Drop for SqlSnapshot {
    fn drop(&mut self) {
        drop(self.store.take());
        let _ = self.storage.delete(&self.path, false);
    }
}
impl SqlSnapshot {
    pub(crate) fn new(storage: Arc<dyn RuntimeStorageHost>) -> Result<Self> {
        let path = format!("runtime/sync/staging/{}.sqlite", uuid::Uuid::new_v4());
        let store = SqliteStore::openWithHost(Arc::new(StagingHost(storage.clone())), &path).map_err(|e|e.to_string())?;
        let snapshot = Self { store:Some(store), storage, path, columns:BTreeMap::new() };
        // These settings apply to the disposable staging database, never to business durability.
        snapshot.store().executeBatch("PRAGMA journal_mode=OFF; PRAGMA synchronous=OFF; PRAGMA cache_size=-2048; PRAGMA temp_store=FILE;
            CREATE TABLE rows(id INTEGER PRIMARY KEY,name TEXT NOT NULL,value TEXT NOT NULL);
            CREATE INDEX row_order ON rows(name,value);
            CREATE TABLE projected(name TEXT NOT NULL,value TEXT NOT NULL);
            CREATE INDEX projected_order ON projected(name,value);").map_err(|e|e.to_string())?;
        Ok(snapshot)
    }
    fn store(&self) -> &SqliteStore { self.store.as_ref().expect("live snapshot") }
    pub(crate) fn from_values(storage:Arc<dyn RuntimeStorageHost>,values:&BTreeMap<String,Value>)->Result<Arc<Self>> {
        let mut snapshot=Self::new(storage)?;
        for (name,table) in values {
            let columns:Vec<String>=serde::Deserialize::deserialize(&table["columns"]).map_err(|e|e.to_string())?;
            for row in table["rows"].as_array().ok_or("Missing SQL snapshot rows")? {
                let row:Vec<Cell>=serde::Deserialize::deserialize(row).map_err(|e|e.to_string())?;
                snapshot.insert(name,&row)?;
            }
            snapshot.columns.insert(name.clone(),columns);
        }
        Ok(Arc::new(snapshot))
    }
    pub(crate) fn capture(storage: Arc<dyn RuntimeStorageHost>, tx: &mut SqliteTransaction<'_>, tables: &[&str]) -> std::result::Result<Arc<Self>,SqliteStoreError> {
        let mut snapshot = Self::new(storage).map_err(fail)?;
        for name in tables {
            snapshot.columns.insert((*name).into(), tx.queryRows(&format!("PRAGMA table_info({})",sql_identifier(name)),vec![])?.iter()
                .map(|row|row.get("name")).collect::<std::result::Result<_,_>>()?);
        }
        snapshot.store().transaction(|stage| {
            for (name,columns) in &snapshot.columns {
                if columns.is_empty() { continue; }
                tx.visitRows(&format!("SELECT {} FROM {}", columns.iter().map(|v|sql_identifier(v)).collect::<Vec<_>>().join(","),sql_identifier(name)), vec![], &mut |row| {
                    let cells=row.values.into_iter().map(Cell::from).collect::<Vec<_>>();
                    if super::serialized_digest(&cells).map_err(host_error)?.1>8*1024*1024 { return Err(host_error("SQL record exceeds the bounded synchronization decoder budget")); }
                    let value = serde_json::to_string(&cells).map_err(host_error)?;
                    stage.execute("INSERT INTO rows(name,value) VALUES(?1,?2)",vec![SqliteValue::Text(name.clone()),SqliteValue::Text(value)]).map_err(host_error)?;
                    Ok(())
                })?;
            }
            Ok(())
        })?;
        Ok(Arc::new(snapshot))
    }
    pub(crate) fn has_rows(&self) -> Result<bool> { self.store().queryOne("SELECT 1 FROM rows LIMIT 1",vec![]).map(|v|v.is_some()).map_err(|e|e.to_string()) }
    pub(crate) fn insert(&self, name: &str, cells: &[Cell]) -> Result<()> {
        self.store().execute("INSERT INTO rows(name,value) VALUES(?1,?2)",vec![SqliteValue::Text(name.into()),SqliteValue::Text(serde_json::to_string(&cells).map_err(|e|e.to_string())?)]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub(crate) fn visit(&self, name: &str, projected: bool, visitor: &mut dyn FnMut(Vec<Cell>) -> Result<()>) -> Result<()> {
        let sql = if projected { "SELECT DISTINCT value FROM projected WHERE name=?1 ORDER BY value" } else { "SELECT value FROM rows WHERE name=?1 ORDER BY value" };
        self.store().visitRows(sql,vec![SqliteValue::Text(name.into())],&mut |row| {
            let value: Vec<Cell> = serde_json::from_str(&row.get::<_,String>(0).map_err(host_error)?).map_err(host_error)?;
            visitor(value).map_err(host_error)
        }).map_err(|e|e.to_string())
    }
    pub(crate) fn install(&self, tx: &mut SqliteTransaction<'_>, names: &[String]) -> std::result::Result<(),SqliteStoreError> {
        if names.len() != self.columns.len() { return Err(fail("Unexpected SQL snapshot tables")); }
        tx.execute("PRAGMA defer_foreign_keys=ON",vec![])?;
        for name in names.iter().rev() { tx.execute(&format!("DELETE FROM {}",sql_identifier(name)),vec![])?; }
        for name in names {
            let columns = self.columns.get(name).ok_or_else(||fail("Missing SQL snapshot table"))?;
            let expected = tx.queryRows(&format!("PRAGMA table_info({})",sql_identifier(name)),vec![])?.iter()
                .map(|row|row.get::<_,String>("name")).collect::<std::result::Result<Vec<_>,_>>()?;
            if columns != &expected { return Err(fail("SQL snapshot schema mismatch")); }
            let query = format!("INSERT INTO {}({}) VALUES({})",sql_identifier(name),columns.iter().map(|v|sql_identifier(v)).collect::<Vec<_>>().join(","),(0..columns.len()).map(|_|"?").collect::<Vec<_>>().join(","));
            self.visit(name,false,&mut |row| {
                if row.len() != columns.len() { return Err("SQL snapshot schema mismatch".into()); }
                tx.execute(&query,row.into_iter().map(Into::into).collect()).map_err(|e|e.to_string())?;
                Ok(())
            }).map_err(fail)?;
        }
        Ok(())
    }
    /// Materialize only schema declarations or a caller-requested small table.
    pub(crate) fn table(&self, name: &str) -> Result<Value> {
        let columns = self.columns.get(name).ok_or("Missing SQL table")?;
        let mut rows = Vec::new();
        self.visit(name,false,&mut |row| { rows.push(serde_json::to_value(row).map_err(|e|e.to_string())?); Ok(()) })?;
        Ok(serde_json::json!({"columns":columns,"rows":rows}))
    }
    pub(crate) fn content_digest(&self, owner: &super::Owner) -> Result<String> {
        if self.columns.values().any(Vec::is_empty) { return Err("Missing synchronized SQL table".into()); }
        let mut projected_columns = self.columns.clone();
        for (name, columns) in &self.columns {
            let (local, identity): (&[&str],Option<(&str,&str)>) = if owner.path == operit_util::RuntimeStorageLayout::SQLITE_DATABASE_PATH {
                match name.as_str() { "messages"=>( &["messageId"],None), "message_variants"=>( &["variantId"],None), _=>continue }
            } else {
                match name.as_str() { "_operit_values"=>( &["version"],None), "_operit_merge_map"=>( &["origin"],Some(("remote_key","canonical_key"))), _=>continue }
            };
            let identity = identity.map(|(a,b)| Ok::<_,String>((columns.iter().position(|v|v==a).ok_or("Missing identity source column")?,columns.iter().position(|v|v==b).ok_or("Missing identity destination column")?))).transpose()?;
            let removed = columns.iter().enumerate().filter_map(|(i,v)|local.contains(&v.as_str()).then_some(i)).collect::<Vec<_>>();
            projected_columns.insert(name.clone(),columns.iter().filter(|v|!local.contains(&v.as_str())).cloned().collect());
            // SQLite projects and sorts on disk without a Rust row or sort-key collection.
            self.store().transaction(|tx| {
                let params=vec![SqliteValue::Text(name.clone())];
                if tx.queryOne("SELECT 1 FROM rows WHERE name=?1 AND json_array_length(value)!=?2 LIMIT 1",
                    vec![params[0].clone(),SqliteValue::Integer(columns.len() as i64)])?.is_some() { return Err(fail("Invalid SQL row width")); }
                tx.execute("DELETE FROM projected WHERE name=?1",params.clone())?;
                let paths=removed.iter().rev().map(|i|format!(",'$[{i}]'")).collect::<String>();
                let value=if paths.is_empty() { "value".into() } else { format!("json_remove(value{paths})") };
                let filter=identity.map_or(String::new(),|(a,b)|format!(" AND json_extract(value,'$[{a}]')!=json_extract(value,'$[{b}]')"));
                tx.execute(&format!("INSERT INTO projected SELECT name,{value} FROM rows WHERE name=?1{filter}"),params)?;
                Ok(())
            }).map_err(|e|e.to_string())?;
        }
        super::digest(&Values { snapshot:self, columns:&projected_columns, projected:true })
    }
    pub(crate) fn values(&self) -> Values<'_> { Values { snapshot:self, columns:&self.columns, projected:false } }
    pub(crate) fn files(&self) -> RuntimeFileSyncStore { RuntimeFileSyncStore::new(self.storage.clone(),"runtime/sync") }
}

pub(crate) struct Values<'a> { snapshot:&'a SqlSnapshot, columns:&'a BTreeMap<String,Vec<String>>, projected:bool }
impl Serialize for Values<'_> {
    fn serialize<S:Serializer>(&self, serializer:S) -> std::result::Result<S::Ok,S::Error> {
        let mut map = serializer.serialize_map(Some(self.columns.len()))?;
        for (name,columns) in self.columns {
            let projected = self.projected && columns != &self.snapshot.columns[name];
            map.serialize_entry(name,&LogicalTable { columns,rows:Rows { snapshot:self.snapshot,name,projected } })?;
        }
        map.end()
    }
}
#[derive(Serialize)]
struct LogicalTable<'a> { columns:&'a Vec<String>, rows:Rows<'a> }
struct Rows<'a> { snapshot:&'a SqlSnapshot,name:&'a str,projected:bool }
impl Serialize for Rows<'_> {
    fn serialize<S:Serializer>(&self, serializer:S) -> std::result::Result<S::Ok,S::Error> {
        let mut seq = serializer.serialize_seq(None)?;
        self.snapshot.visit(self.name,self.projected,&mut |row|seq.serialize_element(&row).map_err(|e|e.to_string())).map_err(serde::ser::Error::custom)?;
        seq.end()
    }
}

/// Both structural Data encoding and the sorted-object encoding used by immutable payloads.
pub(crate) struct DataWire<'a> { pub data:&'a Data, pub canonical:bool }
impl Serialize for DataWire<'_> {
    fn serialize<S:Serializer>(&self, serializer:S) -> std::result::Result<S::Ok,S::Error> {
        let d=self.data;
        let mut map=serializer.serialize_map(Some(7))?;
        let keys=if self.canonical { ["additions","aliases","deleted","owner","revision","values","versions"] } else { ["owner","values","revision","deleted","aliases","versions","additions"] };
        for key in keys { match key {
            "owner"=>if self.canonical { map.serialize_entry(key,&serde_json::to_value(&d.owner).map_err(serde::ser::Error::custom)?)?; } else { map.serialize_entry(key,&d.owner)?; },
            "values"=>if let Some(sql)=&d.sql { map.serialize_entry(key,&sql.values())?; } else { map.serialize_entry(key,&d.values)?; },
            "revision"=>map.serialize_entry(key,&d.revision)?, "deleted"=>map.serialize_entry(key,&d.deleted)?,
            "aliases"=>map.serialize_entry(key,&d.aliases)?, "versions"=>map.serialize_entry(key,&d.versions)?,
            _=>map.serialize_entry(key,&d.additions)?,
        } }
        map.end()
    }
}

struct Initializer<'a>(&'a super::compaction::Initialization);
impl Serialize for Initializer<'_> {
    fn serialize<S:Serializer>(&self, serializer:S) -> std::result::Result<S::Ok,S::Error> {
        let mut map=serializer.serialize_map(None)?;
        map.serialize_entry("covered",&self.0.covered)?;
        map.serialize_entry("data",&DataWire { data:&self.0.data,canonical:true })?;
        if let Some(definition)=&self.0.definition { map.serialize_entry("definition",definition)?; }
        map.end()
    }
}
struct Payload<'a> { initialization:&'a super::compaction::Initialization,encrypted:bool }
impl Serialize for Payload<'_> {
    fn serialize<S:Serializer>(&self, serializer:S) -> std::result::Result<S::Ok,S::Error> {
        let mut map=serializer.serialize_map(Some(4))?;
        map.serialize_entry("encrypted",&self.encrypted)?;
        map.serialize_entry("initialization",&Initializer(self.initialization))?;
        map.serialize_entry("schemaVersion",&self.initialization.data.owner.format)?;
        map.serialize_entry("storagePath",&self.initialization.data.owner.path)?;
        map.end()
    }
}
pub(crate) fn operation(initialization:&super::compaction::Initialization,domain:&str,encrypted:bool) -> Result<crate::SyncOperationStore::SyncOperation> {
    use crate::SyncOperationStore::{SyncOperation,SyncOperationSemantics};
    let origin=format!("~init:{}",super::digest(&(domain,Payload { initialization,encrypted }))?);
    let mut header=SyncOperation { opId:format!("{origin}:1"),originDeviceId:origin,sequence:1,domain:domain.into(),
        entityType:"initialization".into(),entityId:initialization.data.owner.path.clone(),operation:"initialize".into(),semantics:SyncOperationSemantics::Transaction,
        payload:Value::Null,createdAt:initialization.data.revision.as_ref().map_or(0,|r|r.createdAt),schemaVersion:1 };
    let original=header.with_payload(Payload { initialization,encrypted });
    if super::serialized_digest(&original)?.1<=32*1024 { return serde_json::from_value(serde_json::to_value(&original).map_err(|e|e.to_string())?).map_err(|e|e.to_string()); }
    let reference=initialization.data.sql.as_ref().ok_or("Missing staged SQL data")?.files().storeJsonBlob(&original)?;
    header.payload=serde_json::json!({"transactionBlob":reference});
    Ok(header)
}

use serde::de::{self, DeserializeSeed, Visitor, MapAccess, SeqAccess};
use std::{cell::Cell as Counter, rc::Rc, io::{Read, self}};

/// Total metadata and one row have separate limits; the complete row stream has no size cap.
struct ParseBudget<R> { inner:R, row:Rc<Counter<Option<usize>>>, metadata:usize }
impl<R:Read> Read for ParseBudget<R> {
    fn read(&mut self, bytes:&mut [u8]) -> io::Result<usize> {
        let remaining=self.row.get().unwrap_or(self.metadata);
        if remaining==0 { return Err(io::Error::other("Synchronization record or metadata exceeds the bounded decoder budget")); }
        let capacity=bytes.len().min(remaining);
        let n=self.inner.read(&mut bytes[..capacity])?;
        if self.row.get().is_some() { self.row.set(Some(remaining-n)); } else { self.metadata-=n; }
        Ok(n)
    }
}
struct JsonSeed<'a> { snapshot:&'a mut SqlSnapshot,path:Vec<String>,budget:Rc<Counter<Option<usize>>> }
impl<'de> DeserializeSeed<'de> for JsonSeed<'_> {
    type Value=Value;
    fn deserialize<D:de::Deserializer<'de>>(self, deserializer:D) -> std::result::Result<Value,D::Error> {
        if self.path.iter().map(String::as_str).eq(["payload","initialization","data","values"])
            || self.path.iter().map(String::as_str).eq(["snapshot","data","values"])
            || self.path.iter().map(String::as_str).eq(["data","values"]) {
            return deserializer.deserialize_map(TablesSeed { snapshot:self.snapshot,budget:self.budget });
        }
        deserializer.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for JsonSeed<'_> {
    type Value=Value;
    fn expecting(&self,f:&mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("JSON synchronization metadata") }
    fn visit_unit<E:de::Error>(self)->std::result::Result<Value,E>{Ok(Value::Null)}
    fn visit_bool<E:de::Error>(self,v:bool)->std::result::Result<Value,E>{Ok(Value::Bool(v))}
    fn visit_i64<E:de::Error>(self,v:i64)->std::result::Result<Value,E>{Ok(v.into())}
    fn visit_u64<E:de::Error>(self,v:u64)->std::result::Result<Value,E>{Ok(v.into())}
    fn visit_f64<E:de::Error>(self,v:f64)->std::result::Result<Value,E>{serde_json::Number::from_f64(v).map(Value::Number).ok_or_else(||E::custom("Nonfinite JSON number"))}
    fn visit_str<E:de::Error>(self,v:&str)->std::result::Result<Value,E>{Ok(v.into())}
    fn visit_string<E:de::Error>(self,v:String)->std::result::Result<Value,E>{Ok(Value::String(v))}
    fn visit_seq<A:SeqAccess<'de>>(self,mut seq:A)->std::result::Result<Value,A::Error>{
        let mut values=Vec::new();
        while let Some(value)=seq.next_element_seed(JsonSeed { snapshot:self.snapshot,path:self.path.clone(),budget:self.budget.clone() })? { values.push(value); }
        Ok(Value::Array(values))
    }
    fn visit_map<A:MapAccess<'de>>(self,mut map:A)->std::result::Result<Value,A::Error>{
        let mut values=serde_json::Map::new();
        while let Some(key)=map.next_key::<String>()? {
            let mut path=self.path.clone(); path.push(key.clone());
            let value=map.next_value_seed(JsonSeed { snapshot:self.snapshot,path,budget:self.budget.clone() })?;
            if values.insert(key,value).is_some(){return Err(de::Error::custom("Duplicate JSON field"));}
        }
        Ok(Value::Object(values))
    }
}
struct TablesSeed<'a>{snapshot:&'a mut SqlSnapshot,budget:Rc<Counter<Option<usize>>>}
impl<'de> Visitor<'de> for TablesSeed<'_>{
    type Value=Value;
    fn expecting(&self,f:&mut std::fmt::Formatter)->std::fmt::Result{f.write_str("logical SQL tables")}
    fn visit_map<A:MapAccess<'de>>(self,mut map:A)->std::result::Result<Value,A::Error>{
        while let Some(name)=map.next_key::<String>()?{
            if self.snapshot.columns.contains_key(&name){return Err(de::Error::custom("Duplicate SQL table"));}
            let columns=map.next_value_seed(TableSeed { snapshot:self.snapshot,name:&name,budget:self.budget.clone() })?;
            self.snapshot.columns.insert(name,columns);
        }
        Ok(serde_json::json!({}))
    }
}
struct TableSeed<'a>{snapshot:&'a SqlSnapshot,name:&'a str,budget:Rc<Counter<Option<usize>>>}
impl<'de> DeserializeSeed<'de> for TableSeed<'_>{
    type Value=Vec<String>;
    fn deserialize<D:de::Deserializer<'de>>(self,d:D)->std::result::Result<Self::Value,D::Error>{d.deserialize_map(self)}
}
impl<'de> Visitor<'de> for TableSeed<'_>{
    type Value=Vec<String>;
    fn expecting(&self,f:&mut std::fmt::Formatter)->std::fmt::Result{f.write_str("SQL columns and rows")}
    fn visit_map<A:MapAccess<'de>>(self,mut map:A)->std::result::Result<Self::Value,A::Error>{
        let mut columns=None; let mut rows=false;
        while let Some(key)=map.next_key::<String>()?{match key.as_str(){
            "columns" if columns.is_none()=>columns=Some(map.next_value()?),
            "rows" if !rows=>{map.next_value_seed(RowsSeed { snapshot:self.snapshot,name:self.name,budget:self.budget.clone() })?;rows=true;},
            _=>return Err(de::Error::custom("Unexpected SQL table field")),
        }}
        if !rows{return Err(de::Error::custom("Missing SQL rows"));}
        columns.ok_or_else(||de::Error::custom("Missing SQL columns"))
    }
}
struct RowsSeed<'a>{snapshot:&'a SqlSnapshot,name:&'a str,budget:Rc<Counter<Option<usize>>>}
impl<'de> DeserializeSeed<'de> for RowsSeed<'_>{
    type Value=();
    fn deserialize<D:de::Deserializer<'de>>(self,d:D)->std::result::Result<(),D::Error>{d.deserialize_seq(self)}
}
impl<'de> Visitor<'de> for RowsSeed<'_>{
    type Value=();
    fn expecting(&self,f:&mut std::fmt::Formatter)->std::fmt::Result{f.write_str("a stream of SQL records")}
    fn visit_seq<A:SeqAccess<'de>>(self,mut seq:A)->std::result::Result<(),A::Error>{
        loop{
            self.budget.set(Some(8*1024*1024));
            let row=seq.next_element::<Vec<Cell>>()?;
            self.budget.set(None);
            let Some(row)=row else{break;};
            self.snapshot.insert(self.name,&row).map_err(de::Error::custom)?;
        }
        Ok(())
    }
}
pub(crate) fn read_staged<T:de::DeserializeOwned>(storage:Arc<dyn RuntimeStorageHost>,path:&str,reference:&crate::RuntimeFileSyncStore::RuntimeFileSyncReference)->Result<(T,Arc<SqlSnapshot>)>{
    let mut snapshot=SqlSnapshot::new(storage.clone())?;
    let budget=Rc::new(Counter::new(None));
    let reader=crate::SyncBlobIo::reader(storage.as_ref(),path,reference);
    let bounded=ParseBudget { inner:reader,row:budget.clone(),metadata:16*1024*1024 };
    let mut buffered=std::io::BufReader::with_capacity(64*1024,bounded);
    let mut deserializer=serde_json::Deserializer::from_reader(&mut buffered);
    let value=JsonSeed { snapshot:&mut snapshot,path:Vec::new(),budget }.deserialize(&mut deserializer).map_err(|e|e.to_string())?;
    deserializer.end().map_err(|e|e.to_string())?;
    drop(deserializer);
    crate::SyncBlobIo::verify_reader(buffered.into_inner().inner,reference)?;
    let value=serde_json::from_value(value).map_err(|e|e.to_string())?;
    Ok((value,Arc::new(snapshot)))
}
pub(crate) fn validate_initialization(wire:&crate::SyncOperationStore::SyncOperation,mut original:crate::SyncOperationStore::SyncOperation,sql:Arc<SqlSnapshot>)->Result<super::compaction::Initialization>{
    let reference=RuntimeFileSyncStore::requiredBlob(wire)?.ok_or("Missing initialization blob")?;
    let payload=std::mem::replace(&mut original.payload,wire.payload.clone());
    if original!=*wire{return Err("Initialization blob differs from its envelope".into());}
    let mut initialization=<super::compaction::Initialization as serde::Deserialize>::deserialize(&payload["initialization"]).map_err(|e|e.to_string())?;
    let encrypted=payload["encrypted"].as_bool().ok_or("Missing initialization encryption flag")?;
    // Rehash the canonical stream below; this also rejects unknown or reordered metadata.
    drop(payload);
    initialization.data.sql=Some(sql);
    let origin=format!("~init:{}",super::digest(&(&wire.domain,Payload { initialization:&initialization,encrypted }))?);
    if wire.originDeviceId!=origin || wire.opId!=format!("{origin}:1") || wire.sequence!=1 || wire.schemaVersion!=1
        || wire.createdAt!=initialization.data.revision.as_ref().map_or(0,|r|r.createdAt)
        || wire.entityId!=initialization.data.owner.path || wire.semantics!=crate::SyncOperationStore::SyncOperationSemantics::Transaction
        || super::digest(&wire.with_payload(Payload { initialization:&initialization,encrypted }))?!=reference.contentHash {
        return Err("Invalid initialization transaction identity".into());
    }
    Ok(initialization)
}

struct ColumnReader<'a>{tx:&'a mut dyn operit_host_api::RuntimeSqliteTransaction,table:&'static str,column:&'static str,row:i64,offset:usize,length:usize}
impl Read for ColumnReader<'_>{
    fn read(&mut self,bytes:&mut [u8])->io::Result<usize>{
        let length=bytes.len().min(64*1024).min(self.length.saturating_sub(self.offset));
        if length==0{return Ok(0);}
        let data=self.tx.readColumnRange(self.table,self.column,self.row,self.offset,length).map_err(io::Error::other)?;
        if data.is_empty() || data.len()>length {return Err(io::Error::new(io::ErrorKind::UnexpectedEof,"Invalid SQLite column range"));}
        bytes[..data.len()].copy_from_slice(&data);self.offset+=data.len();Ok(data.len())
    }
}
fn store_column(files:&RuntimeFileSyncStore,tx:&mut SqliteTransaction<'_>,table:&'static str,column:&'static str,row:i64,expected:Option<&str>)->std::result::Result<crate::RuntimeFileSyncStore::RuntimeFileSyncReference,SqliteStoreError>{
    use sha2::{Digest,Sha256};
    let length=tx.raw().columnSize(table,column,row)?;
    let mut reader=ColumnReader { tx:tx.raw(),table,column,row,offset:0,length };
    let mut hash=Sha256::new();let mut bytes=vec![0;64*1024];
    loop {let n=reader.read(&mut bytes)?;if n==0{break;}hash.update(&bytes[..n]);}
    let content_hash=format!("{:x}",hash.finalize());
    if expected.is_some_and(|v|!v.is_empty() && v!=content_hash){return Err(fail("Stored SQL initializer fingerprint differs"));}
    let reference=crate::RuntimeFileSyncStore::RuntimeFileSyncReference {contentHash:content_hash,size:i64::try_from(length).map_err(fail)?};
    reader.offset=0;files.storeStream(&reference,&mut reader).map_err(fail)?;
    Ok(reference)
}
/// One-time migration reads old giant TEXT cells through SQLite's incremental BLOB API.
pub(crate) fn migrate_initialization(store:&SqliteStore)->std::result::Result<(),SqliteStoreError>{
    let Some(storage)=&store.storage else{return Ok(());};
    let files=RuntimeFileSyncStore::new(storage.clone(),"runtime/sync");
    store.transaction(|tx|{
        let row=tx.queryOne("SELECT i.id,c.fingerprint FROM _operit_transaction_initialization i LEFT JOIN _operit_initialization_metadata m ON m.id=i.id LEFT JOIN _operit_compacted c ON c.id=i.operation_id WHERE m.id IS NULL LIMIT 1",vec![])?;
        let Some(row)=row else{return Ok(());};
        let fingerprint=match row.values.get(1){Some(SqliteValue::Text(value))=>Some(value.as_str()),_=>None};
        let reference=store_column(&files,tx,"_operit_transaction_initialization","original",row.get(0)?,fingerprint)?;
        let (original,sql)=files.readStagedJson::<crate::SyncOperationStore::SyncOperation>(&reference).map_err(fail)?;
        let mut wire=original.clone();wire.payload=serde_json::json!({"transactionBlob":reference});
        let initialization=validate_initialization(&wire,original,sql).map_err(fail)?;
        super::compaction::save_initialization(tx,&wire,&initialization.covered,&initialization.data)
    })
}
pub(crate) fn migrate_checkpoint(store:&SqliteStore,tx:&mut SqliteTransaction<'_>,row:i64,domain:&str)->std::result::Result<(),SqliteStoreError>{
    let files=RuntimeFileSyncStore::new(store.storage.clone().ok_or_else(||fail("SQL host cannot stage a checkpoint"))?,"runtime/sync");
    let reference=store_column(&files,tx,"_operit_transaction_checkpoint","snapshot",row,None)?;
    let (mut snapshot,sql)=files.readStagedJson::<super::Snapshot>(&reference).map_err(fail)?;
    snapshot.data.sql=Some(sql);
    let operation=super::compaction::Initialization::from_snapshot(&snapshot).operation(domain,false).map_err(fail)?;
    super::compaction::save_initialization(tx,&operation,&snapshot.covered,&snapshot.data)
}
