import 'dart:convert';

/// Installed only inside an explicitly enabled design surface. Targets refer to
/// real DOM nodes; clicks, input events and edits execute in the plugin page.
const operitDesignDomRuntimeBootstrap = r'''
(() => {
  if (window.__operitDesignDomRuntime) return;
  const ids = new WeakMap(), targets = new Map(), edits = new Map();
  let next = 0, seen = new Set();
  const observed = new WeakSet();
  function observe(root) { if(!observed.has(root)){observer.observe(root,{subtree:true,childList:true,characterData:true,attributes:true,attributeFilter:['style']});observed.add(root);} }
  const parent = n => n.nodeType === 1 ? n : n.parentElement;
  const input = el => el && ['INPUT','TEXTAREA'].includes(el.tagName) &&
    !el.disabled && !el.readOnly && !['button','submit','reset','checkbox','radio','file','hidden'].includes(el.type);
  const allowed = n => n.nodeType === 3 ? ['text'] : [
    'fill','radius','width','height','opacity','paddingLeft','paddingRight','paddingTop','paddingBottom',
    'fontSize', ...(n.childNodes.length === 1 && n.firstChild.nodeType === 3 ? ['text'] : []),
  ];
  function describe(n) {
    observe(n.getRootNode());
    if (!ids.has(n)) ids.set(n, 'dom-live-'+next++);
    const id = ids.get(n); targets.set(id, n); seen.add(id);
    const el = parent(n), explicit = el?.getAttribute('data-operit-design-id');
    return {domRuntimeTarget:id, domRuntimeProperties:allowed(n),
      domRuntimeKind:n.nodeType===3?'text':'element',domRuntimeTextIndex:n.nodeType===3?Array.from(el.childNodes).indexOf(n):null,
      domRuntimeInput:input(el), domRuntimeObscure:el?.type==='password',
      domRuntimePersistence:explicit ? 'source' : 'element'};
  }
  function source(n) {
    const el=parent(n), explicit=el?.getAttribute('data-operit-design-id');
    return explicit ? {root:el.getRootNode(), explicit, textIndex:n.nodeType===3?Array.from(el.childNodes).indexOf(n):null} : null;
  }
  function resolve(record) {
    if (record.node.isConnected) return record.node;
    const s=record.source;
    if (!s) return null;
    const matches=s.root.querySelectorAll('[data-operit-design-id="'+CSS.escape(s.explicit)+'"]');
    if (matches.length!==1) return null;
    return s.textIndex===null?matches[0]:matches[0].childNodes[s.textIndex];
  }
  function styleKey(n, key) {
    if (key==='text') return null;
    if (key==='fill') return 'background-color';
    if (key==='radius') return 'border-radius';
    return key.replace(/[A-Z]/g,c=>'-'+c.toLowerCase());
  }
  function read(n,key) {
    if (key==='text') return n.nodeType===3?n.data:n.firstChild.data;
    return [parent(n).style.getPropertyValue(styleKey(n,key)),parent(n).style.getPropertyPriority(styleKey(n,key))];
  }
  const equal = (a,b) => JSON.stringify(a)===JSON.stringify(b);
  function write(n,key,value) {
    if (key==='text') { if (n.nodeType===3) n.data=value; else n.firstChild.data=value; }
    else {
      const style=parent(n).style, css=styleKey(n,key);
      if (value[0]) style.setProperty(css,value[0],value[1]); else style.removeProperty(css);
    }
  }
  function reapply() {
    for (const [key,record] of edits) {
      const node=resolve(record);
      if (!node) { if (!record.source) edits.delete(key); continue; }
      const remounted=node!==record.node; record.node=node;
      if(remounted){ids.set(node,key);targets.set(key,node);}
      for (const [property,cell] of record.cells) {
        const current=read(node,property);
        if (remounted || !equal(current,cell.applied)) cell.base=current;
        if (!equal(current,cell.applied)) write(node,property,cell.applied);
      }
    }
  }
  function restore(record) {
    const node=resolve(record); if (!node) return;
    for (const [key,cell] of record.cells) if (equal(read(node,key),cell.applied)) write(node,key,cell.base);
  }
  function edit(n,properties) {
    const keys=Object.keys(properties);
    if (!keys.length || keys.some(k=>!allowed(n).includes(k))) throw new Error('Unsupported DOM design property');
    const parsed=[];
    for (const key of keys) {
      const value=properties[key]; let applied;
      if (key==='text') {
        if (typeof value!=='string' || value.length>65536) throw new Error('Invalid design text');
        applied=value;
      } else if (key==='fill') {
        if (typeof value!=='string' || !/^#[0-9a-f]{8}$/i.test(value)) throw new Error('fill must be #RRGGBBAA');
        applied=[value,'important'];
      } else {
        if (typeof value!=='number' || !Number.isFinite(value) || value<0 || value>(key==='opacity'?1:10000)) throw new Error('Invalid '+key);
        applied=[String(value)+(key==='opacity'?'':'px'),'important'];
      }
      // CSSOM normalizes colors. Store its normalized form for idempotence.
      if (key!=='text') {
        const scratch=n.ownerDocument.createElement('div').style;
        scratch.setProperty(styleKey(n,key),applied[0],applied[1]);
        applied=[scratch.getPropertyValue(styleKey(n,key)),scratch.getPropertyPriority(styleKey(n,key))];
      }
      parsed.push([key,applied]);
    }
    const id=describe(n).domRuntimeTarget;
    const record=edits.get(id)||{node:n,source:source(n),cells:new Map()};
    for (const [key,applied] of parsed) {
      const previous=record.cells.get(key);
      record.cells.set(key,{base:previous?.base??read(n,key),applied});
      write(n,key,applied);
    }
    edits.set(id,record);
  }
  function request(message) {
    if (message.operation==='clear' && !message.target) {
      for (const record of edits.values()) restore(record); edits.clear(); return {ok:true};
    }
    const n=targets.get(message.target);
    if (!n || !n.isConnected) throw new Error('Stale DOM runtime target');
    const el=parent(n), view=el.ownerDocument.defaultView;
    if (message.operation==='edit') edit(n,message.properties);
    else if (message.operation==='clear') { const record=edits.get(message.target); if(record)restore(record);edits.delete(message.target); }
    else if (message.operation==='event') {
      const event=message.event;
      if (event.type==='tap') el.click();
      else if (event.type==='input') {
        if (!input(el) || typeof event.text!=='string' || event.text.length>65536) throw new Error('Target is not a writable input');
        el.focus();
        const prototype=el.tagName==='TEXTAREA'?view.HTMLTextAreaElement.prototype:view.HTMLInputElement.prototype;
        Object.getOwnPropertyDescriptor(prototype,'value').set.call(el,el.maxLength>=0?event.text.slice(0,el.maxLength):event.text);
        el.dispatchEvent(new view.Event('input',{bubbles:true,composed:true}));
        el.dispatchEvent(new view.Event('change',{bubbles:true,composed:true}));
      } else if (event.type==='scroll') {
        let scroll=el;
        while (scroll.parentElement && scroll.scrollHeight<=scroll.clientHeight && scroll.scrollWidth<=scroll.clientWidth) scroll=scroll.parentElement;
        const dx=event.dx??0,dy=event.dy??0;
        if(!Number.isFinite(dx)||!Number.isFinite(dy))throw new Error('Invalid scroll delta');
        scroll.scrollBy(dx,dy);
      } else throw new Error('Unsupported DOM event');
    } else throw new Error('Unsupported DOM runtime operation');
    return {ok:true};
  }
  const observer=new MutationObserver(reapply);
  observe(document);
  window.__operitDesignDomRuntime={describe,request,reapply,
    begin(){seen=new Set();reapply();},end(){for(const id of targets.keys())if(!seen.has(id))targets.delete(id);}};
})();
''';

String operitDesignDomRuntimeRequestScript(Map<String, Object?> request) =>
    '$operitDesignDomRuntimeBootstrap\nJSON.stringify(window.__operitDesignDomRuntime.request(${jsonEncode(request)}))';
