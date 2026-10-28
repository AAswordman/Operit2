import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {catalog,validate,applyOperations} from '../src/layout/layout-model.mts';
import {componentContext,applyRouteReply} from '../src/source/component-context.mts';
import {routes} from '../src/layout/routes.mts';
import {layoutSourceReference} from '../src/source/source-reference.mts';
import {componentSource} from '../src/source/component-source.mts';
const doc=JSON.parse((await readFile(new URL('../../ui/legacy-draft.json',import.meta.url),'utf8')).replace(/^\uFEFF/,''));
test('starter layout matches firmware limits',()=>assert.deepEqual(validate(doc),[]));
test('Agent patches validate before publishing and leave input untouched',()=>{const updated=applyOperations(doc,[{op:'update',id:'title',changes:{x:20}}]);assert.equal(updated.nodes[0].x,20);assert.equal(doc.nodes[0].x,18);assert.throws(()=>applyOperations(doc,[{op:'update',id:'title',changes:{x:310}}]),/超出/);});
test('remove container also removes descendants',()=>{const updated=applyOperations(doc,[{op:'remove',id:'card'}]);assert.ok(!updated.nodes.some(n=>['card','welcome'].includes(n.id)));});
test('unsupported assets, glyphs, duplicate IDs and heavy layouts rejected',()=>{const invalid=structuredClone(doc);invalid.nodes[0].text='中文';assert.ok(validate(invalid).length);invalid.nodes[0].text='title';invalid.nodes[1].id=invalid.nodes[0].id;assert.ok(validate(invalid).length);assert.equal(catalog.filter(c=>!c.editable).length,4);});

test('click and long-press routes validate; unknown or executable routes are rejected',()=>{
 for(const route of routes){const next=applyOperations(doc,[{op:'update',id:'openApps',changes:{action:route.id,longAction:route.id}}]);assert.deepEqual(validate(next),[]);}
 for(const changes of [{action:'page:missing'},{longAction:'javascript:alert(1)'},{longAction:null}])assert.throws(()=>applyOperations(doc,[{op:'update',id:'openApps',changes}]),/不支持/);
});
test('component chat context includes exact draft, revision and capabilities without mutating source',()=>{
 const payload=componentContext(doc,'openApps','revision-1',true,'点击跳转设置');
 assert.equal(payload.componentId,'openApps');assert.equal(payload.revision,'revision-1');assert.equal(payload.dirty,true);
 assert.deepEqual(payload.document,doc);assert.ok(payload.routes.some(r=>r.id==='page:settings'));
 payload.component.action='page:theme';assert.equal(doc.nodes.find(n=>n.id==='openApps').action,'apps');
 assert.throws(()=>componentContext(doc,'missing','r',false),/删除/);
});
test('AI route proposals are restricted to the referenced component and validated before applying',()=>{
 const reply={componentId:'openApps',operations:[{op:'update',id:'openApps',changes:{action:'page:theme',longAction:'home'}}]};
 const next=applyRouteReply(doc,'openApps',reply);assert.equal(next.nodes.find(n=>n.id==='openApps').longAction,'home');
 assert.equal(doc.nodes.find(n=>n.id==='openApps').longAction,undefined);
 for(const bad of [ {...reply,componentId:'title'}, {...reply,operations:[{op:'remove',id:'openApps'}]}, {...reply,operations:[{op:'update',id:'title',changes:{action:'home'}}]}, {...reply,operations:[{op:'update',id:'openApps',changes:{x:0}}]}, {...reply,operations:[{op:'update',id:'openApps',changes:{action:'made_up_route'}}]} ])assert.throws(()=>applyRouteReply(doc,'openApps',bad));
});

test('source references use real offsets with CRLF, BOM, escaped strings and compact JSON',()=>{
 const raw='\uFEFF{\r\n "nodes": [\r\n  {"text":"fake \\\"id\\\": \\\"target\\\" { }", "id":"first"},\r\n  {\r\n   "id": "target",\r\n   "action": "home"\r\n  }\r\n ]\r\n}';
 const ref=layoutSourceReference(raw,'target','rev');
 assert.equal(ref.startLine,4);assert.equal(ref.endLine,7);assert.equal(ref.fields.action.line,6);assert.equal(ref.jsonPointer,'/nodes/1');
 assert.equal(JSON.parse(ref.excerpt).id,'target');assert.equal(ref.revision,'rev');
 const compact=JSON.stringify(JSON.parse(raw.slice(1)));assert.equal(layoutSourceReference(compact,'target','r').startLine,1);
 assert.equal(layoutSourceReference(compact,'newNode','r'),null);
});
test('source lookup rejects stale revisions and drafts never invent saved line numbers',async()=>{
 const doc=JSON.parse(await readFile(new URL('../../ui/layout.json',import.meta.url),'utf8'));
 const source=await componentSource('home_apps');assert.ok(source.location?.startLine>0);
 assert.ok(source.implementation.every(entry=>entry.line>0 && entry.revision.length===64));
 await assert.rejects(componentSource('home_apps','old-revision'),e=>e.status===409);
 const saved=componentContext(doc,'home_apps',source.revision,false,'',source);assert.equal(saved.codeReference.status,'saved-component');
 const draft=structuredClone(doc);draft.nodes.find(n=>n.id==='home_apps').action='home';
 assert.equal(componentContext(draft,'home_apps',source.revision,true,'',source).codeReference.status,'modified-component');
 const node={...draft.nodes[0],id:'unsavedButton'};draft.nodes.push(node);
 const newSource=await componentSource('unsavedButton',source.revision);
 const context=componentContext(draft,'unsavedButton',source.revision,true,'',newSource);
 assert.equal(context.codeReference.status,'unsaved-component');assert.equal(context.codeReference.location,null);
 assert.equal(context.codeReference.draftPointer,'/nodes/'+(draft.nodes.length-1));
});
