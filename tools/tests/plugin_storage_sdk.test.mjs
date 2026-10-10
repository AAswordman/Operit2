import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import test from "node:test";
import { installScopedHostRuntime } from "./fixtures/scoped_host_runtime.mjs";

const source=readFileSync(new URL("../../core/crates/plugin/sdk/src/js_sdk/storage_runtime.js",import.meta.url),"utf8");

/** Runs production SDK and host Promise registry, with only storage operations supplied by the test. */
function fixture() {
  const pending=[];
  const sandbox=vm.createContext({pending});
  const registry=installScopedHostRuntime(sandbox);
  sandbox.__operitNativeStorageRequestAsync=registry.binding((requestId,request)=>pending.push({requestId,request}));
  vm.runInContext("globalThis.Tools={Storage:{}};",sandbox);
  vm.runInContext(source+"\nTools.Storage.request=__operitStorageRequest;__operitInstallStorageFacade();",sandbox);
  return { sandbox,pending,
    run(code) {return vm.runInContext(code,sandbox);},
    reply(value,isError=false) {const entry=pending.shift(); assert.ok(entry); registry.settle(entry.requestId,value,isError);return entry.request;},
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
  const changed=f.run("db.changes()");assert.deepEqual(f.request(),{op:"changes",handle:"sql",after:"0",limit:100});f.reply([]);await changed;
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

/** Keeps stored nulls, key removals and exact absence checks distinct in one batch. */
test("DataStore creates an atomic key mutation batch and rejects unsupported structured values",async()=>{
  const f=fixture();const opening=f.run("Tools.Storage.dataStore.open({path:'runtime/owned/keys.db'}).then(value=>{globalThis.db=value;})");f.reply({handle:"keys",path:"runtime/owned/keys.db"});await opening;
  const commit=f.run("db.commit({set:{nullable:null,count:3},remove:['old'],expectedVersions:{nullable:null,old:'v1'}})");
  assert.deepEqual(f.request().mutations,[{collection:"keys",key:"nullable",value:null,deleted:false,checkVersion:true,expectedVersion:null},{collection:"keys",key:"count",value:3,deleted:false,checkVersion:false,expectedVersion:null},{collection:"keys",key:"old",value:null,deleted:true,checkVersion:true,expectedVersion:"v1"}]);f.reply({version:"v2"});assert.equal((await commit).version,"v2");
  for(const code of ["db.put('x',undefined)","db.put('x',NaN)","db.put('x',new Date())","db.put('x',Array(1))","db.put('x',Object.defineProperty({},'x',{get(){return 1;},enumerable:true}))","(()=>{const x={};x.self=x;return db.put('x',x)})()"]){assert.throws(()=>f.run(code),/Storage/);assert.equal(f.pending.length,0);}
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
