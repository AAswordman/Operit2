import test from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import { readFileSync } from "node:fs";

const source=readFileSync(new URL("../../sdk/src/js_sdk/storage_runtime.js",import.meta.url),"utf8");

/** Drives the SDK facade with an explicit host contract; native persistence is tested in plugin_preferences.rs. */
function harness(values={},version=0) {
  const state={values:structuredClone(values),version,generation:0};
  const handles=new Map(), requests=[], waits=new Map();
  let id=0, conflict=false;
  /** Resolves the concrete host snapshot without sharing mutable host-owned values. */
  function snapshot(){return {values:structuredClone(state.values),schemaVersion:state.version};}
  /** Signals each subscribed host wait after a committed snapshot change. */
  function wake(){for(const [handle,resolve] of waits){waits.delete(handle);resolve();}}
  /** Supplies deterministic native results and records the exact dispatched protocol. */
  async function request(call) {
    requests.push(structuredClone(call));
    if(call.op==="open_preferences") {const handle=String(++id);handles.set(handle,{edit:null});return {op:"open",handle,path:call.path};}
    const handle=handles.get(call.handle);
    if(!handle) throw new Error("Unknown or closed storage handle");
    switch(call.op) {
      case "close":handles.delete(call.handle);wake();return {op:"close"};
      case "preferences_schema": {
        const {version,steps}=call.schema;
        if(state.version>version)throw new Error("Preference schema is newer than supported");
        const next=structuredClone(state.values);
        for(let n=state.version;n<version;n++)for(const rule of steps[n]) {
          switch(rule.op) {
            case "rename_key":if(Object.hasOwn(next,rule.from)){if(Object.hasOwn(next,rule.to))throw new Error("Key destination exists");next[rule.to]=next[rule.from];delete next[rule.from];}break;
            case "decode_json":if(typeof next[rule.key]==="string")next[rule.key]=JSON.parse(next[rule.key]);break;
            default:throw new Error("Unsupported migration rule");
          }
        }
        state.values=next;state.version=version;return {op:call.op,snapshot:snapshot()};
      }
      case "preferences_data":return {op:call.op,snapshot:snapshot()};
      case "preferences_begin_edit": {
        assert.equal(handle.edit,null,"Only one callback may be active on a handle");
        handle.edit={token:String(++id),generation:state.generation};
        return {op:call.op,edit:{token:handle.edit.token,snapshot:snapshot()}};
      }
      case "preferences_cancel_edit":assert.equal(call.token,handle.edit.token);handle.edit=null;return {op:call.op};
      case "preferences_finish_edit": {
        assert.equal(call.token,handle.edit.token);
        const generation=handle.edit.generation;handle.edit=null;
        if(conflict || generation!==state.generation) throw new Error("DataStore edit conflict: native snapshot changed");
        const next=structuredClone(state.values);
        for(const edit of call.edits) {
          switch(edit.op) {
            case "set":Object.defineProperty(next,edit.key,{value:structuredClone(edit.value),writable:true,configurable:true,enumerable:true});break;
            case "remove":delete next[edit.key];break;
            case "increment_integer":next[edit.key]=(Object.hasOwn(next,edit.key)?next[edit.key]:0)+edit.delta;break;
            case "set_object_field":if(!Object.hasOwn(next,edit.key))next[edit.key]={};Object.defineProperty(next[edit.key],edit.field,{value:structuredClone(edit.value),writable:true,configurable:true,enumerable:true});break;
            default:assert.fail("Unexpected edit intention");
          }
        }
        state.values=next;state.version=call.schemaVersion;state.generation++;wake();
        return {op:call.op,snapshot:snapshot()};
      }
      default:throw new Error("Unexpected host operation: "+call.op);
    }
  }
  /** Waits for a complete snapshot change and honors closure of the collector's handle. */
  async function wait(handle,collection,key,after) {
    assert.equal(collection,"__preferences");assert.equal(key,"snapshot");
    while(handles.has(handle)) {
      if(after===null || after.version!==String(state.generation))return {value:snapshot(),version:String(state.generation)};
      await new Promise(resolve=>waits.set(handle,resolve));
    }
    throw new Error("Storage handle is not active");
  }
  const context=vm.createContext({Tools:{Storage:{request}},console,AbortController,__operitNativeWaitPluginStorageRecord:wait,
    /** Executes the host's existing cancellable asynchronous subscription bridge. */
    __operitInvokeHostAsync(fn,args){return fn(...args);}});
  vm.runInContext(source+"\n__operitInstallStorageFacade();",context);
  return {state,requests,context,
    /** Runs plugin JavaScript within its own realm and ordinary SDK globals. */
    run(code){return vm.runInContext(code,context);},
    /** Executes top-level asynchronous plugin statements in a retained global realm. */
    execute(code){return vm.runInContext("(async()=>{"+code+"})()",context);},
    /** Injects a genuine host conflict without rerunning a JavaScript callback. */
    conflict(){conflict=true;},
    /** Supplies a remote committed snapshot for subscription tests. */
    remote(value){state.values=structuredClone(value);state.generation++;wake();},
  };
}

