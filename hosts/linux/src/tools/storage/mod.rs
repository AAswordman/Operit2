use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

use operit_host_api::{
    HostError, HostResult, HostSecretStore, RuntimeSqliteConnection, RuntimeSqliteHost,
    RuntimeSqliteTransaction, RuntimeStorageEntry, RuntimeStorageHost, SqliteRow, SqliteValue,
};
use rusqlite::types::Value;
#[cfg(target_os = "linux")]
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use zbus::blocking::{Connection, Proxy};
#[cfg(target_os = "linux")]
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value as ZbusValue};

#[derive(Clone, Debug)]
pub struct LinuxRuntimeStorageHost {
    root: PathBuf,
    runtimeRoot: PathBuf,
    workspaceRoot: PathBuf,
}

impl LinuxRuntimeStorageHost {
    #[allow(non_snake_case)]
    pub fn defaultRoot() -> PathBuf {
        if let Some(xdg_data_home) = env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(xdg_data_home).join("operit2");
        }
        let home = env::var_os("HOME").expect("HOME is required for Operit2 runtime storage");
        PathBuf::from(home).join(".local").join("share").join("operit2")
    }

    pub fn new(root: PathBuf) -> Self {
        let runtimeRoot = root.join("runtime");
        let workspaceRoot = root.join("workspaces");
        Self::newWithRoots(root, runtimeRoot, workspaceRoot)
    }

    #[allow(non_snake_case)]
    pub fn newWithRoots(root: PathBuf, runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> Self {
        Self {
            root,
            runtimeRoot,
            workspaceRoot,
        }
    }

    fn resolve(&self, path: &str) -> HostResult<PathBuf> {
        let normalized = normalizeStoragePath(path)?;
        let segments = normalized.iter().map(String::as_str).collect::<Vec<_>>();
        match segments.as_slice() {
            ["runtime", rest @ ..] => Ok(joinSegments(&self.runtimeRoot, rest)),
            ["workspaces", rest @ ..] => Ok(joinSegments(&self.workspaceRoot, rest)),
            _ => Ok(joinSegments(&self.root, &segments)),
        }
    }

    fn storagePathForPhysical(&self, path: &Path) -> HostResult<String> {
        if let Ok(relative) = path.strip_prefix(&self.runtimeRoot) {
            return Ok(prefixedPath("runtime", relative));
        }
        if let Ok(relative) = path.strip_prefix(&self.workspaceRoot) {
            return Ok(prefixedPath("workspaces", relative));
        }
        Ok(path
            .strip_prefix(&self.root)
            .map_err(|error| HostError::new(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/"))
    }
}

impl RuntimeStorageHost for LinuxRuntimeStorageHost {
    fn rootDir(&self) -> Option<PathBuf> {
        Some(self.root.clone())
    }

    fn runtimeRootDir(&self) -> Option<PathBuf> {
        Some(self.runtimeRoot.clone())
    }

    fn workspaceRootDir(&self) -> Option<PathBuf> {
        Some(self.workspaceRoot.clone())
    }

    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        Ok(fs::read(self.resolve(path)?)?)
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        let path = self.resolve(path)?;
        if !path.exists() {
            return Ok(());
        }
        if path.is_dir() {
            if recursive {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_dir(path)?;
            }
        } else {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self.resolve(path)?.exists())
    }

    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        let directory = self.resolve(prefix)?;
        let mut entries = Vec::new();
        if !directory.exists() {
            return Ok(entries);
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            entries.push(RuntimeStorageEntry {
                path: self.storagePathForPhysical(&entry.path())?,
                isDirectory: metadata.is_dir(),
                size: metadata.len() as i64,
            });
        }
        Ok(entries)
    }
}

impl HostSecretStore for LinuxRuntimeStorageHost {
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        linuxReadSecret(key)
    }

    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        linuxWriteSecret(key, content)
    }

    fn deleteSecret(&self, key: &str) -> HostResult<()> {
        linuxDeleteSecret(key)
    }
}

