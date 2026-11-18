import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {setTimeout as delay} from 'node:timers/promises';
import {objectStorage} from '../../qqbot/tests/storage.mjs';

const require = createRequire(import.meta.url);
const {readFrame, writeFrame, utf8, string, Fragments} = require('../dist/wire.js');
const {FeishuService} = require('../dist/service.js');
const {FeishuApi} = require('../dist/openapi.js');
const {incoming} = require('../dist/message.js');
const frame = (headers, payload) => ({seq: 9007199254740993n, log: 2n, service: 42, method: 1, headers, payload});
const event = (type, content) => ({schema:'2.0',header:{event_type:'im.message.receive_v1',app_id:'cli_demo'},event:{
  sender:{sender_type:'user',sender_id:{open_id:'ou_peer'}},message:{chat_type:'p2p',chat_id:'oc_chat',message_id:'om_source',
    message_type:type,create_time:'1700000000000',content:JSON.stringify(content)}}});
const response = data => ({statusCode:200,content:JSON.stringify(data),headers:{}});

test('pbbp2 preserves unsigned 64-bit identifiers and assembles out-of-order fragments', () => {
  const ping = Buffer.from('08001000182a20002a0c0a0474797065120470696e67','hex').toString('base64');
  assert.equal(writeFrame({seq:0n,log:0n,service:42,method:0,headers:{type:'ping'},payload:new Uint8Array()}),ping);
  const fragments = new Fragments(), full = utf8('中文附件');
  const f = i => readFrame(writeFrame(frame({type:'event',sum:'2',seq:String(i),message_id:'e'},i ? full.slice(5) : full.slice(0,5))));
  assert.equal(f(1).seq,9007199254740993n);
  assert.equal(fragments.join(f(1)),null);
  assert.equal(string(fragments.join(f(0))),'中文附件');
  assert.throws(() => fragments.join(frame({sum:'33'},full)),/fragment metadata/);
});

test('rich text and media extract readable text and keep routing separate', () => {
  const data = event('post',{zh_cn:{title:'发布',content:[[{tag:'text',text:'这里'},{tag:'img',image_key:'img_key'},{tag:'media',file_key:'video_key'}]]}});
  const parsed = incoming(data,'cli_demo');
  assert.equal(parsed.text,'发布\n这里');
  assert.deepEqual(parsed.resources.map(item=>item.kind),['image','video']);
  assert.equal(incoming(data,'cli_other'),null);
  data.event.sender.sender_type='app';
  assert.equal(incoming(data,'cli_demo'),null);
});

test('socket acknowledges persisted events and does not duplicate a redelivery', async () => {
  const bytes=utf8(JSON.stringify(event('image',{image_key:'img_private'})));
  const toEvent = (index,total,part)=>({type:'binary',base64:writeFrame(frame({type:'event',sum:String(total),seq:String(index),message_id:'event-id'},part))});
  const acks=[], events=[{type:'opened'},toEvent(1,2,bytes.slice(11)),toEvent(0,2,bytes.slice(0,11)),toEvent(0,1,bytes)];
  let close=0, received;
  const complete = new Promise(resolve=>{received=resolve;});
  globalThis.CryptoJS={binary:{randomHex:()=> 'f'.repeat(32)}};
  globalThis.Tools={Storage:{objects:objectStorage()},Net:{
    async http(input){
      if (input.url.endsWith('/callback/ws/endpoint')) return response({code:0,data:{URL:'wss://feishu.example/ws?service_id=42',ClientConfig:{PingInterval:30}}});
      if (input.url.includes('tenant_access_token')) return response({code:0,tenant_access_token:'private-token',expire:7200});
      if (input.url.includes('/resources/')) {
        assert.match(input.url,/om_source\/resources\/img_private\?type=image$/);
        return {statusCode:200,headers:{'content-type':'image/jpeg'},contentBase64:'cGhvdG8='};
      }
      if (input.url.includes('/messages?')) return response({code:230001,msg:'private platform error'});
      return response({code:0,data:{message_id:'om_reply'}});
    },
    async webSocket(input){
      switch(input.op){
        case 'open_binary':return {op:'open',handle:'h'};
        case 'read':return {op:'read',events:events.splice(0)};
        case 'send_binary':{const decoded=readFrame(input.base64);if(decoded.method===1){acks.push(decoded);if(acks.length===2)received();}return {op:'send'};}
        case 'close':close++;return {op:'close'};
        default:throw Error('unexpected socket operation');
      }
    }
  }};
  globalThis.ToolPkg={getLocalDataDir:()=>'/feishu'};
  const service=new FeishuService({appId:'cli_demo',appSecret:'secret',enabled:true,autoReply:false});
  const deadline=AbortSignal.timeout(2000);
  try {
    service.restore();
    await Promise.race([complete,new Promise((_,reject)=>deadline.addEventListener('abort',()=>reject(Error('no Feishu ACK'))))]);
    assert.deepEqual(acks.map(item=>JSON.parse(string(item.payload)).code),[200,200]);
    assert.ok(acks.every(item=>item.seq===9007199254740993n && item.headers.message_id==='event-id'));
    const [session]=(await service.request({action:'conversations'})).conversations;
    const history=await service.request({action:'history',conversationKey:session.key});
    assert.deepEqual(history.messages.map(message=>message.media),[[{kind:'image',name:'image.jpg',downloadable:true}]]);
    assert.doesNotMatch(JSON.stringify(history),/img_private|secret/);
    assert.deepEqual(await service.request({action:'attachment',conversationKey:session.key,messageId:'om_source',index:0}),
      {fileName:'image.jpg',mimeType:'image/jpeg',contentBase64:'cGhvdG8='});
    await service.request({action:'reply',key:JSON.stringify(['feishu','cli_demo','oc_chat','om_source']),text:'已收到'});
    await assert.rejects(service.request({action:'send',conversationKey:session.key,text:'通知'}),/230001/);
    const timeline=await service.request({action:'history',conversationKey:session.key});
    assert.deepEqual(timeline.messages.filter(message=>message.direction).map(message=>[message.text,message.delivery]),[['已收到','accepted'],['通知','rejected']]);
    assert.doesNotMatch(JSON.stringify(timeline),/private-token|private platform error|img_private/);
  } finally {await service.stop();}
  assert.equal(close,1);
});

