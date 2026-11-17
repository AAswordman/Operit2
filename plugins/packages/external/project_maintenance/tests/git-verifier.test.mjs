import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdtemp,writeFile,rm,mkdir,readFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {exec,execFileSync} from 'node:child_process';
import {promisify} from 'node:util';
const {GitVerifier} = createRequire(import.meta.url)('../dist/git-verifier.js');
const shell = promisify(exec);
const run = async command => {try {const r=await shell(command,{timeout:10000});return {output:r.stdout,exitCode:0,timedOut:false};} catch(e) {return {output:e.stdout ?? '',exitCode:e.code ?? 1,timedOut:!!e.killed};}};
const git=(cwd,...args)=>execFileSync('git',['-C',cwd,...args],{encoding:'utf8'}).trim();
test('verifies the delivered tree equals the reviewed worktree, including new files', async t=>{
 const dir=await mkdtemp(join(tmpdir(),"maint-'quoted-$-")); t.after(()=>rm(dir,{recursive:true,force:true}));
 git(dir,'init','-q'); git(dir,'config','user.name','Test'); git(dir,'config','user.email','test@example.invalid');
 await writeFile(join(dir,'game.txt'),'initial'); git(dir,'add','game.txt'); git(dir,'commit','-qm','initial');
 const verifier=new GitVerifier(run), before=await verifier.head(dir);
 await writeFile(join(dir,'game.txt'),'fixed'); await writeFile(join(dir,'new.txt'),'new');
 const reviewed=await verifier.snapshot(dir);
 assert.equal(reviewed.head,before); assert.equal(git(dir,'status','--porcelain').includes('?? new.txt'),true);
 await assert.rejects(verifier.verify(dir,reviewed),/did not change/);
 git(dir,'add','game.txt','new.txt'); git(dir,'commit','-qm','fix');
 const evidence=await verifier.verify(dir,reviewed);
 assert.equal(evidence.head,git(dir,'rev-parse','HEAD')); assert.equal(evidence.parent,before); assert.equal(evidence.reviewedTree,reviewed.tree);
 assert.equal(evidence.trackedClean,true); assert.equal(evidence.untrackedClean,true);
 await writeFile(join(dir,'game.txt'),'dirty'); await assert.rejects(verifier.verify(dir,reviewed),/changes remain/);
 git(dir,'checkout','--','game.txt'); await writeFile(join(dir,'leftover.txt'),'not delivered');
 await assert.rejects(verifier.verify(dir,reviewed),/changes remain/);
 await rm(join(dir,'leftover.txt')); await writeFile(join(dir,'game.txt'),'another'); git(dir,'commit','-qam','another');
 await assert.rejects(verifier.verify(dir,reviewed),/direct single-parent/);
});

test('rejects a commit altered after the independent review', async t=>{
 const dir=await mkdtemp(join(tmpdir(),'maint-review-')); t.after(()=>rm(dir,{recursive:true,force:true}));
 git(dir,'init','-q'); git(dir,'config','user.name','Test'); git(dir,'config','user.email','test@example.invalid');
 await writeFile(join(dir,'game.txt'),'initial'); git(dir,'add','game.txt'); git(dir,'commit','-qm','initial');
 await writeFile(join(dir,'game.txt'),'reviewed'); const verifier=new GitVerifier(run), reviewed=await verifier.snapshot(dir);
 await writeFile(join(dir,'game.txt'),'changed after review'); git(dir,'commit','-qam','unexpected');
 await assert.rejects(verifier.verify(dir,reviewed),/does not match/);
});
test('failed command and invalid paths do not become evidence',async()=>{
 const verifier=new GitVerifier(async()=>({output:'a'.repeat(40),exitCode:1,timedOut:false}));
 await assert.rejects(verifier.head('/safe'),/failed/);
 await assert.rejects(verifier.head('/unsafe\npath'),/absolute POSIX/);
});
test('serializes shared hidden executor commands and requires bounded output',async()=>{
 let active=0,max=0;
 const verifier=new GitVerifier(async()=>{
  active++; max=Math.max(max,active); await new Promise(r=>setTimeout(r,5)); active--;
  return {output:'shell prompt\nOPERIT_MAINTENANCE_GIT_BEGIN\n'+ 'a'.repeat(40)+'\n\nOPERIT_MAINTENANCE_GIT_END\nshell prompt',exitCode:0,timedOut:false};
 });
 assert.deepEqual(await Promise.all([verifier.head('/one'),verifier.head('/two')]),['a'.repeat(40),'a'.repeat(40)]); assert.equal(max,1);
 const malformed=new GitVerifier(async()=>({output:'a'.repeat(40),exitCode:0,timedOut:false}));
 await assert.rejects(malformed.head('/one'),/markers are missing/);
});


