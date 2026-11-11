use super::*;
use crate::SqliteStore::{SqliteRowGet, SqliteStore, SqliteStoreError};
use operit_host_native_storage::NativeRuntimeStorageHost;
use serde_json::json;
use std::sync::Arc;

struct Fixture {
    root: std::path::PathBuf,
    store: SqliteStore,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("operit-compaction-{}", uuid::Uuid::new_v4()));
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        let store = SqliteStore::openWithHost(host, "runtime/compaction.sqlite").unwrap();
        store.executeBatch(SCHEMA).unwrap();
        store
            .executeBatch("CREATE TABLE payloads(id TEXT PRIMARY KEY,original TEXT NOT NULL)")
            .unwrap();
        Self { root, store }
    }
    fn insert(&self, operation: &SyncOperation, scopes: Option<BTreeSet<String>>) {
        self.store
            .transaction(|tx| {
                if retired(tx.raw(), operation)? {
                    return Ok(());
                }
                tx.execute(
                    "INSERT OR IGNORE INTO payloads VALUES(?1,?2)",
                    vec![
                        text(&operation.opId),
                        text(&serde_json::to_string(operation).unwrap()),
                    ],
                )?;
                for id in record(tx.raw(), operation, scopes)? {
                    tx.execute("DELETE FROM payloads WHERE id=?1", vec![text(&id)])?;
                }
                Ok(())
            })
            .unwrap();
    }
    fn originals(&self) -> Vec<SyncOperation> {
        self.store
            .queryRows("SELECT original FROM payloads ORDER BY id", vec![])
            .unwrap()
            .iter()
            .map(|row| serde_json::from_str(&row.get::<_, String>(0).unwrap()).unwrap())
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn operation(author: &str, time: i64, values: serde_json::Value) -> SyncOperation {
    SyncOperation {
        opId: format!("{author}:{time}"),
        originDeviceId: author.into(),
        sequence: time,
        createdAt: time,
        domain: "test".into(),
        entityType: "transaction".into(),
        entityId: "file".into(),
        operation: "commit".into(),
        semantics: SyncOperationSemantics::Transaction,
        payload: values,
        schemaVersion: 1,
    }
}
fn footprint(operation: &SyncOperation) -> Option<BTreeSet<String>> {
    Some(
        operation
            .payload
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect(),
    )
}
fn replay(
    operations: impl IntoIterator<Item = SyncOperation>,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    let mut state = std::collections::BTreeMap::new();
    for operation in
        super::super::ordered(operations, |op| &op.opId, Revision::fromOperation).unwrap()
    {
        for (key, value) in operation.payload.as_object().unwrap() {
            state.insert(key.clone(), value.clone());
        }
    }
    state
}
#[test]
fn indexed_whole_transaction_compaction_commutes_with_external_absolute_writes() {
    let history = [
        operation("a", 10, json!({"x":"A","y":1})),
        operation("b", 20, json!({"x":"AB"})),
        operation("c", 30, json!({"y":2})),
    ];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let fixture = Fixture::new();
        for index in order {
            let op = &history[index];
            fixture.insert(op, footprint(op));
        }
        assert_eq!(fixture.originals().len(), 2);
        assert!(!fixture.originals().contains(&history[0]));
        for time in [5, 15, 25, 35] {
            let external = operation("external", time, json!({"x":"other","z":true}));
            assert_eq!(
                replay(history.iter().cloned().chain([external.clone()])),
                replay(fixture.originals().into_iter().chain([external]))
            );
        }
        fixture.insert(&history[0], footprint(&history[0]));
        assert_eq!(fixture.originals().len(), 2);
    }
}
#[test]
fn partial_effects_and_unproven_business_rules_stay_pinned() {
    let fixture = Fixture::new();
    let first = operation("a", 10, json!({"x":"A","y":1}));
    let second = operation("b", 20, json!({"x":"AB"}));
    fixture.insert(&first, footprint(&first));
    fixture.insert(&second, footprint(&second));
    assert_eq!(fixture.originals().len(), 2);
    let pinned = operation("c", 25, json!({"x":"catalog effect"}));
    fixture.insert(&pinned, None);
    let final_write = operation("d", 30, json!({"x":"final","y":2}));
    fixture.insert(&final_write, footprint(&final_write));
    assert_eq!(fixture.originals(), vec![pinned, final_write]);
}
#[test]
fn retirement_fingerprints_and_payload_deletion_rollback_together() {
    let fixture = Fixture::new();
    let first = operation("a", 10, json!({"x":"A"}));
    let second = operation("b", 20, json!({"x":"AB"}));
    fixture.insert(&first, footprint(&first));
    let result: Result<(), SqliteStoreError> = fixture.store.transaction(|tx| {
        for id in record(tx.raw(), &second, footprint(&second))? {
            tx.execute("DELETE FROM payloads WHERE id=?1", vec![text(&id)])?;
        }
        Err(SqliteStoreError::Message("injected failure".into()))
    });
    assert!(result.is_err());
    assert_eq!(fixture.originals(), vec![first.clone()]);
    fixture.insert(&second, footprint(&second));
    let mut collision = first;
    collision.payload = json!({"x":"forged"});
    assert!(fixture
        .store
        .transaction(|tx| {
            retired(tx.raw(), &collision)?;
            Ok(())
        })
        .is_err());
    assert_eq!(fixture.originals(), vec![second]);
}