fn normalizeStoragePath(path: &str) -> HostResult<Vec<String>> {
    let path = Path::new(path);
    if path.is_absolute() {
        return Err(HostError::new(format!(
            "Runtime storage path must be relative: {}",
            path.display()
        )));
    }
    let mut segments = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => segments.push(segment.to_string_lossy().to_string()),
            Component::CurDir => {}
            _ => {
                return Err(HostError::new(format!(
                    "Invalid runtime storage path: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(segments)
}

fn joinSegments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut resolved = root.to_path_buf();
    for segment in segments {
        resolved.push(segment);
    }
    resolved
}

fn prefixedPath(prefix: &str, relative: &Path) -> String {
    let relative = relative.to_string_lossy().replace('\\', "/");
    if relative.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{relative}")
    }
}

fn validateSecretKey(key: &str) -> HostResult<String> {
    if key.is_empty()
        || key.chars().any(|character| {
            !(character.is_ascii_alphanumeric()
                || character == '.'
                || character == '_'
                || character == '-')
        })
    {
        return Err(HostError::new(format!("invalid host secret key: {key}")));
    }
    Ok(key.to_string())
}

#[cfg(target_os = "linux")]
const SECRET_SERVICE_DESTINATION: &str = "org.freedesktop.secrets";
#[cfg(target_os = "linux")]
const SECRET_SERVICE_PATH: &str = "/org/freedesktop/secrets";
#[cfg(target_os = "linux")]
const SECRET_SERVICE_INTERFACE: &str = "org.freedesktop.Secret.Service";
#[cfg(target_os = "linux")]
const SECRET_COLLECTION_INTERFACE: &str = "org.freedesktop.Secret.Collection";
#[cfg(target_os = "linux")]
const SECRET_ITEM_INTERFACE: &str = "org.freedesktop.Secret.Item";
#[cfg(target_os = "linux")]
const SECRET_SESSION_PATH: &str = "/org/freedesktop/secrets/session/operit_plain";

#[cfg(target_os = "linux")]
#[derive(serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
struct SecretServiceSecret {
    session: OwnedObjectPath,
    parameters: Vec<u8>,
    value: Vec<u8>,
    content_type: String,
}

#[cfg(target_os = "linux")]
/// Reads a host secret through the Linux Secret Service session bus.
fn linuxReadSecret(key: &str) -> HostResult<Option<Vec<u8>>> {
    let key = validateSecretKey(key)?;
    let connection = Connection::session()
        .map_err(|error| HostError::new(format!("connect Secret Service failed: {error}")))?;
    let service = secretServiceProxy(&connection)?;
    let (_, unlocked) = searchSecretItems(&service, &key)?;
    if unlocked.is_empty() {
        return Ok(None);
    }
    let item = secretItemProxy(&connection, &unlocked[0])?;
    let secret: SecretServiceSecret = item
        .call("GetSecret", &(plainSessionPath()?,))
        .map_err(|error| HostError::new(format!("read Secret Service item failed: {error}")))?;
    Ok(Some(secret.value))
}

#[cfg(not(target_os = "linux"))]
/// Rejects Linux Secret Service reads on non-Linux targets.
fn linuxReadSecret(_key: &str) -> HostResult<Option<Vec<u8>>> {
    Err(HostError::new("Linux Secret Service is only available on Linux"))
}

#[cfg(target_os = "linux")]
/// Writes a host secret through the Linux Secret Service session bus.
fn linuxWriteSecret(key: &str, content: &[u8]) -> HostResult<()> {
    let key = validateSecretKey(key)?;
    let connection = Connection::session()
        .map_err(|error| HostError::new(format!("connect Secret Service failed: {error}")))?;
    let service = secretServiceProxy(&connection)?;
    let collectionPath: OwnedObjectPath = service
        .call("ReadAlias", &("default",))
        .map_err(|error| HostError::new(format!("read default Secret Service collection failed: {error}")))?;
    let collection = secretCollectionProxy(&connection, &collectionPath)?;
    let attributes = secretAttributes(&key);
    let properties = secretItemProperties(&key, &attributes)?;
    let secret = SecretServiceSecret {
        session: plainSessionPath()?,
        parameters: Vec::new(),
        value: content.to_vec(),
        content_type: "application/octet-stream".to_string(),
    };
    let _: (OwnedObjectPath, OwnedObjectPath) = collection
        .call("CreateItem", &(properties, secret, true))
        .map_err(|error| HostError::new(format!("write Secret Service item failed: {error}")))?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
/// Rejects Linux Secret Service writes on non-Linux targets.
fn linuxWriteSecret(_key: &str, _content: &[u8]) -> HostResult<()> {
    Err(HostError::new("Linux Secret Service is only available on Linux"))
}

#[cfg(target_os = "linux")]
/// Deletes matching host secret items from the Linux Secret Service.
fn linuxDeleteSecret(key: &str) -> HostResult<()> {
    let key = validateSecretKey(key)?;
    let connection = Connection::session()
        .map_err(|error| HostError::new(format!("connect Secret Service failed: {error}")))?;
    let service = secretServiceProxy(&connection)?;
    let (locked, unlocked) = searchSecretItems(&service, &key)?;
    for itemPath in locked.into_iter().chain(unlocked.into_iter()) {
        let item = secretItemProxy(&connection, &itemPath)?;
        let _: OwnedObjectPath = item
            .call("Delete", &())
            .map_err(|error| HostError::new(format!("delete Secret Service item failed: {error}")))?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
/// Rejects Linux Secret Service deletes on non-Linux targets.
fn linuxDeleteSecret(_key: &str) -> HostResult<()> {
    Err(HostError::new("Linux Secret Service is only available on Linux"))
}

#[cfg(target_os = "linux")]
/// Creates a proxy for the Linux Secret Service root object.
fn secretServiceProxy(connection: &Connection) -> HostResult<Proxy<'_>> {
    Proxy::new(
        connection,
        SECRET_SERVICE_DESTINATION,
        SECRET_SERVICE_PATH,
        SECRET_SERVICE_INTERFACE,
    )
    .map_err(|error| HostError::new(format!("create Secret Service proxy failed: {error}")))
}

#[cfg(target_os = "linux")]
/// Creates a proxy for a Linux Secret Service collection object.
fn secretCollectionProxy<'a>(
    connection: &'a Connection,
    path: &'a OwnedObjectPath,
) -> HostResult<Proxy<'a>> {
    Proxy::new(
        connection,
        SECRET_SERVICE_DESTINATION,
        path.as_str(),
        SECRET_COLLECTION_INTERFACE,
    )
    .map_err(|error| HostError::new(format!("create Secret Service collection proxy failed: {error}")))
}

#[cfg(target_os = "linux")]
/// Creates a proxy for a Linux Secret Service item object.
fn secretItemProxy<'a>(connection: &'a Connection, path: &'a OwnedObjectPath) -> HostResult<Proxy<'a>> {
    Proxy::new(
        connection,
        SECRET_SERVICE_DESTINATION,
        path.as_str(),
        SECRET_ITEM_INTERFACE,
    )
    .map_err(|error| HostError::new(format!("create Secret Service item proxy failed: {error}")))
}

#[cfg(target_os = "linux")]
/// Searches Linux Secret Service items by the Operit host secret attributes.
fn searchSecretItems(
    service: &Proxy<'_>,
    key: &str,
) -> HostResult<(Vec<OwnedObjectPath>, Vec<OwnedObjectPath>)> {
    service
        .call("SearchItems", &(secretAttributes(key),))
        .map_err(|error| HostError::new(format!("search Secret Service items failed: {error}")))
}

#[cfg(target_os = "linux")]
/// Builds stable Secret Service attributes for one host secret key.
fn secretAttributes(key: &str) -> HashMap<String, String> {
    HashMap::from([
        ("application".to_string(), "operit2".to_string()),
        ("key".to_string(), key.to_string()),
    ])
}

#[cfg(target_os = "linux")]
/// Builds Secret Service item properties for a host secret item.
fn secretItemProperties(
    key: &str,
    attributes: &HashMap<String, String>,
) -> HostResult<HashMap<String, OwnedValue>> {
    Ok(HashMap::from([
        (
            format!("{SECRET_ITEM_INTERFACE}.Label"),
            OwnedValue::try_from(ZbusValue::from(format!("Operit2 {key}")))
                .map_err(|error| HostError::new(error.to_string()))?,
        ),
        (
            format!("{SECRET_ITEM_INTERFACE}.Attributes"),
            OwnedValue::try_from(ZbusValue::from(attributes.clone()))
                .map_err(|error| HostError::new(error.to_string()))?,
        ),
    ]))
}

#[cfg(target_os = "linux")]
/// Returns the plain Secret Service session object path used by this host.
fn plainSessionPath() -> HostResult<OwnedObjectPath> {
    OwnedObjectPath::try_from(SECRET_SESSION_PATH)
        .map_err(|error| HostError::new(format!("invalid Secret Service session path: {error}")))
}

impl RuntimeSqliteHost for LinuxRuntimeStorageHost {
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = rusqlite::Connection::open(path)
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(Box::new(RusqliteRuntimeConnection { connection }))
    }
}

struct RusqliteRuntimeConnection {
    connection: rusqlite::Connection,
}

impl RuntimeSqliteConnection for RusqliteRuntimeConnection {
    fn executeBatch(&mut self, sql: &str) -> HostResult<()> {
        self.connection
            .execute_batch(sql)
            .map_err(|error| HostError::new(error.to_string()))
    }

    fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
        let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
        self.connection
            .execute(sql, rusqlite::params_from_iter(params))
            .map_err(|error| HostError::new(error.to_string()))
    }

    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>> {
        queryRowsConnection(&self.connection, sql, params)
    }

    fn lastInsertRowId(&self) -> HostResult<i64> {
        Ok(self.connection.last_insert_rowid())
    }

    fn beginTransaction(&mut self) -> HostResult<Box<dyn RuntimeSqliteTransaction + '_>> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(Box::new(RusqliteRuntimeTransaction { transaction }))
    }
}

