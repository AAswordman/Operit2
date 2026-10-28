import test from 'node:test';
import assert from 'node:assert/strict';
import {defaultProject} from '../src/layout/default-project.mts';
import {validate,applyOperations} from '../src/layout/layout-model.mts';
import {pageEdges,nodePointer,expandMatrix} from '../src/layout/project-model.mts';
import {resizeFromCorner} from '../src/layout/geometry.mts';
import {layoutSourceReference} from '../src/source/source-reference.mts';

test('default home and all seven pages share valid routes and source locations',()=>{
 const doc=defaultProject();assert.deepEqual(validate(doc),[]);
 const pointer=nodePointer(doc,'theme_palette');assert.equal(pointer,'/pages/1/nodes/3');
 const ref=layoutSourceReference(JSON.stringify(doc,null,2),'theme_palette','rev');
 assert.equal(ref.jsonPointer,pointer);assert.ok(ref.startLine>10);
 assert.ok(pageEdges(doc).some(e=>e.from==='home'&&e.to==='apps'&&e.field==='swipeLeft'));
});
test('page removal clears incoming button, long press and swipe routes, protects entry',()=>{
 let doc=applyOperations(defaultProject(),[{op:'addPage',page:{id:'detail',name:'Detail',background:'#091420',nodes:[]}},
  {op:'update',id:'home_apps',changes:{action:'go:detail',longAction:'go:detail'}},
  {op:'document',changes:{swipeLeft:'detail'}}]);
 assert.throws(()=>applyOperations(doc,[{op:'removePage',id:'home'}]));
 assert.throws(()=>applyOperations(doc,[{op:'document',changes:{entryPage:'detail'}},{op:'removePage',id:'detail'}]));
 doc=applyOperations(doc,[{op:'removePage',id:'detail'}]);
 assert.equal(doc.swipeLeft,'');assert.equal(doc.nodes.at(-1).action,'');assert.equal(doc.nodes.at(-1).longAction,'');
 assert.deepEqual(validate(doc),[]);
});
test('malformed AI project shapes and cross-page references are rejected without crashing validator',()=>{
 for(const nodes of [{},12,'text',null]){const doc=defaultProject();doc.pages[0].nodes=nodes;assert.ok(validate(doc).length);}
 for(const action of [42,{},null,'go:missing','page:missing']){const doc=defaultProject();doc.nodes[0].action=action;assert.ok(validate(doc).length);}
 const doc=defaultProject();doc.pages[0].nodes[0].id=doc.nodes[0].id;assert.ok(validate(doc).length);
 doc.pages[0].nodes[0].parent='home_card';assert.ok(validate(doc).length);
});
test('matrix becomes a panel with independently editable and routable buttons',()=>{
 const parent={...defaultProject().nodes.at(-1),id:'matrix',type:'buttonmatrix',x:0,y:0,w:170,h:70,text:'One|Two|Three'};
 const nodes=expandMatrix(parent),doc=defaultProject();doc.nodes=nodes;
 assert.deepEqual(validate(doc),[]);assert.equal(nodes.length,4);
 const edited=applyOperations(doc,[{op:'update',id:'matrix_cell1',changes:{text:'Edit',action:'go:settings',w:70}}]);
 assert.equal(edited.nodes[2].text,'Edit');assert.equal(edited.nodes[1].text,'One');
});
test('all resize corners preserve opposite corner and constrain children/parent',()=>{
 const n={x:40,y:40,w:80,h:60};
 assert.deepEqual(resizeFromCorner(n,'nw',-8,-12,320,240),{x:32,y:28,w:88,h:72});
 assert.deepEqual(resizeFromCorner(n,'ne',8,-12,320,240),{x:40,y:28,w:88,h:72});
 assert.deepEqual(resizeFromCorner(n,'sw',-8,12,320,240),{x:32,y:40,w:88,h:72});
 assert.deepEqual(resizeFromCorner(n,'se',8,12,320,240),{x:40,y:40,w:88,h:72});
 const bounded=resizeFromCorner(n,'nw',200,200,320,240,[{x:0,y:0,w:70,h:50}]);
 assert.deepEqual(bounded,{x:50,y:50,w:70,h:50});
 assert.deepEqual(resizeFromCorner(n,'se',1000,1000,320,240),{x:40,y:40,w:280,h:200});
});
