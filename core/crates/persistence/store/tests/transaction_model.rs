use std::collections::{BTreeMap, BTreeSet};
use operit_store::Transaction::*;
use serde_json::{json, Value};

fn owner(backend: Backend, version: u32, path: &str) -> Owner {
    Owner { backend, path: path.into(), format: version, layout: 1 }
}
fn set(key: &str, value: Value) -> Edit { Edit::Set { key: key.into(), path: vec![], value } }
fn tx(owner: &Owner, id: &str, time: i64, edits: Vec<Edit>) -> Transaction {
    Transaction { id: id.into(), revision: Revision { createdAt: time, originDeviceId: id.into(), sequence: 1 }, target: Target::Data(owner.clone()), edits }
}
fn migration(old: &Owner, new: &Owner) -> Migrations {
    let mut m = Migrations::default();
    m.owners.insert(new.path.clone(), new.clone());
    if old.path != new.path { m.layouts.insert(old.path.clone(), new.path.clone()); }
    if old.format < new.format {
        m.formats.insert((new.path.clone(), old.format), vec![Rule::RenameKey { from: "name".into(), to: "displayName".into() }]);
    }
    m
}
fn history(transactions: &[Transaction]) -> History {
    let mut h = History::default();
    for tx in transactions { h.insert(tx.clone()).unwrap(); }
    h
}
fn snapshot(node: &str, data: Data, covered: &[&str]) -> Snapshot {
    Snapshot { damage: None, node: node.into(), data, covered: covered.iter().map(|id| id.to_string()).collect(), generation: 0 }
}

#[test]
fn transaction_is_sql_data_and_can_target_both_backends() {
    for backend in [Backend::DataStore, Backend::Sql] {
        let owner = owner(backend, 1, "cards");
        let transaction = tx(&owner, "a", 10, vec![set("name", json!("甲"))]);
        assert_eq!(transaction.backend(), Backend::Sql);
        let mut data = Data::empty(owner);
        data.apply(&transaction).unwrap();
        assert_eq!(data.values["name"], "甲");
    }
}

#[test]
fn format_and_layout_commute_with_application_for_both_backends() {
    for backend in [Backend::DataStore, Backend::Sql] {
        for move_path in [false, true] {
            let old = owner(backend.clone(), 1, "old");
            let new = owner(backend.clone(), 2, if move_path { "new" } else { "old" });
            let m = migration(&old, &new);
            let mut a = Data::empty(old.clone());
            a.values.insert("name".into(), json!("甲"));
            let transactions = vec![tx(&old, "one", 10, vec![set("name", json!("乙")), set("kept", json!(true))]),
                tx(&old, "two", 20, vec![Edit::Delete { key: "name".into(), path: vec![] }])];
            let mut left = a.clone();
            for t in &transactions { left.apply(t).unwrap(); }
            m.migrate(&mut left).unwrap();
            let mut right = a;
            m.migrate(&mut right).unwrap();
            for original in transactions {
                let mut t = original.clone();
                m.migrate(&mut t).unwrap();
                assert_eq!(t.id, original.id);
                assert_eq!(t.revision, original.revision);
                right.apply(&t).unwrap();
            }
            assert_eq!(left, right);
            assert_eq!(left.owner, new);
        }
    }
}

#[test]
fn field_migration_preserves_set_delete_and_increment_operations() {
    let old = owner(Backend::Sql, 1, "table");
    let new = owner(Backend::Sql, 2, "table");
    let mut m = migration(&old, &new);
    m.formats.insert(("table".into(), 1), vec![Rule::RenameField { key: "row".into(), from: "count".into(), to: "total".into() }]);
    let transactions = vec![tx(&old, "one", 1, vec![set("row", json!({"count":10,"other":true}))]),
        tx(&old, "two", 2, vec![Edit::Increment { key: "row".into(), path: vec!["count".into()], delta: 3 }]),
        tx(&old, "three", 3, vec![Edit::Delete { key: "row".into(), path: vec!["count".into()] }])];
    let mut left = Data::empty(old.clone());
    let mut right = Data::empty(old);
    m.migrate(&mut right).unwrap();
    for original in transactions {
        left.apply(&original).unwrap();
        let mut view = original;
        m.migrate(&mut view).unwrap();
        right.apply(&view).unwrap();
        let mut migrated = left.clone();
        m.migrate(&mut migrated).unwrap();
        assert_eq!(migrated, right);
    }
}

