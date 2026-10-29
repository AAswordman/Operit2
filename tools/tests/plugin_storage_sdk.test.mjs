import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import test from "node:test";
import { installScopedHostRuntime, installGeneratedToolsRuntime, domainToolResult } from "./fixtures/scoped_host_runtime.mjs";

const source=readFileSync(new URL("../../core/crates/plugin/sdk/src/js_sdk/storage_runtime.js",import.meta.url),"utf8");

/** Preserves exact mutation-count conditions across the real SDK and structured Host boundary. */
test("SQLite transaction forwards affected-row preconditions and rejects invalid counts", async () => {
  const f = fixture();
  const opening = f.run("Tools.Storage.sqlite.open({path:'runtime/owned/sqlite.db'}).then(value=>{globalThis.db=value;})");
  f.reply({handle:"sql",path:"runtime/owned/sqlite.db"});
  await opening;
  const transaction = f.run("db.transaction([{sql:'UPDATE cards SET name=? WHERE id=?',params:['new','one'],expectedAffectedRows:1},{sql:'DELETE FROM children WHERE parent=?',params:['one'],expectedAffectedRows:0}])");
  assert.deepEqual(f.request().statements.map(statement => statement.expectedAffectedRows), [1, 0]);
  f.reply([{affectedRows:1,lastInsertId:null},{affectedRows:0,lastInsertId:null}]);
  await transaction;
  for (const count of [-1, 0.5, NaN, 9007199254740992, "1"]) {
    f.sandbox.invalidCount = count;
    assert.throws(() => f.run("db.transaction([{sql:'DELETE FROM cards',expectedAffectedRows:invalidCount}])"), /nonnegative safe integer/);
    assert.equal(f.pending.length, 0);
  }
});

/** Runs production SDK and host Promise registry, with only storage operations supplied by the test. */
function fixture() {
  const pending=[];
  const sandbox=vm.createContext({pending});
  const registry=installScopedHostRuntime(sandbox);
  sandbox.__operitNativeCallToolStructured=registry.binding((requestId,type,name,parameters)=>{
    assert.equal(type,"default"); assert.equal(name,"storage_request");
    pending.push({requestId,request:parameters.request});
  });
  installGeneratedToolsRuntime(sandbox);
  return { sandbox,pending,
    run(code) {return vm.runInContext(code,sandbox);},
    reply(value,isError=false) {const entry=pending.shift(); assert.ok(entry); registry.settle(entry.requestId,isError ? value : {success:true,data:domainToolResult("storage_request",value,{request:entry.request})},isError);return entry.request;},
    request() {return structuredClone(pending[0].request);},
  };
}

/** Checks explicit path selection, exact scalar mappings and SQL result identity. */
test("SQLite facade preserves int64 and binary values without a JSON text transport",async()=>{
  const f=fixture(); const opened=f.run("Tools.Storage.sqlite.open({path:'runtime/owned/sqlite.db'}).then(value=>{globalThis.db=value;})");
  assert.deepEqual(f.request(),{op:"open",path:"runtime/owned/sqlite.db",kind:"sqlite"}); f.reply({handle:"sql",path:"runtime/owned/sqlite.db"});await opened;
  const result=f.run("db.query('SELECT ?,?,?', [9223372036854775807n,new Uint8Array([0,255]),null])");
  assert.deepEqual(f.request().statement.params,[{kind:"integer",value:"9223372036854775807"},{kind:"blob",value:[0,255]},{kind:"null"}]);
  f.reply([{columns:["integer","blob","missing"],values:[{kind:"integer",value:"9223372036854775807"},{kind:"blob",value:[0,255]},{kind:"null"}]}]);
  const rows=await result; assert.equal(rows[0].integer,9223372036854775807n);assert.deepEqual(Array.from(rows[0].blob),[0,255]);assert.equal(rows[0].missing,null);
  await assert.rejects(f.run("db.query('SELECT ?', [9007199254740992])"),/Use bigint/);assert.equal(f.pending.length,0);
  const closed=f.run("db.close()");f.reply(null);await closed;await assert.rejects(f.run("db.query('SELECT 1')"),/closed/);assert.equal(f.pending.length,0);
});

/** Rejects removed synchronization options before invoking the structured Host bridge. */
test("storage open accepts only paths and rejects the removed sync parameter",async()=>{
  const f=fixture();
  for(const kind of ["sqlite","objects","dataStore"]){
    for(const sync of [true,false]){
      await assert.rejects(f.run(`Tools.Storage.${kind}.open({path:'runtime/owned/db.sqlite',sync:${sync}})`),/Unknown storage open option: sync/);
      assert.equal(f.pending.length,0);
    }
  }
});

