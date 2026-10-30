//! Join complete checkpoints through the same scope application used by ordinary transactions.
use super::*;

impl Data {
    /// Combines relocated checkpoints without allocating business identities or trusting file time.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.owner != other.owner {
            return Err("Checkpoint owners or migrations differ".into());
        }
        let mut merged = Data::empty(self.owner.clone());
        let mut effects = BTreeMap::<Revision, Vec<Edit>>::new();
        let mut contributions = BTreeMap::<(String, Revision), Vec<i64>>::new();
        for source in [self as &Data, other] {
            let mut base = source.values.clone();
            // The snapshot total already includes these increments. Recover the
            // assignment payload before replaying each retained contribution once.
            for (encoded, additions) in &source.additions {
                let (key, path): (String, Vec<Path>) =
                    serde_json::from_str(encoded).map_err(|e| e.to_string())?;
                for addition in additions.iter().rev() {
                    let previous = nested_value(base.get(&key), &path)
                        .and_then(Value::as_i64)
                        .and_then(|value| value.checked_sub(addition.delta))
                        .ok_or("Invalid checkpoint increment")?;
                    Edit::Set {
                        key: key.clone(),
                        path: path.clone(),
                        value: previous.into(),
                    }
                    .apply(&mut base)?;
                }
                let mut by_revision = BTreeMap::<Revision, Vec<i64>>::new();
                for addition in additions {
                    by_revision
                        .entry(addition.revision.clone())
                        .or_default()
                        .push(addition.delta);
                }
                for (revision, deltas) in by_revision {
                    if contributions
                        .insert((encoded.clone(), revision), deltas.clone())
                        .is_some_and(|previous| previous != deltas)
                    {
                        return Err("Conflicting checkpoint increment identity".into());
                    }
                }
            }
            let mut roots = BTreeSet::new();
            let mut namespaces = Vec::new();
            for (encoded, revision) in &source.versions {
                let (key, path): (String, Vec<Path>) =
                    serde_json::from_str(encoded).map_err(|e| e.to_string())?;
                if path.is_empty() {
                    roots.insert(key.clone());
                }
                let edit = if path == [Path::Records] {
                    namespaces.push(key.clone());
                    Edit::ReplaceRecords {
                        prefix: key.clone(),
                        records: base
                            .iter()
                            .filter_map(|(root, value)| {
                                root.strip_prefix(&key).map(|id| (id.into(), value.clone()))
                            })
                            .collect(),
                    }
                } else {
                    match nested_value(base.get(&key), &path) {
                        Some(value) => Edit::Set {
                            key,
                            path,
                            value: value.clone(),
                        },
                        None => Edit::Delete { key, path },
                    }
                };
                effects.entry(revision.clone()).or_default().push(edit);
            }
            for (key, value) in &base {
                let versioned =
                    roots.contains(key) || namespaces.iter().any(|prefix| key.starts_with(prefix));
                if !versioned
                    && merged
                        .values
                        .insert(key.clone(), value.clone())
                        .is_some_and(|previous| previous != *value)
                {
                    return Err("Conflicting unversioned checkpoint values".into());
                }
            }
            for (alias, canonical) in &source.aliases {
                if merged
                    .aliases
                    .insert(alias.clone(), canonical.clone())
                    .is_some_and(|previous| previous != *canonical)
                {
                    return Err("Conflicting checkpoint identity aliases".into());
                }
            }
        }
        // Assignments at an ancestor precede its child scopes. Identical shared
        // effects are idempotent; shared additive effects are explicitly deduplicated.
        for edits in effects.values_mut() {
            edits.sort_by_cached_key(|edit| {
                let depth = match edit {
                    Edit::ReplaceRecords { .. } => 0,
                    Edit::Set { path, .. } | Edit::Delete { path, .. } => path.len() + 1,
                    _ => unreachable!(),
                };
                (
                    depth,
                    serde_json::to_string(edit).expect("serializable checkpoint effect"),
                )
            });
        }
        for ((encoded, revision), deltas) in contributions {
            let (key, path): (String, Vec<Path>) =
                serde_json::from_str(&encoded).map_err(|e| e.to_string())?;
            effects
                .entry(revision)
                .or_default()
                .extend(deltas.into_iter().map(|delta| Edit::Increment {
                    key: key.clone(),
                    path: path.clone(),
                    delta,
                }));
        }
        let transactions = effects.into_iter().map(|(revision, edits)| Transaction {
            id: format!("{}:{}", revision.originDeviceId, revision.sequence),
            revision,
            target: Target::Data(merged.owner.clone()),
            edits,
        });
        for transaction in ordered(
            transactions,
            |transaction| &transaction.id,
            |transaction| transaction.revision.clone(),
        )? {
            merged.apply(&transaction)?;
        }
        for revision in self.revision.iter().chain(&other.revision) {
            if merged
                .revision
                .as_ref()
                .is_none_or(|previous| newer(revision, previous))
            {
                merged.revision = Some(revision.clone());
            }
        }
        *self = merged;
        Ok(())
    }
}