struct RusqliteRuntimeTransaction<'a> {
    transaction: rusqlite::Transaction<'a>,
}

impl RuntimeSqliteTransaction for RusqliteRuntimeTransaction<'_> {
    fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
        let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
        self.transaction
            .execute(sql, rusqlite::params_from_iter(params))
            .map_err(|error| HostError::new(error.to_string()))
    }

    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>> {
        queryRowsTransaction(&self.transaction, sql, params)
    }

    fn lastInsertRowId(&self) -> HostResult<i64> {
        Ok(self.transaction.last_insert_rowid())
    }

    fn commit(self: Box<Self>) -> HostResult<()> {
        self.transaction
            .commit()
            .map_err(|error| HostError::new(error.to_string()))
    }
}

fn queryRowsConnection(
    connection: &rusqlite::Connection,
    sql: &str,
    params: Vec<SqliteValue>,
) -> HostResult<Vec<SqliteRow>> {
    let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| HostError::new(error.to_string()))?;
    collectRows(&mut statement, params)
}

fn queryRowsTransaction(
    transaction: &rusqlite::Transaction<'_>,
    sql: &str,
    params: Vec<SqliteValue>,
) -> HostResult<Vec<SqliteRow>> {
    let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
    let mut statement = transaction
        .prepare(sql)
        .map_err(|error| HostError::new(error.to_string()))?;
    collectRows(&mut statement, params)
}