test('concurrent token use is coalesced and video upload uses file and cover keys', async () => {
  const uploads=[], posts=[];
  globalThis.CryptoJS={binary:{randomHex:()=> 'f'.repeat(32)}};
  globalThis.Tools={Files:{async info(){return {size:120};}},Net:{
    async http(input){posts.push(input);return response(input.url.includes('tenant_access_token') ? {code:0,tenant_access_token:'token',expire:7200} : {code:0,data:{message_id:'om_reply'}});},
    async uploadFile(input){uploads.push(input);return response({code:0,data:input.url.endsWith('/images')?{image_key:'cover'}:{file_key:'movie'}});}
  }};
  const api=new FeishuApi('cli_demo','secret');
  assert.deepEqual(await Promise.all([api.access(),api.access()]),['token','token']);
  await api.send({messageId:'om_source'},{media:{kind:'video',source:'/local/movie.mp4',cover:'/local/cover.png',durationMs:1250}});
  assert.equal(posts.filter(item=>item.url.includes('tenant_access_token')).length,1);
  assert.equal(uploads[0].files[0].field_name,'image');
  assert.deepEqual(uploads[1].form_data,{file_type:'mp4',file_name:'movie.mp4',duration:'1250'});
  assert.match(posts.at(-1).url,/om_source\/reply$/);
  assert.equal(posts.at(-1).body.msg_type,'media');
  assert.deepEqual(JSON.parse(posts.at(-1).body.content),{file_key:'movie',image_key:'cover'});
  await api.send({messageId:'om_source'},{media:{kind:'video',source:'/local/movie.mp4'}});
  assert.equal(uploads.length,3);
  assert.deepEqual(JSON.parse(posts.at(-1).body.content),{file_key:'movie'});
});

