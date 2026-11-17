import test from 'node:test';
import assert from 'node:assert/strict';
import { load } from './helpers.mjs';
const { clampPiano, transformPiano, panPiano, transformTimeline, wheelPixels } = await load('web/ui/navigation.ts');
const near = (a,b) => assert.ok(Math.abs(a-b)<1e-8, `${a} ≠ ${b}`);
const bounds = { start: 64, end: 96 }, view = { start: 68, span: 16, top: 76, rows: 24 };
test('piano zoom preserves mouse time/pitch anchors', () => {
  const anchor = { x: .6, y: .3 }, next = transformPiano(view,bounds,2,anchor,anchor);
  near(view.start+view.span*anchor.x,next.start+next.span*anchor.x);
  near(view.top-view.rows*anchor.y,next.top-next.rows*anchor.y);
  assert.equal(next.span,8); assert.equal(next.rows,12);
});
test('pinch translates both anchors as well as changing scale', () => {
  const before={ x:.4,y:.5 }, after={ x:.3,y:.7 }, next=transformPiano(view,bounds,1.5,before,after);
  near(view.start+view.span*before.x,next.start+next.span*after.x);
  near(view.top-view.rows*before.y,next.top-next.rows*after.y);
});
test('time-only and pitch-only wheel axes are independent', () => {
  const a={ x:.5,y:.5 };
  const time=transformPiano(view,bounds,2,a,a,'time'), pitch=transformPiano(view,bounds,2,a,a,'pitch');
  near(time.top,view.top); near(time.rows,view.rows); near(pitch.start,view.start); near(pitch.span,view.span);
});
test('pan directions match scrolling: right advances time; down reveals lower notes', () => {
  const next=panPiano(view,bounds,100,50,800,600);
  near(next.start,70); near(next.top,74); assert.equal(next.span,16); assert.equal(next.rows,24);
});
test('continuous viewport clamps at phrase edges, MIDI bounds and zoom limits', () => {
  assert.deepEqual(clampPiano({ start:-1,span:1000,top:1000,rows:1000 },bounds), { start:64,span:32,top:128,rows:128 });
  assert.deepEqual(clampPiano({ start:1000,span:.001,top:-10,rows:1 },bounds), { start:95.75,span:.25,top:4,rows:4 });
  assert.deepEqual(clampPiano(view,{ start:64,end:64.1 }), { ...view,start:64,span:64.1-64 });
});
test('timeline keeps stationary and moving anchor time stable', () => {
  for (const after of [300,250]) {
    const next=transformTimeline(1000,200,500,8000,2,300,after);
    near((200+300)/1000,(next.left+after)/next.width);
  }
});
test('timeline fit/max limits do not produce invalid scrolling', () => {
  assert.deepEqual(transformTimeline(1000,200,500,8000,.001,300),{ width:500,left:0 });
  const next=transformTimeline(1000,900,500,8000,100,800);
  assert.equal(next.width,8000); assert.equal(next.left,7500);
});
test('wheel normalization supports pixel, line and page modes', () => {
  assert.equal(wheelPixels(3,0,800),3); assert.equal(wheelPixels(3,1,800),48); assert.equal(wheelPixels(3,2,800),2400);
});