fn collectRows(
    statement: &mut rusqlite::Statement<'_>,
    params: Vec<Value>,
) -> HostResult<Vec<SqliteRow>> {
    let columns = statement
        .column_names()
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut rows = statement
        .query(rusqlite::params_from_iter(params))
        .map_err(|error| HostError::new(error.to_string()))?;
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|error| HostError::new(error.to_string()))?
    {
        let mut values = Vec::new();
        for index in 0..columns.len() {
            let value = row
                .get::<_, Value>(index)
                .map_err(|error| HostError::new(error.to_string()))?;
            values.push(fromRusqliteValue(value));
        }
        out.push(SqliteRow {
            columns: columns.clone(),
            values,
        });
    }
    Ok(out)
}

fn toRusqliteValue(value: SqliteValue) -> Value {
    match value {
        SqliteValue::Null => Value::Null,
        SqliteValue::Integer(value) => Value::Integer(value),
        SqliteValue::Real(value) => Value::Real(value),
        SqliteValue::Text(value) => Value::Text(value),
        SqliteValue::Blob(value) => Value::Blob(value),
    }
}

fn fromRusqliteValue(value: Value) -> SqliteValue {
    match value {
        Value::Null => SqliteValue::Null,
        Value::Integer(value) => SqliteValue::Integer(value),
        Value::Real(value) => SqliteValue::Real(value),
        Value::Text(value) => SqliteValue::Text(value),
        Value::Blob(value) => SqliteValue::Blob(value),
    }
}