fn data_transaction(author: &str, time: i64, edits: Vec<Edit>) -> Transaction {
    Transaction {
        id: format!("{author}:{time}"),
        revision: Revision {
            createdAt: time,
            originDeviceId: author.into(),
            sequence: time,
        },
        target: Target::Data(super::super::Owner {
            backend: super::super::Backend::DataStore,
            path: "preferences".into(),
            format: 1,
            layout: 1,
        }),
        edits,
    }
}
fn set(key: &str, path: &[&str], value: serde_json::Value) -> Edit {
    Edit::Set {
        key: key.into(),
        path: path
            .iter()
            .map(|part| Path::Field((*part).into()))
            .collect(),
        value,
    }
}
fn data_replay(history: &[Transaction]) -> Data {
    let Target::Data(owner) = &history[0].target else {
        panic!("data owner")
    };
    let mut data = Data::empty(owner.clone());
    for tx in super::super::ordered(history.iter(), |tx| &tx.id, |tx| tx.revision.clone()).unwrap()
    {
        data.apply(tx).unwrap();
    }
    data
}
fn assert_compaction(history: &[Transaction], expected: &[&str], external: &[Transaction]) {
    let retired = superseded(history).unwrap();
    assert_eq!(retired, expected.iter().map(|id| (*id).into()).collect());
    let retained = history
        .iter()
        .filter(|tx| !retired.contains(&tx.id))
        .cloned()
        .collect::<Vec<_>>();
    for incoming in external {
        let mut complete = history.to_vec();
        complete.push(incoming.clone());
        let mut compacted = retained.clone();
        compacted.push(incoming.clone());
        let a = data_replay(&complete);
        let b = data_replay(&compacted);
        assert_eq!(a.values, b.values, "external {}", incoming.id);
        assert_eq!(a.additions, b.additions);
        // Also test late delivery onto the already materialized checkpoints.
        let mut a = data_replay(history);
        let mut b = data_replay(&retained);
        a.apply(incoming).unwrap();
        b.apply(incoming).unwrap();
        assert_eq!(a.values, b.values, "late external {}", incoming.id);
        assert_eq!(a.additions, b.additions);
    }
}

#[test]
fn hierarchical_coverage_retires_patches_and_absorbed_increments_in_any_arrival_order() {
    let history = [
        data_transaction(
            "a",
            10,
            vec![
                Edit::Patch {
                    key: "settings".into(),
                    set: BTreeMap::from([("theme".into(), json!("old"))]),
                    delete: BTreeSet::from(["gone".into()]),
                },
                Edit::Increment {
                    key: "counter".into(),
                    path: vec![],
                    delta: 3,
                },
            ],
        ),
        data_transaction(
            "b",
            20,
            vec![
                set("settings", &[], json!({"theme":"new"})),
                set("counter", &[], json!(100)),
            ],
        ),
        data_transaction(
            "c",
            30,
            vec![Edit::Increment {
                key: "counter".into(),
                path: vec![],
                delta: 7,
            }],
        ),
    ];
    let external = [5, 15, 25, 35].map(|time| {
        data_transaction(
            "offline",
            time,
            vec![
                set("settings", &["other"], json!(time)),
                set("counter", &[], json!(time)),
            ],
        )
    });
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let shuffled = order.map(|index| history[index].clone());
        assert_compaction(&shuffled, &["a:10"], &external);
    }
}

