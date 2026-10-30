//! Storage-independent transaction algebra. SQL stores originals; views retain their identity/order.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[cfg(feature = "full")]
use crate::SqliteStore::SqliteRowGet;
pub use crate::SyncOperationStore::SyncOperationOrder as Revision;

pub type Result<T> = std::result::Result<T, String>;

#[path = "transaction/Checkpoint.rs"]
mod checkpoint;

/// Author sequence remains causal even when its wall clock moves backwards.
pub fn newer(left: &Revision, right: &Revision) -> bool {
    if left.originDeviceId == right.originDeviceId {
        left.sequence > right.sequence
    } else {
        left > right
    }
}

pub fn verify_original<T: PartialEq>(id: &str, existing: &T, incoming: &T) -> Result<()> {
    if existing != incoming {
        return Err(format!("Conflicting immutable transaction {id}"));
    }
    Ok(())
}

/// Immutable identity union followed by business ordering, preserving each author's sequence.
pub fn ordered<T: PartialEq>(
    items: impl IntoIterator<Item = T>,
    id: impl Fn(&T) -> &str,
    revision: impl Fn(&T) -> Revision,
) -> Result<Vec<T>> {
    let mut unique = BTreeMap::<String, T>::new();
    for item in items {
        let key = id(&item).to_owned();
        if let Some(previous) = unique.get(&key) {
            verify_original(&key, previous, &item)?;
        } else {
            unique.insert(key, item);
        }
    }
    let mut origins = BTreeMap::<String, BTreeMap<i64, (Revision, T)>>::new();
    for item in unique.into_values() {
        let order = revision(&item);
        if origins
            .entry(order.originDeviceId.clone())
            .or_default()
            .insert(order.sequence, (order, item))
            .is_some()
        {
            return Err("Conflicting transactions at the same origin sequence".into());
        }
    }
    let mut output = Vec::new();
    while let Some(origin) = origins
        .iter()
        .min_by_key(|(_, queue)| &queue.first_key_value().expect("nonempty queue").1 .0)
        .map(|(origin, _)| origin.clone())
    {
        let queue = origins.get_mut(&origin).expect("selected origin");
        output.push(queue.pop_first().expect("nonempty queue").1 .1);
        if queue.is_empty() {
            origins.remove(&origin);
        }
    }
    Ok(output)
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Backend {
    DataStore,
    Sql,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Owner {
    pub backend: Backend,
    pub path: String,
    pub format: u32,
    pub layout: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Target {
    Data(Owner),
    Transaction(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Path {
    Field(String),
    Item(String),
    Records,
}
impl From<&str> for Path {
    fn from(field: &str) -> Self {
        Self::Field(field.into())
    }
}
impl From<String> for Path {
    fn from(field: String) -> Self {
        Self::Field(field)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Edit {
    Set {
        key: String,
        path: Vec<Path>,
        value: Value,
    },
    Delete {
        key: String,
        path: Vec<Path>,
    },
    Increment {
        key: String,
        path: Vec<Path>,
        delta: i64,
    },
    Patch {
        key: String,
        set: BTreeMap<String, Value>,
        delete: BTreeSet<String>,
    },
    /// Atomic replacement of a record namespace, including absent identities.
    ReplaceRecords {
        prefix: String,
        records: BTreeMap<String, Value>,
    },
}

impl Edit {
    fn key(&self) -> &str {
        match self {
            Self::Set { key, .. }
            | Self::Delete { key, .. }
            | Self::Increment { key, .. }
            | Self::Patch { key, .. } => key,
            Self::ReplaceRecords { prefix, .. } => prefix,
        }
    }
    fn apply(&self, values: &mut BTreeMap<String, Value>) -> Result<()> {
        if let Self::ReplaceRecords { prefix, records } = self {
            values.retain(|key, _| !key.starts_with(prefix));
            values.extend(
                records
                    .iter()
                    .map(|(id, value)| (format!("{prefix}{id}"), value.clone())),
            );
            return Ok(());
        }
        if let Self::Patch { key, set, delete } = self {
            let mut value = values
                .get(key)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let object = value
                .as_object_mut()
                .ok_or("Object patch requires an object entry")?;
            for field in delete {
                object.remove(field);
            }
            object.extend(set.clone());
            values.insert(key.clone(), value);
            return Ok(());
        }
        let path = match self {
            Self::Set { path, .. } | Self::Delete { path, .. } | Self::Increment { path, .. } => {
                path
            }
            _ => unreachable!(),
        };
        if path.contains(&Path::Records) {
            return Err("Namespace scope is not a value path".into());
        }
        let key = self.key();
        if path.is_empty() {
            match self {
                Self::Delete { .. } => {
                    values.remove(key);
                }
                _ => {
                    values.insert(key.into(), self.next(values.get(key))?);
                }
            }
        } else {
            if !values.contains_key(key) {
                if matches!(self, Self::Delete { .. }) {
                    return Ok(());
                }
                values.insert(key.into(), container(&path[0])?);
            }
            apply_nested(values.get_mut(key).expect("edit root"), path, self)?;
        }
        Ok(())
    }
    fn next(&self, previous: Option<&Value>) -> Result<Value> {
        match self {
            Self::Set { value, .. } => Ok(value.clone()),
            Self::Increment { delta, .. } => increment(previous.unwrap_or(&Value::from(0)), *delta),
            _ => Err("Assignment expected".into()),
        }
    }
}

fn increment(value: &Value, delta: i64) -> Result<Value> {
    value
        .as_i64()
        .and_then(|value| value.checked_add(delta))
        .map(Value::from)
        .ok_or_else(|| "Integer edit requires an integer without overflow".into())
}

fn container(segment: &Path) -> Result<Value> {
    match segment {
        Path::Field(_) => Ok(serde_json::json!({})),
        Path::Item(_) => Ok(serde_json::json!([])),
        Path::Records => Err("Namespace scope is not a value path".into()),
    }
}

fn apply_nested(value: &mut Value, path: &[Path], edit: &Edit) -> Result<()> {
    let (segment, rest) = path.split_first().ok_or("Empty edit path")?;
    let compatible = match (&*value, segment) {
        (Value::Object(_), Path::Field(_)) | (Value::Array(_), Path::Item(_)) => true,
        (Value::Array(_), Path::Field(index)) => index.parse::<usize>().is_ok(),
        _ => false,
    };
    if !compatible {
        if matches!(edit, Edit::Delete { .. }) {
            return Ok(());
        }
        *value = container(segment)?;
    }
    match (value, segment) {
        (Value::Object(object), Path::Field(field)) => {
            if !rest.is_empty() {
                if !object.contains_key(field) {
                    if matches!(edit, Edit::Delete { .. }) {
                        return Ok(());
                    }
                    object.insert(field.clone(), container(&rest[0])?);
                }
                return apply_nested(object.get_mut(field).expect("edit field"), rest, edit);
            }
            match edit {
                Edit::Delete { .. } => {
                    object.remove(field);
                }
                _ => {
                    object.insert(field.clone(), edit.next(object.get(field))?);
                }
            }
        }
        (Value::Array(array), segment) => {
            let mut index = match segment {
                Path::Field(index) => Some(
                    index
                        .parse::<usize>()
                        .map_err(|_| "Array edit requires an index")?,
                ),
                Path::Item(id) => array
                    .iter()
                    .position(|item| item.get("id").and_then(Value::as_str) == Some(id)),
                Path::Records => return Err("Namespace scope is not a value path".into()),
            };
            if !rest.is_empty() {
                if index.is_none() {
                    if matches!(edit, Edit::Delete { .. }) {
                        return Ok(());
                    }
                    let Path::Item(id) = segment else {
                        return Err("Missing edit item".into());
                    };
                    let mut item = container(&rest[0])?;
                    item.as_object_mut()
                        .ok_or("Identity item requires an object")?
                        .insert("id".into(), id.clone().into());
                    index = Some(array.len());
                    array.push(item);
                }
                return apply_nested(
                    index
                        .and_then(|index| array.get_mut(index))
                        .ok_or("Missing edit item")?,
                    rest,
                    edit,
                );
            }
            match (index, edit) {
                (Some(index), _) if index >= array.len() => {
                    return Err("Array edit index is out of range".into())
                }
                (Some(index), Edit::Delete { .. }) => {
                    array.remove(index);
                }
                (Some(index), _) => array[index] = edit.next(array.get(index))?,
                (None, Edit::Delete { .. }) => {}
                (None, _) => array.push(edit.next(None)?),
            }
        }
        _ => return Err("Edit path requires an object or array".into()),
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub revision: Revision,
    pub target: Target,
    pub edits: Vec<Edit>,
}

impl Transaction {
    /// A transaction is always SQL data even when its edits target a DataStore.
    pub fn backend(&self) -> Backend {
        Backend::Sql
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Addition {
    pub revision: Revision,
    pub delta: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub owner: Owner,
    pub values: BTreeMap<String, Value>,
    pub revision: Option<Revision>,
    pub deleted: BTreeSet<String>,
    pub aliases: BTreeMap<String, String>,
    #[serde(default)]
    pub versions: BTreeMap<String, Revision>,
    #[serde(default)]
    pub additions: BTreeMap<String, Vec<Addition>>,
}

impl Data {
    pub fn empty(owner: Owner) -> Self {
        Self {
            owner,
            values: BTreeMap::new(),
            revision: None,
            deleted: BTreeSet::new(),
            aliases: BTreeMap::new(),
            versions: BTreeMap::new(),
            additions: BTreeMap::new(),
        }
    }

    pub(crate) fn relocate(&mut self, path: &str) -> Result<()> {
        let old = format!("{}\u{001f}", self.owner.path);
        let mut aliases = BTreeMap::new();
        for (key, value) in &self.aliases {
            let key = key
                .strip_prefix(&old)
                .map(|id| format!("{path}\u{001f}{id}"))
                .unwrap_or_else(|| key.clone());
            if aliases
                .insert(key, value.clone())
                .is_some_and(|previous| previous != *value)
            {
                return Err("Conflicting relocated identity aliases".into());
            }
        }
        self.aliases = aliases;
        self.owner.path = path.into();
        Ok(())
    }

    /// Publish the entire transaction only after all edits succeed.
    pub fn apply(&mut self, transaction: &Transaction) -> Result<()> {
        if transaction.target != Target::Data(self.owner.clone()) {
            return Err("Transaction owner/version is not ready".into());
        }
        let mut staged = self.clone();
        for edit in &transaction.edits {
            if let Edit::Patch { key, set, delete } = edit {
                for (field, value) in set {
                    staged.apply_edit(
                        &Edit::Set {
                            key: key.clone(),
                            path: vec![Path::Field(field.clone())],
                            value: value.clone(),
                        },
                        &transaction.revision,
                    )?;
                }
                for field in delete {
                    staged.apply_edit(
                        &Edit::Delete {
                            key: key.clone(),
                            path: vec![Path::Field(field.clone())],
                        },
                        &transaction.revision,
                    )?;
                }
            } else {
                staged.apply_edit(edit, &transaction.revision)?;
            }
        }
        if staged
            .revision
            .as_ref()
            .is_none_or(|previous| newer(&transaction.revision, previous))
        {
            staged.revision = Some(transaction.revision.clone());
        }
        *self = staged;
        Ok(())
    }
    fn apply_edit(&mut self, edit: &Edit, revision: &Revision) -> Result<()> {
        if let Edit::ReplaceRecords { prefix, records } = edit {
            if prefix.is_empty() {
                return Err("Empty record namespace".into());
            }
            let namespace =
                serde_json::to_string(&(prefix, vec![Path::Records])).map_err(|e| e.to_string())?;
            if self
                .versions
                .get(&namespace)
                .is_some_and(|previous| newer(previous, revision))
            {
                return Ok(());
            }
            let keys = self
                .values
                .keys()
                .filter(|key| key.starts_with(prefix))
                .cloned()
                .chain(records.keys().map(|id| format!("{prefix}{id}")))
                .collect::<BTreeSet<_>>();
            for key in keys {
                let effect = match records.get(&key[prefix.len()..]) {
                    Some(value) => Edit::Set {
                        key,
                        path: vec![],
                        value: value.clone(),
                    },
                    None => Edit::Delete { key, path: vec![] },
                };
                self.apply_edit(&effect, revision)?;
            }
            self.versions.retain(|encoded, previous| {
                let (key, path): (String, Vec<Path>) =
                    serde_json::from_str(encoded).expect("validated scope");
                !(key.starts_with(prefix) && path != [Path::Records]) || newer(previous, revision)
            });
            self.deleted.retain(|key| {
                !key.starts_with(prefix)
                    || self.versions.contains_key(
                        &serde_json::to_string(&(key, Vec::<Path>::new())).expect("scope"),
                    )
            });
            self.versions.insert(namespace, revision.clone());
            return Ok(());
        }
        let path = match edit {
            Edit::Set { path, .. } | Edit::Delete { path, .. } | Edit::Increment { path, .. } => {
                path
            }
            _ => unreachable!(),
        };
        let scope = (edit.key().to_owned(), path.clone());
        let mut descendants = Vec::new();
        for (encoded, previous) in &self.versions {
            let (key, ancestor): (String, Vec<Path>) =
                serde_json::from_str(encoded).map_err(|e| e.to_string())?;
            if ancestor == [Path::Records] && scope.0.starts_with(&key) {
                if newer(previous, revision) {
                    return Ok(());
                }
                continue;
            }
            if key != scope.0 {
                continue;
            }
            if path.starts_with(&ancestor) && newer(previous, revision) {
                return Ok(());
            }
            if ancestor.starts_with(path) && newer(previous, revision) {
                descendants.push((
                    ancestor.clone(),
                    nested_value(self.values.get(&key), &ancestor).cloned(),
                ));
            }
        }
        // An older parent assignment can preserve newer descendants. Rebuild
        // ancestors first, retaining only explicitly surviving descendant values.
        descendants.sort_by_key(|(path, _)| path.len());
        edit.apply(&mut self.values)?;
        if let Edit::Increment { delta, .. } = edit {
            let additions = self
                .additions
                .entry(serde_json::to_string(&scope).map_err(|e| e.to_string())?)
                .or_default();
            additions.push(Addition {
                revision: revision.clone(),
                delta: *delta,
            });
            additions.sort_by(|left, right| left.revision.cmp(&right.revision));
        } else {
            // A checkpoint contains totals and the still-relevant additive effects.
            // Retain only effects after this assignment and reapply them to its new value.
            for (encoded, additions) in &mut self.additions {
                let (key, nested): (String, Vec<Path>) =
                    serde_json::from_str(encoded).map_err(|e| e.to_string())?;
                if key != scope.0 || !nested.starts_with(path) {
                    continue;
                }
                additions.retain(|addition| newer(&addition.revision, revision));
                if descendants
                    .iter()
                    .any(|(ancestor, _)| nested.starts_with(ancestor))
                {
                    continue;
                }
                for addition in additions {
                    Edit::Increment {
                        key: key.clone(),
                        path: nested.clone(),
                        delta: addition.delta,
                    }
                    .apply(&mut self.values)?;
                }
            }
            self.additions.retain(|_, additions| !additions.is_empty());
            for (nested, value) in &descendants {
                let restore = match value {
                    Some(value) => Edit::Set {
                        key: scope.0.clone(),
                        path: nested.clone(),
                        value: value.clone(),
                    },
                    None => Edit::Delete {
                        key: scope.0.clone(),
                        path: nested.clone(),
                    },
                };
                restore.apply(&mut self.values)?;
            }
            self.versions.retain(|encoded, previous| {
                let (key, nested): (String, Vec<Path>) =
                    serde_json::from_str(encoded).expect("validated scope");
                key != scope.0 || !nested.starts_with(path) || newer(previous, revision)
            });
            self.versions.insert(
                serde_json::to_string(&scope).map_err(|e| e.to_string())?,
                revision.clone(),
            );
        }
        if self.values.contains_key(edit.key()) {
            self.deleted.remove(edit.key());
        } else if path.is_empty() {
            self.deleted.insert(edit.key().into());
        }
        Ok(())
    }
}

fn nested_value<'a>(mut value: Option<&'a Value>, path: &[Path]) -> Option<&'a Value> {
    for segment in path {
        value = match (value?, segment) {
            (Value::Object(object), Path::Field(key)) => object.get(key),
            (Value::Array(items), Path::Field(index)) => index
                .parse::<usize>()
                .ok()
                .and_then(|index| items.get(index)),
            (Value::Array(items), Path::Item(id)) => items
                .iter()
                .find(|item| item.get("id").and_then(Value::as_str) == Some(id)),
            _ => None,
        };
    }
    value
}

/// Declarative conveniences operate on the complete migration context.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Rule {
    RenameKey {
        from: String,
        to: String,
    },
    RenameField {
        key: String,
        from: String,
        to: String,
    },
    /// Each declared object field becomes one independent key.
    SplitObject {
        key: String,
        fields: BTreeMap<String, String>,
    },
    DropKey {
        key: String,
    },
    SplitRecords {
        key: String,
        prefix: String,
        identity: String,
    },
}

pub(crate) fn rename_field(value: &mut Value, from: &str, to: &str) -> Result<()> {
    let object = value
        .as_object_mut()
        .ok_or("Field migration requires an object")?;
    if object.contains_key(from) && object.contains_key(to) {
        return Err("Field destination exists".into());
    }
    if let Some(value) = object.remove(from) {
        object.insert(to.into(), value);
    }
    Ok(())
}

fn record_values(value: &Value, identity: &str) -> Result<BTreeMap<String, Value>> {
    let mut records = BTreeMap::new();
    for value in value.as_array().ok_or("Record split requires an array")? {
        let id = value
            .get(identity)
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or("Record identity is missing")?;
        if records.insert(id.to_owned(), value.clone()).is_some() {
            return Err("Duplicate record identity".into());
        }
    }
    Ok(records)
}

impl Rule {
    fn data(&self, data: &mut Data) -> Result<()> {
        match self {
            Self::RenameKey { from, to } => {
                if data.values.contains_key(from) && data.values.contains_key(to) {
                    return Err("Key destination exists".into());
                }
                if let Some(value) = data.values.remove(from) {
                    data.values.insert(to.clone(), value);
                }
                if data.deleted.remove(from) {
                    data.deleted.insert(to.clone());
                }
            }
            Self::RenameField { key, from, to } => {
                if let Some(value) = data.values.get_mut(key) {
                    rename_field(value, from, to)?;
                }
            }
            Self::DropKey { key } => {
                data.values.remove(key);
                data.deleted.remove(key);
            }
            Self::SplitRecords {
                key,
                prefix,
                identity,
            } => {
                if prefix.is_empty() || key.starts_with(prefix) {
                    return Err("Invalid split namespace".into());
                }
                if let Some(value) = data.values.remove(key) {
                    for (id, value) in record_values(&value, identity)? {
                        if data.values.insert(format!("{prefix}{id}"), value).is_some() {
                            return Err("Record split destination exists".into());
                        }
                    }
                }
                data.deleted.remove(key);
            }
            Self::SplitObject { key, fields } => {
                self.validate_split(key, fields)?;
                let active = data.values.contains_key(key)
                    || data.deleted.contains(key)
                    || data.versions.keys().any(|encoded| {
                        serde_json::from_str::<(String, Vec<Path>)>(encoded)
                            .is_ok_and(|(root, _)| &root == key)
                    });
                if !active {
                    return Ok(());
                }
                let value = data.values.remove(key);
                if let Some(value) = &value {
                    if !value.is_object() {
                        return Err("Split migration requires an object".into());
                    }
                }
                let deleted = data.deleted.remove(key);
                for (field, destination) in fields {
                    if data.values.contains_key(destination) {
                        return Err("Split destination exists".into());
                    }
                    if let Some(value) = value.as_ref().and_then(|v| v.get(field)) {
                        data.values.insert(destination.clone(), value.clone());
                    } else if value.is_some() || deleted {
                        data.deleted.insert(destination.clone());
                    }
                }
            }
        }
        data.versions = self.scopes(&data.versions, |a, b| {
            if newer(b, a) {
                b.clone()
            } else {
                a.clone()
            }
        })?;
        data.additions =
            self.scopes(&data.additions, |a, b| a.iter().chain(b).cloned().collect())?;
        if let Self::SplitRecords { prefix, .. } = self {
            for encoded in data.versions.keys() {
                let (key, path): (String, Vec<Path>) =
                    serde_json::from_str(encoded).map_err(|e| e.to_string())?;
                if key.starts_with(prefix) && path.is_empty() && !data.values.contains_key(&key) {
                    data.deleted.insert(key);
                }
            }
        }
        // A field deletion is a root deletion after an object split.
        if let Self::SplitObject { fields, .. } = self {
            for destination in fields.values() {
                if !data.values.contains_key(destination)
                    && data.versions.contains_key(
                        &serde_json::to_string(&(destination, Vec::<Path>::new()))
                            .map_err(|e| e.to_string())?,
                    )
                {
                    data.deleted.insert(destination.clone());
                }
            }
        }
        Ok(())
    }

    fn validate_split(&self, key: &str, fields: &BTreeMap<String, String>) -> Result<()> {
        if fields.is_empty()
            || fields.values().any(|to| to == key)
            || fields.values().collect::<BTreeSet<_>>().len() != fields.len()
        {
            return Err("Split destinations must be distinct and differ from the source".into());
        }
        Ok(())
    }

    fn scope(&self, key: String, mut path: Vec<Path>) -> Result<Vec<(String, Vec<Path>)>> {
        match self {
            Self::RenameKey { from, to } if &key == from => return Ok(vec![(to.clone(), path)]),
            Self::RenameField {
                key: root,
                from,
                to,
            } if &key == root && path.first() == Some(&Path::Field(from.clone())) => {
                path[0] = Path::Field(to.clone())
            }
            Self::DropKey { key: root } if &key == root => return Ok(vec![]),
            Self::SplitRecords {
                key: root, prefix, ..
            } if &key == root => {
                if path.is_empty() {
                    return Ok(vec![(prefix.clone(), vec![Path::Records])]);
                }
                let Path::Item(id) = path.remove(0) else {
                    return Err("Record split requires an identity scope".into());
                };
                return Ok(vec![(format!("{prefix}{id}"), path)]);
            }
            Self::SplitObject { key: root, fields } if &key == root => {
                self.validate_split(root, fields)?;
                if path.is_empty() {
                    return Ok(fields.values().map(|key| (key.clone(), vec![])).collect());
                }
                let Path::Field(field) = path.remove(0) else {
                    return Err("Split source scope requires a field".into());
                };
                return Ok(fields
                    .get(&field)
                    .map(|key| vec![(key.clone(), path)])
                    .unwrap_or_default());
            }
            _ => {}
        }
        Ok(vec![(key, path)])
    }

    fn scopes<T: Clone>(
        &self,
        scopes: &BTreeMap<String, T>,
        combine: impl Fn(&T, &T) -> T,
    ) -> Result<BTreeMap<String, T>> {
        let mut result = BTreeMap::new();
        for (encoded, value) in scopes {
            let (key, path): (String, Vec<Path>) =
                serde_json::from_str(encoded).map_err(|e| e.to_string())?;
            for scope in self.scope(key, path)? {
                let encoded = serde_json::to_string(&scope).map_err(|e| e.to_string())?;
                let value = result
                    .get(&encoded)
                    .map(|old| combine(old, value))
                    .unwrap_or_else(|| value.clone());
                result.insert(encoded, value);
            }
        }
        Ok(result)
    }

    fn transaction(&self, transaction: &mut Transaction) -> Result<()> {
        let mut result = Vec::new();
        for source in &transaction.edits {
            let mut source = source.clone();
            // A patch is the same sequence of field effects as Data::apply.
            if let Edit::Patch { key, set, delete } = &source {
                let edits = set
                    .iter()
                    .map(|(field, value)| Edit::Set {
                        key: key.clone(),
                        path: vec![field.clone().into()],
                        value: value.clone(),
                    })
                    .chain(delete.iter().map(|field| Edit::Delete {
                        key: key.clone(),
                        path: vec![field.clone().into()],
                    }))
                    .collect();
                let mut expanded = Transaction {
                    edits,
                    ..transaction.clone()
                };
                self.transaction(&mut expanded)?;
                result.extend(expanded.edits);
                continue;
            }
            if let Self::RenameField { key, from, to } = self {
                if let Edit::Set {
                    key: root,
                    path,
                    value,
                } = &mut source
                {
                    if root == key && path.is_empty() {
                        rename_field(value, from, to)?;
                    }
                }
            }
            if let Edit::ReplaceRecords { .. } = &source {
                result.push(source);
                continue;
            }
            let (path, value) = match &source {
                Edit::Set { path, value, .. } => (path, Some(value)),
                Edit::Delete { path, .. } | Edit::Increment { path, .. } => (path, None),
                _ => unreachable!(),
            };
            if let Self::SplitRecords {
                key,
                prefix,
                identity,
            } = self
            {
                if source.key() == key && path.is_empty() {
                    let records = match &source {
                        Edit::Set { value, .. } => record_values(value, identity)?,
                        Edit::Delete { .. } => BTreeMap::new(),
                        _ => {
                            return Err(
                                "Record split requires an array assignment or deletion".into()
                            )
                        }
                    };
                    result.push(Edit::ReplaceRecords {
                        prefix: prefix.clone(),
                        records,
                    });
                    continue;
                }
            }
            if let Self::SplitObject { key, fields } = self {
                if source.key() == key && path.is_empty() {
                    self.validate_split(key, fields)?;
                    if value.is_some_and(|v| !v.is_object())
                        || matches!(source, Edit::Increment { .. })
                    {
                        return Err("Split root requires an object assignment or deletion".into());
                    }
                    for (field, destination) in fields {
                        result.push(match value.and_then(|v| v.get(field)) {
                            Some(value) => Edit::Set {
                                key: destination.clone(),
                                path: vec![],
                                value: value.clone(),
                            },
                            None => Edit::Delete {
                                key: destination.clone(),
                                path: vec![],
                            },
                        });
                    }
                    continue;
                }
            }
            for (key, path) in self.scope(source.key().into(), path.clone())? {
                result.push(match &source {
                    Edit::Set { value, .. } => Edit::Set {
                        key,
                        path,
                        value: value.clone(),
                    },
                    Edit::Delete { .. } => Edit::Delete { key, path },
                    Edit::Increment { delta, .. } => Edit::Increment {
                        key,
                        path,
                        delta: *delta,
                    },
                    _ => unreachable!(),
                });
            }
        }
        transaction.edits = result;
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct Migrations {
    pub owners: BTreeMap<String, Owner>,
    pub layouts: BTreeMap<String, String>,
    pub formats: BTreeMap<(String, u32), Vec<Rule>>,
}

impl Migrations {
    pub fn resolve_path(routes: &BTreeMap<String, String>, path: &str) -> Result<String> {
        let mut target = path;
        let mut visited = BTreeSet::new();
        while let Some(next) = routes.get(target) {
            if !visited.insert(target) {
                return Err("Layout migration cycle".into());
            }
            target = next;
        }
        Ok(target.to_owned())
    }

    pub fn migrate_data(&self, data: &mut Data) -> Result<()> {
        let (owner, steps) = self.steps(&data.owner)?;
        let mut staged = data.clone();
        for rule in steps {
            rule.data(&mut staged)?;
        }
        staged.relocate(&owner.path)?;
        staged.owner = owner;
        *data = staged;
        Ok(())
    }
    pub fn migrate_transaction(&self, transaction: &mut Transaction) -> Result<()> {
        let Target::Data(owner) = &transaction.target else {
            return Ok(());
        };
        let (owner, steps) = self.steps(owner)?;
        let mut staged = transaction.clone();
        for rule in steps {
            rule.transaction(&mut staged)?;
        }
        staged.target = Target::Data(owner);
        *transaction = staged;
        Ok(())
    }
    fn steps(&self, source: &Owner) -> Result<(Owner, Vec<&Rule>)> {
        let path = Self::resolve_path(&self.layouts, &source.path)?;
        let current = self
            .owners
            .get(&path)
            .ok_or_else(|| format!("Pending layout owner: {path}"))?;
        if source.format > current.format || source.layout > current.layout {
            return Err("Pending future layout/format".into());
        }
        let mut steps = Vec::new();
        for version in source.format..current.format {
            steps.extend(
                self.formats
                    .get(&(path.clone(), version))
                    .ok_or("Missing format migration")?,
            );
        }
        Ok((current.clone(), steps))
    }
}

/// Immutable originals, deduplicated before deriving any higher-order view.
#[derive(Clone, Debug, Default)]
pub struct History(BTreeMap<String, Transaction>);

impl History {
    pub fn insert(&mut self, transaction: Transaction) -> Result<()> {
        if transaction.id.is_empty() {
            return Err("Empty transaction identity".into());
        }
        if let Some(previous) = self.0.get(&transaction.id) {
            verify_original(&transaction.id, previous, &transaction)?;
        } else {
            self.0.insert(transaction.id.clone(), transaction);
        }
        Ok(())
    }

    pub fn originals(&self) -> impl Iterator<Item = &Transaction> {
        self.0.values()
    }

    fn effective(&self, id: &str, visiting: &mut BTreeSet<String>) -> Result<Transaction> {
        if !visiting.insert(id.into()) {
            return Err("Higher-order transaction dependency cycle".into());
        }
        let original = self.0.get(id).ok_or("Pending transaction target")?;
        let mut effective = original.clone();
        let mut changes = self
            .0
            .values()
            .filter(|tx| tx.target == Target::Transaction(id.into()))
            .map(|tx| self.effective(&tx.id, visiting))
            .collect::<Result<Vec<_>>>()?;
        let changes = ordered(changes, |tx| &tx.id, |tx| tx.revision.clone())?;
        let mut view = BTreeMap::from([(
            "edits".into(),
            serde_json::to_value(&effective.edits).map_err(|e| e.to_string())?,
        )]);
        for transaction in changes {
            let mut staged = view.clone();
            for edit in &transaction.edits {
                if edit.key() != "edits" {
                    return Err("Transaction metadata is immutable".into());
                }
                edit.apply(&mut staged)?;
            }
            let edits = serde_json::from_value(
                staged
                    .get("edits")
                    .cloned()
                    .ok_or("Transaction edits are missing")?,
            )
            .map_err(|e| format!("Invalid effective transaction: {e}"))?;
            effective.edits = edits;
            view = staged;
        }
        visiting.remove(id);
        Ok(effective)
    }

    fn root(&self, id: &str) -> Result<String> {
        let mut id = id;
        let mut visited = BTreeSet::new();
        loop {
            if !visited.insert(id) {
                return Err("Higher-order transaction dependency cycle".into());
            }
            match &self.0.get(id).ok_or("Pending transaction target")?.target {
                Target::Data(_) => return Ok(id.into()),
                Target::Transaction(target) => id = target,
            }
        }
    }

    pub fn materialize(&self, baseline: &Data, migrations: &Migrations) -> Result<Replay> {
        let mut data = baseline.clone();
        migrations.migrate_data(&mut data)?;
        let mut transactions = Vec::new();
        let mut pending = BTreeMap::new();
        for original in self.0.values() {
            match &original.target {
                Target::Transaction(target) => {
                    let _ = target;
                    if let Err(error) = self.root(&original.id) {
                        pending.insert(original.id.clone(), error);
                    }
                    continue;
                }
                Target::Data(_) => {}
            }
            let result = self
                .effective(&original.id, &mut BTreeSet::new())
                .and_then(|mut tx| {
                    migrations.migrate_transaction(&mut tx)?;
                    Ok(tx)
                });
            match result {
                Ok(tx) if tx.target == Target::Data(data.owner.clone()) => transactions.push(tx),
                Ok(_) => {} // Another dataset is independent.
                Err(error) => {
                    pending.insert(original.id.clone(), error);
                }
            }
        }
        let transactions = ordered(transactions, |tx| &tx.id, |tx| tx.revision.clone())?;
        let mut applied = BTreeSet::new();
        for transaction in transactions {
            match data.apply(&transaction) {
                Ok(()) => {
                    for original in self.0.values() {
                        if self.root(&original.id).as_ref() == Ok(&transaction.id) {
                            applied.insert(original.id.clone());
                        }
                    }
                }
                Err(error) => {
                    pending.insert(transaction.id, error);
                }
            }
        }
        Ok(Replay {
            data,
            applied,
            pending,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Replay {
    pub data: Data,
    pub applied: BTreeSet<String>,
    pub pending: BTreeMap<String, String>,
}

pub fn digest<T: Serialize>(value: &T) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(|e| e.to_string())?)
    ))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub node: String,
    pub data: Data,
    pub covered: BTreeSet<String>,
    pub generation: u64,
}

impl Snapshot {
    pub fn token(&self) -> Result<String> {
        digest(self)
    }
    /// Compare the materialized file values/tables, independently of replay bookkeeping.
    pub fn content_digest(&self) -> Result<String> {
        digest(&self.data.values)
    }
    pub fn baseline_digest(&self) -> Result<String> {
        digest(&(&self.data, &self.covered))
    }

    /// Installs the authoritative baseline without allocating a business revision.
    pub fn replace(&mut self, expected: &str, source: &Self) -> Result<bool> {
        if self.token()? != expected {
            return Ok(false);
        }
        if self.data.owner != source.data.owner {
            return Err("Snapshot owners or migrations differ".into());
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or("Snapshot generation overflow")?;
        self.data = source.data.clone();
        self.covered.extend(source.covered.iter().cloned());
        self.generation = generation;
        Ok(true)
    }

    /// Exact coverage prevents old received logs from undoing a durable correction.
    pub fn replay(&self, history: &History, migrations: &Migrations) -> Result<Replay> {
        let mut uncovered = History::default();
        for tx in history
            .originals()
            .filter(|tx| !self.covered.contains(&tx.id))
        {
            uncovered.insert(tx.clone())?;
        }
        uncovered.materialize(&self.data, migrations)
    }
}

/// The bounded wire view; complete contents travel through the existing blob transport.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub owner: Owner,
    pub node: String,
    pub revision: Option<Revision>,
    pub token: String,
    pub digest: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Equal,
    Local,
    Remote,
}

pub fn compare_manifests(local: &Manifest, remote: &Manifest) -> Result<Source> {
    if local.owner != remote.owner {
        return Err("Snapshots are not at the same format/layout".into());
    }
    if local.digest == remote.digest {
        return Ok(Source::Equal);
    }
    Ok(
        if (&local.revision, &local.node) >= (&remote.revision, &remote.node) {
            Source::Local
        } else {
            Source::Remote
        },
    )
}

impl Snapshot {
    pub fn manifest(&self) -> Result<Manifest> {
        Ok(Manifest {
            owner: self.data.owner.clone(),
            node: self.node.clone(),
            revision: self.data.revision.clone(),
            token: self.token()?,
            digest: self.content_digest()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Correction {
    pub source: Snapshot,
    pub local_token: String,
    pub remote_token: String,
}

pub fn compare(local: &Snapshot, remote: &Snapshot) -> Result<Option<Correction>> {
    let source = match compare_manifests(&local.manifest()?, &remote.manifest()?)? {
        Source::Equal => return Ok(None),
        Source::Local => local,
        Source::Remote => remote,
    };
    let mut source = source.clone();
    source
        .covered
        .extend(local.covered.iter().chain(&remote.covered).cloned());
    Ok(Some(Correction {
        source,
        local_token: local.token()?,
        remote_token: remote.token()?,
    }))
}

#[path = "transaction/Journal.rs"]
mod journal;
pub(crate) use journal::{text, Journal, LegacyReceiveState};
#[path = "transaction/Kernel.rs"]
pub mod kernel;
#[path = "transaction/Migration.rs"]
pub mod migration;
#[path = "transaction/PreferenceOwner.rs"]
pub mod preference;

#[cfg(feature = "full")]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Cell {
    Null,
    Integer(i64),
    Real(u64),
    Text(String),
    Blob(Vec<u8>),
}
#[cfg(feature = "full")]
impl From<crate::SqliteStore::SqliteValue> for Cell {
    fn from(value: crate::SqliteStore::SqliteValue) -> Self {
        use crate::SqliteStore::SqliteValue as V;
        match value {
            V::Null => Self::Null,
            V::Integer(v) => Self::Integer(v),
            V::Real(v) => Self::Real(v.to_bits()),
            V::Text(v) => Self::Text(v),
            V::Blob(v) => Self::Blob(v),
        }
    }
}
#[cfg(feature = "full")]
impl From<Cell> for crate::SqliteStore::SqliteValue {
    fn from(value: Cell) -> Self {
        match value {
            Cell::Null => Self::Null,
            Cell::Integer(v) => Self::Integer(v),
            Cell::Real(v) => Self::Real(f64::from_bits(v)),
            Cell::Text(v) => Self::Text(v),
            Cell::Blob(v) => Self::Blob(v),
        }
    }
}
#[cfg(feature = "full")]
#[derive(Serialize, Deserialize)]
struct Table {
    columns: Vec<String>,
    rows: Vec<Vec<Cell>>,
}

/// Logical SQL snapshots include only the owner's explicit synchronized tables, never physical pages.
#[cfg(feature = "full")]
pub struct SqlDataset<'a> {
    pub store: &'a crate::SqliteStore::SqliteStore,
    pub owner: Owner,
    pub node: String,
    /// Parent tables precede child tables. Repair deletes in reverse order and inserts in this order.
    pub tables: &'a [&'a str],
    /// Returns immutable operation identity, business time, origin, sequence in that order.
    pub revisions: &'a str,
    pub cleanup: &'a str,
}

#[cfg(feature = "full")]
fn sql_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(feature = "full")]
impl SqlDataset<'_> {
    fn prepare(&self) -> Result<()> {
        self.store.executeBatch("CREATE TABLE IF NOT EXISTS _operit_transaction_checkpoint(id INTEGER PRIMARY KEY CHECK(id=1),snapshot TEXT NOT NULL)").map_err(|e| e.to_string())
    }

    fn snapshot_in(
        &self,
        tx: &mut crate::SqliteStore::SqliteTransaction<'_>,
    ) -> std::result::Result<Snapshot, crate::SqliteStore::SqliteStoreError> {
        let mut data = Data::empty(self.owner.clone());
        let mut covered = BTreeSet::new();
        let mut generation = 0;
        if let Some(row) = tx.queryOne(
            "SELECT snapshot FROM _operit_transaction_checkpoint WHERE id=1",
            vec![],
        )? {
            let previous: Snapshot =
                serde_json::from_str(&row.get::<_, String>(0)?).map_err(sql_error)?;
            data.revision = previous.data.revision;
            covered = previous.covered;
            generation = previous.generation;
        }
        for table in self.tables {
            let info = tx.queryRows(
                &format!("PRAGMA table_info({})", sql_identifier(table)),
                vec![],
            )?;
            if info.is_empty() {
                return Err(sql_error(format!(
                    "Missing synchronized SQL table: {table}"
                )));
            }
            let columns = info
                .iter()
                .map(|row| row.get::<_, String>("name"))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let query = format!(
                "SELECT {} FROM {}",
                columns
                    .iter()
                    .map(|column| sql_identifier(column))
                    .collect::<Vec<_>>()
                    .join(","),
                sql_identifier(table)
            );
            let mut rows = tx
                .queryRows(&query, vec![])?
                .into_iter()
                .map(|row| row.values.into_iter().map(Cell::from).collect::<Vec<_>>())
                .collect::<Vec<_>>();
            rows.sort_by_cached_key(|row| serde_json::to_string(row).expect("SQL cells serialize"));
            data.values.insert(
                table.to_string(),
                serde_json::to_value(Table { columns, rows }).map_err(sql_error)?,
            );
        }
        for row in tx.queryRows(self.revisions, vec![])? {
            covered.insert(row.get::<_, String>(0)?);
            data.revision = data.revision.max(Some(Revision {
                createdAt: row.get(1)?,
                originDeviceId: row.get(2)?,
                sequence: row.get(3)?,
            }));
        }
        Ok(Snapshot {
            node: self.node.clone(),
            data,
            covered,
            generation,
        })
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        self.prepare()?;
        self.store
            .transaction(|tx| self.snapshot_in(tx))
            .map_err(|e| e.to_string())
    }

    pub fn baseline_digest(&self) -> Result<String> {
        self.prepare()?;
        let row = self.store.queryOne("SELECT snapshot FROM _operit_transaction_checkpoint WHERE id=1", vec![]).map_err(|e|e.to_string())?;
        match row {
            Some(row) => serde_json::from_str::<Snapshot>(&row.get::<_, String>(0).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?.baseline_digest(),
            None => digest(&Option::<u8>::None),
        }
    }

    /// SQL state, deletion/version facts and correction baseline commit in the same SQL transaction.
    pub fn repair(&self, expected: &str, source: &Snapshot) -> Result<bool> {
        use crate::SqliteStore::SqliteValue;
        self.prepare()?;
        let mut changed = false;
        let repaired = self.store.transaction(|tx| {
            let mut current = self.snapshot_in(tx)?;
            let expected = if expected.is_empty() {
                // Provisioning is conditional on an empty database with no committed baseline.
                let checkpoint = tx.queryOne("SELECT 1 FROM _operit_transaction_checkpoint WHERE id=1", vec![])?.is_some();
                let occupied = current.data.values.values().any(|table| table["rows"].as_array().is_none_or(|rows| !rows.is_empty()));
                if checkpoint || !current.covered.is_empty() || occupied { return Ok(false); }
                current.token().map_err(sql_error)?
            } else { expected.to_owned() };
            changed = current.data != source.data;
            if !current.replace(&expected, source).map_err(sql_error)? { return Ok(false); }
            if changed {
                let mut tables = BTreeMap::new();
                if current.data.values.len() != self.tables.len() { return Err(sql_error("Unexpected SQL snapshot tables")); }
                for name in self.tables {
                    let table: Table = serde_json::from_value(current.data.values.get(*name).cloned().ok_or_else(|| sql_error("Missing SQL snapshot table"))?).map_err(sql_error)?;
                    let expected = tx.queryRows(&format!("PRAGMA table_info({})", sql_identifier(name)), vec![])?.iter()
                        .map(|row| row.get::<_, String>("name")).collect::<std::result::Result<Vec<_>, _>>()?;
                    if table.columns != expected || table.rows.iter().any(|row| row.len() != expected.len()) { return Err(sql_error("SQL snapshot schema mismatch")); }
                    tables.insert(*name, table);
                }
                tx.execute("PRAGMA defer_foreign_keys = ON", vec![])?;
                for name in self.tables.iter().rev() { tx.execute(&format!("DELETE FROM {}", sql_identifier(name)), vec![])?; }
                for name in self.tables {
                    let table = tables.remove(name).expect("validated snapshot table");
                    let query = format!("INSERT INTO {}({}) VALUES({})", sql_identifier(name), table.columns.iter().map(|column| sql_identifier(column)).collect::<Vec<_>>().join(","),
                        (0..table.columns.len()).map(|_| "?").collect::<Vec<_>>().join(","));
                    for row in table.rows { tx.execute(&query, row.into_iter().map(Into::into).collect())?; }
                }
                if !self.cleanup.is_empty() { tx.execute(self.cleanup, vec![])?; }
            }
            tx.execute("INSERT INTO _operit_transaction_checkpoint(id,snapshot) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET snapshot=excluded.snapshot",
                vec![SqliteValue::Text(serde_json::to_string(&current).map_err(sql_error)?)])?;
            Ok(true)
        }).map_err(|e| e.to_string())?;
        if repaired && changed {
            self.store.notifyInvalidated().map_err(|e| e.to_string())?;
        }
        Ok(repaired)
    }
}

#[cfg(feature = "full")]
fn sql_error(error: impl std::fmt::Display) -> crate::SqliteStore::SqliteStoreError {
    crate::SqliteStore::SqliteStoreError::Message(error.to_string())
}

/// A correction is a complete authoritative baseline; older late arrivals remain archived.
#[cfg(feature = "full")]
pub fn covered_by_checkpoint(
    tx: &mut crate::SqliteStore::SqliteTransaction<'_>,
    operation: &crate::SyncOperationStore::SyncOperation,
) -> std::result::Result<bool, crate::SqliteStore::SqliteStoreError> {
    if tx.queryOne("SELECT 1 FROM sqlite_master WHERE type='table' AND name='_operit_transaction_checkpoint'", vec![])?.is_none() { return Ok(false); }
    let Some(row) = tx.queryOne(
        "SELECT snapshot FROM _operit_transaction_checkpoint WHERE id=1",
        vec![],
    )?
    else {
        return Ok(false);
    };
    let baseline: Snapshot = serde_json::from_str(&row.get::<_, String>(0)?).map_err(sql_error)?;
    Ok(baseline.covered.contains(&operation.opId))
}

/// The common f representation: complete data and SQL transaction effects obey the same rules.
pub trait Form {
    fn migrate(&mut self, migrations: &Migrations) -> Result<()>;
    fn apply_rule(&mut self, rule: &Rule) -> Result<()>;
}
impl Form for Data {
    fn migrate(&mut self, migrations: &Migrations) -> Result<()> {
        migrations.migrate_data(self)
    }
    fn apply_rule(&mut self, rule: &Rule) -> Result<()> {
        rule.data(self)
    }
}
impl Form for Transaction {
    fn migrate(&mut self, migrations: &Migrations) -> Result<()> {
        migrations.migrate_transaction(self)
    }
    fn apply_rule(&mut self, rule: &Rule) -> Result<()> {
        rule.transaction(self)
    }
}
impl Migrations {
    pub fn migrate(&self, form: &mut (impl Form + Clone)) -> Result<()> {
        let mut staged = form.clone();
        staged.migrate(self)?;
        *form = staged;
        Ok(())
    }
}
impl Rule {
    pub fn apply(&self, form: &mut (impl Form + Clone)) -> Result<()> {
        let mut staged = form.clone();
        staged.apply_rule(self)?;
        *form = staged;
        Ok(())
    }
}
