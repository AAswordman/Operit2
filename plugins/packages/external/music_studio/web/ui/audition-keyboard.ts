import { noteName } from '../../src/shared/model';
export const COMPUTER_KEYS: Record<string,number> = { KeyA:0,KeyW:1,KeyS:2,KeyE:3,KeyD:4,KeyF:5,KeyT:6,KeyG:7,KeyY:8,KeyH:9,KeyU:10,KeyJ:11,KeyK:12,KeyO:13,KeyL:14,KeyP:15,Semicolon:16 };
export function auditionKeyboard(octave: number, velocity: number): string {
  const base=(octave+1)*12, whites=[0,2,4,5,7,9,11];
  let white=0;
  const keys=Array.from({ length:24 },(_,i)=> {
    const black=!whites.includes(i%12), pitch=base+i;
    const key=Object.entries(COMPUTER_KEYS).find(([,n])=>n===i)?.[0].replace('Key','').replace('Semicolon',';')??'';
    const position=black ? (white-.32)/14*100 : white++/14*100;
    return `<button class="preview-key ${black?'black':'white'}" style="left:${position}%;width:${(black?.64:1)/14*100}%" data-preview-pitch="${pitch}" aria-label="试听 ${noteName(pitch)}" aria-pressed="false"><span>${i%12===0?noteName(pitch):''}</span><kbd>${key}</kbd></button>`;
  }).join('');
  return `<section class="audition-panel" aria-label="乐器试听键盘"><div class="audition-toolbar"><span class="audition-title">◉ 试听</span><button data-action="preview-octave-down" aria-label="试听键盘降低八度" ${octave<=0?'disabled':''}>−</button><span class="octave-label mono">C${octave}–B${octave+1}</span><button data-action="preview-octave-up" aria-label="试听键盘升高八度" ${octave>=7?'disabled':''}>＋</button><label class="preview-velocity-label">力度<input id="preview-velocity" type="range" aria-label="试听力度" min=".1" max="1" step=".05" value="${velocity}"></label><span id="preview-status" role="status">按住发声 · A–; 可弹奏</span></div><div class="preview-keyboard" aria-label="两组八度试听钢琴键盘">${keys}</div></section>`;
}
interface AuditionCallbacks {
  enabled(): boolean; base(): number; velocity(): number;
  on(pitch: number,velocity: number): Promise<void>;
  off(pitch: number): void; clear(): void;
  status(text: string,error?: boolean): void;
}
/** Keyboard/pointer notes are ephemeral, independent of editor notes and project transactions. */
export class KeyboardAudition {
  private sources=new Map<string,number>();
  private pointers=new Set<number>();
  private generation=0;
  constructor(private root: HTMLElement,private callbacks: AuditionCallbacks) {
    root.addEventListener('pointerdown',this.down);
    window.addEventListener('pointermove',this.move);
    window.addEventListener('pointerup',this.up);
    window.addEventListener('pointercancel',this.up);
    window.addEventListener('keydown',this.keydown);
    window.addEventListener('keyup',this.keyup);
    window.addEventListener('blur',()=>this.stopAll());
  }
  get pitches(): number[] { return [...new Set(this.sources.values())]; }
  stopAll(): void { this.generation++; this.sources.clear(); this.pointers.clear(); this.callbacks.clear(); this.paint(); }
  private paint(): void {
    const active=new Set(this.sources.values());
    for(const key of this.root.querySelectorAll<HTMLElement>('[data-preview-pitch]')) {
      const pressed=active.has(Number(key.dataset.previewPitch)); key.classList.toggle('pressed',pressed); key.setAttribute('aria-pressed',String(pressed));
    }
  }
  private start(source: string,pitch: number): void {
    if(this.sources.get(source)===pitch)return;
    this.end(source); const already=this.pitches.includes(pitch); this.sources.set(source,pitch); this.paint();
    if(already)return;
    const generation=this.generation;
    this.callbacks.status('启动试听…');
    void this.callbacks.on(pitch,this.callbacks.velocity()).then(()=>{
      if(generation===this.generation)this.callbacks.status(this.sources.size?'试听中 · 松开释音':'按住发声 · A–; 可弹奏');
    }).catch(error=>{
      if(generation!==this.generation)return;
      for(const [key,value] of this.sources)if(value===pitch)this.sources.delete(key);
      this.paint(); this.callbacks.status(`音频未就绪：${String(error)}`,true);
    });
  }
  private end(source: string): void {
    const pitch=this.sources.get(source); if(pitch===undefined)return;
    this.sources.delete(source); if(!this.pitches.includes(pitch))this.callbacks.off(pitch);
    this.paint(); if(!this.sources.size)this.callbacks.status('按住发声 · A–; 可弹奏');
  }
  private down=(event: PointerEvent): void=>{
    if(!this.callbacks.enabled() || (event.pointerType==='mouse' && event.button!==0))return;
    const key=(event.target as Element).closest<HTMLElement>('[data-preview-pitch]'); if(!key)return;
    event.preventDefault();key.focus({ preventScroll:true });
    this.pointers.add(event.pointerId);
    key.closest('.preview-keyboard')?.setPointerCapture(event.pointerId);
    this.start(`pointer:${event.pointerId}`,Number(key.dataset.previewPitch));
  };
  private move=(event: PointerEvent): void=>{
    if(!this.pointers.has(event.pointerId))return;
    const key=document.elementFromPoint(event.clientX,event.clientY)?.closest<HTMLElement>('[data-preview-pitch]');
    if(key)this.start(`pointer:${event.pointerId}`,Number(key.dataset.previewPitch)); else this.end(`pointer:${event.pointerId}`);
  };
  private up=(event: PointerEvent): void=>{ this.pointers.delete(event.pointerId); this.end(`pointer:${event.pointerId}`); };
  private keydown=(event: KeyboardEvent): void=>{
    if(event.defaultPrevented||!this.callbacks.enabled()||event.ctrlKey||event.metaKey||event.altKey)return;
    const target=event.target as HTMLElement;
    if(['INPUT','SELECT','TEXTAREA'].includes(target.tagName) || target.isContentEditable)return;
    let pitch=COMPUTER_KEYS[event.code];
    if((event.code==='Space'||event.code==='Enter') && target.dataset.previewPitch) pitch=Number(target.dataset.previewPitch)-this.callbacks.base();
    else if(target.tagName==='BUTTON' && !target.dataset.previewPitch)return;
    if(pitch===undefined)return;
    event.preventDefault(); if(event.repeat)return;
    this.start(`key:${event.code}`,this.callbacks.base()+pitch);
  };
  private keyup=(event: KeyboardEvent): void=>{ this.end(`key:${event.code}`); };
}
