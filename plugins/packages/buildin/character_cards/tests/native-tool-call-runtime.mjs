import { productionBootstrap } from '../../../../../tools/tests/fixtures/production_bootstrap.mjs';
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const root = new URL("../../../../../", import.meta.url);
const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const { buildSync } = require("esbuild");

/** Reads the exact production source used by the native JavaScript execution boundary. */
export function nativeToolSource(relative) { return readFileSync(new URL(relative, root), "utf8").replaceAll("\r\n", "\n"); }

/** Bundles the current plugin module for the actual native module factory, not a replacement tool implementation. */
export function toolModuleScript(relative) {
  const entry = fileURLToPath(new URL("../" + relative, import.meta.url));
  const result = buildSync({ entryPoints: [entry], bundle: true, format: "cjs", platform: "neutral", target: "es2020", write: false });
  assert.equal(result.outputFiles.length, 1);
  return result.outputFiles[0].text;
}

/** Supplies one explicit native runtime envelope whose exact fields are checked against JsToolManager source. */
export function nativeToolParameters(participantId) {
  return {
    __operit_package_lang: "zh",
    __operit_package_state: "fixture.state",
    __operit_package_caller_name: "Fixture Speaker",
    __operit_package_chat_id: "fixture.chat",
    __operit_package_caller_participant_id: participantId,
    __operit_package_caller_owner: "com.operit.character_cards",
    __operit_package_name: "character_memory_tools",
    __operit_toolpkg_runtime_kind: "sandbox",
    __operit_toolpkg_api_version: "2.0.0",
    __operit_execution_context_key: "toolpkg_main:com.operit.character_cards",
    __operit_toolpkg_subpackage_id: "fixture.tools",
    containerPackageName: "com.operit.character_cards",
    toolPkgId: "com.operit.character_cards",
    __operit_ui_package_name: "com.operit.character_cards",
    __operit_script_screen: "tools/index.js",
  };
}

/** Runs actual native-boundary JavaScript with strict transport endpoints; Rust injection itself is not executed by this Node harness. */
export function createNativeToolRuntime(globals, dispatchIpc) {
  const terminal = [], pending = new Map();
  let nextCall = 0;
  const context=vm.createContext({...globals});
  vm.runInContext(nativeToolSource('hosts/web/src/javascript_promises.js'),context);
  const registry=context.__operitHostPromiseRegistry;
  context.__operitNativeSetCallResult=registry.syncBinding((callId,value)=>{
    const request=pending.get(callId);assert.ok(request,'Unexpected native result '+callId);
    pending.delete(callId);terminal.push({callId,type:'result',value});request.resolve(value);
  });
  context.__operitNativeSetCallError=registry.syncBinding((callId,value)=>{
    const request=pending.get(callId);assert.ok(request,'Unexpected native failure '+callId);
    pending.delete(callId);terminal.push({callId,type:'error',value});request.reject(new Error(value.message));
  });
  context.__operitNativeLogJsExecutionTrace=(callId,message)=>{assert.equal(typeof callId,'string');assert.equal(typeof message,'string');};
  context.__operitNativeLog=()=>{};
  context.__operitNativeCancelJavaScriptPromises=scope=>registry.cancel(scope);
  context.__operitNativeHashText=text=>{let hash=0;for(let i=0;i<text.length;i++)hash=(Math.imul(hash,31)+text.charCodeAt(i))>>>0;return hash.toString(16);};
  context.__operitNativeInvokeToolPkgIpc=registry.binding((requestId,packageTarget,callerContextKey,targetContextKey,targetRuntime,channel,payload)=>{
    assert.equal(packageTarget,'com.operit.character_cards');
    assert.equal(callerContextKey,'toolpkg_main:com.operit.character_cards');
    assert.equal(targetContextKey,'');assert.equal(targetRuntime,'main');
    assert.equal(typeof dispatchIpc,'function');
    Promise.resolve().then(()=>dispatchIpc(channel,payload,{targetRuntime})).then(
      value=>registry.settle(requestId,{success:true,value},false),
      error=>registry.settle(requestId,String(error.message),true),
    );
  });
  vm.runInContext(productionBootstrap(),context);

  /** Invokes the production engine entry point with exactly the supplied converted native parameters. */
  function invoke(script, name, params) {
    const callId = "native-tool-fixture-" + ++nextCall;
    const result = new Promise(
      /** Registers the sole terminal receiver before the real module and target function execute. */
      (resolve, reject) => pending.set(callId, { resolve, reject }),
    );
    context.__operitExecuteScriptFunction(callId, params, script, name, 10, 10000);
    return result;
  }
  return { invoke, terminal };
}