test('preflight requires a clean repository root and preserves existing work',async t=>{
 const dir=await mkdtemp(join(tmpdir(),'maint-baseline-'));t.after(()=>rm(dir,{recursive:true,force:true}));
 git(dir,'init','-q');git(dir,'config','user.name','Test');git(dir,'config','user.email','test@example.invalid');
 await writeFile(join(dir,'game.txt'),'initial');git(dir,'add','game.txt');git(dir,'commit','-qm','initial');
 const verifier=new GitVerifier(run);assert.equal(await verifier.cleanHead(dir),git(dir,'rev-parse','HEAD'));
 await writeFile(join(dir,'new.txt'),'user work');await assert.rejects(verifier.cleanHead(dir),/未提交改动/);
 assert.equal(await readFile(join(dir,'new.txt'),'utf8'), 'user work');
 git(dir,'add','new.txt');await assert.rejects(verifier.cleanHead(dir),/未提交改动/);
 git(dir,'reset','-q');await rm(join(dir,'new.txt'));await writeFile(join(dir,'game.txt'),'modified');
 await assert.rejects(verifier.cleanHead(dir),/未提交改动/);
 git(dir,'checkout','--','game.txt');await mkdir(join(dir,'child'));
 await assert.rejects(verifier.cleanHead(join(dir,'child')),/Git 仓库根目录/);
});

test('reported GitHub PR must match origin, be open and point at the verified commit',async t=>{
 const dir=await mkdtemp(join(tmpdir(),'maintenance-pr-'));t.after(()=>rm(dir,{recursive:true,force:true}));
 git(dir,'init','-q');git(dir,'config','user.name','Test');git(dir,'config','user.email','test@example.invalid');
 await writeFile(join(dir,'game.txt'),'game');git(dir,'add','game.txt');git(dir,'commit','-qm','game');
 git(dir,'remote','add','origin','https://github.com/example/game.git');
 const head=git(dir,'rev-parse','HEAD'),url='https://github.com/example/game/pull/7';
 const bin=join(dir,'fake-bin');await mkdir(bin);const gh=join(bin,'gh');
 await writeFile(gh,'#!/bin/sh\nprintf "%s\\n" "$FAKE_PR_RESPONSE"\n',{mode:0o755});
 let response={url,number:7,state:'OPEN',headRefOid:head};
 const invoke=async command=>{try{const result=await shell(command,{env:{...process.env,PATH:bin+':'+process.env.PATH,FAKE_PR_RESPONSE:JSON.stringify(response)},timeout:10000});return {output:result.stdout,exitCode:0,timedOut:false};}catch(e){return {output:e.stdout??'',exitCode:e.code??1,timedOut:!!e.killed};}};
 const verifier=new GitVerifier(invoke);
 assert.deepEqual(await verifier.verifyPullRequest(dir,url,head),{url,number:7,head,state:'OPEN'});
 response={...response,headRefOid:'b'.repeat(40)};await assert.rejects(verifier.verifyPullRequest(dir,url,head),/verified commit/);
 response={...response,headRefOid:head,state:'CLOSED'};await assert.rejects(verifier.verifyPullRequest(dir,url,head),/not open/);
 response={...response,state:'OPEN',url:'https://github.com/other/game/pull/7'};await assert.rejects(verifier.verifyPullRequest(dir,url,head),/not open/);
 await assert.rejects(verifier.verifyPullRequest(dir,'https://github.com/other/game/pull/7',head),/does not match/);
 await assert.rejects(verifier.verifyPullRequest(dir,url+'?x=$(touch /tmp/should-not-execute)',head),/invalid/);
});
