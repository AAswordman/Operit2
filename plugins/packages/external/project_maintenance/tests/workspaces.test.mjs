import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const {workspaceChats,chatWorkspace}=require('../dist/workspaces.js');

test('workspace identity and VFS directory come from the host chat binding',async()=>{
 let path='/mnt/local/folders/game', native='/Users/demo/game', prefix='', bound=true;
 const calls=[];
 globalThis.Tools={SoftwareSettings:{async exec(args){
  calls.push(args);
  if(args[1]==='chat')return JSON.stringify([{id:'chat',title:'Game',workspaceId:bound?'workspace-id':null,workspacePrimaryPath:bound?path:null}]);
  assert.deepEqual(args,['--json','workspace','exec',path,'git','rev-parse','--show-toplevel','--show-prefix']);
  return JSON.stringify({exitCode:0,stdout:native+'\n'+prefix+'\n'});
 }}};
 assert.deepEqual(await workspaceChats(),[{id:'chat',title:'Game',workspaceId:'workspace-id',workspacePath:path}]);
 assert.equal(await chatWorkspace('chat'),native);
 path='/mnt/local/folders/other';native='/Users/demo/other';
 assert.equal(await chatWorkspace('chat'),native,'a later call follows the actual host rebind');
 prefix='subdirectory/';await assert.rejects(chatWorkspace('chat'),/根目录/);
 bound=false;await assert.rejects(chatWorkspace('chat'),/尚未绑定工作区/);
 assert.ok(calls.every(args=>args[2]==='list'||args[2]==='exec'),'no binding or workspace mutation is performed');
});

test('workspace resolution fails closed on invalid host data and inaccessible repositories',async()=>{
 globalThis.Tools={SoftwareSettings:{async exec(){return '{}'}}};
 await assert.rejects(workspaceChats(),/无效/);
 globalThis.Tools={SoftwareSettings:{async exec(args){return JSON.stringify(args[1]==='chat'?[{id:'c',title:'x',workspaceId:'w',workspacePrimaryPath:'/mount'}]:{exitCode:128,stdout:''})}}};
 await assert.rejects(chatWorkspace('c'),/Git 仓库/);
});


test('empty workspaces explain Git initialization and inaccessible directories retain a bounded cause',async()=>{
 let stderr='fatal: not a git repository (or any of the parent directories): .git';
 globalThis.Tools={SoftwareSettings:{async exec(args){return JSON.stringify(args[1]==='chat'?[{id:'c',title:'Test',workspaceId:'w',workspacePrimaryPath:'/workspace/game'}]:{exitCode:128,stdout:'',stderr})}}};
 await assert.rejects(chatWorkspace('c'),error=>/尚未初始化/.test(error.message)&&/首次提交/.test(error.message)&&/\/workspace\/game/.test(error.message));
 stderr='Permission denied';
 await assert.rejects(chatWorkspace('c'),/访问权限.*Git.*\nPermission denied/);
 stderr='x'.repeat(5000);
 await assert.rejects(chatWorkspace('c'),error=>error.message.length<1000);
});