#[test]
fn ordering_deduplication_and_atomic_scope_match_the_contract() {
    let owner = owner(Backend::DataStore, 1, "data");
    let one = tx(&owner, "one", 10, vec![set("x", json!(1)), set("y", json!(1))]);
    let two = tx(&owner, "two", 20, vec![set("x", json!(2))]);
    let m = migration(&owner, &owner);
    let baseline = Data::empty(owner);
    let expected = history(&[one.clone(), two.clone()]).materialize(&baseline, &m).unwrap().data;
    let actual = history(&[two.clone(), one.clone(), two, one]).materialize(&baseline, &m).unwrap().data;
    assert_eq!(actual, expected);
    assert_eq!(actual.values, BTreeMap::from([("x".into(), json!(2)), ("y".into(), json!(1))]));
}

#[test]
fn old_format_arrival_and_upgrade_time_do_not_change_business_priority() {
    let old = owner(Backend::DataStore, 1, "old");
    let new = owner(Backend::DataStore, 2, "new");
    let m = migration(&old, &new);
    let baseline = Data::empty(old.clone());
    for (time, expected) in [(10, "丙"), (20, "丙"), (40, "乙")] {
        let old_write = tx(&old, "old", time, vec![set("name", json!("乙"))]);
        let new_write = tx(&new, "new", 30, vec![set("displayName", json!("丙"))]);
        let result = history(&[new_write, old_write]).materialize(&baseline, &m).unwrap();
        assert!(result.pending.is_empty());
        assert_eq!(result.data.values["displayName"], expected);
        assert_eq!(result.data.revision.unwrap().createdAt, time.max(30));
    }
}

#[test]
fn future_and_unknown_layout_stay_pending_while_ready_data_commits() {
    let local = owner(Backend::Sql, 1, "ready");
    let future = owner(Backend::Sql, 2, "ready");
    let unknown = owner(Backend::Sql, 1, "unknown");
    let h = history(&[tx(&future, "future", 10, vec![set("x", json!(2))]),
        tx(&unknown, "unknown", 20, vec![set("x", json!(3))]),
        tx(&local, "ready", 30, vec![set("ok", json!(true))])]);
    let mut m = migration(&local, &local);
    let result = h.materialize(&Data::empty(local.clone()), &m).unwrap();
    assert_eq!(result.pending.len(), 2);
    assert_eq!(result.data.values["ok"], true);
    m.owners.insert("ready".into(), future);
    m.formats.insert(("ready".into(), 1), vec![]);
    let upgraded = h.materialize(&Data::empty(local), &m).unwrap();
    assert!(!upgraded.pending.contains_key("future"));
    assert!(upgraded.pending.contains_key("unknown"));
    assert_eq!(upgraded.data.values["x"], 2);
}

#[test]
fn a_failed_transaction_exposes_none_of_its_edits() {
    let owner = owner(Backend::DataStore, 1, "data");
    let mut a = Data::empty(owner.clone());
    let before = a.clone();
    let bad = tx(&owner, "bad", 10, vec![set("first", json!(true)), set("number", json!("text")),
        Edit::Increment { key: "number".into(), path: vec![], delta: 1 }]);
    assert!(a.apply(&bad).is_err());
    assert_eq!(a, before);
}

