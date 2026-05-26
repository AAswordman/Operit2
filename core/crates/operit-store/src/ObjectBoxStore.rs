use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

use crate::RuntimeStorageHost::runtimeStoragePath;
use crate::RuntimeStorePaths::RuntimeStorePaths;
use crate::SqliteStore::{toSqliteValue, SqliteRowGet, SqliteStore, SqliteStoreError};
use crate::SyncOperationStore::{
    NewSyncOperation, SyncOperationStore, SyncOperationStoreError,
};

pub const OBJECTBOX_SYNC_DOMAIN: &str = "objectbox";

#[derive(Debug, Error)]
pub enum ObjectBoxStoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] SqliteStoreError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("sync operation store error: {0}")]
    Sync(#[from] SyncOperationStoreError),
    #[error("{0}")]
    Message(String),
}

pub trait ObjectBoxEntity: Clone {
    fn objectBoxId(&self) -> i64;
    fn setObjectBoxId(&mut self, id: i64);
}

#[derive(Clone)]
pub struct ObjectBox<T> {
    databaseStoragePath: String,
    entityType: String,
    sqliteStore: SqliteStore,
    syncOperationStore: SyncOperationStore,
    changeSignal: Arc<ObjectBoxChangeSignal>,
    marker: PhantomData<T>,
}

#[derive(Debug)]
struct ObjectBoxChangeSignal {
    version: Mutex<u64>,
    changed: Condvar,
}