#[test]
fn nested_assignments_and_deletions_keep_partial_atomic_effects() {
    let history = [
        data_transaction(
            "a",
            10,
            vec![
                set("settings", &["theme"], json!("old")),
                set("settings", &["keep"], json!(true)),
            ],
        ),
        data_transaction("b", 20, vec![set("settings", &["theme"], json!("new"))]),
        data_transaction(
            "c",
            30,
            vec![Edit::Delete {
                key: "settings".into(),
                path: vec![Path::Field("keep".into())],
            }],
        ),
    ];
    assert!(superseded(&history[..2]).unwrap().is_empty());
    let external = [5, 15, 25, 35].map(|time| {
        data_transaction(
            "offline",
            time,
            vec![set("settings", &[], json!({"offline":time}))],
        )
    });
    assert_compaction(&history, &["a:10"], &external);
    let deleted_parent = data_transaction(
        "delete",
        40,
        vec![Edit::Delete {
            key: "settings".into(),
            path: vec![],
        }],
    );
    let new_child = data_transaction(
        "child",
        50,
        vec![set("settings", &["theme"], json!("restored"))],
    );
    assert!(superseded([&deleted_parent, &new_child])
        .unwrap()
        .is_empty());
}

#[test]
fn namespace_replacements_cover_old_records_and_increments_but_not_other_owners() {
    let history = [
        data_transaction(
            "a",
            10,
            vec![
                set("card/a", &[], json!({"name":"old"})),
                Edit::Increment {
                    key: "card/count".into(),
                    path: vec![],
                    delta: 2,
                },
            ],
        ),
        data_transaction(
            "b",
            20,
            vec![Edit::ReplaceRecords {
                prefix: "card/".into(),
                records: BTreeMap::from([("b".into(), json!({"name":"new"}))]),
            }],
        ),
    ];
    let external = [5, 15, 25, 35]
        .map(|time| data_transaction("offline", time, vec![set("card/unseen", &[], json!(time))]));
    assert_compaction(&history, &["a:10"], &external);
    let mut other = history[1].clone();
    if let Target::Data(owner) = &mut other.target {
        owner.path = "other-owner".into();
    }
    assert!(superseded([&history[0], &other]).unwrap().is_empty());
}

#[test]
fn equal_increment_payloads_and_array_structural_effects_are_not_deduplicated() {
    let increments = [10, 20].map(|time| {
        data_transaction(
            "a",
            time,
            vec![Edit::Increment {
                key: "counter".into(),
                path: vec![],
                delta: 3,
            }],
        )
    });
    assert!(superseded(&increments).unwrap().is_empty());
    assert_eq!(data_replay(&increments).values["counter"], json!(6));
    let first = data_transaction(
        "a",
        10,
        vec![Edit::Set {
            key: "items".into(),
            path: vec![Path::Item("id".into()), Path::Field("name".into())],
            value: json!("old"),
        }],
    );
    let mut second = first.clone();
    second.id = "b:20".into();
    second.revision = Revision {
        originDeviceId: "b".into(),
        sequence: 20,
        createdAt: 20,
    };
    assert!(superseded([&first, &second]).unwrap().is_empty());
    let parent = data_transaction("c", 30, vec![set("items", &[], json!([]))]);
    assert_eq!(
        superseded([&first, &second, &parent]).unwrap(),
        BTreeSet::from([first.id, second.id])
    );
}

#[test]
fn correction_dependencies_and_identity_collisions_remain_protected() {
    let first = data_transaction("a", 10, vec![set("x", &[], json!(1))]);
    let second = data_transaction("b", 20, vec![set("x", &[], json!(2))]);
    let mut correction = data_transaction("correction", 30, vec![]);
    correction.target = Target::Transaction(first.id.clone());
    assert!(superseded([&first, &second, &correction])
        .unwrap()
        .is_empty());
    let mut forged = first.clone();
    forged.edits = vec![set("x", &[], json!(999))];
    assert!(superseded([&first, &forged]).is_err());
}

