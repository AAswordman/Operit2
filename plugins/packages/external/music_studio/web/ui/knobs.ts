export interface KnobRange { min: number; max: number; step: number; logarithmic?: boolean }
export function knobPosition(value: number,range: KnobRange): number {
  const {min,max}=range;
  return Math.max(0,Math.min(1,range.logarithmic&&min>0?Math.log(value/min)/Math.log(max/min):(value-min)/(max-min)));
}
export function knobValue(position: number,range: KnobRange): number {
  const p=Math.max(0,Math.min(1,position));if(p===0)return range.min;if(p===1)return range.max;
  const value=range.logarithmic&&range.min>0?range.min*(range.max/range.min)**p:range.min+(range.max-range.min)*p;
  return Math.max(range.min,Math.min(range.max,Number((Math.round((value-range.min)/range.step)*range.step+range.min).toFixed(6))));
}
function range(input: HTMLInputElement): KnobRange { return {min:Number(input.min),max:Number(input.max),step:Number(input.step),logarithmic:input.dataset.knobScale==='log'}; }
export function paintKnob(input: HTMLInputElement): void {
  const position=knobPosition(Number(input.value),range(input));
  input.closest<HTMLElement>('.knob-control')?.style.setProperty('--knob-angle',`${-135+270*position}deg`);
}
/** Uses a native range for keyboard/screen-reader semantics, and vertical/fine pointer dragging. */
export class KnobControls {
  private drag: { input:HTMLInputElement;pointerId:number;x:number;y:number;position:number;changed:boolean } | null=null;
  constructor(private root: HTMLElement) {
    root.addEventListener('pointerdown',this.down);
    root.addEventListener('input',e=>{const input=e.target as HTMLInputElement;if(input.classList.contains('knob-input'))paintKnob(input);});
    window.addEventListener('pointermove',this.move,{passive:false});
    window.addEventListener('pointerup',this.up);
    window.addEventListener('pointercancel',this.up);
    window.addEventListener('blur',()=>this.finish());
  }
  cancel(): void { if(this.drag){try{this.drag.input.releasePointerCapture(this.drag.pointerId);}catch{}this.drag=null;} }
  refresh(): void {for(const input of this.root.querySelectorAll<HTMLInputElement>('.knob-input'))paintKnob(input);}
  private down=(event: PointerEvent): void=>{
    if(this.drag||event.button!==0)return;
    const input=(event.target as Element).closest<HTMLInputElement>('.knob-input');if(!input)return;
    event.preventDefault();input.focus({preventScroll:true});input.setPointerCapture(event.pointerId);
    this.drag={input,pointerId:event.pointerId,x:event.clientX,y:event.clientY,position:knobPosition(Number(input.value),range(input)),changed:false};
  };
  private move=(event: PointerEvent): void=>{
    const drag=this.drag;if(!drag||drag.pointerId!==event.pointerId)return;
    if(!drag.input.isConnected){this.cancel();return;}
    event.preventDefault();
    const movement=drag.y-event.clientY+(event.clientX-drag.x)*.25;
    drag.position=Math.max(0,Math.min(1,drag.position+movement/(event.shiftKey?1400:140)));
    drag.x=event.clientX;drag.y=event.clientY;
    const value=String(knobValue(drag.position,range(drag.input)));
    if(drag.input.value===value)return;
    drag.input.value=value;drag.changed=true;paintKnob(drag.input);
    drag.input.dispatchEvent(new Event('input',{bubbles:true}));
  };
  private finish(): void {
    const drag=this.drag;if(!drag)return;
    this.cancel();if(drag.changed&&drag.input.isConnected)drag.input.dispatchEvent(new Event('change',{bubbles:true}));
  }
  private up=(event: PointerEvent): void=>{if(this.drag?.pointerId===event.pointerId)this.finish();};
}
