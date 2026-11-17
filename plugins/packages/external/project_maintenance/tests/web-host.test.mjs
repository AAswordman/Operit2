import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url),screen=require('../dist/ui/screen.js').default;
function fixture(){
  const state=new Map(),effects=new Map(),calls=[];let bridge,removed;
  globalThis.ToolPkg={async readResource(key,name){calls.push({key,name});return '/cache/control.html';},ipc:{async call(channel,payload){calls.push({channel,payload});return {observedAt:1};}}};
  const controller={addJavascriptInterface(name,value){bridge=value;},removeJavascriptInterface(name){removed=name;}};
  const ctx={createWebViewController:()=>controller,useState(key,value){if(!state.has(key))state.set(key,value);return [state.get(key),v=>state.set(key,v)];},useEffect(key,fn){if(!effects.has(key))effects.set(key,fn());},UI:new Proxy({}, {get:(_,type)=>(props,children=[])=>({type,props,children:[children].flat()})})};
  return {render:()=>screen(ctx),calls,get bridge(){return bridge;},cleanup(){for(const dispose of effects.values())dispose?.();return removed;}};
}
test('WebView serves only the offline control document and rejects other navigation/resources',async()=>{
  const f=fixture();await f.render().props.onLoad();const view=f.render().children[0];
  assert.equal(view.type,'WebView');assert.equal(view.props.domStorageEnabled,false);
  assert.deepEqual(view.props.onShouldOverrideUrlLoading({url:'https://maintenance.operit.local/'}),{action:'allow'});
  assert.deepEqual(view.props.onShouldOverrideUrlLoading({url:'https://example.com/'}),{action:'cancel'});
  assert.equal(view.props.onInterceptRequest({url:'https://maintenance.operit.local/'}).response.filePath,'/cache/control.html');
  assert.deepEqual(view.props.onInterceptRequest({url:'file:///etc/passwd'}),{action:'block'});
  assert.deepEqual(view.props.onInterceptRequest({url:'https://maintenance.operit.local/extra.js'}),{action:'block'});
  assert.equal(f.cleanup(),'MaintenanceHost');
});
test('host validates the handler envelope and exposes only dashboard operations',async()=>{
  const f=fixture();await f.render().props.onLoad();assert.deepEqual(await f.bridge.request([{action:'dashboard'}]),{ok:true,data:{observedAt:1}});
  assert.deepEqual(await f.bridge.request({action:'dashboard'}),{ok:false,error:'Invalid bridge arguments'});
  assert.deepEqual(await f.bridge.request([{action:'capture'}]),{ok:false,error:'Unsupported dashboard action'});
  assert.deepEqual(await f.bridge.request([{action:'channel_status'}]),{ok:false,error:'Unsupported dashboard action'});
  assert.equal(f.calls.filter(x=>x.payload).length,1);
});

test('asynchronous maintenance failures return an error envelope without escaping into Compose',async()=>{
  const f=fixture();await f.render().props.onLoad();
  globalThis.ToolPkg.ipc.call=async()=>{throw Error('工作区尚未初始化为 Git 仓库');};
  assert.deepEqual(await f.bridge.request([{action:'bind_chat',chatId:'chat',enabled:true}]),{ok:false,error:'工作区尚未初始化为 Git 仓库'});
});