/** Transports one native preference callback without exposing the retired collection/CAS facade. */
test("DataStore edits native typed preferences through the structured Host bridge", async () => {
  const f = fixture();
  const opening = f.run("Tools.Storage.dataStore.open({path:'runtime/owned/settings.preferences.json'}).then(value=>{globalThis.db=value;})");
  f.reply({handle:"preferences",path:"runtime/owned/settings.preferences.json"}); await opening;
  assert.equal(f.run("typeof db.commit"), "undefined");
  assert.equal(f.run("typeof db.get"), "undefined");
  const editing = f.run("db.edit(preferences=>{preferences.set('nullable',null);preferences.set('count',3);preferences.remove('old')})");
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(f.request(), {op:"preferences_begin_edit",handle:"preferences"});
  f.reply({token:"callback",snapshot:{values:{old:"retired"},schemaVersion:0}});
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(f.request(), {op:"preferences_finish_edit",handle:"preferences",token:"callback",schemaVersion:0,edits:[{op:"set",key:"nullable",value:null},{op:"set",key:"count",value:3},{op:"remove",key:"old"}]});
  f.reply({values:{nullable:null,count:3},schemaVersion:0});
  const snapshot = await editing;
  assert.equal(snapshot.get("nullable"), null); assert.equal(snapshot.get("count"), 3); assert.equal(snapshot.get("old"), undefined);
});

/** Exercises nested proxy mutations and retries the same CAS version only after an explicit failed save. */
test("record proxies persist only on flush and retain pending edits after a version conflict",async()=>{
  const f=fixture();const opening=f.run("Tools.Storage.objects.open({path:'runtime/owned/objects.db'}).then(value=>{globalThis.db=value;})");f.reply({handle:"objects",path:"runtime/owned/objects.db"});await opening;
  const editing=f.run("db.collection('cards').edit('one').then(value=>{globalThis.editor=value;})");
  const entry=f.run("({value:{nested:{title:'old'},items:[1]},version:'v1'})");f.reply(entry);await editing;
  f.run("editor.value.nested.title='new';editor.value.items.push(2)");assert.equal(f.pending.length,0);
  const first=f.run("editor.flush()");assert.equal(f.request().mutations[0].expectedVersion,"v1");assert.deepEqual(f.request().mutations[0].value,{nested:{title:"new"},items:[1,2]});
  assert.throws(()=>f.run("editor.value.nested.title='during-save'"),/being committed/);f.reply("Storage version conflict",true);await assert.rejects(first,/version conflict/);
  const second=f.run("editor.flush()");assert.equal(f.request().mutations[0].expectedVersion,"v1");f.reply({version:"v2"});await second;
  const unchanged=await f.run("editor.flush()");assert.equal(unchanged.version,"v2");assert.equal(f.pending.length,0);
  f.run("delete editor.value.nested.title");const third=f.run("editor.flush()");assert.equal(f.request().mutations[0].expectedVersion,"v2");f.reply({version:"v3"});await third;
});

/** Restores the originating execution before the awaiting caller resumes, including errors. */
test("storage promises restore their owner without publishing callback functions",async()=>{
  const f=fixture();const promise=f.run("Tools.Storage.request({op:'close',handle:'x'}).catch(error=>({message:error.message,owner:__operitCurrentCallId}))");
  const globals=Object.keys(f.sandbox);assert.equal(typeof f.pending[0].requestId,"number");f.run("__operitCurrentCallId='unrelated'");f.reply("closed on host",true);const value=await promise;
  assert.equal(value.owner,"owner");assert.equal(value.message,"closed on host");assert.deepEqual(Object.keys(f.sandbox),globals);assert.equal(f.sandbox.refs,0);
});

/** Forwards the declared merge contract unchanged through the real SDK facade. */
test("storage open forwards declarative merge metadata without dropping its fields", async () => {
  const f = fixture();
  const opening = f.run("Tools.Storage.sqlite.open({path:'runtime/owned/sqlite.db',merge:{version:2,maxEntities:4,maxReferences:32,entities:[]}})");
  assert.deepEqual(f.request().merge, {version:2,maxEntities:4,maxReferences:32,entities:[]});
  f.reply({handle:"sql",path:"runtime/owned/sqlite.db"});
  const database = await opening;
  assert.equal(database.path, "runtime/owned/sqlite.db");
});

