import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';

const screen=createRequire(import.meta.url)('../dist/console.ui.js').default;
const conversation={key:'lark-room',channel:'feishu',title:'团队反馈'};

function fixture({files=[{path:'/selected/image.png',name:'image.png',mimeType:'image/png'}], messages, feishu, qq, wechat=true, conversations=[conversation]}={}){
  const state=new Map(), refs=new Map(), calls=[], saved=[], navigation=[];
  let login={state:'idle',qrContent:'',error:'',expiresAt:0};
  globalThis.ToolPkg={ipc:{async call(channel,request,options){
    assert.equal(channel,'connector-hub.console');assert.equal(options.targetRuntime,'main');calls.push(request);
    if(request.action==='dashboard')return {connectors:[
      wechat ? {channel:'wechat',available:true,status:{configured:false,service:'stopped'}} : {channel:'wechat',available:false,issue:'微信 Bot 未启用'},
      {channel:'qq',available:!!qq,status:qq}, ...(feishu?[{channel:'feishu',available:true,status:feishu}]:[])],conversations};
    if(request.action==='manage'){
      if(request.command==='group_auto_reply'){feishu.groupAutoReplyEnabled=request.enabled;return feishu;}
      if(request.command==='login_start')login={state:'need_verifycode',qrContent:'https://liteapp.weixin.qq.com/q/test',error:'',expiresAt:100};
      if(request.command==='login_verify')login={state:'confirmed',qrContent:'',error:'',expiresAt:0};
      return {login};
    }
    if(request.action==='history')return {conversation,latestReplyKey:'target-b',messages:messages??[{messageId:'a',replyKey:'target-a',text:'第一条',receivedAt:1},{messageId:'b',replyKey:'target-b',text:'第二条',receivedAt:2}]};
    if(request.action==='console_reply')return {response:null};
    if(request.action==='attachment')return {fileName:'photo.png',mimeType:'image/png',contentBase64:'cGhvdG8='};
    throw Error('unexpected action');
  }}};
  const ctx={
    MaterialTheme:{colorScheme:{onSurfaceVariant:'#444'}},
    Modifier:{widthIn(){return this;},onSizeChanged(){return this;}},
    UI:new Proxy({}, {get:(_,type)=>(props,children=[])=>({type,props,children:[children,props.tabs??[],props.content??[]].flat(Infinity)})}),
    useMemo(_key,read){return read();},useEffect(){},
    useState(key,initial){if(!state.has(key))state.set(key,initial);return [state.get(key),v=>state.set(key,v)];},
    useRef(key,initial){if(!refs.has(key))refs.set(key,{current:initial});return refs.get(key);},
    async openFilePicker(){return {files};},
    async saveFile(options){saved.push(options);return {cancelled:false};},
    async navigate(route){navigation.push(route);},
  };
  const walk=node=>[node,...node.children.flatMap(walk)];
  const get=key=>walk(screen(ctx)).find(node=>node.props.key===key);
  const click=async key=>{const node=get(key);assert.ok(node,`missing ${key}`);assert.notEqual(node.props.enabled,false);await node.props.onClick();};
  return {calls,state,get,click,saved,navigation,load:()=>screen(ctx).props.onLoad()};
}

test('the connector sidebar and toolbox share one registered route',()=>{
  const routes=[], entries=[];
  globalThis.ToolPkg={ipc:{on(){}},registerApi(){},registerUiRoute:r=>routes.push(r),registerNavigationEntry:e=>entries.push(e)};
  const main=createRequire(import.meta.url)('../dist/main.js');
  assert.equal(main.registerToolPkg(),true);
  assert.equal(routes.length,1);
  assert.equal(routes[0].route,'toolpkg:com.operit.connector_hub:ui:connector_console');
  assert.deepEqual(entries.map(entry=>[entry.surface,entry.route]),[['main_sidebar_plugins',routes[0].route],['toolbox',routes[0].route]]);
});

test('pinned Bot tabs reach their own controls and registered native screens',async()=>{
  const f=fixture({qq:{configured:true,listenerEnabled:true,service:'ready'}});
  await f.load();await f.click('tab-qq');
  assert.equal(f.get('tab-qq').props.selected,true);
  assert.ok(f.get('settings-qq-body'));
  await f.click('native-qq');
  await f.click('tab-wechat');
  assert.equal(f.get('tab-wechat').props.selected,true);
  assert.ok(f.get('qr-start'));assert.equal(f.get('settings-qq-body'),undefined);
  await f.click('native-wechat');
  assert.deepEqual(f.navigation,['toolpkg:com.operit.qqbot_bundle:ui:qqbot_settings','toolpkg:com.operit.wechat_bot:ui:wechat_settings']);
});

