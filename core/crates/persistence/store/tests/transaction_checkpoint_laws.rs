//! A layout may join two occupied owners. Its state and transaction sides must commute.
use operit_store::Transaction::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn owner(backend: Backend, path: &str, format: u32) -> Owner {
    Owner {
        backend,
        path: path.into(),
        format,
        layout: 1,
    }
}
fn tx(owner: &Owner, author: &str, n: i64, edits: Vec<Edit>) -> Transaction {
    Transaction {
        id: format!("{author}:{n}"),
        revision: Revision {
            createdAt: n,
            originDeviceId: author.into(),
            sequence: n,
        },
        target: Target::Data(owner.clone()),
        edits,
    }
}
fn set(key: &str, value: Value) -> Edit {
    Edit::Set {
        key: key.into(),
        path: vec![],
        value,
    }
}
fn delete(key: &str) -> Edit {
    Edit::Delete {
        key: key.into(),
        path: vec![],
    }
}
fn increment(key: &str, delta: i64) -> Edit {
    Edit::Increment {
        key: key.into(),
        path: vec![],
        delta,
    }
}
fn migrations(source: &Owner, destination: &Owner) -> Migrations {
    Migrations {
        owners: BTreeMap::from([(destination.path.clone(), destination.clone())]),
        layouts: BTreeMap::from([(source.path.clone(), destination.path.clone())]),
        formats: BTreeMap::from([(
            (destination.path.clone(), 1),
            vec![
                Rule::RenameKey {
                    from: "name".into(),
                    to: "displayName".into(),
                },
                Rule::SplitRecords {
                    key: "cards".into(),
                    prefix: "card_".into(),
                    identity: "id".into(),
                },
            ],
        )]),
    }
}
fn moved(source: &Data, destination: &Data, migrations: &Migrations) -> Data {
    let mut converted = source.clone();
    migrations.migrate(&mut converted).unwrap();
    let mut merged = destination.clone();
    merged.merge(&converted).unwrap();
    merged
}
fn law(source: &Data, destination: &Data, migrations: &Migrations, transaction: &Transaction) {
    let mut applied = source.clone();
    applied.apply(transaction).unwrap();
    let left = moved(&applied, destination, migrations);
    let mut right = moved(source, destination, migrations);
    let mut converted = transaction.clone();
    migrations.migrate(&mut converted).unwrap();
    right.apply(&converted).unwrap();
    assert_eq!(
        left, right,
        "m_L(Apply(A,T)) must equal Apply(m_L(A),m_L(T))"
    );
}

#[test]
fn occupied_layout_commutes_for_atomic_assignments_deletions_and_increments() {
    for backend in [Backend::DataStore, Backend::Sql] {
        let source_owner = owner(backend.clone(), "old", 1);
        let destination_owner = owner(backend, "new", 2);
        let migrations = migrations(&source_owner, &destination_owner);
        let mut source = Data::empty(source_owner.clone());
        source
            .apply(&tx(
                &source_owner,
                "source",
                10,
                vec![
                    set("name", json!("old")),
                    set("sourceOnly", json!(true)),
                    set("counter", json!(10)),
                    set("gone", json!("old")),
                ],
            ))
            .unwrap();
        source
            .apply(&tx(
                &source_owner,
                "source",
                50,
                vec![increment("counter", 3), delete("gone")],
            ))
            .unwrap();
        let mut destination = Data::empty(destination_owner.clone());
        destination
            .apply(&tx(
                &destination_owner,
                "destination",
                40,
                vec![
                    set("displayName", json!("new")),
                    set("destinationOnly", json!(true)),
                    set("counter", json!(20)),
                    set("gone", json!("stale")),
                ],
            ))
            .unwrap();
        destination
            .apply(&tx(
                &destination_owner,
                "destination",
                80,
                vec![increment("counter", 7)],
            ))
            .unwrap();
        let merged = moved(&source, &destination, &migrations);
        assert_eq!(merged.values["counter"], 30);
        assert!(!merged.values.contains_key("gone"));
        for (n, edits) in [
            (
                30,
                vec![
                    set("name", json!("older")),
                    set("atomicSibling", json!(true)),
                ],
            ),
            (45, vec![set("counter", json!(30))]),
            (60, vec![increment("counter", 2)]),
            (70, vec![delete("name"), set("atomicSibling", json!(false))]),
        ] {
            law(
                &source,
                &destination,
                &migrations,
                &tx(&source_owner, "late", n, edits),
            );
        }
    }
}