/** Observes one exact key through the real facade and releases only its owned handle. */
test("record watch sends exact collection and key, emits initial absence and cancels pending reads", async () => {
  const f=fixture();const registry=f.sandbox.__operitHostPromiseRegistry;const waits=[];
  f.sandbox.__operitNativeWaitPluginStorageRecord=registry.binding((id,handle,collection,key,after)=>waits.push({id,handle,collection,key,after}));
  const opening=f.run("Tools.Storage.objects.open({path:'runtime/owned/live.sqlite'}).then(value=>{globalThis.db=value})");
  f.reply({handle:"parent",path:"runtime/owned/live.sqlite"});await opening;
  assert.equal(f.run("typeof db.watch"),"undefined");assert.equal(f.run("typeof db.changes"),"function");
  f.run("globalThis.stream=db.collection('goals').watch('chat-a')");const first=f.run("stream.next()");
  f.reply({handle:"watcher",path:"runtime/owned/live.sqlite"});await new Promise(resolve=>setImmediate(resolve));
  assert.deepEqual({...waits[0],id:null},{id:null,handle:"watcher",collection:"goals",key:"chat-a",after:null});
  registry.settle(waits.shift().id,null,false);assert.equal((await first).value,null);
  const second=f.run("stream.next()");await new Promise(resolve=>setImmediate(resolve));
  assert.deepEqual(structuredClone(waits[0].after),{version:null});
  const entry={value:{objective:"target"},version:"remote:9007199254740993"};
  registry.settle(waits.shift().id,entry,false);assert.equal((await second).value.version,entry.version);
  const pending=f.run("stream.next()");await new Promise(resolve=>setImmediate(resolve));
  assert.equal(waits[0].after.version,entry.version);await assert.rejects(f.run("stream.next()"),/pending read/);
  const closing=f.run("stream.return()");await new Promise(resolve=>setImmediate(resolve));
  assert.deepEqual(f.request(),{op:"close",handle:"watcher"});f.reply(null);
  registry.settle(waits.shift().id,"Storage handle is not active",true);
  assert.equal((await closing).done,true);assert.equal((await pending).done,true);
  const get=f.run("db.collection('goals').get('chat-a')");assert.equal(f.request().handle,"parent");f.reply(null);await get;
  assert.equal(f.sandbox.refs,0);
});

/** Avoids opening a retained handle for an unused collector. */
test("record watch return before next allocates no resources", async()=>{
  const f=fixture();const opening=f.run("Tools.Storage.objects.open({path:'runtime/owned/live.sqlite'}).then(value=>{globalThis.db=value})");
  f.reply({handle:"parent",path:"runtime/owned/live.sqlite"});await opening;
  f.run("globalThis.stream=db.collection('goals').watch('chat-a')");await f.run("stream.return()");
  assert.equal((await f.run("stream.next()")).done,true);assert.equal(f.pending.length,0);
});

/** Observes native preference snapshots and cancels the collector's retained handle. */
test("DataStore flow abort closes its exact retained handle",async()=>{
  const f=fixture();const registry=f.sandbox.__operitHostPromiseRegistry;const waits=[];const listeners=new Set();
  f.sandbox.signal={aborted:false,addEventListener(name,listener){assert.equal(name,"abort");listeners.add(listener);},removeEventListener(name,listener){listeners.delete(listener);}};
  f.sandbox.__operitNativeWaitPluginStorageRecord=registry.binding((id,handle,collection,key,after)=>waits.push({id,handle,collection,key,after}));
  const opening=f.run("Tools.Storage.dataStore.open({path:'runtime/owned/settings.preferences.json'}).then(value=>{globalThis.db=value})");f.reply({handle:"parent",path:"runtime/owned/settings.preferences.json"});await opening;
  f.run("globalThis.stream=db.dataFlow({signal})");const pending=f.run("stream.next()");
  await new Promise(resolve=>setImmediate(resolve));
  assert.equal(f.request().op,"preferences_data");f.reply({values:{theme:"dark"},schemaVersion:0});
  await new Promise(resolve=>setImmediate(resolve));
  assert.equal(f.request().op,"open_preferences");f.reply({handle:"watcher",path:"runtime/owned/settings.preferences.json"});
  await new Promise(resolve=>setImmediate(resolve));
  assert.equal(waits[0].collection,"__preferences");assert.equal(waits[0].key,"snapshot");f.sandbox.signal.aborted=true;
  for(const listener of [...listeners])listener();await new Promise(resolve=>setImmediate(resolve));
  assert.deepEqual(f.request(),{op:"close",handle:"watcher"});f.reply(null);registry.settle(waits.shift().id,"closed",true);
  assert.equal((await pending).done,true);assert.equal(listeners.size,0);assert.equal(f.sandbox.refs,0);
});