test('unavailable Bots show a package manager shortcut instead of a broken settings link',async()=>{
  const f=fixture({wechat:false});await f.load();await f.click('tab-wechat');
  assert.equal(f.get('native-wechat'),undefined);assert.equal(f.get('qr-start'),undefined);
  await f.click('extensions');assert.deepEqual(f.navigation,['native.package_manager']);
  assert.equal(f.calls.some(request=>request.action==='manage'),false);
});

test('Bot shortcuts filter sessions and tab switches preserve an open draft',async()=>{
  const f=fixture({conversations:[conversation,{key:'qq-room',channel:'qq',title:'项目交流群'}]});await f.load();
  await f.click('tab-feishu');await f.click('conversations-feishu');
  assert.equal(f.get('filters').props.value,'feishu');
  assert.equal(f.get('open-qq-room'),undefined);
  await f.click('open-lark-room');f.get('draft').props.onValueChange('先保留');
  await f.click('tab-qq');await f.click('tab-conversations');
  assert.equal(f.get('draft').props.value,'先保留');
  await f.click('back');f.get('filters').props.onValueChange('all');
  assert.ok(f.get('open-qq-room'));
});

test('console renders a local QR and submits the verification code in the same session',async()=>{
  const f=fixture();await f.load();await f.click('settings-wechat');await f.click('qr-start');
  assert.match(f.get('qr').props.source,/^<svg/);
  const input=f.get('verify');assert.equal(input.props.isPassword,true);input.props.onValueChange('123456');
  await f.click('submit-code');assert.equal(f.state.get('code'),'');assert.equal(f.get('qr'),undefined);
  assert.deepEqual(f.calls.find(request=>request.command==='login_verify'),{action:'manage',channel:'wechat',command:'login_verify',code:'123456'});
});

test('console uses the selected message and the picker typed attachment without asking for an ID',async()=>{
  const f=fixture();await f.load();await f.click('open-lark-room');await f.click('reply-a');await f.click('attachment');await f.click('send');
  assert.deepEqual(f.calls.find(request=>request.action==='console_reply'),{action:'console_reply',channel:'feishu',conversationKey:'lark-room',key:'target-a',media:{kind:'image',source:'/selected/image.png',fileName:'image.png'}});
  assert.equal(f.state.get('media'),null);
});

test('MP4 without a cover is sent as native video in one action',async()=>{
  const f=fixture({files:[{path:'/selected/clip.mp4',name:'clip.mp4',mimeType:'video/mp4'}]});
  await f.load();await f.click('open-lark-room');await f.click('attachment');await f.click('send');
  assert.deepEqual(f.calls.find(request=>request.action==='console_reply').media,{kind:'video',source:'/selected/clip.mp4',fileName:'clip.mp4'});
});

test('the Feishu group switch sends its own setting without changing private replies',async()=>{
  const status={configured:true,listenerEnabled:true,service:'ready',autoReplyEnabled:false,groupAutoReplyEnabled:false};
  const f=fixture({feishu:status});await f.load();await f.click('settings-feishu');
  f.get('group-auto').children.find(node=>node.type==='Switch').props.onCheckedChange(true);
  assert.deepEqual(f.calls.find(request=>request.command==='group_auto_reply'),{action:'manage',channel:'feishu',command:'group_auto_reply',enabled:true});
  assert.equal(status.autoReplyEnabled,false);
});

test('only rejected text can be restored as a draft, without sending it',async()=>{
  const f=fixture({messages:[{messageId:'reject',direction:'outgoing',delivery:'rejected',text:'重写此条',receivedAt:1},{messageId:'uncertain',direction:'outgoing',delivery:'unknown',text:'已经发出？',receivedAt:2}]});
  await f.load();await f.click('open-lark-room');
  assert.equal(f.get('draft-uncertain'),undefined);
  await f.click('draft-reject');
  assert.equal(f.state.get('draft'),'重写此条');
  assert.equal(f.calls.some(request=>request.action==='console_reply'),false);
  await f.click('send');
  assert.equal(f.calls.find(request=>request.action==='console_reply').key,'target-b');
});

test('the console saves selected attachment bytes without downloading on view',async()=>{
  const f=fixture({messages:[{messageId:'a',media:[{kind:'image',downloadable:true}]}]});
  await f.load();await f.click('open-lark-room');
  assert.equal(f.calls.some(request=>request.action==='attachment'),false);
  await f.click('save-a-0');
  assert.deepEqual(f.calls.at(-1),{action:'attachment',channel:'feishu',conversationKey:'lark-room',messageId:'a',index:0});
  assert.deepEqual(f.saved,[{suggestedName:'photo.png',mimeType:'image/png',contentBase64:'cGhvdG8=',extensions:['png']}]);
});