/** Exposes Rust-style preference methods without the old object collection or commit facade. */
test("DataStore exposes data/edit/schema/flow rather than collection APIs",async()=>{
  const h=harness();
  await h.execute('globalThis.store=await Tools.Storage.dataStore.open({path:"roles.preferences.json"})');
  assert.equal(h.run('typeof store.edit'),"function");
  assert.equal(h.run('typeof store.withSchema'),"function");
  for(const key of ["get","getMany","list","put","remove","commit","collection"])assert.equal(h.run(`typeof store.${key}`),"undefined");
  assert.deepEqual(h.requests[0],{op:"open_preferences",path:"roles.preferences.json"});
  await assert.rejects(h.run('Tools.Storage.dataStore.open({path:"x.preferences.json",sync:true})'),/Unknown storage open option/);
});

/** Keeps stored null distinct from absence and makes read snapshots deeply immutable. */
test("data returns typed immutable preference snapshots",async()=>{
  const h=harness({nullable:null,card:{name:"A",tags:["x"]}});
  await h.execute('globalThis.store=await Tools.Storage.dataStore.open({path:"roles.preferences.json"});globalThis.data=await store.data()');
  assert.equal(h.run('data.get("nullable")'),null);assert.equal(h.run('data.get("missing")'),undefined);
  assert.equal(h.run('data.contains("nullable")'),true);assert.equal(h.run('data.contains("missing")'),false);
  assert.equal(h.run('Object.isFrozen(data.get("card").tags)'),true);
  assert.throws(()=>h.run('"use strict";data.get("card").name="broken"'),/read only|readonly/i);
  assert.throws(()=>h.run('data.get("__operit_preferences_merge")'),/metadata/);
  assert.equal(h.state.values.card.name,"A");
});

/** Publishes one successful callback and preserves the native delta and field operations. */
test("edit commits once and invalidates retained mutable preferences",async()=>{
  const h=harness({obsolete:true});
  await h.execute(`globalThis.store=await Tools.Storage.dataStore.open({path:"roles.preferences.json"});
    globalThis.committed=await store.edit(p=>{globalThis.retained=p;p.set("nullable",null);p.remove("obsolete");p.incrementInteger("counter",4);p.setObjectField("settings","theme","dark");});`);
  assert.deepEqual(h.state.values,{nullable:null,counter:4,settings:{theme:"dark"}});
  const submit=h.requests.filter(call=>call.op==="preferences_finish_edit");assert.equal(submit.length,1);
  assert.deepEqual(submit[0].edits.map(edit=>edit.op),["set","remove","increment_integer","set_object_field"]);
  assert.equal(h.run('committed.get("counter")'),4);
  assert.throws(()=>h.run('retained.set("after",1)'),/callback has completed/);
});

/** Discards a callback failure and retains the exact original JavaScript exception. */
test("callback errors cancel the native edit without publication",async()=>{
  const h=harness({original:true});
  await h.execute('globalThis.store=await Tools.Storage.dataStore.open({path:"roles.preferences.json"});globalThis.failure=new Error("callback failure")');
  await assert.rejects(h.run('store.edit(p=>{p.set("uncommitted",1);throw failure})'),error=>error===h.context.failure);
  assert.deepEqual(h.state.values,{original:true});
  assert.equal(h.requests.filter(call=>call.op==="preferences_finish_edit").length,0);
  assert.equal(h.requests.filter(call=>call.op==="preferences_cancel_edit").length,1);
  await h.run('store.edit(p=>p.set("next",true))');assert.equal(h.state.values.next,true);
});

/** Rejects invalid values before crossing the host boundary and cancels the outstanding baseline. */
test("unsupported preference mutations do not reach native publication",async()=>{
  const h=harness();await h.execute('globalThis.store=await Tools.Storage.dataStore.open({path:"roles.preferences.json"})');
  for(const expression of ['p.set("x",undefined)','p.set("__operit_preferences_schema_version",3)','p.incrementInteger("count",Number.MAX_SAFE_INTEGER+1)','p.setObjectField("scalar","field",()=>{})']) {
    await assert.rejects(h.run(`store.edit(p=>{${expression}})`));
  }
  assert.deepEqual(h.state.values,{});
  assert.equal(h.requests.filter(call=>call.op==="preferences_finish_edit").length,0);
  assert.equal(h.requests.filter(call=>call.op==="preferences_cancel_edit").length,4);
});

