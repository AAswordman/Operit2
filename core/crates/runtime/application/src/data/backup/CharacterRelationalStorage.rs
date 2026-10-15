//! Typed character tables shared by upgrade/import writers and the plugin repository.
use operit_host_api::PluginStorage::{
    StorageColumn, StorageColumnType, StorageKind, StorageRequest, StorageResponse,
    StorageSqlValue, StorageStatement, StorageTable,
};
use operit_host_api::{RuntimeSqliteHost, RuntimeStorageHost};
use operit_store::PluginStorage::PluginStorageSession;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Clone, Deserialize)]
/// Describes declared typed fields and ordered child relations, never stored object payloads.
struct Node {
    kind: String,
    #[serde(default)]
    nullable: bool,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    fields: BTreeMap<String, Node>,
    item: Option<Box<Node>>,
}
#[derive(Deserialize)]
/// Selects the exact independently versioned character SQL schema.
struct Schema {
    version: i64,
    collections: BTreeMap<String, Node>,
}
#[derive(Clone)]
/// Retains the ordered physical tables corresponding to one domain record.
struct Layout {
    table: StorageTable,
    node: Node,
    children: Vec<Layout>,
}
type Row = BTreeMap<String, StorageSqlValue>;

/// Quotes only identifiers generated from the checked-in schema.
fn quote(name: &str) -> String {
    format!("\"{name}\"")
}
/// Assigns a scalar item its explicit column name.
fn field(path: &str) -> &str {
    if path.is_empty() {
        "_value"
    } else {
        path
    }
}
/// Produces the same flattened object paths as the TypeScript codec.
fn path(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.into()
    } else {
        format!("{parent}__{name}")
    }
}
/// Creates a portable table column from a declared field.
fn column(name: &str, affinity: StorageColumnType, nullable: bool) -> StorageColumn {
    StorageColumn {
        name: name.into(),
        affinity,
        nullable,
    }
}
/// Builds exact typed parent and child table declarations.
fn layout(name: String, node: Node, child: bool) -> Layout {
    let mut columns = vec![
        column("_key", StorageColumnType::Text, false),
        column("_revision", StorageColumnType::Text, false),
    ];
    if child {
        columns.extend([
            column("_root", StorageColumnType::Text, false),
            column("_parent", StorageColumnType::Text, false),
            column("_position", StorageColumnType::Integer, false),
            column("_map_key", StorageColumnType::Text, true),
        ]);
    }
    let mut children = Vec::new();
    /// Flattens objects while declaring each collection as a separate table.
    fn visit(
        name: &str,
        node: &Node,
        prefix: &str,
        absent: bool,
        columns: &mut Vec<StorageColumn>,
        children: &mut Vec<Layout>,
    ) {
        if node.optional || node.nullable {
            columns.push(column(
                &format!("{}__state", field(prefix)),
                StorageColumnType::Integer,
                absent,
            ));
        }
        let nullable = absent || node.optional || node.nullable;
        match node.kind.as_str() {
            "object" => {
                for (key, value) in &node.fields {
                    visit(name, value, &path(prefix, key), nullable, columns, children);
                }
            }
            "array" | "map" => {
                columns.push(column(
                    &format!("{}__count", field(prefix)),
                    StorageColumnType::Integer,
                    nullable,
                ));
                children.push(layout(
                    format!("{name}_{}", field(prefix)),
                    (**node.item.as_ref().expect("Declared collection item")).clone(),
                    true,
                ));
            }
            kind => columns.push(column(
                field(prefix),
                match kind {
                    "text" => StorageColumnType::Text,
                    "boolean" | "integer" => StorageColumnType::Integer,
                    "vector" => StorageColumnType::Blob,
                    _ => StorageColumnType::Real,
                },
                nullable,
            )),
        }
    }
    visit(&name, &node, "", false, &mut columns, &mut children);
    Layout {
        table: StorageTable {
            name,
            primary_key: "_key".into(),
            columns,
        },
        node,
        children,
    }
}
/// Enumerates physical tables in declaration order.
fn tables(root: &Layout) -> Vec<&Layout> {
    let mut result = vec![root];
    for child in &root.children {
        result.extend(tables(child));
    }
    result
}
/// Encodes a relation identity without ambiguous delimiter parsing.
fn child_key(parent: &str, index: usize) -> String {
    format!("{}:{parent}:{index}", parent.encode_utf16().count())
}
/// Converts one finite field into its lossless SQLite representation.
fn scalar(node: &Node, value: &Value) -> Result<StorageSqlValue, String> {
    match node.kind.as_str() {
        "text" => Ok(StorageSqlValue::Text(
            value.as_str().ok_or("Expected text field")?.into(),
        )),
        "boolean" => Ok(StorageSqlValue::Integer(
            if value.as_bool().ok_or("Expected boolean field")? {
                "1"
            } else {
                "0"
            }
            .into(),
        )),
        "integer" => Ok(StorageSqlValue::Integer(
            value.as_i64().ok_or("Expected integer field")?.to_string(),
        )),
        "number" => Ok(StorageSqlValue::Real(
            value.as_f64().ok_or("Expected numeric field")?,
        )),
        "vector" => {
            let mut bytes = Vec::new();
            for item in value.as_array().ok_or("Expected embedding vector")? {
                bytes.extend_from_slice(
                    &item
                        .as_f64()
                        .ok_or("Invalid embedding component")?
                        .to_le_bytes(),
                );
            }
            Ok(StorageSqlValue::Blob(bytes))
        }
        _ => Err("Unknown declared scalar".into()),
    }
}
/// Encodes one complete root and its child rows using declared fields only.
fn encode(
    root: &Layout,
    key: &str,
    value: &Value,
    revision: &str,
) -> Result<BTreeMap<String, Vec<Row>>, String> {
    let mut rows = tables(root)
        .into_iter()
        .map(|table| (table.table.name.clone(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    /// Projects one item and all its nested ordered relations.
    fn record(
        current: &Layout,
        root_key: &str,
        key: &str,
        input: &Value,
        revision: &str,
        extra: Row,
        rows: &mut BTreeMap<String, Vec<Row>>,
    ) -> Result<(), String> {
        let mut row = extra;
        row.insert("_key".into(), StorageSqlValue::Text(key.into()));
        row.insert("_revision".into(), StorageSqlValue::Text(revision.into()));
        /// Validates every field, preserving absent and explicitly null objects separately.
        fn visit(
            current: &Layout,
            node: &Node,
            prefix: &str,
            input: Option<&Value>,
            absent: bool,
            root_key: &str,
            key: &str,
            revision: &str,
            row: &mut Row,
            rows: &mut BTreeMap<String, Vec<Row>>,
        ) -> Result<(), String> {
            if node.optional || node.nullable {
                row.insert(
                    format!("{}__state", field(prefix)),
                    StorageSqlValue::Integer(
                        match input {
                            None => "0",
                            Some(Value::Null) => "1",
                            _ => "2",
                        }
                        .into(),
                    ),
                );
            }
            if !absent && input.is_none() && !node.optional {
                return Err(format!("Missing field {prefix}"));
            }
            if !absent && input.is_some_and(Value::is_null) && !node.nullable {
                return Err(format!("Null field {prefix}"));
            }
            let empty = absent || input.is_none() || input.is_some_and(Value::is_null);
            match node.kind.as_str() {
                "object" => {
                    let object = if empty {
                        None
                    } else {
                        Some(
                            input
                                .unwrap()
                                .as_object()
                                .ok_or_else(|| format!("Expected object {prefix}"))?,
                        )
                    };
                    if let Some(object) = object {
                        for key in object.keys() {
                            if !node.fields.contains_key(key) {
                                return Err(format!("Undeclared field {prefix}.{key}"));
                            }
                        }
                    }
                    for (name, schema) in &node.fields {
                        visit(
                            current,
                            schema,
                            &path(prefix, name),
                            object.and_then(|object| object.get(name)),
                            empty,
                            root_key,
                            key,
                            revision,
                            row,
                            rows,
                        )?;
                    }
                }
                "array" | "map" => {
                    if empty {
                        row.insert(format!("{}__count", field(prefix)), StorageSqlValue::Null);
                        return Ok(());
                    }
                    let child_name = format!("{}_{}", current.table.name, field(prefix));
                    let child = current
                        .children
                        .iter()
                        .find(|child| child.table.name == child_name)
                        .ok_or("Missing child table")?;
                    let items = if node.kind == "array" {
                        input
                            .unwrap()
                            .as_array()
                            .ok_or("Expected array")?
                            .iter()
                            .map(|item| (None, item))
                            .collect::<Vec<_>>()
                    } else {
                        input
                            .unwrap()
                            .as_object()
                            .ok_or("Expected map")?
                            .iter()
                            .map(|(key, item)| (Some(key), item))
                            .collect()
                    };
                    row.insert(
                        format!("{}__count", field(prefix)),
                        StorageSqlValue::Integer(items.len().to_string()),
                    );
                    for (index, (map_key, item)) in items.into_iter().enumerate() {
                        let extra = BTreeMap::from([
                            ("_root".into(), StorageSqlValue::Text(root_key.into())),
                            ("_parent".into(), StorageSqlValue::Text(key.into())),
                            (
                                "_position".into(),
                                StorageSqlValue::Integer(index.to_string()),
                            ),
                            (
                                "_map_key".into(),
                                map_key
                                    .map(|key| StorageSqlValue::Text(key.clone()))
                                    .unwrap_or(StorageSqlValue::Null),
                            ),
                        ]);
                        record(
                            child,
                            root_key,
                            &child_key(key, index),
                            item,
                            revision,
                            extra,
                            rows,
                        )?;
                    }
                }
                _ => {
                    row.insert(
                        field(prefix).into(),
                        if empty {
                            StorageSqlValue::Null
                        } else {
                            scalar(node, input.unwrap())?
                        },
                    );
                }
            }
            Ok(())
        }
        visit(
            current,
            &current.node,
            "",
            Some(input),
            false,
            root_key,
            key,
            revision,
            &mut row,
            rows,
        )?;
        rows.get_mut(&current.table.name)
            .ok_or("Missing table row set")?
            .push(row);
        Ok(())
    }
    record(root, key, key, value, revision, Row::new(), &mut rows)?;
    Ok(rows)
}
/// Reads one exact scalar field from a typed query row.
fn text(row: &Row, name: &str) -> Result<String, String> {
    match row.get(name) {
        Some(StorageSqlValue::Text(value)) => Ok(value.clone()),
        _ => Err(format!("Invalid text column {name}")),
    }
}
/// Reads a field-state or ordered position as an exact integer.
fn integer(row: &Row, name: &str) -> Result<i64, String> {
    match row.get(name) {
        Some(StorageSqlValue::Integer(value)) => value.parse::<i64>().map_err(|e| e.to_string()),
        _ => Err(format!("Invalid integer column {name}")),
    }
}
/// Reconstructs one record solely from typed root columns and child relations.
fn decode(current: &Layout, row: &Row, rows: &BTreeMap<String, Vec<Row>>) -> Result<Value, String> {
    /// Reconstructs a declared field without consulting a legacy source.
    fn visit(
        current: &Layout,
        node: &Node,
        prefix: &str,
        row: &Row,
        rows: &BTreeMap<String, Vec<Row>>,
    ) -> Result<Option<Value>, String> {
        if node.optional || node.nullable {
            match integer(row, &format!("{}__state", field(prefix)))? {
                0 if node.optional => return Ok(None),
                1 if node.nullable => return Ok(Some(Value::Null)),
                2 => {}
                _ => return Err(format!("Invalid field presence {prefix}")),
            }
        }
        let value = match node.kind.as_str() {
            "object" => {
                let mut result = Map::new();
                for (key, child) in &node.fields {
                    if let Some(value) = visit(current, child, &path(prefix, key), row, rows)? {
                        result.insert(key.clone(), value);
                    }
                }
                Value::Object(result)
            }
            "array" | "map" => {
                let name = format!("{}_{}", current.table.name, field(prefix));
                let child = current
                    .children
                    .iter()
                    .find(|child| child.table.name == name)
                    .ok_or("Missing child table")?;
                let key = text(row, "_key")?;
                let root_revision = text(row, "_revision")?;
                let mut children = rows[&name].iter().filter(|row| matches!(row.get("_parent"), Some(StorageSqlValue::Text(parent)) if parent == &key)).map(|row| Ok((integer(row, "_position")?, row))).collect::<Result<Vec<_>, String>>()?;
                children.sort_by_key(|(position, _)| *position);
                if integer(row, &format!("{}__count", field(prefix)))? != children.len() as i64 {
                    return Err(format!("Incomplete relation {name}"));
                }
                let mut items = Vec::new();
                let mut map = Map::new();
                for (index, (position, row)) in children.into_iter().enumerate() {
                    if position != index as i64
                        || text(row, "_key")? != child_key(&key, index)
                        || text(row, "_revision")? != root_revision
                    {
                        return Err(format!("Incomplete ordered relation {name}"));
                    }
                    let value = decode(child, row, rows)?;
                    if node.kind == "array" {
                        items.push(value);
                    } else if map.insert(text(row, "_map_key")?, value).is_some() {
                        return Err("Duplicate map key".into());
                    }
                }
                if node.kind == "array" {
                    Value::Array(items)
                } else {
                    Value::Object(map)
                }
            }
            "text" => json!(text(row, field(prefix))?),
            "boolean" => match integer(row, field(prefix))? {
                0 => json!(false),
                1 => json!(true),
                _ => return Err("Invalid boolean column".into()),
            },
            "vector" => {
                let Some(StorageSqlValue::Blob(bytes)) = row.get(field(prefix)) else {
                    return Err("Invalid vector column".into());
                };
                if bytes.len() % 8 != 0 {
                    return Err("Invalid vector length".into());
                }
                let vector = bytes
                    .chunks_exact(8)
                    .map(|bytes| f64::from_le_bytes(bytes.try_into().unwrap()))
                    .collect::<Vec<_>>();
                json!(vector)
            }
            "integer" => json!(integer(row, field(prefix))?),
            "number" => match row.get(field(prefix)) {
                Some(StorageSqlValue::Real(value)) => json!(value),
                _ => return Err(format!("Invalid real column {prefix}")),
            },
            kind => return Err(format!("Unknown schema kind {kind}")),
        };
        Ok(Some(value))
    }
    visit(current, &current.node, "", row, rows)?.ok_or("Missing root record".into())
}

/// Writes migrations through the same synchronized host SQL contract used by the plugin.
pub struct CharacterRelationalStorage {
    session: PluginStorageSession,
    sqlite: Arc<dyn RuntimeSqliteHost>,
    storage: Arc<dyn RuntimeStorageHost>,
    device: String,
    handle: String,
    layouts: BTreeMap<String, Layout>,
    sequence: AtomicU64,
    execution_id: String,
}
impl CharacterRelationalStorage {
    /// Declares the exact current tables without changing the released objects database contract.
    pub fn open(
        sqlite: Arc<dyn RuntimeSqliteHost>,
        storage: Arc<dyn RuntimeStorageHost>,
        device: String,
        directory: &str,
    ) -> Result<Self, String> {
        let schema: Schema = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../plugins/packages/buildin/character_cards/src/storage/schema.json"
        )))
        .map_err(|e| e.to_string())?;
        if schema.version != 1 {
            return Err("Unsupported relational schema".into());
        }
        let session = PluginStorageSession::default();
        let StorageResponse::Open(opened) = session
            .request(
                sqlite.clone(),
                storage.clone(),
                StorageRequest::Open {
                    path: format!("{directory}/characters-relational.sqlite"),
                    kind: StorageKind::Sqlite,
                },
                &device,
            )
            .map_err(|e| e.to_string())?
        else {
            return Err("Invalid SQL open result".into());
        };
        let result = Self {
            session,
            sqlite,
            storage,
            device,
            handle: opened.handle,
            layouts: schema
                .collections
                .into_iter()
                .map(|(name, node)| (name.clone(), layout(name, node, false)))
                .collect(),
            sequence: AtomicU64::new(0),
            execution_id: uuid::Uuid::new_v4().to_string(),
        };
        for root in result.layouts.values() {
            for table in tables(root) {
                result.request(StorageRequest::DefineTable {
                    handle: result.handle.clone(),
                    table: table.table.clone(),
                })?;
            }
            for child in tables(root).into_iter().skip(1) {
                result.request(StorageRequest::Execute {
                    handle: result.handle.clone(),
                    statement: StorageStatement {
                        sql: format!("CREATE INDEX IF NOT EXISTS {} ON {} (\"_root\", \"_parent\", \"_position\")", quote(&format!("{}_root_idx", child.table.name)), quote(&child.table.name)),
                        params: Vec::new(),
                        expected_affected_rows: None,
                    },
                })?;
            }
        }
        result.request(StorageRequest::Execute {
            handle: result.handle.clone(),
            statement: StorageStatement {
                sql: "CREATE INDEX IF NOT EXISTS \"chat_binding_selection_idx\" ON \"chat_bindings\" (\"selection\")".into(),
                params: Vec::new(),
                expected_affected_rows: None,
            },
        })?;
        Ok(result)
    }
    /// Propagates the original registered host storage error.
    fn request(&self, request: StorageRequest) -> Result<StorageResponse, String> {
        self.session
            .request(
                self.sqlite.clone(),
                self.storage.clone(),
                request,
                &self.device,
            )
            .map_err(|e| e.to_string())
    }
    /// Executes a typed read through the host handle.
    fn query(&self, sql: String, params: Vec<StorageSqlValue>) -> Result<Vec<Row>, String> {
        let StorageResponse::Query(rows) = self.request(StorageRequest::Query {
            handle: self.handle.clone(),
            statement: StorageStatement {
                sql,
                params,
                expected_affected_rows: None,
            },
        })?
        else {
            return Err("Invalid SQL query result".into());
        };
        Ok(rows
            .into_iter()
            .map(|row| row.columns.into_iter().zip(row.values).collect())
            .collect())
    }
    /// Reads one exact root and every corresponding child row.
    pub fn get(&self, collection: &str, key: &str) -> Result<Option<Value>, String> {
        Ok(self.read(collection, Some(key))?.into_iter().next())
    }
    /// Reads an ordered collection of reconstructed migration records.
    pub fn list(&self, collection: &str) -> Result<Vec<Value>, String> {
        self.read(collection, None)
    }
    /// Canonicalizes only declared REAL fields and vector components for exact migration comparisons.
    pub fn comparisonValue(&self, collection: &str, value: &Value) -> Result<Value, String> {
        /// Preserves every field and exact integer identity while applying the SQL numeric representation.
        fn normalize(node: &Node, value: &mut Value) -> Result<(), String> {
            if value.is_null() && node.nullable {
                return Ok(());
            }
            match node.kind.as_str() {
                "object" => {
                    let object = value.as_object_mut().ok_or("Expected object field")?;
                    for (key, child) in &node.fields {
                        if let Some(value) = object.get_mut(key) {
                            normalize(child, value)?;
                        }
                    }
                }
                "array" => {
                    let item = node.item.as_ref().ok_or("Missing array schema")?;
                    for value in value.as_array_mut().ok_or("Expected array field")? {
                        normalize(item, value)?;
                    }
                }
                "map" => {
                    let item = node.item.as_ref().ok_or("Missing map schema")?;
                    for value in value
                        .as_object_mut()
                        .ok_or("Expected map field")?
                        .values_mut()
                    {
                        normalize(item, value)?;
                    }
                }
                "number" => {
                    *value = json!(value.as_f64().ok_or("Expected numeric field")?);
                }
                "vector" => {
                    for value in value.as_array_mut().ok_or("Expected embedding vector")? {
                        *value = json!(value.as_f64().ok_or("Invalid embedding component")?);
                    }
                }
                _ => {}
            }
            Ok(())
        }
        let root = self
            .layouts
            .get(collection)
            .ok_or_else(|| format!("Unknown relational collection {collection}"))?;
        let mut result = value.clone();
        normalize(&root.node, &mut result)?;
        Ok(result)
    }
    /// Reconstructs typed rows for migration verification and exact version checks.
    fn read(&self, collection: &str, key: Option<&str>) -> Result<Vec<Value>, String> {
        let root = self
            .layouts
            .get(collection)
            .ok_or_else(|| format!("Unknown relational collection {collection}"))?;
        let mut rows = BTreeMap::new();
        for table in tables(root) {
            let condition = key
                .map(|_| {
                    format!(
                        " WHERE {} = ?",
                        quote(if table.table.name == collection {
                            "_key"
                        } else {
                            "_root"
                        })
                    )
                })
                .unwrap_or_default();
            let params = key
                .map(|key| vec![StorageSqlValue::Text(key.into())])
                .unwrap_or_default();
            rows.insert(
                table.table.name.clone(),
                self.query(
                    format!(
                        "SELECT * FROM {}{condition} ORDER BY \"_key\"",
                        quote(&table.table.name)
                    ),
                    params,
                )?,
            );
        }
        rows[collection].iter().map(|row| Ok(json!({ "key": text(row, "_key")?, "version": text(row, "_revision")?, "value": decode(root, row, &rows)? }))).collect()
    }
    /// Commits exact-version root changes before their descendants in one host transaction.
    pub fn put(&self, records: &[(String, String, Value, Option<String>)]) -> Result<(), String> {
        let revision = format!(
            "{}:{}:{}",
            self.device,
            self.execution_id,
            self.sequence.fetch_add(1, Ordering::Relaxed)
        );
        let mut statements = Vec::new();
        for (collection, key, value, expected) in records {
            let root = self
                .layouts
                .get(collection)
                .ok_or("Unknown relational collection")?;
            let encoded = encode(root, key, value, &revision)?;
            let row = &encoded[collection][0];
            let statement = match expected {
                None => insert_row(collection, row),
                Some(expected) => {
                    let columns = row
                        .keys()
                        .filter(|name| name.as_str() != "_key")
                        .collect::<Vec<_>>();
                    let mut params = columns
                        .iter()
                        .map(|name| row[*name].clone())
                        .collect::<Vec<_>>();
                    params.extend([
                        StorageSqlValue::Text(key.clone()),
                        StorageSqlValue::Text(expected.clone()),
                    ]);
                    StorageStatement {
                        sql: format!(
                            "UPDATE {} SET {} WHERE \"_key\" = ? AND \"_revision\" = ?",
                            quote(collection),
                            columns
                                .iter()
                                .map(|name| format!("{} = ?", quote(name)))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        params,
                        expected_affected_rows: Some(1),
                    }
                }
            };
            statements.push(statement);
            if expected.is_some() {
                for child in tables(root).into_iter().skip(1).rev() {
                    statements.push(StorageStatement {
                        sql: format!(
                            "DELETE FROM {} WHERE \"_root\" = ?",
                            quote(&child.table.name)
                        ),
                        params: vec![StorageSqlValue::Text(key.clone())],
                        expected_affected_rows: None,
                    });
                }
            }
            for table in tables(root).into_iter().skip(1) {
                for row in &encoded[&table.table.name] {
                    statements.push(insert_row(&table.table.name, row));
                }
            }
        }
        if statements.is_empty() || statements.len() > 1000 {
            return Err("Relational migration batch exceeds 1000 SQL statements".into());
        }
        self.request(StorageRequest::Transaction {
            handle: self.handle.clone(),
            statements,
        })?;
        Ok(())
    }

    /// Deletes a prepared or retired record with an exact transaction-local revision check.
    pub fn delete(&self, collection: &str, key: &str, revision: &str) -> Result<(), String> {
        let root = self
            .layouts
            .get(collection)
            .ok_or("Unknown relational collection")?;
        let mut statements = vec![StorageStatement {
            sql: format!(
                "DELETE FROM {} WHERE \"_revision\" = ? AND \"_key\" = ?",
                quote(collection)
            ),
            params: vec![
                StorageSqlValue::Text(revision.into()),
                StorageSqlValue::Text(key.into()),
            ],
            expected_affected_rows: Some(1),
        }];
        for child in tables(root).into_iter().skip(1).rev() {
            statements.push(StorageStatement {
                sql: format!(
                    "DELETE FROM {} WHERE \"_root\" = ?",
                    quote(&child.table.name)
                ),
                params: vec![StorageSqlValue::Text(key.into())],
                expected_affected_rows: None,
            });
        }
        self.request(StorageRequest::Transaction {
            handle: self.handle.clone(),
            statements,
        })?;
        Ok(())
    }
}
impl Drop for CharacterRelationalStorage {
    /// Releases the execution handle without deleting synchronized durable data.
    fn drop(&mut self) {
        let _ = self.request(StorageRequest::Close {
            handle: self.handle.clone(),
        });
    }
}

/// Inserts one typed row and requires exactly one affected identity.
fn insert_row(table: &str, row: &Row) -> StorageStatement {
    let columns = row
        .keys()
        .map(|name| quote(name))
        .collect::<Vec<_>>()
        .join(", ");
    let slots = vec!["?"; row.len()].join(", ");
    StorageStatement {
        sql: format!("INSERT INTO {} ({columns}) VALUES ({slots})", quote(table)),
        params: row.values().cloned().collect(),
        expected_affected_rows: Some(1),
    }
}
