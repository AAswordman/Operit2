//! The equation compares complete states, including future conflict behavior.
use operit_store::Transaction::{
    migration::{Context, Definition},
    *,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn owner(backend: Backend) -> Owner {
    Owner {
        backend,
        path: "old".into(),
        format: 1,
        layout: 1,
    }
}
fn tx(owner: &Owner, n: i64, edits: Vec<Edit>) -> Transaction {
    Transaction {
        id: format!("author:{n}"),
        revision: Revision {
            createdAt: n,
            originDeviceId: "author".into(),
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
fn definition(rules: Vec<Rule>) -> Definition {
    Definition::new(move |_, context| {
        for rule in &rules {
            rule.apply(context)
                .map_err(operit_store::PreferencesDataStore::PreferencesDataStoreError::Message)?;
        }
        Ok(())
    })
}
fn migrate(step: &Definition, data: &Data) -> Data {
    let mut context = Context::state(data.clone());
    step.apply(1, &mut context).unwrap();
    context.after
}
fn law(step: &Definition, before: &Data, original: &Transaction) -> Data {
    let mut after = before.clone();
    after.apply(original).unwrap();
    let expected = migrate(step, &after);
    let mut context = Context::state(before.clone());
    context.after = after.clone();
    context.original = Some(original.clone());
    context.transactions = vec![original.clone()];
    step.apply(1, &mut context).unwrap();
    let mut actual = migrate(step, before);
    for converted in &context.transactions {
        actual.apply(converted).unwrap();
    }
    assert_eq!(actual, expected);
    assert_eq!(actual.revision, after.revision);
    after
}

#[test]
fn rename_split_and_drop_commute_for_complete_states_and_noop_effects() {
    for backend in [Backend::DataStore, Backend::Sql] {
        let owner = owner(backend);
        let step = definition(vec![
            Rule::RenameKey {
                from: "name".into(),
                to: "displayName".into(),
            },
            Rule::SplitObject {
                key: "settings".into(),
                fields: BTreeMap::from([
                    ("count".into(), "counter".into()),
                    ("flag".into(), "enabled".into()),
                ]),
            },
            Rule::DropKey {
                key: "retired_index".into(),
            },
        ]);
        let mut state = Data::empty(owner.clone());
        let edits = [
            vec![
                set("name", json!("same")),
                set("settings", json!({"count":10,"flag":true})),
                set("retired_index", json!(["a"])),
            ],
            vec![set("name", json!("same"))],
            vec![Edit::Increment {
                key: "settings".into(),
                path: vec!["count".into()],
                delta: 3,
            }],
            vec![Edit::Delete {
                key: "settings".into(),
                path: vec!["flag".into()],
            }],
            vec![Edit::Delete {
                key: "name".into(),
                path: vec![],
            }],
            vec![Edit::Delete {
                key: "name".into(),
                path: vec![],
            }],
            vec![Edit::Patch {
                key: "settings".into(),
                set: BTreeMap::from([("flag".into(), json!(false))]),
                delete: Default::default(),
            }],
            vec![Edit::Delete {
                key: "settings".into(),
                path: vec![],
            }],
        ];
        for (index, edits) in edits.into_iter().enumerate() {
            state = law(&step, &state, &tx(&owner, index as i64 + 1, edits));
        }
    }
}

#[test]
fn collection_split_preserves_namespace_absence_and_late_independent_records() {
    for backend in [Backend::DataStore, Backend::Sql] {
        let owner = owner(backend);
        let step = definition(vec![Rule::SplitRecords {
            key: "cards".into(),
            prefix: "card_".into(),
            identity: "id".into(),
        }]);
        let mut state = Data::empty(owner.clone());
        for (n, edit) in [
            (
                10,
                set(
                    "cards",
                    json!([{"id":"a","count":10},{"id":"removed","count":1}]),
                ),
            ),
            (50, set("cards", json!([{"id":"a","count":20}]))),
            (
                40,
                set(
                    "cards",
                    json!([{"id":"a","count":0},{"id":"removed","count":1},{"id":"unseen","count":2}]),
                ),
            ),
        ] {
            // Distinct authors model delayed transactions without violating author sequence.
            let mut original = tx(&owner, n, vec![edit]);
            original.revision.originDeviceId = format!("author-{n}");
            state = law(&step, &state, &original);
        }
        let migrated = migrate(&step, &state);
        assert_eq!(
            migrated.values,
            BTreeMap::from([("card_a".into(), json!({"id":"a","count":20}))])
        );
        let mut snapshot = Snapshot {
            node: "a".into(),
            data: migrated,
            covered: Default::default(),
            generation: 0,
        };
        let mut target = snapshot.clone();
        target.node = "b".into();
        assert!(target.replace(&target.token().unwrap(), &snapshot).unwrap());
        let target_owner = target.data.owner.clone();
        let late = Transaction {
            revision: Revision {
                createdAt: 40,
                originDeviceId: "unseen".into(),
                sequence: 1,
            },
            ..tx(
                &target_owner,
                1,
                vec![set("card_unseen", json!({"id":"unseen","count":99}))],
            )
        };
        snapshot.data.apply(&late).unwrap();
        target.data.apply(&late).unwrap();
        assert!(!target.data.values.contains_key("card_unseen"));
        assert_eq!(target.data, snapshot.data);
    }
}

#[test]
fn collection_split_preserves_nested_deletion_and_additive_contributions() {
    let owner = owner(Backend::Sql);
    let step = definition(vec![Rule::SplitRecords {
        key: "cards".into(),
        prefix: "card_".into(),
        identity: "id".into(),
    }]);
    let mut state = Data::empty(owner.clone());
    for (n, edit) in [
        (
            10,
            set("cards", json!([{"id":"a","count":10},{"id":"b","count":1}])),
        ),
        (
            50,
            Edit::Increment {
                key: "cards".into(),
                path: vec![Path::Item("a".into()), "count".into()],
                delta: 3,
            },
        ),
        (
            40,
            set("cards", json!([{"id":"a","count":20},{"id":"b","count":1}])),
        ),
        (
            60,
            Edit::Delete {
                key: "cards".into(),
                path: vec![Path::Item("b".into())],
            },
        ),
    ] {
        let mut original = tx(&owner, n, vec![edit]);
        original.revision.originDeviceId = format!("author-{n}");
        state = law(&step, &state, &original);
    }
    assert_eq!(migrate(&step, &state).values["card_a"]["count"], 23);
}

#[test]
fn invalid_migrations_fail_atomically_instead_of_confirming_an_incorrect_effect() {
    let owner = owner(Backend::DataStore);
    let original = tx(&owner, 50, vec![set("name", json!("new"))]);
    let before = Data::empty(owner);
    let mut context = Context::state(before.clone());
    context.after.apply(&original).unwrap();
    context.original = Some(original.clone());
    context.transactions = vec![original];
    let expected = context.after.clone();
    let bad = Definition::new(|_, context| {
        context.after.versions.clear();
        Ok(())
    });
    assert!(bad
        .apply(1, &mut context)
        .unwrap_err()
        .to_string()
        .contains("violates"));
    assert_eq!(context.after, expected);
    assert_eq!(context.before, before);
}

#[test]
fn combined_layout_and_format_commute_for_sequences_and_aliases() {
    for backend in [Backend::DataStore, Backend::Sql] {
        let owner = owner(backend);
        let destination = Owner {
            path: "new".into(),
            format: 2,
            layout: 2,
            ..owner.clone()
        };
        let migrations = Migrations {
            owners: BTreeMap::from([("new".into(), destination.clone())]),
            layouts: BTreeMap::from([("old".into(), "new".into())]),
            formats: BTreeMap::from([(
                ("new".into(), 1),
                vec![Rule::RenameKey {
                    from: "name".into(),
                    to: "title".into(),
                }],
            )]),
        };
        let mut source = Data::empty(owner.clone());
        source
            .aliases
            .insert("old\u{001f}alias".into(), "canonical".into());
        let mut converted = source.clone();
        migrations.migrate(&mut converted).unwrap();
        assert_eq!(converted.aliases["new\u{001f}alias"], "canonical");
        for (n, edits) in [
            (
                10,
                vec![set("name", json!("same")), set("count", json!(10))],
            ),
            (
                20,
                vec![
                    set("name", json!("same")),
                    Edit::Increment {
                        key: "count".into(),
                        path: vec![],
                        delta: 3,
                    },
                ],
            ),
            (
                30,
                vec![Edit::Delete {
                    key: "name".into(),
                    path: vec![],
                }],
            ),
        ] {
            let original = tx(&owner, n, edits);
            source.apply(&original).unwrap();
            let mut expected = source.clone();
            migrations.migrate(&mut expected).unwrap();
            let mut effect = original.clone();
            migrations.migrate(&mut effect).unwrap();
            converted.apply(&effect).unwrap();
            assert_eq!(converted, expected);
            assert_eq!(effect.id, original.id);
            assert_eq!(effect.revision, original.revision);
            assert_eq!(converted.owner, destination);
        }
    }
}

#[test]
fn failed_structural_steps_leave_state_and_transaction_intact() {
    let owner = owner(Backend::Sql);
    let mut state = Data::empty(owner.clone());
    state
        .apply(&tx(
            &owner,
            1,
            vec![set("cards", json!([{"id":"a"},{"id":"a"}]))],
        ))
        .unwrap();
    let original = state.clone();
    let split = Rule::SplitRecords {
        key: "cards".into(),
        prefix: "card_".into(),
        identity: "id".into(),
    };
    assert!(split.apply(&mut state).is_err());
    assert_eq!(state, original);
    let migrations = Migrations {
        owners: BTreeMap::from([(
            "old".into(),
            Owner {
                format: 2,
                ..owner.clone()
            },
        )]),
        layouts: Default::default(),
        formats: BTreeMap::from([(
            ("old".into(), 1),
            vec![
                Rule::RenameKey {
                    from: "cards".into(),
                    to: "renamed".into(),
                },
                Rule::SplitRecords {
                    key: "renamed".into(),
                    prefix: "card_".into(),
                    identity: "id".into(),
                },
            ],
        )]),
    };
    assert!(migrations.migrate_data(&mut state).is_err());
    assert_eq!(state, original);
    let mut transaction = tx(
        &owner,
        2,
        vec![set("cards", json!([{"id":"a"},{"id":"a"}]))],
    );
    let original = transaction.clone();
    assert!(migrations.migrate_transaction(&mut transaction).is_err());
    assert_eq!(transaction, original);
}

#[test]
fn codecs_convert_repeated_effects_using_their_intermediate_context() {
    use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStoreError};
    let step = Definition::from_preferences(|_, state| {
        let key = stringPreferencesKey("encoded");
        if let Some(value) = state.get(&key) {
            let value: Value = serde_json::from_str(value.as_str().unwrap())?;
            state.set(&key, value);
        }
        Ok::<_, PreferencesDataStoreError>(())
    });
    let owner = owner(Backend::DataStore);
    let original = tx(
        &owner,
        1,
        vec![
            set("encoded", json!("1")),
            set("encoded", json!("2")),
            Edit::Delete {
                key: "encoded".into(),
                path: vec![],
            },
        ],
    );
    law(&step, &Data::empty(owner.clone()), &original);
    let mut context = Context::state(Data::empty(owner));
    context.after.apply(&original).unwrap();
    context.original = Some(original.clone());
    context.transactions = vec![original];
    step.apply(1, &mut context).unwrap();
    assert_eq!(context.transactions[0].edits[0], set("encoded", json!(1)));
    assert_eq!(context.transactions[0].edits[1], set("encoded", json!(2)));
}

#[test]
fn transaction_conversion_must_use_the_same_state_function_as_file_migration() {
    let owner = owner(Backend::DataStore);
    let original = tx(&owner, 1, vec![set("x", json!(1))]);
    let mut context = Context::state(Data::empty(owner));
    context.after.apply(&original).unwrap();
    context.original = Some(original.clone());
    context.transactions = vec![original];
    let expected = context.after.clone();
    let inconsistent = Definition::new(|_, context| {
        if context.original.is_some() {
            context.before.values.insert("x".into(), json!(2));
            context.after.values.insert("x".into(), json!(2));
            context.transactions[0].edits = vec![set("x", json!(2))];
        }
        Ok(())
    });
    // Applying the converted effects to the converter's own before/after views
    // would pass; independent state migration must reject this different m.
    assert!(inconsistent
        .apply(1, &mut context)
        .unwrap_err()
        .to_string()
        .contains("shared state function"));
    assert_eq!(context.after, expected);
}

#[test]
fn codecs_preserve_unrelated_nested_versions_and_additions() {
    use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStoreError};
    let step = Definition::from_preferences(|_, state| {
        let key = stringPreferencesKey("provider");
        if let Some(value) = state.get(&key) {
            let mut value = value.clone();
            if value["type"] == "IFLOW" {
                value["type"] = json!("OTHER");
            }
            state.set(&key, value);
        }
        Ok::<_, PreferencesDataStoreError>(())
    });
    let owner = owner(Backend::DataStore);
    let mut before = Data::empty(owner.clone());
    before
        .apply(&tx(
            &owner,
            1,
            vec![set(
                "provider",
                json!({"type":"IFLOW","name":"old","count":0}),
            )],
        ))
        .unwrap();
    before
        .apply(&tx(
            &owner,
            2,
            vec![Edit::Increment {
                key: "provider".into(),
                path: vec!["count".into()],
                delta: 2,
            }],
        ))
        .unwrap();
    for (index, edits) in [
        vec![Edit::Patch {
            key: "provider".into(),
            set: BTreeMap::from([("name".into(), json!("new"))]),
            delete: Default::default(),
        }],
        vec![Edit::Set {
            key: "provider".into(),
            path: vec!["type".into()],
            value: json!("IFLOW"),
        }],
    ]
    .into_iter()
    .enumerate()
    {
        before = law(&step, &before, &tx(&owner, 3 + index as i64, edits));
    }
}

#[test]
fn default_projection_maps_root_and_field_deletion_to_the_same_versioned_default() {
    use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStoreError};
    let step = Definition::from_preferences(|_, state| {
        let key = stringPreferencesKey("record");
        let mut value = state.get(&key).cloned().unwrap_or(json!({}));
        value
            .as_object_mut()
            .unwrap()
            .entry("choice")
            .or_insert(json!("default"));
        state.set(&key, value);
        Ok::<_, PreferencesDataStoreError>(())
    });
    let owner = owner(Backend::DataStore);
    let mut before = Data::empty(owner.clone());
    for (n, edits) in [
        (1, vec![set("record", json!({"choice":"custom"}))]),
        (
            2,
            vec![Edit::Patch {
                key: "record".into(),
                set: BTreeMap::new(),
                delete: ["choice".into()].into(),
            }],
        ),
        (
            3,
            vec![Edit::Delete {
                key: "record".into(),
                path: vec![],
            }],
        ),
        (
            4,
            vec![Edit::Delete {
                key: "record".into(),
                path: vec![],
            }],
        ),
    ] {
        before = law(&step, &before, &tx(&owner, n, edits));
    }
}

#[test]
fn default_records_survive_atomic_namespace_replacement() {
    use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStoreError};
    let step = Definition::from_preferences(|_, state| {
        let key = stringPreferencesKey("profile_default");
        if !state.contains(&key) {
            state.set(&key, json!({"id":"default"}));
        }
        Ok::<_, PreferencesDataStoreError>(())
    });
    let owner = owner(Backend::DataStore);
    let mut before = Data::empty(owner.clone());
    for (n, records) in [
        (
            1,
            BTreeMap::from([("custom".into(), json!({"id":"custom"}))]),
        ),
        (2, BTreeMap::new()),
    ] {
        before = law(
            &step,
            &before,
            &tx(
                &owner,
                n,
                vec![Edit::ReplaceRecords {
                    prefix: "profile_".into(),
                    records,
                }],
            ),
        );
    }
}
