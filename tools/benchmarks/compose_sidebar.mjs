/** Measures identical sidebar inputs; projections and hashing remain outside all timed regions. */
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';
import { createHash } from 'node:crypto';
import vm from 'node:vm';
import { packageRuntime } from '../../plugins/packages/buildin/character_cards/tests/package-runtime.mjs';
import { buildUiScreenScripts } from '../../plugins/packages/buildin/character_cards/scripts/build.mjs';
import { composeStreamFixture } from '../tests/support/compose_stream_fixture.mjs';
import { composeExecutionScript, composeManifest, composeRoot, composeSource } from '../runtime/compose_scripts.mjs';

const count = Number(process.argv[2] ?? 129);
const archive = process.argv[3];
if (!Number.isInteger(count) || count < 1) throw new RangeError('Row count must be a positive integer');
// An explicitly selected archive supplies its complete compiler, runtime and assertion adapter.
const manifest = archive ? JSON.parse(readFileSync(resolve(archive,'sources.json'),'utf8')) : composeManifest;
/** Reads one selected SDK version without mixing execution engines. */
function sdk(name) { return archive ? readFileSync(resolve(archive,name),'utf8') : composeSource(name); }
const scripts = await buildUiScreenScripts({precompile:false});
const raw = Buffer.from(scripts['dist/ui/chat-sidebar/index.ui.js']).toString('utf8');
const compiler = vm.createContext({});
const parser = sdk(manifest.parser).replace('__OPERIT_COMPOSE_PARSER_SOURCE__', () => 'function(){return '+JSON.stringify(readFileSync(new URL(composeManifest.parserSource,composeRoot),'utf8'))+';}');
vm.runInContext(parser+'\n'+sdk(manifest.artifact)+'\n'+sdk(manifest.compiler),compiler);
const source = compiler.OperitComposeCompiler.artifact(raw,'com.operit.character_cards:dist/ui/chat-sidebar/index.ui.js');
const chats = Array.from({ length: count }, (_, index) => ({ id: 'chat-' + index, title: 'Conversation ' + index,
  updatedAt: '1792751500000', displayOrder: index, workspaceId: null, workspaceName: null,
  locked: false, pinned: true, group: null, parentChatId: null }));
const input = { view: 'characters' };
const catalog = { view: 'characters', sections: [{ id: 'card:operit', title: 'Operit', avatarUri: null, kind: 'card', selection: 'card:operit', chats, conversationGroups: [], ungroupedChats: chats }] };
const runtime = packageRuntime({ 'dist/ui/chat-sidebar/index.ui.js': source });
if (archive) vm.runInContext(manifest.context.map(sdk).join('\n')+'\n'+sdk(manifest.compiler),runtime.context);
runtime.context.module = { exports: runtime.load('dist/ui/chat-sidebar/index.ui.js') };
vm.runInContext(archive ? '(function(){'+manifest.execution.map(sdk).join('\n')+'\n})();' : composeExecutionScript(),runtime.context);
const fixtureFactory = archive ? (await import(pathToFileURL(resolve(archive,'fixture.mjs')))).composeStreamFixture : composeStreamFixture;
const stream = fixtureFactory(runtime.context,{projectTree:false,inspectStorage:false});
const options = { packageName: 'com.operit.character_cards', executionContextKey: 'sidebar-perf' };
const state = { input, chatSidebar: { chats, currentChatId: 'chat-0', activeStreamingChatIds: [] }, chatSidebarPrepared: { context: JSON.stringify({ input, chats }), data: catalog, defaultAvatar: null } };
let renderMs=0, commitMs=0;
const render=runtime.context.OperitComposeReactive.render;
/** Measures the exact production reactive execution boundary. */
runtime.context.OperitComposeReactive.render=function(...args) {const start=performance.now();try{return render(...args);}finally{renderMs+=performance.now()-start;}};
const create=runtime.context.OperitComposeDslRuntime.createContext;
/** Measures the selected version's sole transaction boundary. */
runtime.context.OperitComposeDslRuntime.createContext=function(...args) {
 const bundle=create(...args), name=archive?'commit':'finish',commit=bundle.composition[name];
 /** Preserves commit arguments and result while measuring writer cost. */
 bundle.composition[name]=function(...args){const start=performance.now();try{return commit(...args);}finally{commitMs+=performance.now()-start;}};
 return bundle;
};
/** Canonicalizes visible behavior independently of source IDs and callback numbering. */
function canonical(nodes,id) {
 const node=nodes.get(id);
 assert.ok(node,'Missing benchmark node '+id);
 /** Compares callback presence while excluding engine-private addresses. */
 function value(input) {
  if(input===null||typeof input!=='object')return input;
  if(Object.hasOwn(input,'__actionId'))return {event:true};
  if(input.__composeUndefined===true)return undefined;
  return Array.isArray(input)?input.map(value):Object.fromEntries(Object.entries(input).map(([k,v])=>[k,value(v)]));
 }
 return {type:node.nodeType,props:value(node.props),children:node.children.map(id=>canonical(nodes,id)),slots:Object.fromEntries(Object.entries(node.slots).map(([key,ids])=>[key,ids.map(id=>canonical(nodes,id))]))};
}
/** Measures real construction and commit, then validates output after stopping the clock. */
async function measure(name, operation) {
 renderMs=0;commitMs=0;
 const frames=[];
 runtime.context.__operit_call_runtime_ref={...runtime.context.__operit_call_runtime_ref,
  /** Captures production frames without charging assertion decoding to construction. */
  sendComposeResponse(phase,frame){frames.push({phase,frame});}
 };
 const start=performance.now();
 assert.equal(await operation(),null);
 const ms=performance.now()-start;
 for(const {phase,frame} of frames)stream.accept(phase,frame);
 const result=stream.latest;
 const nodes=stream.nodes;
 const root=archive?result.update.rootId:result.update.root.instanceId+'#'+result.update.root.node;
 const tree=canonical(nodes,root);
 const digest=createHash('sha256').update(JSON.stringify(tree)).digest('hex');
 const metrics=runtime.context.__operit_compose_bundle.composition.metrics();
 const report={name,count,version:archive?'before':'program',ms,renderMs,commitMs,digest,nodes:nodes.size,
  transportEntries:(archive?result.update.upserts:result.update.instances).length,bytes:archive?Buffer.byteLength(JSON.stringify(result.update)):frames.reduce((total,{frame})=>total+frame.byteLength,0),metrics};
 console.log(JSON.stringify(report));
 if(process.env.OPERIT_COMPOSE_BENCH_OUTPUT&&name==='initial')writeFileSync(process.env.OPERIT_COMPOSE_BENCH_OUTPUT,archive?JSON.stringify(result.update):frames.find(({phase})=>phase==='final').frame);
}
await measure('initial',()=>runtime.context.__operit_render_compose_dsl({...options,state}));
for(let i=0;i<4;i++)await measure('unchanged',()=>runtime.context.__operit_render_compose_dsl(options));
for(let i=0;i<20;i++) {
 const nodes=stream.nodes;
 const group=[...nodes.values()].find(node=>node.props.key==='sidebar-group-drop-ungrouped:card:operit');
 const header=nodes.get(nodes.get(group.children[0]).children[0]);
 await measure(i%2===0?'collapse':'expand',()=>runtime.context.__operit_dispatch_compose_dsl_action({...options,actionId:header.props.onClick.__actionId}));
}