#[test]
fn checkpoint_join_is_idempotent_commutative_and_deduplicates_shared_increments() {
    let owner = owner(Backend::Sql, "new", 2);
    let mut common = Data::empty(owner.clone());
    common
        .apply(&tx(
            &owner,
            "common",
            10,
            vec![
                set("counter", json!(10)),
                set("profile", json!({"a":0,"b":0})),
            ],
        ))
        .unwrap();
    common
        .apply(&tx(
            &owner,
            "common",
            20,
            vec![increment("counter", 1), increment("counter", 2)],
        ))
        .unwrap();
    let mut a = common.clone();
    a.apply(&tx(
        &owner,
        "a",
        30,
        vec![Edit::Patch {
            key: "profile".into(),
            set: BTreeMap::from([("a".into(), json!(1))]),
            delete: Default::default(),
        }],
    ))
    .unwrap();
    let mut b = common.clone();
    b.apply(&tx(
        &owner,
        "b",
        40,
        vec![Edit::Patch {
            key: "profile".into(),
            set: BTreeMap::from([("b".into(), json!(2))]),
            delete: Default::default(),
        }],
    ))
    .unwrap();
    let mut ab = a.clone();
    ab.merge(&b).unwrap();
    let mut ba = b.clone();
    ba.merge(&a).unwrap();
    assert_eq!(ab, ba);
    assert_eq!(ab.values["counter"], 13);
    assert_eq!(ab.values["profile"], json!({"a":1,"b":2}));
    let before = ab.clone();
    ab.merge(&ab.clone()).unwrap();
    assert_eq!(ab, before);
    let mut c = common.clone();
    c.apply(&tx(&owner, "c", 50, vec![increment("counter", 4)]))
        .unwrap();
    let mut left = ab.clone();
    left.merge(&c).unwrap();
    let mut bc = b.clone();
    bc.merge(&c).unwrap();
    let mut right = a.clone();
    right.merge(&bc).unwrap();
    assert_eq!(left, right);
    assert_eq!(left.values["counter"], 17);
}

#[test]
fn collection_absence_survives_joining_an_existing_destination_and_late_history() {
    for backend in [Backend::DataStore, Backend::Sql] {
        let old = owner(backend.clone(), "old", 1);
        let new = owner(backend, "new", 2);
        let migrations = migrations(&old, &new);
        let mut source = Data::empty(old.clone());
        source
            .apply(&tx(
                &old,
                "source",
                50,
                vec![set("cards", json!([{"id":"a","count":10}]))],
            ))
            .unwrap();
        let mut destination = Data::empty(new.clone());
        destination
            .apply(&tx(
                &new,
                "destination",
                40,
                vec![
                    set("card_stale", json!({"id":"stale"})),
                    set("independent", json!(true)),
                ],
            ))
            .unwrap();
        destination
            .apply(&tx(
                &new,
                "destination",
                60,
                vec![set("card_newer", json!({"id":"newer"}))],
            ))
            .unwrap();
        let merged = moved(&source, &destination, &migrations);
        assert!(!merged.values.contains_key("card_stale"));
        assert!(merged.values.contains_key("card_newer"));
        for n in [30, 70] {
            law(
                &source,
                &destination,
                &migrations,
                &tx(
                    &old,
                    "late",
                    n,
                    vec![set("cards", json!([{"id":"unseen","count":1}]))],
                ),
            );
        }
    }
}

#[test]
fn conflicting_checkpoint_aliases_fail_without_publishing_partial_state() {
    let owner = owner(Backend::DataStore, "new", 2);
    let mut a = Data::empty(owner.clone());
    a.aliases.insert("new\u{001f}old".into(), "a".into());
    let mut b = Data::empty(owner);
    b.aliases.insert("new\u{001f}old".into(), "b".into());
    let before = a.clone();
    assert!(a.merge(&b).is_err());
    assert_eq!(a, before);
}

#[test]
fn occupied_layout_preserves_newer_nested_scopes_across_parent_replacement_and_deletion() {
    let old = owner(Backend::DataStore, "old", 1);
    let new = owner(Backend::DataStore, "new", 2);
    let migrations = migrations(&old, &new);
    let mut source = Data::empty(old.clone());
    source
        .apply(&tx(
            &old,
            "source",
            10,
            vec![set("profile", json!({"nested":{"a":0,"b":0},"obsolete":1}))],
        ))
        .unwrap();
    let mut destination = Data::empty(new.clone());
    destination
        .apply(&tx(
            &new,
            "destination",
            20,
            vec![set("profile", json!({"nested":{"a":0,"b":0}}))],
        ))
        .unwrap();
    destination
        .apply(&tx(
            &new,
            "destination",
            50,
            vec![Edit::Set {
                key: "profile".into(),
                path: vec!["nested".into(), "a".into()],
                value: json!(5),
            }],
        ))
        .unwrap();
    for (n, edit) in [
        (30, set("profile", json!({"fresh":1}))),
        (40, delete("profile")),
        (40, set("profile", json!(0))),
    ] {
        law(
            &source,
            &destination,
            &migrations,
            &tx(&old, "late", n, vec![edit]),
        );
    }
}

