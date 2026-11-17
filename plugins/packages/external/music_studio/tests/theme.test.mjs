import test from 'node:test';
import assert from 'node:assert/strict';
import { load } from './helpers.mjs';
const { default: screen } = await load('src/ui/screen.ts');

test('native screen bootstraps, publishes and disposes host theme without reloading its WebView', async () => {
  let theme={brightness:'dark'}, listener, bridge, cleanup, unsubscribed=false, removed=false;
  const scripts=[], state=new Map(), refs=new Map();
  const controller={
    addJavascriptInterface(name,value){assert.equal(name,'MusicHost');bridge=value;},
    removeJavascriptInterface(name){assert.equal(name,'MusicHost');removed=true;},
    async evaluateJavascript(script){scripts.push(script);},
  };
  const ctx={
    createWebViewController(){return controller;},
    useRef(key,initial){if(!refs.has(key))refs.set(key,{current:initial});return refs.get(key);},
    useEffect(_key,setup){if(!cleanup)cleanup=setup();},
    useState(key,initial){if(!state.has(key))state.set(key,initial);return [state.get(key),value=>state.set(key,value)];},
    Theme:{getCurrent:()=>theme,subscribe(fn){listener=fn;return ()=>{unsubscribed=true;};}},
    UI:{Box:(props,child)=>({props,child}),WebView:props=>({props}),Text:props=>({props})},
  };
  globalThis.ToolPkg={readResource:async()=>'/bundled/studio.html'};
  try {
    await screen(ctx).props.onLoad();
    await listener(theme);assert.equal(scripts.length,0,'no publishing before browser bootstrap');
    assert.deepEqual(bridge.getTheme(),{brightness:'dark'});
    theme={brightness:'light'};await listener(theme);
    assert.equal(scripts.length,1);assert.match(scripts[0],/"brightness":"light"/);
    const view=screen(ctx).child;
    assert.equal(view.props.url,'https://music.operit.local/');
    view.props.onPageStarted();await listener({brightness:'dark'});
    assert.equal(scripts.length,1,'wait for fresh bootstrap after navigation');
    assert.deepEqual(bridge.getTheme(),{brightness:'light'});
    cleanup();assert.ok(unsubscribed&&removed);
  } finally { delete globalThis.ToolPkg; }
});
