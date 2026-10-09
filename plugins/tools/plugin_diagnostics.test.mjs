import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFile} from 'node:fs/promises';

async function plugin(name, toolCall = () => {throw Error('unexpected tool call');}) {
  const exports = {};
  const source = await readFile(new URL(`../packages/buildin/${name}/dist/main.js`, import.meta.url), 'utf8');
  const context = vm.createContext({exports, toolCall,
    require: () => ({}), ToolPkg: {ipc: {on: () => {}}}});
  vm.runInContext(source, context);
  return exports;
}
for(const name of ['daily_life','goal_mode','message_translation','plan_mode','thinking_guidance']) {
  test(`${name} connectivity runs its real compiled main export without invoking business tools`,async()=>{
    const exports = await plugin(name);
    assert.equal(exports.test_connection().passed,true);
    if(name!=='daily_life') {
      const result = await exports.test_tool_call();
      assert.equal(result.passed,false);
      assert(result.message.includes('没有业务工具'));
    }
  });
}
test('daily-life tool diagnostic uses the normal package tool dispatcher and validates its output',async()=>{
  const calls=[];
  const exports=await plugin('daily_life',async(name,args)=>{
    calls.push({name,args});return {iso:'2026-10-09T00:00:00.000Z',timestamp:1791504000000};
  });
  assert.equal((await exports.test_tool_call()).passed,true);
  assert.equal(calls.length,1);assert.equal(calls[0].name,'daily_life:get_current_date');
  assert.deepEqual(Object.keys(calls[0].args),[]);
  const invalid=await plugin('daily_life',async()=>({}));
  assert.equal((await invalid.test_tool_call()).passed,false);
  const rejected=await plugin('daily_life',async()=>{throw Error('permission denied');});
  await assert.rejects(()=>rejected.test_tool_call(),/permission denied/);
});