#[test]
fn higher_order_views_keep_originals_and_support_third_order() {
    let owner = owner(Backend::Sql, 1, "rows");
    let one = tx(&owner, "one", 10, vec![set("name", json!("original"))]);
    let two = Transaction { id: "two".into(), revision: Revision { createdAt: 20, ..one.revision.clone() },
        target: Target::Transaction(one.id.clone()), edits: vec![set("edits", serde_json::to_value(vec![set("name", json!("second"))]).unwrap())] };
    let three = Transaction { id: "three".into(), revision: Revision { createdAt: 30, ..one.revision.clone() },
        target: Target::Transaction(two.id.clone()), edits: vec![set("edits", serde_json::to_value(vec![set("edits", serde_json::to_value(vec![set("name", json!("third"))]).unwrap())]).unwrap())] };
    let h = history(&[three, two, one.clone()]);
    let result = h.materialize(&Data::empty(owner.clone()), &migration(&owner, &owner)).unwrap();
    assert_eq!(result.data.values["name"], "third");
    assert_eq!(result.data.revision, Some(one.revision.clone()));
    assert_eq!(h.originals().find(|tx| tx.id == "one").unwrap(), &one);
}

#[test]
fn missing_higher_order_targets_wait_and_immutable_conflicts_are_errors() {
    let owner = owner(Backend::Sql, 1, "rows");
    let one = tx(&owner, "one", 10, vec![set("x", json!(1))]);
    let mut h = history(&[one.clone()]);
    let mut conflict = one.clone();
    conflict.edits = vec![set("x", json!(2))];
    assert!(h.insert(conflict).is_err());
    h.insert(Transaction { id: "missing".into(), target: Target::Transaction("absent".into()), ..one }).unwrap();
    assert!(h.materialize(&Data::empty(owner.clone()), &migration(&owner, &owner)).unwrap().pending.contains_key("missing"));
}

/// Equal-size business contents retain revision and node ordering, with CAS enforced.
#[test]
fn correction_breaks_size_ties_by_business_revision_then_node_and_uses_cas() {
    let owner = owner(Backend::DataStore, 1, "file");
    let mut a = Data::empty(owner.clone());
    a.apply(&tx(&owner, "older", 20, vec![set("x", json!("a"))])).unwrap();
    let mut b = Data::empty(owner.clone());
    b.apply(&tx(&owner, "newer", 30, vec![set("x", json!("b"))])).unwrap();
    let mut local = snapshot("a", a, &["older"]);
    let remote = snapshot("b", b, &["newer"]);
    let correction = compare(&local, &remote).unwrap().unwrap();
    assert_eq!(correction.source.node, "b");
    local.generation += 1; // A write followed by restoration still invalidates the captured token.
    assert!(!local.replace(&correction.local_token, &correction.source).unwrap());
    let correction = compare(&local, &remote).unwrap().unwrap();
    assert!(local.replace(&correction.local_token, &correction.source).unwrap());
    assert!(compare(&local, &remote).unwrap().is_none());
    assert_eq!(local.covered, BTreeSet::from(["older".into(), "newer".into()]));
    let mut tied = remote.clone();
    tied.node = "z".into();
    tied.data.values.insert("x".into(), json!("z"));
    assert_eq!(compare(&remote, &tied).unwrap().unwrap().source.node, "z");
}

#[test]
fn checkpoint_replay_keeps_uncovered_older_edits_on_an_independent_scope() {
    let owner = owner(Backend::DataStore, 1, "file");
    let newer_x = tx(&owner, "newer-x", 50, vec![set("x", json!("new"))]);
    let older_y = tx(&owner, "older-y", 40, vec![set("y", json!("independent"))]);
    let mut data = Data::empty(owner.clone());
    data.apply(&newer_x).unwrap();
    let checkpoint = snapshot("a", data, &["newer-x"]);
    let replay = checkpoint.replay(&history(&[newer_x, older_y]), &migration(&owner, &owner)).unwrap();
    assert_eq!(replay.data.values["x"], "new");
    assert_eq!(replay.data.values["y"], "independent");
}

