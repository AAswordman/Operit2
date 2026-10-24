import "../../plugins/types/index";
import type { Storage, StorageRequest, StorageSqlValue } from "../../plugins/types/storage";

/** Checks the public author-facing facade and generated wire discriminators together. */
async function exerciseStorage(path: string): Promise<void> {
  const localDirectory: string = ToolPkg.getLocalDataDir();
  await Tools.Storage.dataStore.open({path:localDirectory + "/cache.sqlite"});
  // @ts-expect-error Database synchronization derives from directory ownership.
  void Tools.Storage.sqlite.open({path, sync: true});
  // @ts-expect-error The structured open request has no synchronization parameter.
  const removedOption: StorageRequest = {op:"open", path, kind:"sqlite", sync:false};
  void removedOption;
  const sql = await Tools.Storage.sqlite.open({path});
  await sql.defineTable({name:"cards",primaryKey:"id",columns:[{name:"id",affinity:"text",nullable:false}]});
  await sql.transaction([{sql:"INSERT INTO cards VALUES(?)",params:["one"]}]);
  const rows=await sql.query("SELECT id FROM cards WHERE id=?",["one"]);
  const value: Storage.SqlValue=rows[0].id;
  // @ts-expect-error The transaction journal is internal to Rust synchronization.
  void sql.changes();
  await sql.close();
  const objects=await Tools.Storage.objects.open({path:path+".objects"});
  const cards=objects.collection<{title:string; nested:{count:number}}>("cards");
  await cards.put("one",{title:"One",nested:{count:1}},{expectedVersion:null});
  for await (const entry of cards.watch("one")) {
    if (entry !== null) { const title: string = entry.value.title; void title; }
    break;
  }
  // @ts-expect-error Record watches select a key, never a journal cursor and page limit.
  void cards.watch("0", 100);
  // @ts-expect-error Whole-database transaction watches are not an author-facing API.
  void objects.watch("0", 100);
  const record=await cards.edit("one");record.value.nested.count++;await record.flush();
  const datastore=await Tools.Storage.dataStore.open({path:path+".keys"});
  await datastore.commit({set:{theme:"dark",nullable:null},remove:["old"],expectedVersions:{theme:null}});
  const scalar:StorageSqlValue={kind:"integer",value:"9223372036854775807"};
  const request:StorageRequest={op:"query",handle:"internal",statement:{sql:"SELECT ?",params:[scalar],expectedAffectedRows:null}};
  await Tools.Storage.request(request);
  void value;
}
void exerciseStorage;