/** Serializes accepted edits on one handle across asynchronous callback suspension. */
test("concurrent asynchronous edits on a handle observe prior commits",async()=>{
  const h=harness({counter:0});await h.execute('globalThis.store=await Tools.Storage.dataStore.open({path:"roles.preferences.json"})');
  await h.run('Promise.all([store.edit(async p=>{await Promise.resolve();p.set("counter",p.get("counter")+1)}),store.edit(p=>p.set("counter",p.get("counter")+1))])');
  assert.equal(h.state.values.counter,2);
  assert.deepEqual(h.requests.filter(call=>call.op.startsWith("preferences_")).map(call=>call.op),["preferences_begin_edit","preferences_finish_edit","preferences_begin_edit","preferences_finish_edit"]);
});

/** Propagates native contention exactly once without secretly rerunning an asynchronous callback. */
test("native edit conflicts reject without rerunning callbacks",async()=>{
  const h=harness({value:0});await h.execute('globalThis.store=await Tools.Storage.dataStore.open({path:"roles.preferences.json"});globalThis.calls=0');h.conflict();
  await assert.rejects(h.run('store.edit(p=>{calls++;p.set("value",1)})'),/edit conflict/);
  assert.equal(h.context.calls,1);assert.equal(h.state.values.value,0);
  assert.equal(h.requests.filter(call=>call.op==="preferences_finish_edit").length,1);
  assert.equal(h.requests.filter(call=>call.op==="preferences_cancel_edit").length,0);
});

/** Sends portable numbered rules once; only the host executes migration. */
test("schema rules are registered once with native persistence",async()=>{
  const h=harness({old:{id:"1"},profile:'{"enabled":true}'});
  await h.execute(`globalThis.store=(await Tools.Storage.dataStore.open({path:"roles.preferences.json"})).withSchema(2,
    [[{op:"rename_key",from:"old",to:"card"}],[{op:"decode_json",key:"profile"}]]);await store.data();await store.data();`);
  assert.deepEqual(h.state.values,{card:{id:"1"},profile:{enabled:true}});assert.equal(h.state.version,2);
  assert.equal(h.requests.filter(call=>call.op==="preferences_schema").length,1);
  assert.equal(h.requests.filter(call=>call.op==="preferences_finish_edit").length,0);
  assert.equal(await h.run('store.schemaVersion()'),2);
  assert.throws(()=>h.run('store.withSchema(3,[[],[],[]])'),/before using/);
});

/** Rejects arbitrary JS migrations and incomplete rule histories at the public boundary. */
test("schema declarations require portable complete steps",async()=>{
  const h=harness();await h.execute('globalThis.store=await Tools.Storage.dataStore.open({path:"roles.preferences.json"})');
  assert.throws(()=>h.run('store.withSchema(1,()=>{})'),/rule array/);
  assert.throws(()=>h.run('store.withSchema(2,[[]])'),/rule array/);
});

/** A failed native migration publishes neither earlier steps nor a version bump. */
test("schema failures preserve the complete source snapshot",async()=>{
  const h=harness({original:true,bad:"not json"});
  await h.execute(`globalThis.store=(await Tools.Storage.dataStore.open({path:"roles.preferences.json"})).withSchema(2,
    [[{op:"rename_key",from:"original",to:"draft"}],[{op:"decode_json",key:"bad"}]]);`);
  await assert.rejects(h.run('store.data()'),SyntaxError);
  assert.equal(h.state.version,0);assert.deepEqual(h.state.values,{original:true,bad:"not json"});
});

/** A future native format rejects the declaration before executing any user edit. */
test("future schema versions cannot invoke edit callbacks or overwrite data",async()=>{
  const h=harness({future:true},3);
  await h.execute('globalThis.calls=0;globalThis.store=(await Tools.Storage.dataStore.open({path:"roles.preferences.json"})).withSchema(2,[[],[]])');
  await assert.rejects(h.run('store.data()'),/newer than supported/);
  await assert.rejects(h.run('store.edit(()=>{calls++})'),/newer than supported/);
  assert.equal(h.context.calls,0);assert.equal(h.state.version,3);assert.deepEqual(h.state.values,{future:true});
});

/** Observes complete immutable snapshots and releases a pending collector independently of its owner. */
test("dataFlow observes remote snapshots and cancellation preserves the parent handle",async()=>{
  const h=harness({first:true});
  await h.execute('globalThis.store=await Tools.Storage.dataStore.open({path:"roles.preferences.json"});globalThis.flow=store.dataFlow();globalThis.initial=await flow.next()');
  assert.equal(h.run('initial.value.get("first")'),true);
  const next=h.run('flow.next()');h.remote({second:true});const changed=await next;
  assert.equal(changed.value.get("second"),true);
  const pending=h.run('flow.next()');await h.run('flow.return()');assert.equal((await pending).done,true);
  assert.equal((await h.run('store.data()')).get("second"),true);
  assert.equal(h.requests.filter(call=>call.op==="close").length,1);
  await h.run('store.close()');
  await assert.rejects(h.run('store.data()'),/handle is closed/);
});
