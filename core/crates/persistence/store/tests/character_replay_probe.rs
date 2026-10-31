use std::sync::Arc;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::PluginStorage::PluginStorageSession;
use serde_json::{json,Value};
#[test]
fn reopen_existing_character_metadata_through_plugin_api() {
 let root=std::path::PathBuf::from(std::fs::read_to_string("/tmp/operit-character-open-root.txt").unwrap());
 let path="runtime/plugin_data/space/com.operit.character_cards/meta.preferences.json";
 let read=|path:&str| std::fs::read(root.join(path)).unwrap();
 let initial:Value=serde_json::from_slice(&read(path)).unwrap();
 assert!(initial.get("state").is_none());
 let expected:Value=serde_json::from_str(&std::fs::read_to_string("/tmp/operit-meta-init-69.json").unwrap()).unwrap();
 let expected=&expected["payload"]["initialization"]["data"]["values"]["state"];
 let cards="runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json";
 let chats="runtime/plugin_data/space/com.operit.character_cards/chats.preferences.json";
 let before=(read(cards),read(chats));
 let host=Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"),root.join("workspaces")));
 let session=PluginStorageSession::default();
 let call=|v:Value| serde_json::to_value(session.request(host.clone(),host.clone(),serde_json::from_value(v).unwrap(),"test").unwrap()).unwrap();
 let handle=call(json!({"op":"open_preferences","path":path,"merge":initial["__operit_preferences_merge"]}))["handle"].clone();
 for _ in 0..2 {
  call(json!({"op":"preferences_schema","handle":handle,"schema":initial["__operit_preferences_schema"]}));
  let state=call(json!({"op":"preferences_data","handle":handle}));
  assert_eq!(&state["values"]["state"],expected);
  let id=state["values"]["state"]["active"]["CharacterCard"]["id"].as_str().unwrap();
  let values:Value=serde_json::from_slice(&read(cards)).unwrap();
  assert!(values.get(format!("record_{id}")).is_some());
  assert_eq!((read(cards),read(chats)),before);
 }
 println!("metadata recovered, settings match journal, active card resolves, cards/chats unchanged, repeated open is idempotent");
 call(json!({"op":"close","handle":handle}));
}
