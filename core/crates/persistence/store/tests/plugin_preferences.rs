//! Exercises plugin preference callbacks against real native JSON storage and the Space journal.
use operit_host_api::{PluginStorage::StorageRequest, RuntimeStorageHost};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{
    PluginStorage::{self, PluginStorageSession},
    PreferenceMerge::{self, PreferenceMergeSchema},
    PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore},
    RuntimeFileSyncStore::RuntimeFileSyncStore,
    SyncOperationStore::{SyncClock, SyncOperation, SyncOperationStore},
};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};

struct Fixture {
    root: PathBuf,
    host: Arc<NativeRuntimeStorageHost>,
    session: PluginStorageSession,
    directory: String,
    path: String,
}

impl Fixture {
    /// Creates an isolated real host with an explicit plugin data ownership scope.
    fn new(scope: &str) -> Self {
        let root = std::env::temp_dir().join(format!("operit-plugin-preferences-{}",uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let host = Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")));
        let directory = format!("runtime/plugin_data/{scope}/preferences_test");
        let path = format!("{directory}/roles.preferences.json");
        Self {root, host, session: PluginStorageSession::default(), directory, path}
    }

    /// Dispatches the exact serialized SDK wire protocol through the authenticated session.
    fn call(&self, request: Value) -> Result<Value, String> {
        self.session.request(self.host.clone(),self.host.clone(),serde_json::from_value::<StorageRequest>(request).unwrap(),"test")
            .map_err(|error| error.to_string()).map(|value| serde_json::to_value(value).unwrap())
    }

    /// Opens a native preference file and returns its engine-owned handle.
    fn open(&self) -> String {
        self.call(json!({"op":"open_preferences","path":self.path})).unwrap()["handle"].as_str().unwrap().to_owned()
    }

    /// Reads complete application data and the native schema version.
    fn data(&self, handle: &str) -> Value {
        self.call(json!({"op":"preferences_data","handle":handle})).unwrap()
    }

    /// Captures an immutable callback baseline without persisting anything.
    fn begin(&self, handle: &str) -> String {
        self.call(json!({"op":"preferences_begin_edit","handle":handle})).unwrap()["token"].as_str().unwrap().to_owned()
    }

    /// Submits callback intentions once, preserving native increment and field-patch semantics.
    fn finish(&self, handle: &str, token: &str, edits: Value, version: u32) -> Result<Value, String> {
        self.call(json!({"op":"preferences_finish_edit","handle":handle,"token":token,"edits":edits,"schemaVersion":version}))
    }

    /// Executes one ordinary edit with the file's current schema version.
    fn edit(&self, handle: &str, edits: Value) -> Value {
        let version=self.data(handle)["schemaVersion"].as_u64().unwrap() as u32;
        let token=self.begin(handle);
        self.finish(handle,&token,edits,version).unwrap()
    }

    /// Reads immutable native preference history without using the SQLite object outbox.
    fn history(&self) -> Vec<SyncOperation> {
        SyncOperationStore::new(self.host.clone(),"runtime/sync").preferenceHistory().unwrap()
    }

    /// Releases retained handles and removes only the fixture's verified temporary directory.
    fn dispose(mut self) {
        self.session=PluginStorageSession::default();
        assert_eq!(self.root.parent().unwrap(),std::env::temp_dir());
        std::fs::remove_dir_all(self.root).unwrap();
    }
}

/// Constructs one native preference assignment.
fn set(key: &str,value: Value) -> Value {json!({"op":"set","key":key,"value":value})}

/// Builds an explicit owner alias transaction independently of receiving or paging business data.
fn aliasOperation(path: &str, source: &str, target: &str) -> SyncOperation {
    let origin = format!("test-owner-{}", uuid::Uuid::new_v4());
    let key = format!("card_{source}");
    SyncOperation {
        opId: format!("{origin}:1"), originDeviceId: origin, sequence: 1,
        domain: "preferences".into(), entityType: "entry".into(),
        entityId: serde_json::to_string(&(path, &key)).unwrap(), operation: "alias".into(),
        semantics: operit_store::SyncOperationStore::SyncOperationSemantics::Transaction,
        payload: json!({"storagePath":path,"key":key,"value":format!("card_{target}"),"encrypted":false,"formatVersion":2,"schemaVersion":0}),
        createdAt: operit_host_api::TimeUtils::tryCurrentTimeMillis().unwrap(), schemaVersion: 1,
    }
}

/// Keeps typed JSON, stored null, absent keys, schema metadata and reopened snapshots distinct.
#[test]
fn preferences_are_native_json_without_object_tables_or_hidden_key_versions() {
    for scope in ["space","device"] {
        let fixture=Fixture::new(scope);
        let handle=fixture.open();
        let token=fixture.begin(&handle);
        let result=fixture.finish(&handle,&token,json!([
            set("card",json!({"id":"38ed9140-7b3f-4ce0-8aa1-0388c8d278f7","tags":["one"],"enabled":true})),
            set("nullable",Value::Null),set("number",json!(2))
        ]),2).unwrap();
        assert_eq!(result["schemaVersion"],2);
        assert_eq!(result["values"]["nullable"],Value::Null);
        assert!(result["values"].get("absent").is_none());
        let bytes=fixture.host.readBytes(&fixture.path).unwrap();
        let disk:Value=serde_json::from_slice(&bytes).unwrap();
        assert_eq!(disk["card"],result["values"]["card"]);
        assert_eq!(disk["__operit_preferences_schema_version"],2);
        assert!(disk.get("__operit_storage_key_versions").is_none());
        assert!(!fixture.host.exists(&format!("{}.operit-storage.json",fixture.path)).unwrap());
        assert!(!fixture.host.exists("runtime/extensions/device/storage/catalog.sqlite").unwrap());
        assert_eq!(fixture.history().is_empty(),scope=="device");
        assert!(fixture.history().iter().all(|operation|operation.domain=="preferences"));
        fixture.call(json!({"op":"close","handle":handle})).unwrap();
        let reopened=fixture.open();
        assert_eq!(fixture.data(&reopened),result);
        assert!(fixture.call(json!({"op":"get","handle":reopened,"collection":"keys","key":"card"})).is_err());
        assert!(fixture.call(json!({"op":"commit","handle":reopened,"mutations":[]})).is_err());
        fixture.dispose();
    }
}

/// Cancels failed callbacks and rejects an invalid later intention before publishing earlier edits.
#[test]
fn cancelled_and_rejected_edits_preserve_json_and_native_history() {
    let fixture=Fixture::new("space");let handle=fixture.open();
    fixture.edit(&handle,json!([set("counter",json!("invalid integer"))]));
    let history=fixture.history();let before=fixture.host.readBytes(&fixture.path).unwrap();
    let token=fixture.begin(&handle);
    fixture.call(json!({"op":"preferences_cancel_edit","handle":handle,"token":token})).unwrap();
    let token=fixture.begin(&handle);
    let error=fixture.finish(&handle,&token,json!([set("uncommitted",json!(true)),{"op":"increment_integer","key":"counter","delta":1}]),0).unwrap_err();
    assert!(error.contains("Integer increment requires an integer entry"));
    assert_eq!(fixture.host.readBytes(&fixture.path).unwrap(),before);
    assert_eq!(fixture.history(),history);
    assert!(fixture.finish(&handle,&token,json!([]),0).is_err());
    fixture.edit(&handle,json!([set("after",json!(true))]));
    fixture.dispose();
}

/// Detects cross-handle writes and value restoration without a persistent per-key version map.
#[test]
fn edit_conflicts_include_aba_and_native_rust_writes() {
    let fixture=Fixture::new("space");let first=fixture.open();let second=fixture.open();
    fixture.edit(&first,json!([set("value",json!(1))]));
    let token=fixture.begin(&first);
    fixture.edit(&second,json!([set("value",json!(2))]));
    fixture.edit(&second,json!([set("value",json!(1))]));
    let before=fixture.history();
    assert!(fixture.finish(&first,&token,json!([set("stale",json!(true))]),0).unwrap_err().contains("edit conflict"));
    assert_eq!(fixture.history(),before);
    let token=fixture.begin(&first);
    PreferencesDataStore::newWithStorage(fixture.host.clone(),fixture.path.clone()).edit(|preferences|preferences.set(&stringPreferencesKey("native"),true)).unwrap();
    assert!(fixture.finish(&first,&token,json!([set("stale",json!(true))]),0).is_err());
    assert!(fixture.data(&first)["values"].get("stale").is_none());
    fixture.dispose();
}

/// Records delta and field intentions so independent Space edits converge and replay only once.
#[test]
fn native_preference_delta_and_field_sync_converge_between_spaces() {
    let first=Fixture::new("space");let second=Fixture::new("space");
    let left=first.open();let right=second.open();
    first.edit(&left,json!([{"op":"increment_integer","key":"counter","delta":7},{"op":"set_object_field","key":"settings","field":"left","value":true}]));
    second.edit(&right,json!([{"op":"increment_integer","key":"counter","delta":11},{"op":"set_object_field","key":"settings","field":"right","value":false}]));
    let leftHistory=first.history();let rightHistory=second.history();
    let schema=PreferenceMergeSchema::default();
    for _ in 0..2 {
        PreferenceMerge::applyOperations(first.host.clone(),&schema,&leftHistory,&rightHistory,|_,_|true).unwrap();
        PreferenceMerge::applyOperations(second.host.clone(),&schema,&rightHistory,&leftHistory,|_,_|true).unwrap();
        for (fixture,handle) in [(&first,&left),(&second,&right)] {
            let data=fixture.data(handle);
            assert_eq!(data["values"]["counter"],18);
            assert_eq!(data["values"]["settings"],json!({"left":true,"right":false}));
        }
    }
    assert!(PluginStorage::operationsSince(first.host.clone(),&SyncClock::empty(),&[],100).unwrap().is_empty());
    first.dispose();second.dispose();
}

/// Excludes managed preference bytes while preserving ordinary plugin JSON files in file synchronization.
#[test]
fn preference_files_use_the_native_journal_instead_of_whole_file_lww() {
    let fixture=Fixture::new("space");let handle=fixture.open();
    let files=RuntimeFileSyncStore::new(fixture.host.clone(),"runtime/sync");
    files.trackChanges(&[fixture.directory.clone()],|| {
        fixture.edit(&handle,json!([set("card",json!({"name":"Role"}))]));
        fixture.host.writeBytes(&format!("{}/ordinary.json",fixture.directory),b"{}").map_err(|error|error.to_string())
    }).unwrap();
    let operations=SyncOperationStore::new(fixture.host.clone(),"runtime/sync").operationsSince(&SyncClock::empty(),&[],100).unwrap();
    assert!(operations.iter().any(|operation|operation.domain=="preferences"));
    assert!(operations.iter().any(|operation|operation.entityId.ends_with("/ordinary.json")));
    assert!(!operations.iter().any(|operation|operation.entityId==fixture.path));
    assert!(files.writeBytes(&fixture.path,b"{}").is_err());
    assert!(!PluginStorage::ownsFile(fixture.host.clone(),"runtime/config/preferences/model_configs.preferences.json").unwrap());
    fixture.dispose();
}

/// Constructs a native complete-record catalog plus an array reference to its identities.
fn descriptor(path: &str) -> Value {
    json!({"version":1,"catalog":{"keyPrefix":"card_","keySuffix":"","identityField":"id","nameField":"name","ignoredFields":["updatedAt"],"retiredKeys":[]},
        "references":[{"key":{"kind":"exact","key":"selection"},"path":[{"kind":"array_values"}],"target":path}],"projection":null})
}

/// Executes plugin-declared content/name merging and array references on a peer without loading the plugin.
#[test]
fn declarations_preserve_stable_ids_and_local_storage_version_boundaries() {
    let first=Fixture::new("space");let second=Fixture::new("space");let receiver=Fixture::new("space");
    let open=json!({"op":"open_preferences","path":first.path,"merge":descriptor(&first.path)});
    let left=first.call(open.clone()).unwrap()["handle"].as_str().unwrap().to_owned();
    let right=second.call(open).unwrap()["handle"].as_str().unwrap().to_owned();
    let small="00000000-0000-4000-8000-000000000001";let large="ffffffff-ffff-4fff-8fff-ffffffffffff";
    let token=first.begin(&left);
    first.finish(&left,&token,json!([set(&format!("card_{small}"),json!({"id":small,"name":"Same","text":"identical","updatedAt":1})),set("selection",json!([small]))]),2).unwrap();
    let token=second.begin(&right);
    second.finish(&right,&token,json!([set(&format!("card_{large}"),json!({"id":large,"name":"Same","text":"identical","updatedAt":2})),set("selection",json!([large]))]),2).unwrap();
    PreferencesDataStore::newWithStorage(receiver.host.clone(), &receiver.path)
        .withSchema(2, |_, _| Ok(())).data().unwrap();
    let leftOriginal=first.history();let rightOriginal=second.history();
    let leftHistory=leftOriginal.iter().filter(|op| op.payload["schemaVersion"] == 2).cloned().collect::<Vec<_>>();
    let rightHistory=rightOriginal.iter().filter(|op| op.payload["schemaVersion"] == 2).cloned().collect::<Vec<_>>();
    let schema=PreferenceMergeSchema::default();
    let refs=PreferenceMerge::applyOperations(receiver.host.clone(),&schema,&leftHistory,&rightHistory,|_,_|true).unwrap();
    assert!(refs.is_empty());
    let handle=receiver.open();let snapshot=receiver.data(&handle);
    assert_eq!(snapshot["schemaVersion"],2);
    assert!(snapshot["values"].get(format!("card_{small}")).is_some());
    assert!(snapshot["values"].get(format!("card_{large}")).is_some());
    let order = operit_store::SyncOperationStore::SyncOperationOrder::fromOperation;
    let selected = if order(leftHistory.last().unwrap()) < order(rightHistory.last().unwrap()) { large } else { small };
    assert_eq!(snapshot["values"]["selection"],json!([selected]));
    assert!(snapshot["values"].get("__operit_preferences_merge").is_none());
    assert_eq!(first.history(),leftOriginal);assert_eq!(second.history(),rightOriginal);
    let mut changed=descriptor(&receiver.path);changed["catalog"]["nameField"]=json!("title");
    assert!(receiver.call(json!({"op":"open_preferences","path":receiver.path,"merge":changed})).is_err());
    first.dispose();second.dispose();receiver.dispose();
}

/// Retires an already materialized source identity when an explicit alias arrives later.
#[test]
fn aliases_retire_existing_catalog_sources_on_every_peer() {
    let source = Fixture::new("space");
    let receiver = Fixture::new("space");
    let handle = source.call(json!({
        "op":"open_preferences", "path":source.path, "merge":descriptor(&source.path)
    })).unwrap()["handle"].as_str().unwrap().to_owned();
    source.edit(&handle, json!([
        set("card_old", json!({"id":"old", "name":"Role"})),
        set("card_current", json!({"id":"current", "name":"Role"})),
        set("selection", json!(["old"])),
        set("unrelated", json!(true))
    ]));
    let history = source.history();
    let schema = PreferenceMergeSchema::default();
    PreferenceMerge::applyOperations(receiver.host.clone(), &schema, &[], &history, |_, _| true).unwrap();
    let received = receiver.open();
    assert!(receiver.data(&received)["values"].get("card_old").is_some());
    let alias = aliasOperation(&source.path, "old", "current");
    for _ in 0..2 {
        PreferenceMerge::applyOperations(receiver.host.clone(), &schema, &history, &[alias.clone()], |_, _| true).unwrap();
        let values = receiver.data(&received)["values"].clone();
        assert!(values.get("card_old").is_none(), "Retired identity survived alias replay: {values}");
        assert_eq!(values["card_current"]["id"], "current");
        assert_eq!(values["selection"], json!(["current"]));
        assert_eq!(values["unrelated"], true);
    }
    let mut expected = history;
    expected.push(alias);
    let archived = receiver.history();
    assert_eq!(archived.len(), expected.len());
    assert!(expected.iter().all(|operation| archived.contains(operation)));
    source.dispose();
    receiver.dispose();
}

/// Rejects old object operations, wrong file kinds and malicious metadata edits explicitly.
#[test]
fn datastore_contract_rejects_database_apis_and_engine_metadata_mutations() {
    let fixture=Fixture::new("space");
    assert!(fixture.call(json!({"op":"open_preferences","path":format!("{}/roles.sqlite",fixture.directory)})).is_err());
    assert!(serde_json::from_value::<StorageRequest>(json!({"op":"open","kind":"data_store","path":fixture.path})).is_err());
    let handle=fixture.open();let token=fixture.begin(&handle);
    assert!(fixture.finish(&handle,&token,json!([set("__operit_preferences_schema_version",json!(8))]),0).is_err());
    let token=fixture.begin(&handle);
    assert!(fixture.finish(&handle,&token,json!([set("__operit_preferences_merge",json!({}))]),0).is_err());
    assert!(fixture.history().is_empty());
    assert!(!fixture.host.exists(&fixture.path).unwrap());
    fixture.dispose();
}

/// Canonicalizes dependent record arrays before deduplicating their owning catalog or assigning name suffixes.
#[test]
fn dependent_json_catalogs_merge_after_nested_reference_rewrites() {
    let first=Fixture::new("space");let second=Fixture::new("space");let receiver=Fixture::new("space");
    let tagsPath=format!("{}/tags.preferences.json",first.directory);
    let mut tagDescriptor=descriptor(&tagsPath);tagDescriptor["references"]=json!([]);
    let mut cardDescriptor=descriptor(&first.path);
    cardDescriptor["references"]=json!([
        {"key":{"kind":"records","prefix":"card_","suffix":""},"path":[{"kind":"field","name":"tags"},{"kind":"array_values"}],"target":tagsPath},
        {"key":{"kind":"exact","key":"selection"},"path":[],"target":first.path}
    ]);
    let small="00000000-0000-4000-8000-000000000001";let large="ffffffff-ffff-4fff-8fff-ffffffffffff";
    let smallTag="10000000-0000-4000-8000-000000000001";let largeTag="efffffff-ffff-4fff-8fff-ffffffffffff";
    for (fixture,card,tag) in [(&first,small,smallTag),(&second,large,largeTag)] {
        let tagHandle=fixture.call(json!({"op":"open_preferences","path":tagsPath,"merge":tagDescriptor})).unwrap()["handle"].as_str().unwrap().to_owned();
        fixture.edit(&tagHandle,json!([set(&format!("card_{tag}"),json!({"id":tag,"name":"Tag","content":"identical"}))]));
        let cardHandle=fixture.call(json!({"op":"open_preferences","path":first.path,"merge":cardDescriptor})).unwrap()["handle"].as_str().unwrap().to_owned();
        fixture.edit(&cardHandle,json!([set(&format!("card_{card}"),json!({"id":card,"name":"Role","tags":[tag]})),set("selection",json!(card))]));
    }
    let schema=PreferenceMergeSchema::default();
    let left=first.history();let mut right=second.history();
    right.push(aliasOperation(&tagsPath, largeTag, smallTag));
    right.push(aliasOperation(&first.path, large, small));
    let refs=PreferenceMerge::applyOperations(receiver.host.clone(),&schema,&left,&right,|_,_|true).unwrap();
    assert_eq!(refs.get(&format!("{}\u{001f}{large}",first.path)),Some(&small.to_owned()));
    assert_eq!(refs.get(&format!("{tagsPath}\u{001f}{largeTag}")),Some(&smallTag.to_owned()));
    let handle=receiver.open();let snapshot=receiver.data(&handle);
    assert_eq!(snapshot["values"][format!("card_{small}")],json!({"id":small,"name":"Role","tags":[smallTag]}));
    assert!(snapshot["values"].get(format!("card_{large}")).is_none());
    assert_eq!(snapshot["values"]["selection"],small);
    first.dispose();second.dispose();receiver.dispose();
}

/// Gives colliding names deterministic suffixes while retaining distinct complete JSON content.
#[test]
fn json_catalog_name_collisions_preserve_uuid_order_and_history() {
    let first=Fixture::new("space");let second=Fixture::new("space");
    let small="00000000-0000-4000-8000-000000000001";let large="ffffffff-ffff-4fff-8fff-ffffffffffff";
    for (fixture,id,content) in [(&first,small,"first"),(&second,large,"second")] {
        let handle=fixture.call(json!({"op":"open_preferences","path":fixture.path,"merge":descriptor(&fixture.path)})).unwrap()["handle"].as_str().unwrap().to_owned();
        fixture.edit(&handle,json!([set(&format!("card_{id}"),json!({"id":id,"name":"Same","content":content}))]));
    }
    let schema=PreferenceMergeSchema::default();
    let left=first.history();let right=second.history();
    for (fixture,history,incoming) in [(&first,&left,&right),(&second,&right,&left)] {
        let refs=PreferenceMerge::applyOperations(fixture.host.clone(),&schema,history,incoming,|_,_|true).unwrap();
        assert!(refs.is_empty());
        let handle=fixture.open();let snapshot=fixture.data(&handle);
        assert_eq!(snapshot["values"][format!("card_{small}")]["name"],"Same");
        assert_eq!(snapshot["values"][format!("card_{large}")]["name"],"Same（1）");
        let archived = fixture.history();
        assert_eq!(archived.len(), left.len() + right.len());
        assert!(archived.iter().all(|operation| left.iter().chain(&right).any(|original| original == operation)));
        assert!(archived.iter().all(|operation| operation.operation != "alias"));
    }
    first.dispose();second.dispose();
}

/// Counts native observer notifications without requiring timers or a Flutter runtime.
#[derive(Default)]
struct ChangeWake(std::sync::atomic::AtomicUsize);
impl std::task::Wake for ChangeWake {
    /// Records each wake of an outstanding preference snapshot read.
    fn wake(self: Arc<Self>) {self.0.fetch_add(1,std::sync::atomic::Ordering::SeqCst);}
    /// Records a borrowed wake without consuming the observer.
    fn wake_by_ref(self: &Arc<Self>) {self.0.fetch_add(1,std::sync::atomic::Ordering::SeqCst);}
}

/// Publishes snapshots after native Rust edits and remote replay, then releases outstanding waits on close.
#[test]
fn complete_preference_observation_sees_native_changes_and_closes_cleanly() {
    use std::{future::Future,task::{Context,Poll,Waker}};
    use operit_host_api::PluginStorage::StorageRecordWatchState;
    for scope in ["space","device"] {
        let fixture=Fixture::new(scope);let handle=fixture.open();
        let wake=Arc::new(ChangeWake::default());let waker=Waker::from(wake.clone());let mut context=Context::from_waker(&waker);
        let mut initial=Box::pin(fixture.session.waitForRecord(handle.clone(),"__preferences".into(),"snapshot".into(),None));
        let Poll::Ready(Ok(Some(initialValue)))=initial.as_mut().poll(&mut context) else {panic!("Expected initial preference snapshot")};drop(initial);
        let mut pending=Box::pin(fixture.session.waitForRecord(handle.clone(),"__preferences".into(),"snapshot".into(),Some(StorageRecordWatchState{version:Some(initialValue.version)})));
        assert!(pending.as_mut().poll(&mut context).is_pending());
        match scope {
            "space" => PreferencesDataStore::newWithStorage(fixture.host.clone(),fixture.path.clone()).edit(|preferences|preferences.set(&stringPreferencesKey("native"),true)).unwrap(),
            "device" => operit_store::PreferencesDataStore::CoreNodeStateStore::newWithStorage(fixture.host.clone(),fixture.path.clone()).edit(|preferences|preferences.set(&stringPreferencesKey("native"),true)).unwrap(),
            _ => unreachable!(),
        }
        assert!(wake.0.load(std::sync::atomic::Ordering::SeqCst)>0);
        let Poll::Ready(Ok(Some(changed)))=pending.as_mut().poll(&mut context) else {panic!("Expected native preference update")};drop(pending);
        assert_eq!(changed.value["values"]["native"],true);
        let mut pending=Box::pin(fixture.session.waitForRecord(handle.clone(),"__preferences".into(),"snapshot".into(),Some(StorageRecordWatchState{version:Some(changed.version)})));
        assert!(pending.as_mut().poll(&mut context).is_pending());
        fixture.call(json!({"op":"close","handle":handle})).unwrap();
        assert!(matches!(pending.as_mut().poll(&mut context),Poll::Ready(Err(_))));drop(pending);
        fixture.dispose();
    }
}

/// Rejects device-path injection before changing local user data or creating preference files.
#[test]
fn remote_preferences_cannot_write_node_local_plugin_data() {
    let source=Fixture::new("space");let receiver=Fixture::new("device");let handle=source.open();
    source.edit(&handle,json!([set("one",json!(true))]));
    let mut operation=source.history().remove(0);
    operation.payload["storagePath"]=json!(receiver.path);
    operation.entityId=serde_json::to_string(&(&receiver.path,"one")).unwrap();
    let schema=PreferenceMergeSchema::default();
    assert!(PreferenceMerge::applyOperations(receiver.host.clone(),&schema,&[],&[operation],|_,_|true).unwrap_err().contains("Space-owned"));
    assert!(!receiver.host.exists(&receiver.path).unwrap());
    source.dispose();receiver.dispose();
}

/// Uses the existing alias executor for discriminated selections and explicitly prefixed chat references.
#[test]
fn declared_variants_and_prefixed_references_use_native_aliases() {
    let first = Fixture::new("space");
    let second = Fixture::new("space");
    let receiver = Fixture::new("space");
    let mut merge = descriptor(&first.path);
    merge["references"] = json!([
        {"key":{"kind":"exact","key":"active"},"path":[{"kind":"variant","name":"CharacterCard"},{"kind":"field","name":"id"}],"target":first.path},
        {"key":{"kind":"exact","key":"model"},"path":[{"kind":"variant","name":"CharacterCard"},{"kind":"field","name":"id"}],"target":first.path},
        {"key":{"kind":"records","prefix":"binding_","suffix":""},"path":[{"kind":"field","name":"selection"},{"kind":"identity","prefix":"card:","suffix":""}],"target":first.path}
    ]);
    let small = "00000000-0000-4000-8000-000000000001";
    let large = "ffffffff-ffff-4fff-8fff-ffffffffffff";
    for (fixture, id) in [(&first, small), (&second, large)] {
        let handle = fixture.call(json!({"op":"open_preferences","path":first.path,"merge":merge})).unwrap()["handle"].as_str().unwrap().to_owned();
        fixture.edit(&handle, json!([
            set(&format!("card_{id}"), json!({"id":id,"name":"Role","text":"same"})),
            set("active", json!({"CharacterCard":{"id":id}})),
            set("model", json!({"Model":{"id":"chat"}})),
            set("binding_chat", json!({"version":1,"selection":format!("card:{id}")})),
            set("binding_group", json!({"version":1,"selection":"group:external"}))
        ]));
    }
    let schema = PreferenceMergeSchema::default();
    let mut right = second.history();
    right.push(aliasOperation(&first.path, large, small));
    PreferenceMerge::applyOperations(receiver.host.clone(), &schema, &first.history(), &right, |_, _| true).unwrap();
    let handle = receiver.open();
    let snapshot = receiver.data(&handle);
    assert_eq!(snapshot["values"]["active"], json!({"CharacterCard":{"id":small}}));
    assert_eq!(snapshot["values"]["model"], json!({"Model":{"id":"chat"}}));
    assert_eq!(snapshot["values"]["binding_chat"]["selection"], format!("card:{small}"));
    assert_eq!(snapshot["values"]["binding_group"]["selection"], "group:external");
    first.dispose(); second.dispose(); receiver.dispose();
}


/// Merges Rust-owned catalogs and plugin references through the identical declaration model.
#[test]
fn rust_owner_and_plugin_declarations_share_catalog_aliases() {
    use operit_store::PreferenceMerge::PreferenceMergeStore;
    let first = Fixture::new("space");
    let second = Fixture::new("space");
    let receiver = Fixture::new("space");
    let native_path = "runtime/config/preferences/theme_test.preferences.json";
    let small = "00000000-0000-4000-8000-000000000001";
    let large = "ffffffff-ffff-4fff-8fff-ffffffffffff";
    let mut native = descriptor(native_path);
    native["references"] = json!([]);
    let schema = PreferenceMergeSchema { stores: vec![PreferenceMergeStore {
        path: native_path.to_string(), encrypted: false,
            open: |storage, path| PreferencesDataStore::newWithStorage(storage, path),
            decode: None,
        merge: serde_json::from_value(native).unwrap(),
    }] };
    let mut plugin = descriptor(&first.path);
    plugin["catalog"] = Value::Null;
    plugin["references"][0]["target"] = json!(native_path);
    for (fixture, id) in [(&first, small), (&second, large)] {
        PreferencesDataStore::newWithStorage(fixture.host.clone(), native_path.to_string()).edit(|preferences| {
            preferences.set(&stringPreferencesKey(&format!("card_{id}")), json!({"id":id,"name":"Theme","text":"same"}));
        }).unwrap();
        let handle = fixture.call(json!({"op":"open_preferences","path":first.path,"merge":plugin})).unwrap()["handle"].as_str().unwrap().to_owned();
        fixture.edit(&handle, json!([set("selection", json!([id]))]));
    }
    let mut right = second.history();
    right.push(aliasOperation(native_path, large, small));
    let aliases = PreferenceMerge::applyOperations(receiver.host.clone(), &schema, &first.history(), &right, |_, _| true).unwrap();
    assert_eq!(aliases.get(&format!("{native_path}\u{001f}{large}")), Some(&small.to_string()));
    let handle = receiver.open();
    assert_eq!(receiver.data(&handle)["values"]["selection"], json!([small]));
    let native = PreferencesDataStore::newWithStorage(receiver.host.clone(), native_path.to_string()).data().unwrap();
    assert!(native.contains(&stringPreferencesKey(&format!("card_{small}"))));
    assert!(!native.contains(&stringPreferencesKey(&format!("card_{large}"))));
    first.dispose(); second.dispose(); receiver.dispose();
}

/// Rejects malformed shared paths for both Rust declarations and plugin metadata before any file writes.
#[test]
fn shared_declaration_validation_precedes_storage_publication() {
    use operit_store::PreferenceMerge::PreferenceMergeStore;
    let fixture = Fixture::new("space");
    for path in [
        json!([{"kind":"variant","name":""}]),
        json!([{"kind":"identity","prefix":"card:","suffix":""},{"kind":"array_values"}]),
    ] {
        let mut invalid = descriptor(&fixture.path);
        invalid["references"][0]["path"] = path;
        let schema = PreferenceMergeSchema { stores: vec![PreferenceMergeStore {
            path: fixture.path.clone(), encrypted: false,
            open: |storage, path| PreferencesDataStore::newWithStorage(storage, path),
            decode: None,
            merge: serde_json::from_value(invalid.clone()).unwrap(),
        }] };
        assert!(PreferenceMerge::applyOperations(fixture.host.clone(), &schema, &[], &[], |_, _| true).is_err());
        assert!(fixture.call(json!({"op":"open_preferences","path":fixture.path,"merge":invalid})).is_err());
        assert!(!fixture.host.exists(&fixture.path).unwrap());
        assert!(fixture.history().is_empty());
    }
    fixture.dispose();
}