impl<T> ObjectBox<T>
where
    T: ObjectBoxEntity + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    pub fn new(path: PathBuf, entityType: impl Into<String>) -> Self {
        let databasePath = path.with_extension("sqlite");
        let databaseStoragePath = runtimeStoragePath(&databasePath);
        let sqliteStore = SqliteStore::open(databasePath.clone())
            .expect("ObjectBox sqlite store must open");
        initializeSchema(&sqliteStore).expect("ObjectBox sqlite schema must initialize");
        let changeSignal = objectBoxChangeSignal(&databasePath);
        Self {
            databaseStoragePath,
            entityType: entityType.into(),
            sqliteStore,
            syncOperationStore: SyncOperationStore::native(RuntimeStorePaths::default()),
            changeSignal,
            marker: PhantomData,
        }
    }

    pub fn all(&self) -> Result<Vec<T>, ObjectBoxStoreError> {
        self.readEntities()
    }

    pub fn get(&self, id: i64) -> Result<Option<T>, ObjectBoxStoreError> {
        let row = self.sqliteStore.queryOne(
            "SELECT payload FROM objectbox_entities WHERE entity_type = ?1 AND id = ?2",
            vec![toSqliteValue(&self.entityType), toSqliteValue(&id)],
        )?;
        match row {
            Some(row) => {
                let payload: String = row.get("payload")?;
                Ok(Some(serde_json::from_str(&payload)?))
            }
            None => Ok(None),
        }
    }

    #[allow(non_snake_case)]
    pub fn getMany(&self, ids: &[i64]) -> Result<Vec<T>, ObjectBoxStoreError> {
        let selected = ids.iter().copied().collect::<std::collections::BTreeSet<_>>();
        Ok(self
            .readEntities()?
            .into_iter()
            .filter(|entity| selected.contains(&entity.objectBoxId()))
            .collect())
    }

    pub fn put(&self, mut entity: T) -> Result<T, ObjectBoxStoreError> {
        let saved = self.sqliteStore.transaction(|transaction| {
            if entity.objectBoxId() == 0 {
                entity.setObjectBoxId(nextObjectBoxId(transaction, &self.entityType)?);
            }
            let payload = serde_json::to_string(&entity)
                .map_err(|error| SqliteStoreError::Message(error.to_string()))?;
            transaction.execute(
                "INSERT INTO objectbox_entities(entity_type, id, payload, updated_at)
                 VALUES(?1, ?2, ?3, ?4)
                 ON CONFLICT(entity_type, id)
                 DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at",
                vec![
                    toSqliteValue(&self.entityType),
                    toSqliteValue(&entity.objectBoxId()),
                    toSqliteValue(&payload),
                    toSqliteValue(&nowMillis()),
                ],
            )?;
            Ok(entity.clone())
        })?;
        self.recordUpsertOperation(&saved)?;
        self.notifyChanged();
        Ok(saved)
    }

    #[allow(non_snake_case)]
    pub fn putMany(&self, incoming: Vec<T>) -> Result<Vec<T>, ObjectBoxStoreError> {
        let saved = self.sqliteStore.transaction(|transaction| {
            let mut saved = Vec::with_capacity(incoming.len());
            for mut entity in incoming {
                if entity.objectBoxId() == 0 {
                    entity.setObjectBoxId(nextObjectBoxId(transaction, &self.entityType)?);
                }
                let payload = serde_json::to_string(&entity)
                    .map_err(|error| SqliteStoreError::Message(error.to_string()))?;
                transaction.execute(
                    "INSERT INTO objectbox_entities(entity_type, id, payload, updated_at)
                     VALUES(?1, ?2, ?3, ?4)
                     ON CONFLICT(entity_type, id)
                     DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at",
                    vec![
                        toSqliteValue(&self.entityType),
                        toSqliteValue(&entity.objectBoxId()),
                        toSqliteValue(&payload),
                        toSqliteValue(&nowMillis()),
                    ],
                )?;
                saved.push(entity);
            }
            Ok(saved)
        })?;
        for entity in &saved {
            self.recordUpsertOperation(entity)?;
        }
        self.notifyChanged();
        Ok(saved)
    }

    pub fn remove(&self, id: i64) -> Result<bool, ObjectBoxStoreError> {
        let affected = self.sqliteStore.execute(
            "DELETE FROM objectbox_entities WHERE entity_type = ?1 AND id = ?2",
            vec![toSqliteValue(&self.entityType), toSqliteValue(&id)],
        )?;
        let removed = affected > 0;
        if removed {
            self.recordDeleteOperation(id)?;
            self.notifyChanged();
        }
        Ok(removed)
    }

    #[allow(non_snake_case)]
    pub fn removeEntity(&self, entity: &T) -> Result<bool, ObjectBoxStoreError> {
        self.remove(entity.objectBoxId())
    }

    #[allow(non_snake_case)]
    pub fn removeByIds(&self, ids: &[i64]) -> Result<usize, ObjectBoxStoreError> {
        let mut removed = 0usize;
        for id in ids {
            if self.remove(*id)? {
                removed += 1;
            }
        }
        Ok(removed)
    }

    #[allow(non_snake_case)]
    pub fn editEntities<F, R>(&self, transform: F) -> Result<R, ObjectBoxStoreError>
    where
        F: FnOnce(&mut Vec<T>) -> R,
    {
        let mut entities = self.readEntities()?;
        let result = transform(&mut entities);
        self.replaceEntities(&entities)?;
        Ok(result)
    }

    pub fn query(&self) -> ObjectBoxQueryBuilder<T> {
        ObjectBoxQueryBuilder {
            objectBox: self.clone(),
            predicates: Vec::new(),
        }
    }

    #[allow(non_snake_case)]
    pub fn applySyncedEntity(
        entityId: &str,
        operation: &str,
        payload: serde_json::Value,
    ) -> Result<(), ObjectBoxStoreError> {
        let (databaseStoragePath, id) = parseSyncedEntityId(entityId)?;
        let databasePath = RuntimeStorePaths::default().root_dir().join(databaseStoragePath);
        let sqliteStore = SqliteStore::open(databasePath.clone())?;
        initializeSchema(&sqliteStore)?;
        let entityType = payload
            .get("__entityType")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string)
            .ok_or_else(|| ObjectBoxStoreError::Message("ObjectBox sync payload missing __entityType".to_string()))?;
        match operation {
            "upsert" => {
                let entityPayload = payload
                    .get("entity")
                    .cloned()
                    .ok_or_else(|| ObjectBoxStoreError::Message("ObjectBox sync payload missing entity".to_string()))?;
                let mut entity: T = serde_json::from_value(entityPayload)?;
                entity.setObjectBoxId(id);
                let encoded = serde_json::to_string(&entity)?;
                sqliteStore.execute(
                    "INSERT INTO objectbox_entities(entity_type, id, payload, updated_at)
                     VALUES(?1, ?2, ?3, ?4)
                     ON CONFLICT(entity_type, id)
                     DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at",
                    vec![
                        toSqliteValue(&entityType),
                        toSqliteValue(&id),
                        toSqliteValue(&encoded),
                        toSqliteValue(&nowMillis()),
                    ],
                )?;
            }
            "delete" => {
                sqliteStore.execute(
                    "DELETE FROM objectbox_entities WHERE entity_type = ?1 AND id = ?2",
                    vec![toSqliteValue(&entityType), toSqliteValue(&id)],
                )?;
            }
            other => {
                return Err(ObjectBoxStoreError::Message(format!(
                    "unsupported ObjectBox sync operation: {other}"
                )));
            }
        }
        let signal = objectBoxChangeSignal(&databasePath);
        let mut version = signal
            .version
            .lock()
            .expect("ObjectBox version mutex must not be poisoned");
        *version += 1;
        signal.changed.notify_all();
        Ok(())
    }

    fn readEntities(&self) -> Result<Vec<T>, ObjectBoxStoreError> {
        let rows = self.sqliteStore.queryRows(
            "SELECT payload FROM objectbox_entities WHERE entity_type = ?1 ORDER BY id ASC",
            vec![toSqliteValue(&self.entityType)],
        )?;
        rows.into_iter()
            .map(|row| {
                let payload: String = row.get("payload")?;
                Ok(serde_json::from_str(&payload)?)
            })
            .collect()
    }

    fn replaceEntities(&self, entities: &[T]) -> Result<(), ObjectBoxStoreError> {
        self.sqliteStore.transaction(|transaction| {
            transaction.execute(
                "DELETE FROM objectbox_entities WHERE entity_type = ?1",
                vec![toSqliteValue(&self.entityType)],
            )?;
            for entity in entities {
                let payload = serde_json::to_string(entity)
                    .map_err(|error| SqliteStoreError::Message(error.to_string()))?;
                transaction.execute(
                    "INSERT INTO objectbox_entities(entity_type, id, payload, updated_at)
                     VALUES(?1, ?2, ?3, ?4)",
                    vec![
                        toSqliteValue(&self.entityType),
                        toSqliteValue(&entity.objectBoxId()),
                        toSqliteValue(&payload),
                        toSqliteValue(&nowMillis()),
                    ],
                )?;
            }
            Ok(())
        })?;
        for entity in entities {
            self.recordUpsertOperation(entity)?;
        }
        self.notifyChanged();
        Ok(())
    }

    #[allow(non_snake_case)]
    fn recordUpsertOperation(&self, entity: &T) -> Result<(), ObjectBoxStoreError> {
        let deviceId = self.syncOperationStore.localDeviceId()?;
        self.syncOperationStore.appendLocalOperation(
            &deviceId,
            NewSyncOperation {
                domain: OBJECTBOX_SYNC_DOMAIN.to_string(),
                entityType: self.entityType.clone(),
                entityId: self.syncedEntityId(entity.objectBoxId()),
                operation: "upsert".to_string(),
                payload: serde_json::json!({
                    "__entityType": self.entityType,
                    "entity": entity,
                }),
            },
        )?;
        Ok(())
    }

    #[allow(non_snake_case)]
    fn recordDeleteOperation(&self, id: i64) -> Result<(), ObjectBoxStoreError> {
        let deviceId = self.syncOperationStore.localDeviceId()?;
        self.syncOperationStore.appendLocalOperation(
            &deviceId,
            NewSyncOperation {
                domain: OBJECTBOX_SYNC_DOMAIN.to_string(),
                entityType: self.entityType.clone(),
                entityId: self.syncedEntityId(id),
                operation: "delete".to_string(),
                payload: serde_json::json!({
                    "__entityType": self.entityType,
                }),
            },
        )?;
        Ok(())
    }

    #[allow(non_snake_case)]
    fn syncedEntityId(&self, id: i64) -> String {
        format!("{}#{id}", self.databaseStoragePath)
    }

    #[allow(non_snake_case)]
    fn notifyChanged(&self) {
        let mut version = self
            .changeSignal
            .version
            .lock()
            .expect("ObjectBox version mutex must not be poisoned");
        *version += 1;
        self.changeSignal.changed.notify_all();
    }
}