/** Exercises the real facade's ordered atomic reads, absence semantics, bounds and disposal. */
test("object getMany preserves ordered null records and rejects invalid batches", async () => {
  const f = fixture();
  const opening = f.run("Tools.Storage.objects.open({path:'runtime/owned/objects.db'}).then(value=>{globalThis.db=value;})");
  f.reply({handle:"objects",path:"runtime/owned/objects.db"}); await opening;
  const reading = f.run("db.collection('memories').getMany(['present','missing','nullable','present'])");
  assert.deepEqual(f.request(), {op:"get_many",handle:"objects",collection:"memories",keys:["present","missing","nullable","present"]});
  const values = [{value:{id:"present",tags:["one"]},version:"v1"},null,{value:null,version:"v2"},{value:{id:"present",tags:["one"]},version:"v1"}];
  f.reply(values);
  assert.deepEqual(structuredClone(await reading), values);
  assert.deepEqual(structuredClone(await f.run("db.collection('memories').getMany([])")), []);
  for (const batch of ["null", "[1]", "Array(1001).fill('key')"]) {
    await assert.rejects(f.run("db.collection('memories').getMany(" + batch + ")"), /1000 string keys/);
    assert.equal(f.pending.length, 0);
  }
  const closing = f.run("db.close()"); f.reply(null); await closing;
  await assert.rejects(f.run("db.collection('memories').getMany([])"), /closed/);
});

/** Exercises read-only object SQL through the exact shared SQLite facade. */
test("object SQL queries preserve typed parameters and rows", async () => {
  const f = fixture();
  const opening = f.run("Tools.Storage.objects.open({path:'runtime/owned/objects.db'}).then(value=>{globalThis.db=value;})");
  f.reply({handle:"objects",path:"runtime/owned/objects.db"}); await opening;
  const reading = f.run("db.query('SELECT key FROM plugin_objects WHERE collection=? LIMIT ?', ['memories',3])");
  const request = f.request(); assert.equal(request.op,"query"); assert.equal(request.handle,"objects");
  assert.deepEqual(request.statement.params,[{kind:"text",value:"memories"},{kind:"integer",value:"3"}]);
  f.reply([{columns:["key"],values:[{kind:"text",value:"character:one/memory"}]}]);
  assert.equal((await reading)[0].key,"character:one/memory");
});

/** Exercises related-record preconditions without rewriting the guarded owner. */
test("object commits forward related-record version guards", async () => {
  const f = fixture();
  const opening = f.run("Tools.Storage.objects.open({path:'runtime/owned/objects.db'}).then(value=>{globalThis.db=value;})");
  f.reply({handle:"objects",path:"runtime/owned/objects.db"}); await opening;
  const writing = f.run("db.commit([{op:'put',collection:'memories',key:'owner/memory',value:{record:'source'},expectedVersion:null}],[{collection:'owners',key:'owner',expectedVersion:'v1'}])");
  assert.deepEqual(f.request().guards,[{collection:"owners",key:"owner",expectedVersion:"v1"}]);
  f.reply({version:"v2"}); assert.equal((await writing).version,"v2");
});

/** The generic SDK forwards opaque cursors and preserves deletion versus a stored null. */
test("object change pages expose the host cursor and current envelopes", async () => {
  const f = fixture();
  const opening = f.run("Tools.Storage.objects.open({path:'runtime/owned/live.sqlite'}).then(value=>{globalThis.db=value;})");
  f.reply({handle:"objects",path:"runtime/owned/live.sqlite"}); await opening;
  const pending = f.run("db.changes('opaque#17',25)");
  assert.deepEqual(f.request(), {op:"changes",handle:"objects",after:"opaque#17",limit:25});
  const page = {cursor:"opaque#19",reset:false,hasMore:false,changes:[{collection:"memories",key:"one",entry:{version:"version",value:null}},{collection:"memories",key:"removed",entry:null}]};
  f.reply(page);
  assert.deepEqual(JSON.parse(JSON.stringify(await pending)), page);
});
