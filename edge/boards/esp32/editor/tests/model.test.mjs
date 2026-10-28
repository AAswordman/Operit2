import test from 'node:test';
import assert from 'node:assert/strict';
import {point,pixel} from '../web/model.ts';
test('scaled pointer uses the firmware logical coordinates',()=>{
  for(const zoom of [1,1.5,2,3]){
    const rect={left:43,top:71,width:320*zoom,height:240*zoom};
    assert.deepEqual(point(43+264*zoom,71+72*zoom,rect),{x:264,y:72});
    assert.deepEqual(point(-100,-100,rect),{x:0,y:0});
  }
});
test('RGB formats preserve endpoints and reduce channels',()=>{for(const mode of ['rgb565','rgb332']){assert.deepEqual(pixel(0,0,0,mode),[0,0,0]);assert.deepEqual(pixel(255,255,255,mode),[255,255,255]);}assert.equal(new Set(Array.from({length:256},(_,r)=>pixel(r,0,0,'rgb565')[0])).size,32);assert.equal(new Set(Array.from({length:256},(_,r)=>pixel(r,0,0,'rgb332')[0])).size,8);});
