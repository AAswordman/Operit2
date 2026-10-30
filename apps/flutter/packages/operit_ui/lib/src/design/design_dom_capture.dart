import 'design_dom_runtime.dart';

/// DOM collector executed only on an explicit design capture request. It does
/// not fetch URLs or call plugin actions. Keep synchronous for WKWebView.
const operitDesignDomCaptureScript =
    operitDesignDomRuntimeBootstrap +
    r'''
(function collectOperitDom(view = window, frameDepth = 0) {
  const runtime=window.__operitDesignDomRuntime;
  if(frameDepth===0)runtime.begin();
  const document = view.document, innerWidth = view.innerWidth, innerHeight = view.innerHeight;
  const location = view.location, getComputedStyle = view.getComputedStyle.bind(view);
  const assets = [], warnings = new Set();
  let next = 0;
  const viewport = {left:0,top:0,right:innerWidth,bottom:innerHeight};
  const color = value => {
    if (!value || value === 'transparent') return '#00000000';
    const canvas = document.createElement('canvas'); canvas.width=canvas.height=1;
    const ctx=canvas.getContext('2d'); ctx.clearRect(0,0,1,1); ctx.fillStyle=value;ctx.fillRect(0,0,1,1);
    return '#'+Array.from(ctx.getImageData(0,0,1,1).data).map(n=>n.toString(16).padStart(2,'0')).join('').toUpperCase();
  };
  const number = value => Number.parseFloat(value)||0;
  const visible = r => r.width>0 && r.height>0 && r.right>0 && r.bottom>0 && r.left<innerWidth && r.top<innerHeight;
  const metadata = (el, textNode=null) => ({...runtime.describe(textNode||el),kind:'plugin-webview',tag:el.tagName.toLowerCase(),domId:el.id||null,
    selector:el.id ? '#'+CSS.escape(el.id) : el.tagName.toLowerCase()+(el.getAttribute('class')?'.'+Array.from(el.classList).map(CSS.escape).join('.'):''),
    designId:el.getAttribute('data-operit-design-id'),url:location.origin+location.pathname});
  function node(el, origin, depth) {
    if (++next>8000 || depth>70) {warnings.add('网页超过 8000 图层或 70 层深度；请分区采集。');return null;}
    if (['SCRIPT','STYLE','LINK','META','NOSCRIPT'].includes(el.tagName)) return null;
    const cs=getComputedStyle(el),r=el.getBoundingClientRect();
    if(cs.display==='none'||cs.visibility==='hidden'||number(cs.opacity)===0||!visible(r))return null;
    const result={id:'dom-'+next,name:el.getAttribute('aria-label')||el.id||el.tagName.toLowerCase(),type:'frame',
      x:r.left-origin.left,y:r.top-origin.top,width:r.width,height:r.height,
      fill:color(cs.backgroundColor),opacity:number(cs.opacity),radius:number(cs.borderTopLeftRadius),
      source:metadata(el),clipsContent:['hidden','clip','scroll','auto'].includes(cs.overflow),children:[]};
    if(number(cs.borderTopWidth)>0){result.stroke=color(cs.borderTopColor);result.strokeWidth=number(cs.borderTopWidth);}
    if(cs.backgroundImage!=='none')warnings.add('CSS 渐变或背景图片保留容器，需参照实际网页；没有请求远程资源。');
    if(cs.transform!=='none')warnings.add('CSS 变换按可见包围框导出。');
    if(['IMG','CANVAS','VIDEO'].includes(el.tagName)){
      try{
        const canvas=document.createElement('canvas');canvas.width=Math.min(4096,Math.ceil(r.width));canvas.height=Math.min(4096,Math.ceil(r.height));
        canvas.getContext('2d').drawImage(el,0,0,canvas.width,canvas.height);
        const data=canvas.toDataURL('image/png').split(',')[1],path='assets/images/dom_'+next+'.png';
        assets.push({path,mediaType:'image/png',base64:data});result.type='image';result.asset=path;delete result.children;
        if(el.tagName!=='IMG')warnings.add('Canvas／视频当前帧作为独立图片，可替换素材，内部绘制不提供文字图层。');
      }catch{warnings.add('跨域或尚未加载的图片／媒体无法读取像素；保留对应位置和来源。');}
      return result;
    }
    if(el.tagName.toLowerCase()==='svg'){
      const svg=el.cloneNode(true),originals=[el,...el.querySelectorAll('*')],clones=[svg,...svg.querySelectorAll('*')];
      clones.forEach((copy,i)=>{
        const style=getComputedStyle(originals[i]);
        ['fill','stroke','stroke-width','opacity','font-family','font-size'].forEach(k=>copy.setAttribute(k,style.getPropertyValue(k)));
        for(const attr of Array.from(copy.attributes))if(/^on/i.test(attr.name)||/href$/i.test(attr.name))copy.removeAttribute(attr.name);
      });
      svg.querySelectorAll('script,foreignObject,image,use,style').forEach(n=>n.remove());
      result.type='vector';result.svg=new XMLSerializer().serializeToString(svg);delete result.children;return result;
    }
    if(el.tagName==='IFRAME'){
      try{
        const doc=el.contentDocument;
        if(!doc||!doc.body)throw new Error();
        if(frameDepth>=5){warnings.add('iframe 嵌套超过 5 层，请分区采集。');return result;}
        const nested=JSON.parse(collectOperitDom(el.contentWindow,frameDepth+1)),prefix='iframe-'+next+'-';
        const paths=new Map();
        for(const asset of nested.assets){const path=asset.path.replace('assets/images/','assets/images/'+prefix);paths.set(asset.path,path);assets.push({...asset,path});}
        function remap(n){n.id=prefix+n.id;if(n.asset)n.asset=paths.get(n.asset)||n.asset;(n.children||[]).forEach(remap);}
        remap(nested.scene);nested.scene.x=number(cs.borderLeftWidth);nested.scene.y=number(cs.borderTopWidth);
        result.children.push(nested.scene);nested.warnings.forEach(w=>warnings.add(w));
      }catch{warnings.add('跨域 iframe 无法读取；保留独立区域和来源。');}
      return result;
    }
    const addText=(text,rect,suffix,textNode=null)=>{
      if(!text||!visible(rect))return;
      result.children.push({id:'dom-'+(++next),name:suffix,type:'text',x:rect.left-r.left,y:rect.top-r.top,width:rect.width,height:rect.height,
        text,fontSize:number(cs.fontSize)||14,fontFamily:cs.fontFamily.split(',')[0].replace(/["']/g,''),fontWeight:number(cs.fontWeight)||400,
        italic:cs.fontStyle==='italic',fill:color(cs.color),lineHeight:number(cs.lineHeight)||undefined,
        textAlign:cs.textAlign==='center'?'CENTER':cs.textAlign==='right'?'RIGHT':'LEFT',source:metadata(el,textNode)});
    };
    if(['INPUT','TEXTAREA'].includes(el.tagName)){
      addText(el.type==='password'?'••••••':el.value||el.placeholder,r,'输入文字');return result;
    }
    const children=el.shadowRoot?el.shadowRoot.childNodes:el.childNodes;
    for(const child of children){
      if(child.nodeType===Node.TEXT_NODE&&child.textContent.trim()){
        const range=document.createRange();range.selectNodeContents(child);addText(child.textContent,range.getBoundingClientRect(),'文字',child);
      }else if(child.nodeType===Node.ELEMENT_NODE){const exported=node(child,r,depth+1);if(exported)result.children.push(exported);}
    }
    for(const pseudo of ['::before','::after'])if(!['none','normal','""'].includes(getComputedStyle(el,pseudo).content))warnings.add('CSS 伪元素未生成独立图层，请参照网页。');
    return result;
  }
  const scene={id:'dom-root',name:document.title||'Plugin WebView',type:'frame',x:0,y:0,width:innerWidth,height:innerHeight,clipsContent:true,children:[]};
  const body=node(document.documentElement,viewport,0);if(body)scene.children.push(body);
  if(frameDepth===0)runtime.end();
  return JSON.stringify({scene,assets,warnings:[...warnings]});
})()
''';