#[test]
fn occupied_layout_preserves_identity_item_edits_after_parent_and_item_deletion() {
    for backend in [Backend::DataStore, Backend::Sql] {
        let old = owner(backend.clone(), "old", 1);
        let new = owner(backend, "new", 2);
        let migrations = migrations(&old, &new);
        let initial = json!([{"id":"a","details":{"count":0,"obsolete":true}}, {"id":"b"}]);
        let mut source = Data::empty(old.clone());
        source
            .apply(&tx(&old, "source", 10, vec![set("items", initial.clone())]))
            .unwrap();
        let mut destination = Data::empty(new.clone());
        destination
            .apply(&tx(&new, "destination", 20, vec![set("items", initial)]))
            .unwrap();
        destination
            .apply(&tx(
                &new,
                "destination",
                50,
                vec![Edit::Set {
                    key: "items".into(),
                    path: vec![Path::Item("a".into()), "details".into(), "count".into()],
                    value: json!(5),
                }],
            ))
            .unwrap();
        for edit in [
            delete("items"),
            set("items", json!([])),
            Edit::Delete {
                key: "items".into(),
                path: vec![Path::Item("a".into())],
            },
        ] {
            law(
                &source,
                &destination,
                &migrations,
                &tx(&old, "late", 30, vec![edit]),
            );
        }
    }
}

#[test]
fn typed_paths_rebuild_only_surviving_fields_and_invalid_indices_remain_atomic() {
    let owner = owner(Backend::Sql, "new", 2);
    let mut data = Data::empty(owner.clone());
    let path = vec![Path::Item("a".into()), "details".into(), "count".into()];
    data.apply(&tx(
        &owner,
        "delete",
        30,
        vec![Edit::Delete {
            key: "items".into(),
            path: path.clone(),
        }],
    ))
    .unwrap();
    assert!(data.values.is_empty());
    let before = data.clone();
    data.apply(&tx(
        &owner,
        "stale",
        20,
        vec![Edit::Set {
            key: "items".into(),
            path: path.clone(),
            value: json!(1),
        }],
    ))
    .unwrap();
    assert_eq!(data, before);
    data.apply(&tx(
        &owner,
        "write",
        40,
        vec![Edit::Set {
            key: "items".into(),
            path: path.clone(),
            value: json!(5),
        }],
    ))
    .unwrap();
    assert_eq!(
        data.values["items"],
        json!([{"id":"a","details":{"count":5}}])
    );
    let before = data.clone();
    assert!(data
        .apply(&tx(
            &owner,
            "bad",
            50,
            vec![
                set("sibling", json!(true)),
                Edit::Set {
                    key: "items".into(),
                    path: vec!["999".into()],
                    value: json!(1)
                }
            ]
        ))
        .is_err());
    assert_eq!(data, before);
}

#[test]
fn occupied_layout_routes_newer_source_children_past_destination_parent_tombstones() {
    let old = owner(Backend::DataStore, "old", 1);
    let new = owner(Backend::DataStore, "new", 2);
    let migrations = migrations(&old, &new);
    let mut source = Data::empty(old.clone());
    source
        .apply(&tx(
            &old,
            "source",
            10,
            vec![set("profile", json!({"name":"old"}))],
        ))
        .unwrap();
    for edit in [delete("profile"), set("profile", json!(0))] {
        let mut destination = Data::empty(new.clone());
        destination
            .apply(&tx(&new, "destination", 30, vec![edit]))
            .unwrap();
        law(
            &source,
            &destination,
            &migrations,
            &tx(
                &old,
                "late",
                50,
                vec![Edit::Set {
                    key: "profile".into(),
                    path: vec!["name".into()],
                    value: json!("new"),
                }],
            ),
        );
    }
}

#[test]
fn occupied_layout_matches_the_equation_across_parent_child_and_increment_orderings() {
    for backend in [Backend::DataStore, Backend::Sql] {
        let old = owner(backend.clone(), "old", 1);
        let new = owner(backend, "new", 2);
        let migrations = migrations(&old, &new);
        let nested_set = || Edit::Set {
            key: "profile".into(),
            path: vec!["nested".into(), "count".into()],
            value: json!(9),
        };
        let nested_delete = || Edit::Delete {
            key: "profile".into(),
            path: vec!["nested".into(), "count".into()],
        };
        let nested_increment = || Edit::Increment {
            key: "profile".into(),
            path: vec!["nested".into(), "count".into()],
            delta: 3,
        };
        let choices = || {
            vec![
                set("profile", json!({"fresh":true})),
                set("profile", json!(0)),
                delete("profile"),
                nested_set(),
                nested_delete(),
                nested_increment(),
            ]
        };
        let mut source = Data::empty(old.clone());
        source
            .apply(&tx(
                &old,
                "source",
                10,
                vec![set(
                    "profile",
                    json!({"nested":{"count":2,"sibling":true},"obsolete":true}),
                )],
            ))
            .unwrap();
        for destination_edit in choices() {
            let mut destination = Data::empty(new.clone());
            destination
                .apply(&tx(
                    &new,
                    "destination",
                    20,
                    vec![set("profile", json!({"nested":{"count":4,"sibling":true}}))],
                ))
                .unwrap();
            destination
                .apply(&tx(&new, "destination", 40, vec![destination_edit]))
                .unwrap();
            for n in [30, 50] {
                for edit in choices() {
                    law(
                        &source,
                        &destination,
                        &migrations,
                        &tx(&old, "late", n, vec![edit]),
                    );
                }
            }
        }
    }
}