#[test]
fn batch_replay_matches_sequential_conflicts_and_rolls_back_the_whole_failed_batch() {
    let owner = owner(Backend::DataStore, 1, "batch");
    let operations = vec![
        tx(&owner, "base", 10, vec![set("obj", json!({"field":1})), set("count", json!(0))]),
        tx(&owner, "new", 30, vec![Edit::Set { key:"obj".into(), path:vec![Path::Field("field".into())], value:json!(3) }]),
        tx(&owner, "old", 20, vec![set("obj", json!({"field":2, "other":2}))]),
        tx(&owner, "add", 40, vec![Edit::Increment { key:"count".into(), path:vec![], delta:5 }]),
    ];
    let mut sequential = Data::empty(owner.clone());
    for operation in &operations { sequential.apply(operation).unwrap(); }
    let mut batched = Data::empty(owner.clone());
    batched.apply_batch(&operations).unwrap();
    assert_eq!(batched, sequential);
    assert_eq!(batched.values["obj"]["field"], 3);
    let before = batched.clone();
    let good = tx(&owner, "prefix", 50, vec![set("new-key", json!(true))]);
    let bad = tx(&owner, "bad", 60, vec![Edit::Increment { key:"obj".into(), path:vec![], delta:1 }]);
    assert!(batched.apply_batch(&[good, bad]).is_err());
    assert_eq!(batched, before);
    assert!(batched.apply_batch(&[tx(&Owner { format:2, path:"other".into(), ..owner.clone() }, "wrong", 70, vec![])]).is_err());
    assert_eq!(batched, before);
}

#[test]
fn batched_scope_filter_keeps_namespace_tombstones_and_escaped_keys() {
    let owner = owner(Backend::DataStore, 1, "scopes");
    let mut data = Data::empty(owner.clone());
    data.apply_batch(&[
        tx(&owner, "replace", 30, vec![Edit::ReplaceRecords { prefix:"record_".into(), records:BTreeMap::new() }]),
        tx(&owner, "stale", 20, vec![set("record_late", json!(1))]),
        tx(&owner, "escaped", 40, vec![set("quote\"\\中文", json!({"x":1}))]),
        tx(&owner, "patch", 50, vec![Edit::Set { key:"quote\"\\中文".into(), path:vec![Path::Field("x".into())], value:json!(2) }]),
        tx(&owner, "other", 60, vec![set("quote", json!(3))]),
    ]).unwrap();
    assert!(!data.values.contains_key("record_late"));
    assert_eq!(data.values["quote\"\\中文"]["x"], 2);
    assert_eq!(data.values["quote"], 3);
    let before = data.clone();
    data.versions.insert("malformed".into(), Revision {createdAt:1,originDeviceId:"bad".into(),sequence:1});
    let malformed = data.clone();
    assert!(data.apply_batch(&[tx(&owner, "unrelated", 70, vec![set("other", json!(4))])]).is_err());
    assert_eq!(data, malformed);
    assert_ne!(data, before);
}

#[test]
fn restored_noncanonical_scopes_keep_versions_and_additive_effects() {
    let owner = owner(Backend::DataStore, 1, "restored-scopes");
    let mut canonical = Data::empty(owner.clone());
    canonical.apply_batch(&[
        tx(&owner, "base", 10, vec![set("中文", json!({"count":0}))]),
        tx(&owner, "add", 30, vec![Edit::Increment { key:"中文".into(), path:vec!["count".into()], delta:5 }]),
        tx(&owner, "field", 40, vec![Edit::Set { key:"中文".into(), path:vec!["new".into()], value:json!(true) }]),
    ]).unwrap();
    let encode = |key: String| {
        let scope: (String,Vec<Path>) = serde_json::from_str(&key).unwrap();
        serde_json::to_string_pretty(&scope).unwrap().replace("中文", "\\u4e2d\\u6587")
    };
    let mut restored = canonical.clone();
    restored.versions = restored.versions.into_iter().map(|(k,v)|(encode(k),v)).collect();
    restored.additions = restored.additions.into_iter().map(|(k,v)|(encode(k),v)).collect();
    let edits = vec![tx(&owner, "older-parent", 20, vec![set("中文", json!({"count":2}))])];
    canonical.apply_batch(&edits).unwrap(); restored.apply_batch(&edits).unwrap();
    assert_eq!(restored.values, canonical.values);
    assert_eq!(restored.values["中文"]["count"], 7);
    assert_eq!(restored.values["中文"]["new"], true);
}
