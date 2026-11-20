import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url),{connectorIssue}=require('../dist/connectors.js');

test('optional connector availability uses the loaded registry and exact enablement',async()=>{
 let plugins=[];const calls=[];
 globalThis.Tools={SoftwareSettings:{async exec(args){calls.push(args);return JSON.stringify(plugins);}}};
 assert.match(await connectorIssue(),/安装并启用/);
 plugins=[{name:'com.operit.connector_hub',enabled:false}];assert.match(await connectorIssue(),/请先启用/);
 plugins[0].enabled=true;assert.equal(await connectorIssue(),null);
 plugins=[{name:'com.operit.connector_hub_other',enabled:true}];assert.match(await connectorIssue(),/安装并启用/);
 for(const invalid of [null,{},[null],[{name:'com.operit.connector_hub'}],[{name:'com.operit.connector_hub',enabled:'true'}]]){
  plugins=invalid;await assert.rejects(connectorIssue(),/无效的插件列表/);
 }
 assert.ok(calls.every(args=>JSON.stringify(args)==='["--json","plugin","list"]'));
});