pub struct ObjectBoxQueryBuilder<T>
where
    T: ObjectBoxEntity + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    objectBox: ObjectBox<T>,
    predicates: Vec<Box<dyn Fn(&T) -> bool + Send + Sync>>,
}

impl<T> ObjectBoxQueryBuilder<T>
where
    T: ObjectBoxEntity + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    pub fn filter<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        self.predicates.push(Box::new(predicate));
        self
    }

    pub fn build(self) -> ObjectBoxQuery<T> {
        ObjectBoxQuery {
            objectBox: self.objectBox,
            predicates: self.predicates,
        }
    }
}

pub struct ObjectBoxQuery<T>
where
    T: ObjectBoxEntity + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    objectBox: ObjectBox<T>,
    predicates: Vec<Box<dyn Fn(&T) -> bool + Send + Sync>>,
}

impl<T> ObjectBoxQuery<T>
where
    T: ObjectBoxEntity + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    pub fn find(&self) -> Result<Vec<T>, ObjectBoxStoreError> {
        Ok(self
            .objectBox
            .all()?
            .into_iter()
            .filter(|entity| self.predicates.iter().all(|predicate| predicate(entity)))
            .collect())
    }

    #[allow(non_snake_case)]
    pub fn findFirst(&self) -> Result<Option<T>, ObjectBoxStoreError> {
        Ok(self.find()?.into_iter().next())
    }

    #[allow(non_snake_case)]
    pub fn findUnique(&self) -> Result<Option<T>, ObjectBoxStoreError> {
        let found = self.find()?;
        match found.len() {
            0 => Ok(None),
            1 => Ok(found.into_iter().next()),
            count => Err(ObjectBoxStoreError::Message(format!(
                "ObjectBox query expected one result, got {count}"
            ))),
        }
    }
}