#[test]
fn mixed_scope_histories_remain_equivalent_under_offline_writes_and_replay() {
    let edit = |kind: u64, n: i64| match kind % 9 {
        0 => set("settings", &[], json!({"theme":n,"other":true})),
        1 => set("settings", &["theme"], json!(n)),
        2 => Edit::Delete {
            key: "settings".into(),
            path: vec![Path::Field("theme".into())],
        },
        3 => Edit::Patch {
            key: "settings".into(),
            set: BTreeMap::from([("other".into(), json!(n))]),
            delete: BTreeSet::from(["theme".into()]),
        },
        4 => Edit::Increment {
            key: "counter".into(),
            path: vec![],
            delta: n,
        },
        5 => set("counter", &[], json!(n)),
        6 => Edit::ReplaceRecords {
            prefix: "card/".into(),
            records: BTreeMap::from([("b".into(), json!(n))]),
        },
        7 => set("card/a", &[], json!(n)),
        _ => Edit::Delete {
            key: "card/a".into(),
            path: vec![],
        },
    };
    let mut seed = 0x123456789abcdef_u64;
    for _ in 0..96 {
        let mut history = Vec::new();
        for time in [10, 20, 30, 40, 50, 60] {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            history.push(data_transaction(
                &format!("author{}", seed % 3),
                time,
                vec![edit(seed, time), edit(seed >> 8, time)],
            ));
        }
        let fixture = Fixture::new();
        let mut indexed_retired = BTreeSet::new();
        for transaction in &history {
            indexed_retired.extend(indexed_preference(&fixture, transaction).0);
        }
        for retired in [superseded(&history).unwrap(), indexed_retired] {
            let retained = history
                .iter()
                .filter(|tx| !retired.contains(&tx.id))
                .cloned()
                .collect::<Vec<_>>();
            assert!(!retained.is_empty());
            for time in [5, 15, 35, 55, 65] {
                for kind in 0..9 {
                    let external = data_transaction("offline", time, vec![edit(kind, time)]);
                    let mut complete = history.clone();
                    complete.push(external.clone());
                    let mut compacted = retained.clone();
                    compacted.push(external.clone());
                    let a = data_replay(&complete);
                    let b = data_replay(&compacted);
                    assert_eq!(
                        a.values, b.values,
                        "retired={retired:?}, external={external:?}, history={history:?}"
                    );
                    assert_eq!(a.additions, b.additions);
                    let mut a = data_replay(&history);
                    let mut b = data_replay(&retained);
                    a.apply(&external).unwrap();
                    b.apply(&external).unwrap();
                    assert_eq!(
                        a.values, b.values,
                        "late retired={retired:?}, external={external:?}, history={history:?}"
                    );
                    assert_eq!(a.additions, b.additions);
                }
            }
        }
    }
}

#[test]
fn child_deletion_does_not_erase_an_earlier_container_creation() {
    let first = data_transaction(
        "a",
        10,
        vec![set("settings", &["nested", "field"], json!(true))],
    );
    let deletion = data_transaction(
        "b",
        20,
        vec![Edit::Delete {
            key: "settings".into(),
            path: vec![Path::Field("nested".into()), Path::Field("field".into())],
        }],
    );
    assert!(superseded([&first, &deletion]).unwrap().is_empty());
    assert_eq!(
        data_replay(&[first.clone(), deletion.clone()]).values["settings"],
        json!({"nested":{}})
    );
    let sibling = data_transaction(
        "c",
        30,
        vec![set("settings", &["nested", "other"], json!(true))],
    );
    assert_eq!(
        superseded([&first, &deletion, &sibling]).unwrap(),
        BTreeSet::from([first.id])
    );
}

#[test]
fn legacy_clock_regressions_keep_causal_ordering_barriers() {
    let first = data_transaction("a", 100, vec![set("x", &[], json!(1))]);
    let mut second = data_transaction("a", 101, vec![set("x", &[], json!(2))]);
    second.revision.createdAt = 1;
    let external = data_transaction("b", 50, vec![set("x", &[], json!(3))]);
    assert!(superseded([&first, &second, &external]).unwrap().is_empty());
}