test('group replies require an exact Bot mention, stay threaded, and can be stopped independently', async () => {
  const group=(id,openId,mentionedType='bot')=>{
    const input=event('text',{text:'@_user_1 请帮忙'});
    Object.assign(input.event.message,{chat_type:'group',chat_id:'oc_group',message_id:id,
      mentions:openId?[{key:'@_user_1',id:{open_id:openId},mentioned_type:mentionedType}]:[]});
    return input;
  };
  const frames=[{type:'opened'}], acks=[], replies=[], started=[], cancelled=[];
  let identityRequests=0, identityAvailable=false, releaseLate, releaseSuperseded;
  const late=new Promise(resolve=>{releaseLate=resolve;});
  const superseded=new Promise(resolve=>{releaseSuperseded=resolve;});
  const emit=(id,input)=>frames.push({type:'binary',base64:writeFrame(frame({type:'event',message_id:id},utf8(JSON.stringify(input))))});
  const awaitState=async check=>{
    for(let i=0;i<160;i++){if(await check())return;await delay(20);}
    assert.fail('Feishu fixture did not reach expected state');
  };
  globalThis.CryptoJS={binary:{randomHex:()=> 'f'.repeat(32)}};
  globalThis.PluginConfig={async flush(){}};
  globalThis.ToolPkg={getLocalDataDir:()=>'/feishu'};
  globalThis.Tools={Storage:{objects:objectStorage()},Net:{
    async http(req){
      if(req.url.endsWith('/callback/ws/endpoint'))return response({code:0,data:{URL:'wss://feishu.example/ws?service_id=42',ClientConfig:{PingInterval:30}}});
      if(req.url.includes('tenant_access_token'))return response({code:0,tenant_access_token:'token',expire:7200});
      if(req.url.endsWith('/bot/v3/info')){
        identityRequests++;assert.equal(req.method,'GET');assert.equal(req.headers.Authorization,'Bearer token');
        return identityAvailable?response({code:0,bot:{open_id:'ou_self'}}):response({code:1,msg:'disabled'});
      }
      assert.match(req.url,/\/reply$/);replies.push(req);return response({code:0,data:{message_id:'om_reply'}});
    },
    async webSocket(req){
      if(req.op==='open_binary')return {op:'open',handle:'h'};
      if(req.op==='read')return {op:'read',events:frames.splice(0)};
      if(req.op==='send_binary'){
        const item=readFrame(req.base64);
        if(item.method===1){assert.equal(JSON.parse(string(item.payload)).code,200);acks.push(item.headers.message_id);}
        return {op:'send'};
      }
      return {op:'close'};
    },
  },Chat:{
    async createNew(){return {chatId:started.length ? 'operit-peer' : 'operit-group'};},async updateTitle(){},
    async cancel(id){cancelled.push(id);},
    async *sendMessageStreaming(request){
      started.push(request);
      if(request.input.text.startsWith('迟到'))await late;
      if(request.input.text.startsWith('旧回复'))await superseded;
      yield {type:'completed',result:{outcome:{type:'committed',status:'completed'},aiResponse:'收到'}};
    }
  }};
  const config={appId:'cli_demo',appSecret:'secret',enabled:true,autoReply:false};
  const service=new FeishuService(config);
  try {
    service.restore();
    emit('before',group('om_before','ou_self'));
    await awaitState(()=>acks.length===1);
    assert.equal(started.length,0);
    await assert.rejects(service.request({action:'manage',command:'group_auto_reply',enabled:true}),/disabled/);
    assert.equal(service.status().groupAutoReplyEnabled,false);
    identityAvailable=true;
    await service.request({action:'manage',command:'group_auto_reply',enabled:true});
    assert.equal(service.status().autoReplyEnabled,false);
    assert.equal(service.status().groupAutoReplyEnabled,true);
    assert.doesNotMatch(JSON.stringify(service.status()),/ou_self|secret/);
    emit('ordinary',group('om_ordinary'));
    emit('other',group('om_other','ou_other'));
    emit('user',group('om_user','ou_self','user'));
    emit('self',group('om_self','ou_self'));
    emit('duplicate',group('om_self','ou_self'));
    await awaitState(()=>acks.length===6 && replies.length===1);
    assert.equal(started.length,1);
    assert.match(started[0].input.text,/^请帮忙/);
    assert.match(replies[0].url,/om_self\/reply$/);
    assert.equal(replies[0].body.reply_in_thread,true);
    assert.equal(identityRequests,2);
    const delayed=group('om_late','ou_self');delayed.event.message.content=JSON.stringify({text:'@_user_1 迟到'});
    emit('late',delayed);
    await awaitState(()=>acks.length===7 && started.length===2);
    await service.request({action:'manage',command:'group_auto_reply',enabled:false});
    await service.request({action:'manage',command:'group_auto_reply',enabled:true});
    releaseLate();
    await awaitState(async()=>await service.request({action:'active_context',chatId:'operit-group'})===null);
    assert.deepEqual(cancelled,['operit-group']);
    assert.equal(replies.length,1);
    assert.equal(identityRequests,2);

    const old=group('om_old','ou_self');old.event.message.content=JSON.stringify({text:'@_user_1 旧回复'});
    emit('old',old);
    await awaitState(()=>started.length===3);
    await service.request({action:'manage',command:'start',autoReply:true});
    emit('ordinary-during-reply',group('om_ordinary2'));
    emit('peer',event('text',{text:'另一个会话'}));
    await awaitState(()=>acks.includes('peer') && replies.length===2);
    assert.deepEqual(cancelled,['operit-group']);
    emit('intermediate',group('om_intermediate','ou_self'));
    emit('latest',group('om_latest','ou_self'));
    await awaitState(()=>acks.includes('latest') && cancelled.length===2);
    assert.deepEqual(cancelled,['operit-group','operit-group']);
    assert.equal(started.length,4);
    releaseSuperseded();
    await awaitState(()=>replies.length===3);
    assert.equal(started.length,5);
    assert.match(replies[2].url,/om_latest\/reply$/);
    const session=(await service.request({action:'conversations'})).conversations.find(item=>item.conversationId==='oc_group');
    const history=await service.request({action:'history',conversationKey:session.key});
    for(const messageId of ['om_old','om_intermediate','om_latest']) {
      assert.ok(history.messages.some(message=>message.messageId===messageId));
    }
  }finally{releaseLate();releaseSuperseded();await service.stop();}
});