#[allow(non_snake_case)]
fn initializeSchema(sqliteStore: &SqliteStore) -> Result<(), SqliteStoreError> {
    sqliteStore.executeBatch(
        r#"
        CREATE TABLE IF NOT EXISTS objectbox_entities(
            entity_type TEXT NOT NULL,
            id INTEGER NOT NULL,
            payload TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY(entity_type, id)
        );
        CREATE INDEX IF NOT EXISTS idx_objectbox_entities_updated_at
            ON objectbox_entities(entity_type, updated_at);
        "#,
    )
}

#[allow(non_snake_case)]
fn nextObjectBoxId(
    transaction: &mut crate::SqliteStore::SqliteTransaction<'_>,
    entityType: &str,
) -> Result<i64, SqliteStoreError> {
    let row = transaction.queryOne(
        "SELECT COALESCE(MAX(id), 0) + 1 FROM objectbox_entities WHERE entity_type = ?1",
        vec![toSqliteValue(entityType)],
    )?;
    let row = row.ok_or_else(|| SqliteStoreError::Message("ObjectBox id query returned no row".to_string()))?;
    row.get(0)
}

#[allow(non_snake_case)]
fn parseSyncedEntityId(entityId: &str) -> Result<(String, i64), ObjectBoxStoreError> {
    let Some((databaseStoragePath, idText)) = entityId.rsplit_once('#') else {
        return Err(ObjectBoxStoreError::Message(format!(
            "invalid ObjectBox sync entity id: {entityId}"
        )));
    };
    let id = idText
        .parse::<i64>()
        .map_err(|error| ObjectBoxStoreError::Message(error.to_string()))?;
    Ok((databaseStoragePath.to_string(), id))
}

#[allow(non_snake_case)]
fn nowMillis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis() as i64
}

#[allow(non_snake_case)]
fn objectBoxChangeSignal(path: &Path) -> Arc<ObjectBoxChangeSignal> {
    static CHANGE_SIGNALS: OnceLock<Mutex<HashMap<PathBuf, Weak<ObjectBoxChangeSignal>>>> =
        OnceLock::new();
    let signals = CHANGE_SIGNALS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut signals = signals
        .lock()
        .expect("ObjectBox change signal registry mutex must not be poisoned");
    if let Some(signal) = signals.get(path).and_then(Weak::upgrade) {
        return signal;
    }
    let signal = Arc::new(ObjectBoxChangeSignal {
        version: Mutex::new(0),
        changed: Condvar::new(),
    });
    signals.insert(path.to_path_buf(), Arc::downgrade(&signal));
    signal
}