/// Count the metadata actually visited by commit compaction. This also rejects
/// accidental payload reads: the ledger needs only scopes and immutable headers.
struct CountingTransaction<'a> {
    inner: &'a mut dyn RuntimeSqliteTransaction,
    scope_rows: usize,
}
impl RuntimeSqliteTransaction for CountingTransaction<'_> {
    fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
        self.inner.execute(sql, params)
    }
    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<operit_host_api::SqliteRow>> {
        assert!(!sql.contains("originals") && !sql.contains("payloads"), "compaction read a business payload: {sql}");
        let rows = self.inner.query(sql, params)?;
        if sql.starts_with("SELECT s.scope,s.id") { self.scope_rows += rows.len(); }
        Ok(rows)
    }
    fn lastInsertRowId(&self) -> HostResult<i64> { self.inner.lastInsertRowId() }
    fn commit(self: Box<Self>) -> HostResult<()> { panic!("borrowed test transaction") }
    fn rollback(self: Box<Self>) -> HostResult<()> { panic!("borrowed test transaction") }
}
fn indexed_preference(fixture: &Fixture, transaction: &Transaction) -> (Vec<String>, usize) {
    let original = operation(&transaction.revision.originDeviceId, transaction.revision.sequence, json!(transaction.edits));
    fixture.store.transaction(|tx| {
        let mut counting = CountingTransaction { inner: tx.raw(), scope_rows: 0 };
        let retired = record_preference(&mut counting, &original, transaction)?;
        Ok((retired, counting.scope_rows))
    }).unwrap()
}

#[test]
fn local_index_visits_only_changed_scopes_and_forgets_covered_partial_effects() {
    let fixture = Fixture::new();
    for n in 1..=256 {
        indexed_preference(&fixture, &data_transaction("local", n,
            vec![set(&format!("unrelated-{n}"), &[], json!(n))]));
    }
    let mut last = None;
    for n in 257..=320 {
        let current = data_transaction("local", n, vec![
            set("settings", &["theme"], json!(n)),
            Edit::Increment { key: "counter".into(), path: vec![], delta: 1 },
        ]);
        let (retired, rows) = indexed_preference(&fixture, &current);
        assert!(retired.is_empty(), "an uncovered increment pins its atomic original");
        assert!(rows <= 2, "already covered theme scopes accumulated: {rows}");
        last = Some(current);
    }
    let final_write = data_transaction("local", 321, vec![set("counter", &[], json!(100))]);
    let (retired, _) = indexed_preference(&fixture, &final_write);
    assert_eq!(retired.len(), 63, "only the last theme assignment is still live");
    let (retired, rows) = indexed_preference(&fixture,
        &data_transaction("local", 322, vec![set("settings", &[], json!({"theme":"final"}))]));
    assert_eq!(retired, vec![last.unwrap().id]);
    assert_eq!(rows, 2);
    // Both range and exact lookups start at the scope primary key, then seek IDs.
    for predicate in ["s.scope>=?1 AND s.scope<?2", "s.scope=?1 AND ?2=?2"] {
        let plan = fixture.store.queryRows(&format!(
            "EXPLAIN QUERY PLAN SELECT s.scope,s.id FROM _operit_compaction_scopes s
             CROSS JOIN _operit_compacted c ON c.id=s.id WHERE {predicate} AND c.origin=?3 AND c.sequence<?4"),
            vec![text("preference/"), text("preference/g"), text("local"), SqliteValue::Integer(999)])
            .unwrap().iter().map(|row| row.get::<_, String>(3).unwrap()).collect::<Vec<_>>().join("\n");
        assert!(plan.contains("SEARCH s USING COVERING INDEX"), "{plan}");
        assert!(plan.contains("SEARCH c USING INDEX"), "{plan}");
        assert!(!plan.contains("SCAN"), "{plan}");
    }
}

#[test]
fn indexed_local_coverage_preserves_containers_arrays_and_namespace_boundaries() {
    let fixture = Fixture::new();
    let created = data_transaction("local", 1, vec![set("对象/", &["nested", "field"], json!(1))]);
    indexed_preference(&fixture, &created);
    assert!(indexed_preference(&fixture, &data_transaction("local", 2, vec![Edit::Delete {
        key: "对象/".into(), path: vec!["nested".into(), "field".into()],
    }])).0.is_empty(), "deleting a field leaves its parents");
    let (retired, _) = indexed_preference(&fixture,
        &data_transaction("local", 3, vec![set("对象/", &["nested", "sibling"], json!(true))]));
    assert_eq!(retired, vec![created.id]);
    indexed_preference(&fixture, &data_transaction("local", 4, vec![set("对象/", &["list", "0"], json!(1))]));
    assert!(indexed_preference(&fixture,
        &data_transaction("local", 5, vec![set("对象/", &["list", "0"], json!(2))])).0.is_empty());
    indexed_preference(&fixture, &data_transaction("local", 6, vec![set("对象/else", &[], json!(true))]));
    let (retired, _) = indexed_preference(&fixture, &data_transaction("local", 7, vec![Edit::ReplaceRecords {
        prefix: "对象/".into(), records: BTreeMap::new(),
    }]));
    assert_eq!(retired.len(), 5);
    assert!(indexed_preference(&fixture,
        &data_transaction("local", 8, vec![set("对象/", &[], json!("one key"))])).0.is_empty(),
        "one key cannot replace an entire namespace");
}

#[test]
fn indexed_preference_retirement_rolls_back_and_preserves_other_authors() {
    let fixture = Fixture::new();
    let first = data_transaction("local", 1, vec![set("x", &[], json!(1))]);
    indexed_preference(&fixture, &first);
    let second = data_transaction("local", 2, vec![set("x", &[], json!(2))]);
    let original = operation("local", 2, json!(second.edits));
    let failure: Result<(), SqliteStoreError> = fixture.store.transaction(|tx| {
        assert_eq!(record_preference(tx.raw(), &original, &second)?, vec![first.id.clone()]);
        Err(SqliteStoreError::Message("rollback".into()))
    });
    assert!(failure.is_err());
    assert!(indexed_preference(&fixture, &data_transaction("peer", 3, vec![set("x", &[], json!(3))])).0.is_empty());
    assert_eq!(indexed_preference(&fixture, &second).0, vec![first.id]);
    assert!(indexed_preference(&fixture, &second).0.is_empty(), "recovery must be idempotent");
    let mut forged = original;
    forged.payload = json!("forged");
    assert!(fixture.store.transaction(|tx| Ok(record_preference(tx.raw(), &forged, &second)?)).is_err());
}

/// The allocation-free identity check must accept exactly the former canonical representation.
#[test]
fn initialization_validation_preserves_canonical_identity() {
    use sha2::Digest;
    let data = Data::empty(super::super::Owner { backend: super::super::Backend::Sql, path: "runtime/probe.db".into(), format: 1, layout: 1 });
    let original = Initialization { data, covered: BTreeSet::from(["a:1".into(),"b:2".into()]), definition: None }.operation("plugin_storage",false).unwrap();
    let mut variants = vec![original.clone()];
    for pointer in ["/initialization/data/versions", "/initialization/data/additions", "/initialization/data/owner/layout", "/encrypted"] {
        let mut changed = original.clone();
        *changed.payload.pointer_mut(pointer).unwrap() = serde_json::Value::Null;
        variants.push(changed);
    }
    let mut changed = original.clone(); changed.payload["initialization"]["unknown"] = json!(true); variants.push(changed);
    let mut changed = original.clone(); changed.payload["initialization"]["data"]["unknown"] = json!(true); variants.push(changed);
    let mut changed = original.clone(); changed.payload["initialization"]["covered"] = json!(["b:2","a:1","a:1"]); variants.push(changed);
    for mut operation in variants {
        // Re-sign malformed payloads too, to exercise schema canonicalization rather than just hashing.
        operation.originDeviceId = format!("~init:{}", digest(&(&operation.domain,&operation.payload)).unwrap());
        operation.opId = format!("{}:1",operation.originDeviceId);
        let old = serde_json::from_value::<Initialization>(operation.payload["initialization"].clone()).ok()
            .and_then(|value|value.operation(&operation.domain,operation.payload["encrypted"] == true).ok()) == Some(operation.clone());
        assert_eq!(Initialization::decode(&operation).is_ok(), old, "{}", operation.payload);
        assert_eq!(digest(&operation).unwrap(), format!("{:x}",sha2::Sha256::digest(serde_json::to_vec(&operation).unwrap())));
    }
}

#[test]
fn disk_initializers_preserve_bytes_migrate_old_text_and_reject_changed_envelopes() {
    use super::super::{Backend, Owner, SqlDataset};
    use crate::RuntimeFileSyncStore::RuntimeFileSyncStore;
    for checkpoint in [false, true] {
        let fixture = Fixture::new();
        fixture.store.executeBatch("CREATE TABLE business(id INTEGER PRIMARY KEY,value TEXT)").unwrap();
        fixture.store.execute("INSERT INTO business VALUES(1,?1)",vec![SqliteValue::Text("中文\\\"\n".repeat(8192))]).unwrap();
        let owner = SqlDataset { store:&fixture.store, owner:Owner { backend:Backend::Sql,
            path:"runtime/compaction.sqlite".into(),format:1,layout:1 },node:"probe".into(),tables:&["business"],
            revisions:"SELECT 'peer:1',1,'peer',1",cleanup:"",restore_schema:None };
        let snapshot = owner.snapshot().unwrap();
        let mut eager = snapshot.clone();
        eager.data = serde_json::from_slice(&serde_json::to_vec(&snapshot.data).unwrap()).unwrap();
        let original = Initialization::from_snapshot(&eager).operation("plugin_storage",false).unwrap();
        let wire = Initialization::from_snapshot(&snapshot).operation("plugin_storage",false).unwrap();
        let reference = RuntimeFileSyncStore::requiredBlob(&wire).unwrap().unwrap();
        assert_eq!(reference.contentHash,digest(&original).unwrap());
        let files = RuntimeFileSyncStore::new(fixture.store.storage.clone().unwrap(),"runtime/sync");
        assert_eq!(files.readSqlInitialization(&wire).unwrap().data,eager.data);
        let mut forged = wire.clone(); forged.entityId = "runtime/other.sqlite".into();
        assert!(crate::SyncBlobIo::verify_transaction(fixture.store.storage.as_ref().unwrap().as_ref(),"runtime/sync",&forged).is_err());
        assert!(files.readSqlInitialization(&forged).is_err());
        // An unverified pending envelope must be replaceable after its blob arrives.
        let storage = fixture.store.storage.as_ref().unwrap();
        let path = files.blobStoragePath(&reference).unwrap();
        let bytes = storage.readBytes(&path).unwrap();
        storage.delete(&path,false).unwrap();
        let journal = crate::SyncOperationStore::SyncOperationStore::new(storage.clone(),"runtime/sync");
        journal.receiveOperations(&[forged]).unwrap();
        files.writeBlob(&reference,&bytes).unwrap();
        journal.receiveOperations(&[wire.clone()]).unwrap();
        assert_eq!(journal.pendingOperations().unwrap(),vec![wire.clone()]);
        if checkpoint {
            fixture.store.executeBatch("CREATE TABLE _operit_transaction_checkpoint(id INTEGER PRIMARY KEY,snapshot TEXT)").unwrap();
            fixture.store.execute("INSERT INTO _operit_transaction_checkpoint VALUES(1,?1)",vec![text(&serde_json::to_string(&eager).unwrap())]).unwrap();
        } else {
            fixture.store.transaction(|tx| save_initialization(tx,&original,&eager.covered,&eager.data)).unwrap();
            fixture.store.executeBatch("DELETE FROM _operit_initialization_metadata; UPDATE _operit_compacted SET fingerprint='corrupt'").unwrap();
            assert!(owner.snapshot().is_err());
            fixture.store.execute("UPDATE _operit_compacted SET fingerprint=?1",vec![text(&digest(&original).unwrap())]).unwrap();
        }
        assert_eq!(owner.snapshot().unwrap().data,eager.data);
        let migrated = owner.initialization().unwrap().unwrap();
        assert_eq!(migrated,wire);
        assert!(fixture.store.queryOne("SELECT 1 FROM sqlite_master WHERE name='_operit_transaction_checkpoint'",vec![]).unwrap().is_none());
        drop(snapshot);
        assert!(fixture.store.storage.as_ref().unwrap().list("runtime/sync/staging").unwrap().is_empty());
    }
}
